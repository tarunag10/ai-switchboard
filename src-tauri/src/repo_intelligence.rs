use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;

use crate::models::{RepoContextPack, RepoFileRole, RepoFileSignal, RepoIntelligenceSummary};
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
    let role = if bytes > MAX_INDEXED_FILE_BYTES {
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

fn estimate_tokens(bytes: u64) -> u64 {
    std::cmp::max(1, bytes.saturating_add(3) / 4)
}

fn lockfile_name(name: &str) -> bool {
    matches!(
        name,
        "Cargo.lock" | "package-lock.json" | "pnpm-lock.yaml" | "yarn.lock" | "bun.lockb"
    )
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
