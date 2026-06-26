use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;

use crate::models::{
    RepoContextPack, RepoFileRole, RepoFileSignal, RepoGraphEdge, RepoGraphEdgeKind, RepoGraphNode,
    RepoGraphSummary,
    RepoIntelligenceSummary,
};
use crate::storage::{app_data_dir, config_file, ensure_data_dirs};

const MAX_SCAN_FILES: usize = 2_500;
const MAX_INDEXED_FILE_BYTES: u64 = 1_000_000;
const MAX_PACK_FILES: usize = 40;
const IGNORED_DIRS: [&str; 12] = [
    ".git",
    "node_modules",
    "dist",
    "build",
    "coverage",
    "target",
    ".next",
    ".turbo",
    "vendor",
    ".venv",
    "__pycache__",
    ".pytest_cache",
];
const SECRET_FILE_NAMES: [&str; 7] = [
    ".env",
    ".env.local",
    ".env.production",
    ".npmrc",
    ".pypirc",
    "id_rsa",
    "id_ed25519",
];
const SECRET_EXTENSIONS: [&str; 6] = [".pem", ".p8", ".p12", ".key", ".crt", ".cer"];
const SECRET_PATH_SEGMENTS: [&str; 4] = ["secrets", ".secrets", "private_keys", ".private_keys"];

pub fn summarize_repo(path: impl AsRef<Path>) -> Result<RepoIntelligenceSummary> {
    let repo_root = normalize_repo_root(path.as_ref())?;
    let mut files = Vec::new();
    walk_repo(&repo_root, &repo_root, &mut files)?;

    let total_files = files.len() as u64;
    let signals: Vec<RepoFileSignal> = files
        .into_iter()
        .map(|file| classify_file(&file.relative_path, file.bytes))
        .collect();
    let indexed: Vec<RepoFileSignal> = signals
        .iter()
        .filter(|signal| signal.include_by_default)
        .cloned()
        .collect();
    let estimated_full_scan_tokens = signals
        .iter()
        .map(|signal| signal.estimated_tokens)
        .sum::<u64>();
    let mut role_counts = BTreeMap::new();
    for signal in &signals {
        *role_counts.entry(role_key(&signal.role).to_string()).or_insert(0) += 1;
    }

    let graph = build_repo_graph_summary(&indexed);
    let packs = vec![
        build_context_pack(
            "implementation",
            "Implementation Pack",
            "Source files likely needed for feature work.",
            indexed
                .iter()
                .filter(|signal| matches!(signal.role, RepoFileRole::Source | RepoFileRole::Config))
                .cloned()
                .collect(),
            estimated_full_scan_tokens,
        ),
        build_context_pack(
            "verification",
            "Verification Pack",
            "Tests, scripts, and config likely needed before committing.",
            indexed
                .iter()
                .filter(|signal| matches!(signal.role, RepoFileRole::Test | RepoFileRole::Config))
                .cloned()
                .collect(),
            estimated_full_scan_tokens,
        ),
        build_context_pack(
            "handoff",
            "Handoff Pack",
            "Docs and project metadata useful for another agent or maintainer.",
            indexed
                .iter()
                .filter(|signal| matches!(signal.role, RepoFileRole::Docs | RepoFileRole::Config))
                .cloned()
                .collect(),
            estimated_full_scan_tokens,
        ),
    ];

    Ok(RepoIntelligenceSummary {
        indexed_at: Utc::now().to_rfc3339(),
        repo_root: repo_root.display().to_string(),
        total_files,
        indexed_files: indexed.len() as u64,
        skipped_files: signals.len().saturating_sub(indexed.len()) as u64,
        estimated_full_scan_tokens,
        role_counts,
        graph: Some(graph),
        packs,
    })
}

pub fn save_latest_summary(summary: &RepoIntelligenceSummary) -> Result<()> {
    let app_dir = app_data_dir();
    ensure_data_dirs(&app_dir)?;
    let path = latest_summary_path();
    let json = serde_json::to_vec_pretty(summary)?;
    std::fs::write(&path, json)
        .with_context(|| format!("writing repo intelligence summary {}", path.display()))?;
    Ok(())
}

pub fn load_latest_summary() -> Result<Option<RepoIntelligenceSummary>> {
    let path = latest_summary_path();
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read(&path)
        .with_context(|| format!("reading repo intelligence summary {}", path.display()))?;
    let summary = serde_json::from_slice(&raw)
        .with_context(|| format!("parsing repo intelligence summary {}", path.display()))?;
    Ok(Some(summary))
}

pub fn clear_latest_summary() -> Result<bool> {
    let path = latest_summary_path();
    if !path.exists() {
        return Ok(false);
    }
    std::fs::remove_file(&path)
        .with_context(|| format!("removing repo intelligence summary {}", path.display()))?;
    Ok(true)
}

fn latest_summary_path() -> PathBuf {
    config_file(&app_data_dir(), "repo-intelligence-latest.json")
}

fn normalize_repo_root(path: &Path) -> Result<PathBuf> {
    let expanded = expand_home(path);
    let canonical = expanded
        .canonicalize()
        .with_context(|| format!("repo path not found: {}", expanded.display()))?;
    if !canonical.is_dir() {
        return Err(anyhow!("repo path must be a directory: {}", canonical.display()));
    }
    Ok(canonical)
}

fn expand_home(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    if raw == "~" {
        return dirs::home_dir().unwrap_or_else(|| path.to_path_buf());
    }
    if let Some(rest) = raw.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    path.to_path_buf()
}

#[derive(Debug)]
struct RepoFile {
    relative_path: String,
    bytes: u64,
}

fn walk_repo(root: &Path, dir: &Path, files: &mut Vec<RepoFile>) -> Result<()> {
    if files.len() >= MAX_SCAN_FILES {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        if files.len() >= MAX_SCAN_FILES {
            break;
        }
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        if file_type.is_dir() {
            if should_skip_dir(&name) {
                continue;
            }
            walk_repo(root, &path, files)?;
        } else if file_type.is_file() {
            let metadata = entry.metadata()?;
            let relative_path = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            files.push(RepoFile {
                relative_path,
                bytes: metadata.len(),
            });
        }
    }

    Ok(())
}

fn should_skip_dir(name: &OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return true;
    };
    IGNORED_DIRS.iter().any(|ignored| ignored == &name)
}

fn classify_file(path: &str, bytes: u64) -> RepoFileSignal {
    let name = path.rsplit('/').next().unwrap_or(path);
    let lower = path.to_lowercase();
    let extension = Path::new(name)
        .extension()
        .and_then(OsStr::to_str)
        .map(|extension| format!(".{}", extension.to_lowercase()))
        .unwrap_or_default();
    let mut reasons = Vec::new();
    let role = if is_secret_like_path(path, name, &extension) {
        reasons.push("secret-like path excluded from default packs".to_string());
        RepoFileRole::Generated
    } else if bytes > MAX_INDEXED_FILE_BYTES {
        reasons.push("large file skipped from default packs".to_string());
        RepoFileRole::Generated
    } else if lockfile_name(name) {
        reasons.push("package lockfile".to_string());
        RepoFileRole::Lockfile
    } else if lower.contains(".test.") || lower.contains(".spec.") || lower.contains("/tests/") {
        reasons.push("test path".to_string());
        RepoFileRole::Test
    } else if lower.ends_with(".md") || lower.starts_with("docs/") || lower.contains("/docs/") {
        reasons.push("documentation".to_string());
        RepoFileRole::Docs
    } else if name.starts_with('.')
        || matches!(extension.as_str(), ".toml" | ".json" | ".yml" | ".yaml")
    {
        reasons.push("configuration".to_string());
        RepoFileRole::Config
    } else if matches!(
        extension.as_str(),
        ".png" | ".jpg" | ".jpeg" | ".gif" | ".svg" | ".ico" | ".webp"
    ) {
        reasons.push("static asset".to_string());
        RepoFileRole::Asset
    } else if language_for_extension(&extension) != "Unknown" {
        reasons.push("source file".to_string());
        RepoFileRole::Source
    } else {
        RepoFileRole::Unknown
    };
    let include_by_default = matches!(
        role,
        RepoFileRole::Source | RepoFileRole::Test | RepoFileRole::Config | RepoFileRole::Docs
    );

    RepoFileSignal {
        path: path.to_string(),
        role,
        language: language_for_extension(&extension).to_string(),
        estimated_tokens: estimate_tokens(bytes),
        include_by_default,
        reasons,
    }
}

fn build_context_pack(
    id: &str,
    title: &str,
    purpose: &str,
    mut files: Vec<RepoFileSignal>,
    estimated_full_scan_tokens: u64,
) -> RepoContextPack {
    files.sort_by(|a, b| {
        a.estimated_tokens
            .cmp(&b.estimated_tokens)
            .then_with(|| a.path.cmp(&b.path))
    });
    files.truncate(MAX_PACK_FILES);
    let estimated_tokens = files
        .iter()
        .map(|signal| signal.estimated_tokens)
        .sum::<u64>();
    let savings_vs_full_scan_pct = if estimated_full_scan_tokens > 0 {
        let saved = 1.0 - (estimated_tokens as f64 / estimated_full_scan_tokens as f64);
        (saved.max(0.0) * 1000.0).round() / 10.0
    } else {
        0.0
    };

    RepoContextPack {
        id: id.to_string(),
        title: title.to_string(),
        purpose: purpose.to_string(),
        files,
        estimated_tokens,
        savings_vs_full_scan_pct,
    }
}

fn build_repo_graph_summary(files: &[RepoFileSignal]) -> RepoGraphSummary {
    let included = files
        .iter()
        .filter(|signal| signal.include_by_default)
        .cloned()
        .collect::<Vec<_>>();
    let source_and_config = included
        .iter()
        .filter(|signal| matches!(signal.role, RepoFileRole::Source | RepoFileRole::Config))
        .cloned()
        .collect::<Vec<_>>();
    let import_edges = build_repo_graph_edges(&included);

    RepoGraphSummary {
        top_directories: summarize_graph_nodes(&included, top_directory, 6),
        top_languages: summarize_graph_nodes(
            &included
                .iter()
                .filter(|signal| signal.language != "Unknown")
                .cloned()
                .collect::<Vec<_>>(),
            |signal| signal.language.clone(),
            6,
        ),
        entrypoints: source_and_config
            .iter()
            .filter(|signal| is_likely_entrypoint(signal))
            .take(12)
            .cloned()
            .collect(),
        likely_tests: included
            .iter()
            .filter(|signal| matches!(signal.role, RepoFileRole::Test))
            .take(12)
            .cloned()
            .collect(),
        config_hubs: included
            .iter()
            .filter(|signal| matches!(signal.role, RepoFileRole::Config))
            .take(12)
            .cloned()
            .collect(),
        dependency_hubs: files
            .iter()
            .filter(|signal| is_dependency_hub(signal))
            .take(12)
            .cloned()
            .collect(),
        reverse_dependency_hubs: build_reverse_dependency_hubs(&included, &import_edges),
        import_edges,
    }
}

fn build_repo_graph_edges(files: &[RepoFileSignal]) -> Vec<RepoGraphEdge> {
    let dependency_hubs = files
        .iter()
        .filter(|signal| is_dependency_hub(signal))
        .cloned()
        .collect::<Vec<_>>();
    let config_hubs = files
        .iter()
        .filter(|signal| matches!(signal.role, RepoFileRole::Config))
        .cloned()
        .collect::<Vec<_>>();
    let mut edges = Vec::new();

    for file in files {
        if matches!(file.role, RepoFileRole::Test) {
            if let Some(target) = find_test_target(file, files) {
                push_graph_edge(
                    &mut edges,
                    RepoGraphEdge {
                        from: file.path.clone(),
                        to: target.path.clone(),
                        kind: RepoGraphEdgeKind::TestToSource,
                        reason: "test filename matches source module".into(),
                    },
                );
            }
        }

        if is_likely_entrypoint(file) {
            if let Some(config) = find_nearest_config_hub(file, &config_hubs) {
                push_graph_edge(
                    &mut edges,
                    RepoGraphEdge {
                        from: file.path.clone(),
                        to: config.path.clone(),
                        kind: RepoGraphEdgeKind::EntrypointToConfig,
                        reason: "entrypoint shares closest config surface".into(),
                    },
                );
            }
        }

        if matches!(file.role, RepoFileRole::Source) {
            if let Some(dependency_hub) = find_nearest_dependency_hub(file, &dependency_hubs) {
                push_graph_edge(
                    &mut edges,
                    RepoGraphEdge {
                        from: file.path.clone(),
                        to: dependency_hub.path.clone(),
                        kind: RepoGraphEdgeKind::SourceToDependencyHub,
                        reason: "source file belongs to dependency hub scope".into(),
                    },
                );
            }
        }
    }

    edges
}

fn push_graph_edge(edges: &mut Vec<RepoGraphEdge>, edge: RepoGraphEdge) {
    if edge.from == edge.to || edges.len() >= 24 {
        return;
    }
    if edges
        .iter()
        .any(|existing| existing.from == edge.from && existing.to == edge.to && existing.kind == edge.kind)
    {
        return;
    }
    edges.push(edge);
}

fn build_reverse_dependency_hubs(
    files: &[RepoFileSignal],
    edges: &[RepoGraphEdge],
) -> Vec<RepoGraphNode> {
    let mut inbound: BTreeMap<String, RepoGraphNode> = BTreeMap::new();
    for edge in edges {
        let target = files.iter().find(|file| file.path == edge.to);
        let node = inbound.entry(edge.to.clone()).or_insert_with(|| RepoGraphNode {
            label: edge.to.clone(),
            count: 0,
            estimated_tokens: target.map(|file| file.estimated_tokens).unwrap_or(0),
            examples: Vec::new(),
        });
        node.count += 1;
        if node.examples.len() < 4 {
            node.examples.push(edge.from.clone());
        }
    }

    let mut nodes = inbound.into_values().collect::<Vec<_>>();
    nodes.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| b.estimated_tokens.cmp(&a.estimated_tokens))
            .then_with(|| a.label.cmp(&b.label))
    });
    nodes.truncate(12);
    nodes
}

fn find_test_target(file: &RepoFileSignal, files: &[RepoFileSignal]) -> Option<RepoFileSignal> {
    test_target_candidates(&file.path)
        .into_iter()
        .find_map(|candidate| files.iter().find(|file| file.path == candidate).cloned())
}

fn test_target_candidates(path: &str) -> Vec<String> {
    let extension = extension_for_path(path);
    let base = extension
        .strip_prefix('.')
        .and_then(|_| path.strip_suffix(&extension))
        .unwrap_or(path);
    let Some(base) = base
        .strip_suffix(".test")
        .or_else(|| base.strip_suffix(".spec"))
    else {
        return Vec::new();
    };
    let mut extensions = vec![extension, ".tsx".into(), ".ts".into(), ".jsx".into(), ".js".into(), ".rs".into()];
    extensions.sort();
    extensions.dedup();
    extensions
        .into_iter()
        .filter(|extension| !extension.is_empty())
        .map(|extension| format!("{base}{extension}"))
        .collect()
}

fn find_nearest_config_hub(
    file: &RepoFileSignal,
    config_hubs: &[RepoFileSignal],
) -> Option<RepoFileSignal> {
    nearest_scoped_file(file, config_hubs).or_else(|| {
        config_hubs
            .iter()
            .find(|candidate| !candidate.path.contains('/'))
            .cloned()
    })
}

fn find_nearest_dependency_hub(
    file: &RepoFileSignal,
    dependency_hubs: &[RepoFileSignal],
) -> Option<RepoFileSignal> {
    nearest_scoped_file(file, dependency_hubs).or_else(|| {
        dependency_hubs
            .iter()
            .find(|candidate| !candidate.path.contains('/'))
            .cloned()
    })
}

fn nearest_scoped_file(file: &RepoFileSignal, candidates: &[RepoFileSignal]) -> Option<RepoFileSignal> {
    candidates
        .iter()
        .filter(|candidate| candidate.path != file.path)
        .filter_map(|candidate| {
            let score = shared_path_prefix_score(&file.path, &candidate.path);
            (score > 0).then_some((candidate, score))
        })
        .min_by(|(left, left_score), (right, right_score)| {
            right_score
                .cmp(left_score)
                .then_with(|| left.path.split('/').count().cmp(&right.path.split('/').count()))
                .then_with(|| left.path.cmp(&right.path))
        })
        .map(|(candidate, _)| candidate.clone())
}

fn shared_path_prefix_score(left: &str, right: &str) -> usize {
    if !right.contains('/') && left.contains('/') {
        return 1;
    }
    left.split('/')
        .zip(right.split('/'))
        .take_while(|(left, right)| left == right)
        .count()
}

fn extension_for_path(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(OsStr::to_str)
        .map(|extension| format!(".{extension}"))
        .unwrap_or_default()
}


fn summarize_graph_nodes<F>(files: &[RepoFileSignal], label_for_file: F, limit: usize) -> Vec<RepoGraphNode>
where
    F: Fn(&RepoFileSignal) -> String,
{
    let mut nodes: BTreeMap<String, RepoGraphNode> = BTreeMap::new();

    for file in files {
        let label = label_for_file(file);
        let node = nodes.entry(label.clone()).or_insert_with(|| RepoGraphNode {
            label,
            count: 0,
            estimated_tokens: 0,
            examples: Vec::new(),
        });
        node.count += 1;
        node.estimated_tokens += file.estimated_tokens;
        if node.examples.len() < 4 {
            node.examples.push(file.path.clone());
        }
    }

    let mut nodes = nodes.into_values().collect::<Vec<_>>();
    nodes.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| b.estimated_tokens.cmp(&a.estimated_tokens))
            .then_with(|| a.label.cmp(&b.label))
    });
    nodes.truncate(limit);
    nodes
}

fn top_directory(file: &RepoFileSignal) -> String {
    file.path
        .split_once('/')
        .map(|(first, _)| first.to_string())
        .unwrap_or_else(|| ".".to_string())
}

fn is_likely_entrypoint(file: &RepoFileSignal) -> bool {
    if !matches!(file.role, RepoFileRole::Source) {
        return false;
    }
    let normalized = file.path.to_lowercase();
    let name = normalized.rsplit('/').next().unwrap_or(&normalized);
    matches!(
        name,
        "main.ts"
            | "main.tsx"
            | "main.js"
            | "index.ts"
            | "index.tsx"
            | "index.js"
            | "app.tsx"
            | "app.ts"
            | "lib.rs"
            | "main.rs"
    ) || normalized.ends_with("/src-tauri/src/lib.rs")
}

fn estimate_tokens(bytes: u64) -> u64 {
    std::cmp::max(1, bytes.saturating_add(3) / 4)
}

fn lockfile_name(name: &str) -> bool {
    matches!(
        name,
        "Cargo.lock" | "package-lock.json" | "pnpm-lock.yaml" | "yarn.lock" | "bun.lockb"
    )
}

fn is_dependency_hub(file: &RepoFileSignal) -> bool {
    let name = file
        .path
        .rsplit('/')
        .next()
        .unwrap_or(file.path.as_str())
        .to_lowercase();
    matches!(file.role, RepoFileRole::Lockfile)
        || matches!(
            name.as_str(),
            "package.json"
                | "pyproject.toml"
                | "requirements.txt"
                | "cargo.toml"
                | "go.mod"
                | "gemfile"
                | "podfile"
        )
}

fn is_secret_like_path(path: &str, name: &str, extension: &str) -> bool {
    let normalized = path.replace('\\', "/");
    let lower_path = normalized.to_lowercase();
    let lower_name = name.to_lowercase();

    SECRET_FILE_NAMES
        .iter()
        .any(|secret_name| lower_name == *secret_name)
        || SECRET_EXTENSIONS
            .iter()
            .any(|secret_extension| extension == *secret_extension)
        || lower_name.starts_with("authkey_") && extension == ".p8"
        || lower_path.split('/').any(|segment| {
            SECRET_PATH_SEGMENTS
                .iter()
                .any(|secret_segment| segment == *secret_segment)
        })
}

fn language_for_extension(extension: &str) -> &'static str {
    match extension {
        ".css" => "CSS",
        ".html" => "HTML",
        ".js" | ".mjs" => "JavaScript",
        ".json" => "JSON",
        ".jsx" | ".tsx" => "React",
        ".md" => "Markdown",
        ".py" => "Python",
        ".rs" => "Rust",
        ".sh" => "Shell",
        ".swift" => "Swift",
        ".toml" => "TOML",
        ".ts" => "TypeScript",
        ".yml" | ".yaml" => "YAML",
        _ => "Unknown",
    }
}

fn role_key(role: &RepoFileRole) -> &'static str {
    match role {
        RepoFileRole::Source => "source",
        RepoFileRole::Test => "test",
        RepoFileRole::Config => "config",
        RepoFileRole::Docs => "docs",
        RepoFileRole::Asset => "asset",
        RepoFileRole::Lockfile => "lockfile",
        RepoFileRole::Generated => "generated",
        RepoFileRole::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_repo_files() {
        assert!(matches!(
            classify_file("src/App.tsx", 100).role,
            RepoFileRole::Source
        ));
        assert!(matches!(
            classify_file("src/App.test.tsx", 100).role,
            RepoFileRole::Test
        ));
        assert!(matches!(
            classify_file("docs/install.md", 100).role,
            RepoFileRole::Docs
        ));
        assert!(matches!(
            classify_file("package-lock.json", 100).role,
            RepoFileRole::Lockfile
        ));
        assert!(matches!(
            classify_file("dist/assets/index.js", MAX_INDEXED_FILE_BYTES + 1).role,
            RepoFileRole::Generated
        ));
    }

    #[test]
    fn excludes_secret_like_paths_from_default_packs() {
        for path in [
            ".env",
            ".env.local",
            ".npmrc",
            ".secrets/app.json",
            "secrets/prod.toml",
            "private_keys/app.pem",
            "authkey_ABC123.p8",
            "certs/distribution.p12",
            "keys/service-account.key",
            "certs/root.crt",
        ] {
            let signal = classify_file(path, 100);

            assert!(matches!(signal.role, RepoFileRole::Generated), "{path}");
            assert!(!signal.include_by_default, "{path}");
            assert!(
                signal
                    .reasons
                    .contains(&"secret-like path excluded from default packs".to_string()),
                "{path}"
            );
        }
    }

    #[test]
    fn builds_repo_graph_summary_for_agent_context() {
        let files = vec![
            classify_file("src/App.tsx", 4000),
            classify_file("src/main.tsx", 1400),
            classify_file("src/App.test.tsx", 2000),
            classify_file("src-tauri/src/lib.rs", 5000),
            classify_file("scripts/release.mjs", 1200),
            classify_file("package.json", 800),
            classify_file("package-lock.json", 1600),
            classify_file(".env.local", 200),
        ];
        let graph = build_repo_graph_summary(&files);

        assert_eq!(graph.top_directories[0].label, "src");
        assert!(graph.top_languages.iter().any(|node| node.label == "React"));
        assert!(graph.entrypoints.iter().any(|file| file.path == "src/main.tsx"));
        assert!(graph
            .likely_tests
            .iter()
            .any(|file| file.path == "src/App.test.tsx"));
        assert!(graph
            .config_hubs
            .iter()
            .any(|file| file.path == "package.json"));
        assert!(!graph
            .config_hubs
            .iter()
            .any(|file| file.path == ".env.local"));
        assert!(graph
            .dependency_hubs
            .iter()
            .any(|file| file.path == "package.json"));
        assert!(graph
            .dependency_hubs
            .iter()
            .any(|file| file.path == "package-lock.json"));
        assert!(graph.import_edges.iter().any(|edge| {
            edge.from == "src/App.test.tsx"
                && edge.to == "src/App.tsx"
                && matches!(edge.kind, RepoGraphEdgeKind::TestToSource)
        }));
        assert!(graph.import_edges.iter().any(|edge| {
            edge.from == "src/main.tsx"
                && edge.to == "package.json"
                && matches!(edge.kind, RepoGraphEdgeKind::EntrypointToConfig)
        }));
        assert!(graph
            .reverse_dependency_hubs
            .iter()
            .any(|node| node.label == "package.json"));
    }

    #[test]
    fn builds_bounded_context_pack() {
        let pack = build_context_pack(
            "implementation",
            "Implementation Pack",
            "Source files likely needed for feature work.",
            vec![
                classify_file("src/large.ts", 800),
                classify_file("src/small.ts", 80),
                classify_file("src/medium.ts", 400),
            ],
            1_000,
        );

        assert_eq!(pack.files[0].path, "src/small.ts");
        assert_eq!(pack.estimated_tokens, 320);
        assert!(pack.savings_vs_full_scan_pct > 60.0);
    }
}
