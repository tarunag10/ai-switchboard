//! Cursor native-provider safety gate.
//!
//! Cursor documents API-key setup through its Settings > Models UI.  It does
//! not document a stable, file-backed provider/model/base-url schema that a
//! third-party tool can safely mutate.  This module therefore deliberately
//! limits itself to profile-surface discovery and evidence reporting.  It
//! never opens settings.json, profile data, globalStorage, state.vscdb, or any
//! credential-bearing file.  A future adapter can be promoted by replacing
//! the gate only after a documented schema and fixture-backed lifecycle are
//! available.

use std::path::{Path, PathBuf};

pub(crate) const CURSOR_NATIVE_SCHEMA_ID: &str = "cursor-native-provider-schema";
pub(crate) const CURSOR_API_KEYS_DOCS_URL: &str =
    "https://cursor.com/help/models-and-usage/api-keys";
pub(crate) const CURSOR_NATIVE_GATE_REASON: &str =
    "Cursor documents provider API keys in Settings > Models, but does not document a stable on-disk provider/model/base-url schema for safe third-party writes.";

const SETTINGS_CANDIDATES: [&str; 6] = [
    "User/settings.json",
    "User/settings.jsonc",
    "settings.json",
    "settings.jsonc",
    "profiles/User/settings.json",
    "profiles/User/settings.jsonc",
];

/// A path-level Cursor profile discovery result.  The adapter intentionally
/// carries no settings values or key names, so secrets and account state can
/// never leak into this evidence object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CursorProfileSurface {
    pub(crate) root: PathBuf,
    pub(crate) settings_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CursorNativeSchemaAssessment {
    pub(crate) schema_id: &'static str,
    pub(crate) supported: bool,
    pub(crate) reason: &'static str,
    pub(crate) docs_url: &'static str,
    pub(crate) surfaces: Vec<CursorProfileSurface>,
}

/// Discover only documented editor settings filenames beneath a Cursor app
/// profile.  This function uses metadata/path checks and does not read file
/// contents.  In particular, it never traverses `globalStorage` or profile
/// databases.
pub(crate) fn discover_profile_surfaces(home: &Path) -> Vec<CursorProfileSurface> {
    let root = home
        .join("Library")
        .join("Application Support")
        .join("Cursor");
    if !root.is_dir() {
        return Vec::new();
    }

    let mut surfaces = Vec::new();
    let mut roots = vec![root.clone()];
    let profiles_dir = root.join("User").join("profiles");
    if let Ok(entries) = std::fs::read_dir(&profiles_dir) {
        roots.extend(entries.flatten().filter_map(|entry| {
            let path = entry.path();
            path.is_dir().then_some(path)
        }));
    }

    for profile_root in roots {
        let settings_files = SETTINGS_CANDIDATES
            .iter()
            .map(|relative| profile_root.join(relative))
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        if !settings_files.is_empty() {
            surfaces.push(CursorProfileSurface {
                root: profile_root,
                settings_files,
            });
        }
    }
    surfaces
}

/// Return the native-provider assessment used by Doctor/connector detection.
/// `supported` is intentionally false until Cursor publishes an allowlisted
/// file schema and Switchboard has fixture-home apply/verify/rollback/Off
/// proof for that schema.
pub(crate) fn assess_native_schema(home: &Path) -> CursorNativeSchemaAssessment {
    CursorNativeSchemaAssessment {
        schema_id: CURSOR_NATIVE_SCHEMA_ID,
        supported: false,
        reason: CURSOR_NATIVE_GATE_REASON,
        docs_url: CURSOR_API_KEYS_DOCS_URL,
        surfaces: discover_profile_surfaces(home),
    }
}

pub(crate) fn evidence_lines(assessment: &CursorNativeSchemaAssessment) -> Vec<String> {
    let mut evidence = vec![format!(
        "Cursor native schema {}: {}.",
        assessment.schema_id,
        if assessment.supported {
            "allowlisted"
        } else {
            "not allowlisted"
        }
    )];
    if assessment.surfaces.is_empty() {
        evidence.push("Cursor native settings surfaces: none detected yet.".to_string());
    } else {
        for surface in &assessment.surfaces {
            evidence.push(format!(
                "Cursor native settings surface: {} ({})",
                surface.root.display(),
                surface
                    .settings_files
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    if !assessment.supported {
        evidence.push(format!(
            "Cursor native provider/model/base-url writes remain blocked: {} See {}.",
            assessment.reason, assessment.docs_url
        ));
        evidence.push(
            "Switchboard does not read or write Cursor settings contents, globalStorage, state.vscdb, account data, model credentials, or secrets.".to_string(),
        );
    }
    evidence
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture_home(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "mac-ai-switchboard-cursor-native-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("fixture home");
        path
    }

    #[test]
    fn discovers_global_and_named_profiles_without_opening_storage() {
        let home = fixture_home("discovery");
        let cursor = home.join("Library/Application Support/Cursor");
        let global = cursor.join("User/settings.json");
        let profile = cursor.join("User/profiles/work/settings.json");
        let forbidden = cursor.join("User/globalStorage/state.vscdb");
        fs::create_dir_all(global.parent().expect("global parent")).expect("global dir");
        fs::create_dir_all(profile.parent().expect("profile parent")).expect("profile dir");
        fs::create_dir_all(forbidden.parent().expect("storage parent")).expect("storage dir");
        fs::write(&global, r#"{"cursor.model":"secret-model"}"#).expect("global settings");
        fs::write(&profile, r#"{"cursor.model":"profile-model"}"#).expect("profile settings");
        fs::write(&forbidden, "credential database").expect("forbidden storage");

        let assessment = assess_native_schema(&home);
        assert!(!assessment.supported);
        assert_eq!(assessment.surfaces.len(), 2);
        assert!(assessment
            .surfaces
            .iter()
            .flat_map(|surface| surface.settings_files.iter())
            .any(|path| path == &global));
        assert!(assessment
            .surfaces
            .iter()
            .flat_map(|surface| surface.settings_files.iter())
            .any(|path| path == &profile));
        assert!(!assessment
            .surfaces
            .iter()
            .flat_map(|surface| surface.settings_files.iter())
            .any(|path| path == &forbidden));
        let evidence = evidence_lines(&assessment).join(" ");
        assert!(!evidence.contains("secret-model"));
        assert!(!evidence.contains("profile-model"));
        assert!(!evidence.contains("credential database"));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn missing_profile_is_a_precise_schema_gate() {
        let home = fixture_home("missing");
        let assessment = assess_native_schema(&home);
        assert_eq!(assessment.schema_id, CURSOR_NATIVE_SCHEMA_ID);
        assert!(!assessment.supported);
        assert!(assessment.reason.contains("does not document"));
        assert_eq!(assessment.docs_url, CURSOR_API_KEYS_DOCS_URL);
        let evidence = evidence_lines(&assessment).join(" ");
        assert!(evidence.contains("none detected yet"));
        assert!(evidence.contains("remain blocked"));
        let _ = fs::remove_dir_all(home);
    }
}
