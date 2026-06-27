use std::collections::{hash_map::DefaultHasher, BTreeMap};
use std::ffi::OsStr;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::{anyhow, Context, Result};
use chrono::Utc;

use crate::models::{
    RepoContextPack, RepoContextPackGraphBrief, RepoContextPackResponse, RepoContextPackSafety,
    RepoFileIndexEntry, RepoFileRole, RepoFileSignal, RepoGraphEdge, RepoGraphEdgeKind,
    RepoGraphInputEntry, RepoGraphNode, RepoGraphSummary, RepoIndexMetadata,
    RepoIntelligenceSummary, RepoSkippedIndexEntry, RepoSymbol, RepoSymbolKind,
};
use crate::storage::{app_data_dir, config_file, ensure_data_dirs};

const MAX_SCAN_FILES: usize = 2_500;
const MAX_INDEXED_FILE_BYTES: u64 = 1_000_000;
const MAX_PACK_FILES: usize = 40;
const INDEXER_VERSION: &str = "path-graph-v2";
const INDEX_METADATA_SCHEMA_VERSION: u64 = 1;
const PARSER_VERSION: &str = "metadata-fingerprint-v1";
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
const SECRET_FILE_NAMES: [&str; 13] = [
    ".env",
    ".env.local",
    ".env.production",
    ".envrc",
    ".git-credentials",
    ".netrc",
    "settings.local.json",
    "credentials.toml",
    ".npmrc",
    ".pypirc",
    "headroom_memory.db",
    "id_rsa",
    "id_ed25519",
];
const SECRET_EXTENSIONS: [&str; 10] = [
    ".pem", ".p8", ".p12", ".key", ".crt", ".cer", ".db", ".sqlite", ".sqlite3", ".log",
];
const SECRET_PATH_SEGMENTS: [&str; 10] = [
    "secrets",
    ".secrets",
    "private_keys",
    ".private_keys",
    ".aws",
    ".azure",
    ".config/gh",
    ".gnupg",
    ".playwright-mcp",
    ".ssh",
];

pub fn summarize_repo(path: impl AsRef<Path>) -> Result<RepoIntelligenceSummary> {
    let repo_root = normalize_repo_root(path.as_ref())?;
    let previous_summary = load_latest_summary().ok().flatten();
    let mut files = Vec::new();
    walk_repo(&repo_root, &repo_root, &mut files)?;

    let total_files = files.len() as u64;
    let signals: Vec<RepoFileSignal> = files
        .iter()
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
        *role_counts
            .entry(role_key(&signal.role).to_string())
            .or_insert(0) += 1;
    }

    let graph = build_repo_graph_summary(&repo_root, &indexed);
    let index_metadata =
        build_index_metadata(&repo_root, &files, &signals, previous_summary.as_ref());
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
        indexer_version: Some(INDEXER_VERSION.to_string()),
        total_files,
        indexed_files: indexed.len() as u64,
        skipped_files: signals.len().saturating_sub(indexed.len()) as u64,
        estimated_full_scan_tokens,
        role_counts,
        index_metadata: Some(index_metadata),
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

pub fn latest_context_pack(pack_id: Option<&str>) -> Result<Option<RepoContextPackResponse>> {
    let Some(summary) = load_latest_summary()? else {
        return Ok(None);
    };
    build_context_pack_response(&summary, pack_id).map(Some)
}

pub fn build_context_pack_response(
    summary: &RepoIntelligenceSummary,
    pack_id: Option<&str>,
) -> Result<RepoContextPackResponse> {
    let selected_pack_id = pack_id.unwrap_or("implementation");
    let pack = summary
        .packs
        .iter()
        .find(|candidate| candidate.id == selected_pack_id)
        .cloned()
        .ok_or_else(|| anyhow!("repo intelligence pack not found: {selected_pack_id}"))?;
    let graph = summary.graph.as_ref();

    Ok(RepoContextPackResponse {
        repo_root: summary.repo_root.clone(),
        indexed_at: summary.indexed_at.clone(),
        pack,
        index_metadata: summary.index_metadata.clone(),
        graph_brief: RepoContextPackGraphBrief {
            available: graph.is_some(),
            dependency_hub_count: graph
                .map(|graph| graph.dependency_hubs.len())
                .unwrap_or_default(),
            import_edge_count: graph
                .map(|graph| graph.import_edges.len())
                .unwrap_or_default(),
            reverse_dependency_hub_count: graph
                .map(|graph| graph.reverse_dependency_hubs.len())
                .unwrap_or_default(),
            symbol_count: graph.map(|graph| graph.symbols.len()).unwrap_or_default(),
            symbol_edge_count: graph
                .map(|graph| graph.symbol_edges.len())
                .unwrap_or_default(),
        },
        safety: RepoContextPackSafety {
            read_only: true,
            excludes_secret_like_paths: true,
            modifies_repository: false,
        },
    })
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
        return Err(anyhow!(
            "repo path must be a directory: {}",
            canonical.display()
        ));
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
    modified_unix_ms: u64,
    fingerprint: String,
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
                modified_unix_ms: metadata_modified_unix_ms(&metadata),
                fingerprint: fingerprint_file_metadata(&path, &metadata),
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

fn metadata_modified_unix_ms(metadata: &std::fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn fingerprint_file_metadata(path: &Path, metadata: &std::fs::Metadata) -> String {
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    metadata_modified_unix_ms(metadata).hash(&mut hasher);
    if let Ok(mut file) = std::fs::File::open(path) {
        let mut buffer = [0_u8; 8192];
        loop {
            match file.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => buffer[..read].hash(&mut hasher),
                Err(_) => break,
            }
        }
    }
    format!("{:016x}", hasher.finish())
}

fn build_index_metadata(
    repo_root: &Path,
    files: &[RepoFile],
    signals: &[RepoFileSignal],
    previous_summary: Option<&RepoIntelligenceSummary>,
) -> RepoIndexMetadata {
    let include_by_path = signals
        .iter()
        .map(|signal| (signal.path.as_str(), signal.include_by_default))
        .collect::<BTreeMap<_, _>>();
    let mut file_fingerprints = files
        .iter()
        .filter(|file| {
            include_by_path
                .get(file.relative_path.as_str())
                .copied()
                .unwrap_or(false)
        })
        .map(|file| RepoFileIndexEntry {
            path: file.relative_path.clone(),
            bytes: file.bytes,
            modified_unix_ms: file.modified_unix_ms,
            fingerprint: file.fingerprint.clone(),
        })
        .collect::<Vec<_>>();
    file_fingerprints.sort_by(|a, b| a.path.cmp(&b.path));
    let fingerprint_by_path = file_fingerprints
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let mut skipped_files = signals
        .iter()
        .filter(|signal| !signal.include_by_default)
        .map(|signal| RepoSkippedIndexEntry {
            path: if signal
                .reasons
                .iter()
                .any(|reason| reason == "secret-like path excluded default packs")
            {
                "<secret-like path>".to_string()
            } else {
                signal.path.clone()
            },
            role: signal.role.clone(),
            reasons: if signal.reasons.is_empty() {
                vec!["not included in default repo index".to_string()]
            } else {
                signal.reasons.clone()
            },
        })
        .collect::<Vec<_>>();
    skipped_files.sort_by(|a, b| a.path.cmp(&b.path));
    let mut graph_inputs = signals
        .iter()
        .filter(|signal| {
            signal.include_by_default
                && matches!(
                    signal.role,
                    RepoFileRole::Source | RepoFileRole::Test | RepoFileRole::Config
                )
        })
        .map(|signal| {
            let fingerprint = fingerprint_by_path.get(signal.path.as_str());
            RepoGraphInputEntry {
                path: signal.path.clone(),
                role: signal.role.clone(),
                language: signal.language.clone(),
                bytes: fingerprint.map(|entry| entry.bytes).unwrap_or(0),
                fingerprint: fingerprint
                    .map(|entry| entry.fingerprint.clone())
                    .unwrap_or_default(),
            }
        })
        .collect::<Vec<_>>();
    graph_inputs.sort_by(|a, b| a.path.cmp(&b.path));

    let cache_key = build_cache_key(repo_root, &file_fingerprints, &graph_inputs);
    let previous_metadata = previous_summary.and_then(|summary| {
        if summary.repo_root == repo_root.to_string_lossy() {
            summary.index_metadata.as_ref()
        } else {
            None
        }
    });
    let cache_state = previous_metadata
        .map(|metadata| {
            if metadata.cache_key == cache_key
                && metadata.indexer_version == INDEXER_VERSION
                && metadata.parser_version == PARSER_VERSION
            {
                "unchanged"
            } else {
                "changed"
            }
        })
        .unwrap_or("new")
        .to_string();

    RepoIndexMetadata {
        schema_version: INDEX_METADATA_SCHEMA_VERSION,
        indexer_version: INDEXER_VERSION.to_string(),
        parser_version: PARSER_VERSION.to_string(),
        cache_key,
        cache_state,
        generated_at: Utc::now().to_rfc3339(),
        previous_indexed_at: previous_summary
            .filter(|summary| summary.repo_root == repo_root.to_string_lossy())
            .map(|summary| summary.indexed_at.clone()),
        file_count: files.len() as u64,
        indexed_file_count: signals
            .iter()
            .filter(|signal| signal.include_by_default)
            .count() as u64,
        skipped_file_count: signals
            .iter()
            .filter(|signal| !signal.include_by_default)
            .count() as u64,
        file_fingerprints,
        skipped_files,
        graph_inputs,
    }
}

fn build_cache_key(
    repo_root: &Path,
    file_fingerprints: &[RepoFileIndexEntry],
    graph_inputs: &[RepoGraphInputEntry],
) -> String {
    let mut hasher = DefaultHasher::new();
    INDEX_METADATA_SCHEMA_VERSION.hash(&mut hasher);
    INDEXER_VERSION.hash(&mut hasher);
    PARSER_VERSION.hash(&mut hasher);
    repo_root.to_string_lossy().hash(&mut hasher);
    for entry in file_fingerprints {
        entry.path.hash(&mut hasher);
        entry.bytes.hash(&mut hasher);
        entry.modified_unix_ms.hash(&mut hasher);
        entry.fingerprint.hash(&mut hasher);
    }
    for entry in graph_inputs {
        entry.path.hash(&mut hasher);
        entry.role.hash(&mut hasher);
        entry.language.hash(&mut hasher);
        entry.bytes.hash(&mut hasher);
        entry.fingerprint.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
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

fn build_repo_graph_summary(repo_root: &Path, files: &[RepoFileSignal]) -> RepoGraphSummary {
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
    let mut import_edges = build_repo_graph_edges(&included);
    import_edges.extend(build_import_reference_edges(repo_root, &included));
    let symbols = build_repo_symbols(repo_root, &included);
    let mut symbol_edges = build_symbol_edges(&included, &symbols);
    symbol_edges.extend(build_call_reference_edges(repo_root, &included, &symbols));

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
        symbols,
        symbol_edges,
        import_edges,
    }
}

fn build_repo_symbols(repo_root: &Path, files: &[RepoFileSignal]) -> Vec<RepoSymbol> {
    let mut symbols = Vec::new();
    for file in files.iter().filter(|file| {
        matches!(file.role, RepoFileRole::Source | RepoFileRole::Test)
            && matches!(
                file.language.as_str(),
                "TypeScript" | "JavaScript" | "React" | "Rust" | "Python"
            )
    }) {
        if symbols.len() >= 200 {
            break;
        }
        let Ok(content) = std::fs::read_to_string(repo_root.join(&file.path)) else {
            continue;
        };
        symbols.extend(extract_file_symbols(file, &content, 200 - symbols.len()));
    }
    symbols
}

fn extract_file_symbols(file: &RepoFileSignal, content: &str, remaining: usize) -> Vec<RepoSymbol> {
    let mut symbols = Vec::new();
    let mut parents: Vec<(usize, String)> = Vec::new();
    for (index, line) in content.lines().enumerate() {
        if symbols.len() >= remaining {
            break;
        }
        let indent = line.chars().take_while(|ch| ch.is_whitespace()).count();
        while parents
            .last()
            .is_some_and(|(parent_indent, _)| indent <= *parent_indent)
        {
            parents.pop();
        }
        let trimmed = line.trim_start();
        let Some((kind, name)) = extract_symbol_from_line(&file.language, trimmed) else {
            continue;
        };
        let parent = parents.last().map(|(_, parent)| parent.clone());
        if matches!(
            kind,
            RepoSymbolKind::Class
                | RepoSymbolKind::Struct
                | RepoSymbolKind::Enum
                | RepoSymbolKind::Trait
        ) {
            parents.push((indent, name.clone()));
        }
        symbols.push(RepoSymbol {
            name,
            kind,
            file: file.path.clone(),
            line: (index + 1) as u64,
            parent,
        });
    }
    symbols
}

fn extract_symbol_from_line(language: &str, line: &str) -> Option<(RepoSymbolKind, String)> {
    let line = line
        .trim_start_matches("pub ")
        .trim_start_matches("async ")
        .trim_start_matches("export ")
        .trim_start_matches("default ");
    if matches!(language, "TypeScript" | "JavaScript" | "React") {
        if let Some(name) = symbol_name_after(line, "function ") {
            return Some((RepoSymbolKind::Function, name));
        }
        if let Some(name) = symbol_name_after(line, "class ") {
            return Some((RepoSymbolKind::Class, name));
        }
        if let Some(name) = symbol_name_after(line, "interface ") {
            return Some((RepoSymbolKind::Trait, name));
        }
        if let Some(name) = symbol_name_after(line, "type ") {
            return Some((RepoSymbolKind::Trait, name));
        }
        if let Some(name) = symbol_name_after(line, "const ") {
            return Some((RepoSymbolKind::Const, name));
        }
    }
    if language == "Rust" {
        if let Some(name) = symbol_name_after(line, "fn ") {
            return Some((RepoSymbolKind::Function, name));
        }
        if let Some(name) = symbol_name_after(line, "struct ") {
            return Some((RepoSymbolKind::Struct, name));
        }
        if let Some(name) = symbol_name_after(line, "enum ") {
            return Some((RepoSymbolKind::Enum, name));
        }
        if let Some(name) = symbol_name_after(line, "trait ") {
            return Some((RepoSymbolKind::Trait, name));
        }
        if let Some(name) = symbol_name_after(line, "const ") {
            return Some((RepoSymbolKind::Const, name));
        }
    }
    if language == "Python" {
        if let Some(name) = symbol_name_after(line, "def ") {
            return Some((RepoSymbolKind::Function, name));
        }
        if let Some(name) = symbol_name_after(line, "class ") {
            return Some((RepoSymbolKind::Class, name));
        }
    }
    None
}

fn symbol_name_after(line: &str, prefix: &str) -> Option<String> {
    let rest = line.strip_prefix(prefix)?;
    let name = rest
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '$')
        .collect::<String>();
    (!name.is_empty()).then_some(name)
}

fn build_symbol_edges(files: &[RepoFileSignal], symbols: &[RepoSymbol]) -> Vec<RepoGraphEdge> {
    let mut edges = Vec::new();
    for symbol in symbols.iter().take(80) {
        for file in files.iter().filter(|file| file.path != symbol.file) {
            if edges.len() >= 80 {
                return edges;
            }
            if file
                .path
                .to_ascii_lowercase()
                .contains(&symbol.name.to_ascii_lowercase())
            {
                push_graph_edge(
                    &mut edges,
                    RepoGraphEdge {
                        from: file.path.clone(),
                        to: format!("{}#{}", symbol.file, symbol.name),
                        kind: RepoGraphEdgeKind::SymbolReference,
                        reason: "file path references indexed symbol name".into(),
                    },
                );
            }
        }
    }
    edges
}

fn build_import_reference_edges(repo_root: &Path, files: &[RepoFileSignal]) -> Vec<RepoGraphEdge> {
    let mut edges = Vec::new();
    for file in files.iter().filter(|file| {
        matches!(file.role, RepoFileRole::Source | RepoFileRole::Test)
            && matches!(
                file.language.as_str(),
                "TypeScript" | "JavaScript" | "React" | "Rust"
            )
    }) {
        let Ok(content) = std::fs::read_to_string(repo_root.join(&file.path)) else {
            continue;
        };
        for specifier in extract_import_specifiers(&content, &file.language) {
            if !specifier.starts_with('.') {
                continue;
            }
            let Some(target) = resolve_import_specifier(&file.path, &specifier, files) else {
                continue;
            };
            push_unbounded_graph_edge(
                &mut edges,
                RepoGraphEdge {
                    from: file.path.clone(),
                    to: target.path.clone(),
                    kind: RepoGraphEdgeKind::ImportReference,
                    reason: format!("source imports {specifier}"),
                },
                80,
            );
            if edges.len() >= 80 {
                return edges;
            }
        }
    }
    edges
}

fn build_call_reference_edges(
    repo_root: &Path,
    files: &[RepoFileSignal],
    symbols: &[RepoSymbol],
) -> Vec<RepoGraphEdge> {
    let callable_symbols = symbols
        .iter()
        .filter(|symbol| {
            matches!(
                symbol.kind,
                RepoSymbolKind::Function | RepoSymbolKind::Const
            )
        })
        .take(120)
        .collect::<Vec<_>>();
    let mut edges = Vec::new();
    for file in files.iter().filter(|file| {
        matches!(file.role, RepoFileRole::Source | RepoFileRole::Test)
            && matches!(
                file.language.as_str(),
                "TypeScript" | "JavaScript" | "React" | "Rust" | "Python"
            )
    }) {
        let Ok(content) = std::fs::read_to_string(repo_root.join(&file.path)) else {
            continue;
        };
        for symbol in &callable_symbols {
            if file.path == symbol.file {
                continue;
            }
            if !contains_call_reference(&content, &symbol.name) {
                continue;
            }
            push_unbounded_graph_edge(
                &mut edges,
                RepoGraphEdge {
                    from: file.path.clone(),
                    to: format!("{}#{}", symbol.file, symbol.name),
                    kind: RepoGraphEdgeKind::CallReference,
                    reason: "source text references callable symbol".into(),
                },
                80,
            );
            if edges.len() >= 80 {
                return edges;
            }
        }
    }
    edges
}

fn extract_import_specifiers(content: &str, language: &str) -> Vec<String> {
    let mut specifiers = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if matches!(language, "TypeScript" | "JavaScript" | "React") {
            if let Some(specifier) = quoted_import_specifier(trimmed) {
                specifiers.push(specifier);
            }
        }
        if language == "Rust" {
            if let Some(module) = trimmed
                .strip_prefix("mod ")
                .and_then(|rest| rest.strip_suffix(';'))
            {
                specifiers.push(format!("./{}", module.trim()));
            }
        }
    }
    specifiers
}

fn quoted_import_specifier(line: &str) -> Option<String> {
    if !(line.starts_with("import ") || line.starts_with("export ") || line.contains("require(")) {
        return None;
    }
    for quote in ['"', '\''] {
        let Some(start) = line.rfind(quote) else {
            continue;
        };
        let before = &line[..start];
        let Some(second) = before.rfind(quote) else {
            continue;
        };
        let value = before[second + 1..].trim();
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn resolve_import_specifier<'a>(
    from_path: &str,
    specifier: &str,
    files: &'a [RepoFileSignal],
) -> Option<&'a RepoFileSignal> {
    let base_dir = from_path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
    let normalized = normalize_repo_path(&format!("{base_dir}/{specifier}"));
    let candidates = [
        normalized.clone(),
        format!("{normalized}.ts"),
        format!("{normalized}.tsx"),
        format!("{normalized}.js"),
        format!("{normalized}.jsx"),
        format!("{normalized}.mjs"),
        format!("{normalized}.rs"),
        format!("{normalized}/index.ts"),
        format!("{normalized}/index.tsx"),
        format!("{normalized}/index.js"),
    ];
    candidates
        .iter()
        .find_map(|candidate| files.iter().find(|file| file.path == *candidate))
}

fn normalize_repo_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            parts.pop();
            continue;
        }
        parts.push(part);
    }
    parts.join("/")
}

fn contains_call_reference(content: &str, symbol_name: &str) -> bool {
    let needle = format!("{symbol_name}(");
    content.contains(&needle)
        || content.contains(&format!("{symbol_name} ("))
        || content.contains(&format!(".{needle}"))
}

fn push_unbounded_graph_edge(edges: &mut Vec<RepoGraphEdge>, edge: RepoGraphEdge, limit: usize) {
    if edge.from == edge.to || edges.len() >= limit {
        return;
    }
    if edges.iter().any(|existing| {
        existing.from == edge.from && existing.to == edge.to && existing.kind == edge.kind
    }) {
        return;
    }
    edges.push(edge);
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
    if edges.iter().any(|existing| {
        existing.from == edge.from && existing.to == edge.to && existing.kind == edge.kind
    }) {
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
        let node = inbound
            .entry(edge.to.clone())
            .or_insert_with(|| RepoGraphNode {
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
    let mut extensions = vec![
        extension,
        ".tsx".into(),
        ".ts".into(),
        ".jsx".into(),
        ".js".into(),
        ".rs".into(),
    ];
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

fn nearest_scoped_file(
    file: &RepoFileSignal,
    candidates: &[RepoFileSignal],
) -> Option<RepoFileSignal> {
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
                .then_with(|| {
                    left.path
                        .split('/')
                        .count()
                        .cmp(&right.path.split('/').count())
                })
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

fn summarize_graph_nodes<F>(
    files: &[RepoFileSignal],
    label_for_file: F,
    limit: usize,
) -> Vec<RepoGraphNode>
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
        || lower_path == ".cargo/credentials"
        || lower_path == ".cargo/credentials.toml"
        || lower_path.ends_with("/.cargo/credentials")
        || lower_path.ends_with("/.cargo/credentials.toml")
        || lower_path == ".config/gh/hosts.yml"
        || lower_path.contains("/.config/gh/")
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
            ".envrc",
            ".git-credentials",
            ".netrc",
            ".npmrc",
            ".cargo/credentials.toml",
            ".config/gh/hosts.yml",
            ".ssh/config",
            ".aws/credentials",
            ".claude/settings.local.json",
            ".playwright-mcp/console.log",
            "headroom_memory.db",
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
        let graph = build_repo_graph_summary(Path::new("."), &files);

        assert_eq!(graph.top_directories[0].label, "src");
        assert!(graph.top_languages.iter().any(|node| node.label == "React"));
        assert!(graph
            .entrypoints
            .iter()
            .any(|file| file.path == "src/main.tsx"));
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
    fn builds_symbol_graph_from_indexed_sources() {
        let root = tempfile::tempdir().expect("create repo");
        let src = root.path().join("src");
        std::fs::create_dir_all(&src).expect("create src");
        std::fs::write(
        src.join("App.tsx"),
        "import { helper } from './helper';\nexport function App() { helper(); return null; }\nexport class ViewModel {}\n",
    )
    .expect("write tsx");
        std::fs::write(src.join("helper.ts"), "export function helper() {}\n")
            .expect("write helper");
        std::fs::write(
            src.join("lib.rs"),
            "pub struct RuntimeState {}\npub fn run_app() {}\n",
        )
        .expect("write rust");

        let summary = summarize_repo(root.path()).expect("summarize repo");
        let graph = summary.graph.expect("graph");
        assert!(graph
            .symbols
            .iter()
            .any(|symbol| symbol.name == "App" && symbol.file == "src/App.tsx"));
        assert!(graph
            .symbols
            .iter()
            .any(|symbol| symbol.name == "RuntimeState" && symbol.file == "src/lib.rs"));
        assert!(graph.import_edges.iter().any(|edge| {
            edge.from == "src/App.tsx"
                && edge.to == "src/helper.ts"
                && edge.kind == RepoGraphEdgeKind::ImportReference
        }));
        assert!(graph.symbol_edges.iter().any(|edge| {
            edge.from == "src/App.tsx"
                && edge.to == "src/helper.ts#helper"
                && edge.kind == RepoGraphEdgeKind::CallReference
        }));
    }

    #[test]
    fn builds_persistent_index_metadata_cache_states() {
        let root = tempfile::tempdir().expect("create repo");
        let files = vec![
            RepoFile {
                relative_path: "src/App.tsx".to_string(),
                bytes: 400,
                modified_unix_ms: 10,
                fingerprint: "app".to_string(),
            },
            RepoFile {
                relative_path: "package.json".to_string(),
                bytes: 80,
                modified_unix_ms: 5,
                fingerprint: "pkg".to_string(),
            },
            RepoFile {
                relative_path: "assets/logo.png".to_string(),
                bytes: 1_200,
                modified_unix_ms: 4,
                fingerprint: "bundle".to_string(),
            },
        ];
        let signals = files
            .iter()
            .map(|file| classify_file(&file.relative_path, file.bytes))
            .collect::<Vec<_>>();

        let first = build_index_metadata(root.path(), &files, &signals, None);
        assert_eq!(first.cache_state, "new");
        assert_eq!(first.file_count, 3);
        assert_eq!(first.indexed_file_count, 2);
        assert_eq!(first.skipped_file_count, 1);
        assert_eq!(
            first
                .file_fingerprints
                .iter()
                .map(|entry| entry.path.as_str())
                .collect::<Vec<_>>(),
            vec!["package.json", "src/App.tsx"]
        );
        assert_eq!(
            first
                .skipped_files
                .iter()
                .map(|entry| entry.path.as_str())
                .collect::<Vec<_>>(),
            vec!["assets/logo.png"]
        );
        assert!(first.skipped_files[0]
            .reasons
            .contains(&"static asset".to_string()));
        assert_eq!(
            first
                .graph_inputs
                .iter()
                .map(|entry| entry.path.as_str())
                .collect::<Vec<_>>(),
            vec!["package.json", "src/App.tsx"]
        );

        let previous = RepoIntelligenceSummary {
            indexed_at: "2026-06-27T12:00:00Z".to_string(),
            repo_root: root.path().to_string_lossy().to_string(),
            indexer_version: Some(INDEXER_VERSION.to_string()),
            total_files: 3,
            indexed_files: 2,
            skipped_files: 1,
            estimated_full_scan_tokens: 120,
            role_counts: BTreeMap::new(),
            index_metadata: Some(first.clone()),
            graph: None,
            packs: Vec::new(),
        };

        let unchanged = build_index_metadata(root.path(), &files, &signals, Some(&previous));
        assert_eq!(unchanged.cache_state, "unchanged");
        assert_eq!(
            unchanged.previous_indexed_at.as_deref(),
            Some("2026-06-27T12:00:00Z")
        );

        let mut changed_files = files;
        changed_files[0].bytes = 401;
        changed_files[0].fingerprint = "app-changed".to_string();
        let changed = build_index_metadata(root.path(), &changed_files, &signals, Some(&previous));
        assert_eq!(changed.cache_state, "changed");
    }

    #[test]
    fn file_fingerprint_changes_when_same_size_content_changes() {
        let root = tempfile::tempdir().expect("create repo");
        let path = root.path().join("src.ts");
        std::fs::write(&path, "alpha").expect("write file");
        let first_metadata = std::fs::metadata(&path).expect("first metadata");
        let first = fingerprint_file_metadata(&path, &first_metadata);

        std::fs::write(&path, "bravo").expect("rewrite same size file");
        let second_metadata = std::fs::metadata(&path).expect("second metadata");
        let second = fingerprint_file_metadata(&path, &second_metadata);

        assert_eq!(first_metadata.len(), second_metadata.len());
        assert_ne!(first, second);
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

    #[test]
    fn builds_read_only_context_pack_response_from_summary() {
        let root = tempfile::tempdir().expect("create repo");
        std::fs::create_dir_all(root.path().join("src")).expect("create src");
        std::fs::write(
            root.path().join("src/App.tsx"),
            "export function App() {}\n",
        )
        .expect("write source");
        std::fs::write(
            root.path().join("src/App.test.tsx"),
            "test('app', () => {})\n",
        )
        .expect("write test");
        std::fs::write(root.path().join(".env.local"), "SECRET=value\n").expect("write secret");
        std::fs::write(root.path().join("package.json"), "{}\n").expect("write package");

        let summary = summarize_repo(root.path()).expect("summarize repo");
        let response = build_context_pack_response(&summary, None).expect("context pack");

        assert_eq!(response.pack.id, "implementation");
        assert!(response.graph_brief.available);
        assert!(response.graph_brief.symbol_count > 0);
        assert!(response.safety.read_only);
        assert!(response.safety.excludes_secret_like_paths);
        assert!(!response.safety.modifies_repository);
        assert!(!response
            .pack
            .files
            .iter()
            .any(|file| file.path == ".env.local"));
    }

    #[test]
    fn selects_requested_context_pack_or_errors() {
        let summary = RepoIntelligenceSummary {
            indexed_at: "2026-06-27T12:00:00Z".to_string(),
            repo_root: "/tmp/example".to_string(),
            indexer_version: Some(INDEXER_VERSION.to_string()),
            total_files: 1,
            indexed_files: 1,
            skipped_files: 0,
            estimated_full_scan_tokens: 100,
            role_counts: BTreeMap::new(),
            index_metadata: None,
            graph: None,
            packs: vec![
                build_context_pack(
                    "implementation",
                    "Implementation Pack",
                    "Source files likely needed for feature work.",
                    vec![classify_file("src/App.tsx", 100)],
                    100,
                ),
                build_context_pack(
                    "verification",
                    "Verification Pack",
                    "Tests and config likely needed before committing.",
                    vec![classify_file("src/App.test.tsx", 100)],
                    100,
                ),
            ],
        };

        let verification =
            build_context_pack_response(&summary, Some("verification")).expect("verification pack");
        assert_eq!(verification.pack.id, "verification");
        assert!(!verification.graph_brief.available);

        let error = build_context_pack_response(&summary, Some("missing")).unwrap_err();
        assert!(error.to_string().contains("pack not found"));
    }
}
