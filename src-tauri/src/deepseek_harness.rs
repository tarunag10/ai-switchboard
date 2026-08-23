//! Experimental DeepSeek Harness (`dsh`) adapter.
//!
//! Upstream is explicitly a developer preview, so this adapter is fail-closed:
//! only the exact CLI/schema snapshot verified below may be changed. The
//! supported seam is dsh's documented home-level Cordis patch layer, not a
//! patch to Harness core. Unknown versions and ambiguous user patches remain
//! available as guided setup but are never written.

use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde_yaml::Value;
use sha2::{Digest, Sha256};

use crate::client_adapter_contract::{
    validate_plan_and_consent, ApplyReceipt, CleanupReport, CodingClientAdapter, ConfigDiff,
    ConfigPlan, ConfigPlanAction, ConsentToken, DetectionResult, ManagedFootprint, RollbackReport,
    VerificationReport, CODING_CLIENT_ADAPTER_CONTRACT_VERSION,
};
use crate::managed_files::{remove_managed_block, upsert_managed_block};
use crate::models::{ManagedFootprintItem, SwitchboardMode};
use crate::process_runner::run_command_capture_with_timeout;

pub(crate) const DSH_ADAPTER_ID: &str = "deepseek_harness";
pub(crate) const DSH_SUPPORTED_VERSION: &str = "0.1.0-rc.5";
pub(crate) const DSH_SUPPORTED_UPSTREAM_SHA: &str = "47f943859bef60e4160492346772ded9b24f765a";
pub(crate) const DSH_UPSTREAM_REPOSITORY: &str = "https://github.com/deepseek-ai/deepseek-harness";
pub(crate) const DSH_UPSTREAM_HOME_PATCH_SOURCE: &str = "https://github.com/deepseek-ai/deepseek-harness/blob/47f943859bef60e4160492346772ded9b24f765a/apps/cli/src/profile-boot.ts";
pub(crate) const DSH_UPSTREAM_LLM_SCHEMA_SOURCE: &str = "https://github.com/deepseek-ai/deepseek-harness/blob/47f943859bef60e4160492346772ded9b24f765a/packages/llm/llm-deepseek/README.md";

const DSH_PATCH_FILE: &str = "cordis.patch.yml";
const DSH_VERSION_TIMEOUT: Duration = Duration::from_secs(2);
const DSH_MANAGED_BLOCK_ID: &str = "deepseek-harness";
const SWITCHBOARD_BASE_URL: &str = "http://127.0.0.1:6767/v1";
const DSH_PATCH_BODY: &str = r#"- id: llm-deepseek
  config:
    baseURL: http://127.0.0.1:6767/v1"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DshReadiness {
    Managed,
    Guided { reason: String },
}

#[derive(Debug, Clone)]
struct Inspection {
    installed: bool,
    version: Option<String>,
    patch_path: PathBuf,
    configured: bool,
    readiness: DshReadiness,
    evidence: Vec<String>,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct DeepSeekHarnessAdapter;

fn dsh_home() -> PathBuf {
    std::env::var_os("DSH_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join(".dsh")
        })
}

fn patch_path() -> PathBuf {
    dsh_home().join(DSH_PATCH_FILE)
}

fn dsh_binary() -> Option<PathBuf> {
    std::env::var_os("PATH")?
        .to_string_lossy()
        .split(':')
        .map(PathBuf::from)
        .map(|directory| directory.join("dsh"))
        .find(|candidate| candidate.is_file())
}

fn detected_version() -> (bool, Option<String>, String) {
    let Some(binary) = dsh_binary() else {
        return (
            false,
            None,
            "dsh was not found on PATH; setup remains guided.".to_string(),
        );
    };
    match run_command_capture_with_timeout(
        &binary,
        &["--version"],
        Path::new("."),
        DSH_VERSION_TIMEOUT,
    ) {
        Ok((stdout, stderr)) => {
            let version = parse_dsh_version(&stdout, &stderr);
            let version_found = version.is_some();
            (
                true,
                version,
                if version_found {
                    "dsh --version completed without loading a profile or credentials.".to_string()
                } else {
                    "dsh --version output was ambiguous or did not contain an explicit version; setup remains guided.".to_string()
                },
            )
        }
        Err(error) => (
            true,
            None,
            format!("dsh version inspection failed safely without reading credentials: {error}"),
        ),
    }
}

fn parse_dsh_version(stdout: &str, stderr: &str) -> Option<String> {
    fn normalize(value: &str) -> Option<String> {
        let trimmed = value.trim().trim_matches(|character: char| {
            !character.is_ascii_alphanumeric() && character != '.' && character != '-'
        });
        let normalized = trimmed.strip_prefix('v').unwrap_or(trimmed);
        let mut pieces = normalized.split('-');
        let Some(release) = pieces.next() else {
            return None;
        };
        let numbers = release.split('.').collect::<Vec<_>>();
        if numbers.len() != 3 || numbers.iter().any(|part| part.is_empty() || !part.chars().all(|character| character.is_ascii_digit())) {
            return None;
        }
        if pieces.any(|part| part.is_empty() || !part.chars().all(|character| character.is_ascii_alphanumeric() || character == '.')) {
            return None;
        }
        Some(normalized.to_string())
    }

    let mut exact = Vec::new();
    let mut contextual = Vec::new();
    for text in [stdout, stderr] {
        for line in text.lines() {
            if let Some(version) = normalize(line) {
                exact.push(version);
            } else if line.to_ascii_lowercase().contains("dsh") {
                for token in line.split_whitespace() {
                    if let Some(version) = normalize(token) {
                        contextual.push(version);
                    }
                }
            }
        }
    }
    let candidates = if exact.is_empty() { contextual } else { exact };
    let mut unique = candidates;
    unique.sort();
    unique.dedup();
    (unique.len() == 1).then(|| unique.remove(0))
}

fn exact_managed_block_present(raw: &str) -> bool {
    let start = "# >>> ai-switchboard:deepseek-harness >>>";
    let end = "# <<< ai-switchboard:deepseek-harness <<<";
    raw.find(start)
        .and_then(|start_index| {
            raw[start_index + start.len()..]
                .find(end)
                .map(|relative_end| (start_index, start_index + start.len() + relative_end))
        })
        .is_some_and(|(start_index, end_index)| {
            raw[start_index..end_index].trim_start_matches(start).trim() == DSH_PATCH_BODY
        })
}

fn content_outside_managed_block(raw: &str) -> String {
    let start = "# >>> ai-switchboard:deepseek-harness >>>";
    let end = "# <<< ai-switchboard:deepseek-harness <<<";
    let Some(start_index) = raw.find(start) else {
        return raw.to_string();
    };
    let body_start = start_index + start.len();
    let Some(relative_end) = raw[body_start..].find(end) else {
        return raw.to_string();
    };
    let end_index = body_start + relative_end + end.len();
    format!("{}\n{}", &raw[..start_index], &raw[end_index..])
}

fn has_user_owned_deepseek_base_url(value: &Value) -> bool {
    let Some(rows) = value.as_sequence() else {
        return false;
    };
    let key = |name: &str| Value::String(name.to_string());
    rows.iter().any(|row| {
        let Some(row) = row.as_mapping() else {
            return false;
        };
        if row.get(key("id")).and_then(Value::as_str) != Some("llm-deepseek") {
            return false;
        }
        row.get(key("config"))
            .and_then(Value::as_mapping)
            .is_some_and(|config| config.contains_key(key("baseURL")))
    })
}

fn user_patch_shape_is_safe(raw: &str) -> Result<()> {
    if raw.trim().is_empty() {
        return Ok(());
    }
    let value =
        serde_yaml::from_str::<Value>(raw).context("the dsh home patch is not valid YAML")?;
    if !value.is_sequence() {
        return Err(anyhow!(
            "the dsh home patch is not a top-level Cordis PatchOptions sequence"
        ));
    }

    let unmanaged = if exact_managed_block_present(raw) {
        content_outside_managed_block(raw)
    } else {
        raw.to_string()
    };
    if unmanaged.contains("ai-switchboard:") {
        return Err(anyhow!(
            "an unmanaged ai-switchboard route or marker already exists in the dsh home patch"
        ));
    }
    if !unmanaged.trim().is_empty() {
        let unmanaged_value = serde_yaml::from_str::<Value>(&unmanaged)
            .context("the unmanaged dsh home patch is not valid YAML")?;
        if has_user_owned_deepseek_base_url(&unmanaged_value) {
            return Err(anyhow!(
                "a user-owned llm-deepseek baseURL patch already exists and will not be overwritten"
            ));
        }
    }
    Ok(())
}

fn inspect() -> Inspection {
    let (installed, version, version_evidence) = detected_version();
    let path = patch_path();
    let (raw, read_error) = if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(raw) => (raw, None),
            Err(error) => (
                String::new(),
                Some(format!(
                    "the dsh home patch could not be inspected safely: {error}"
                )),
            ),
        }
    } else {
        (String::new(), None)
    };
    let configured = exact_managed_block_present(&raw);
    let mut evidence = vec![
        version_evidence,
        format!(
            "Supported upstream pin: dsh {DSH_SUPPORTED_VERSION}, commit {DSH_SUPPORTED_UPSTREAM_SHA}."
        ),
        format!("Source: {DSH_UPSTREAM_REPOSITORY}"),
        format!("Home patch seam: {DSH_UPSTREAM_HOME_PATCH_SOURCE}"),
        format!("LLM adapter schema: {DSH_UPSTREAM_LLM_SCHEMA_SOURCE}"),
        "Inspection never reads $DSH_HOME/.credentials.yaml, API keys, tokens, or provider account state."
            .to_string(),
    ];

    let readiness = if !installed {
        DshReadiness::Guided {
            reason: "dsh is not installed on PATH.".to_string(),
        }
    } else if version.as_deref() != Some(DSH_SUPPORTED_VERSION) {
        DshReadiness::Guided {
            reason: format!(
                "Installed dsh version {} does not exactly match the verified developer-preview version {DSH_SUPPORTED_VERSION}.",
                version.as_deref().unwrap_or("unknown")
            ),
        }
    } else if let Some(reason) = read_error {
        DshReadiness::Guided { reason }
    } else if let Err(error) = user_patch_shape_is_safe(&raw) {
        DshReadiness::Guided {
            reason: format!("The supported dsh patch seam is ambiguous: {error}"),
        }
    } else {
        DshReadiness::Managed
    };
    if let DshReadiness::Guided { reason } = &readiness {
        evidence.push(format!(
            "Guided mode (no writes): {reason} Upgrade the adapter pin only after upstream schema review."
        ));
    } else {
        evidence.push(
            "Exact upstream version and the supported top-level PatchOptions sequence were verified."
                .to_string(),
        );
    }
    Inspection {
        installed,
        version,
        patch_path: path,
        configured,
        readiness,
        evidence,
    }
}

fn readiness_error(readiness: &DshReadiness) -> Option<anyhow::Error> {
    match readiness {
        DshReadiness::Managed => None,
        DshReadiness::Guided { reason } => Some(anyhow!(
            "DeepSeek Harness is guided-only and no configuration was written: {reason}"
        )),
    }
}

fn proxy_reachable() -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], 6767));
    TcpStream::connect_timeout(&address, Duration::from_millis(150)).is_ok()
}

fn plan_id(mode: &SwitchboardMode, version: Option<&str>, raw: &str) -> String {
    let input = format!(
        "{DSH_ADAPTER_ID}|{mode:?}|{}|{raw}",
        version.unwrap_or("unknown")
    );
    let digest = Sha256::digest(input.as_bytes());
    let suffix = digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("dsh-{suffix}")
}

impl CodingClientAdapter for DeepSeekHarnessAdapter {
    fn id(&self) -> &'static str {
        DSH_ADAPTER_ID
    }

    fn detect(&self) -> DetectionResult {
        let inspection = inspect();
        DetectionResult {
            contract_version: CODING_CLIENT_ADAPTER_CONTRACT_VERSION,
            client_id: DSH_ADAPTER_ID.to_string(),
            installed: inspection.installed,
            configured: inspection.configured,
            sources: vec!["PATH: dsh".to_string(), "$DSH_HOME or ~/.dsh".to_string()],
            evidence: inspection.evidence,
            config_locations: vec!["$DSH_HOME/cordis.patch.yml".to_string()],
            lifecycle_fixture_complete: true,
        }
    }

    fn plan(&self, mode: SwitchboardMode) -> Result<ConfigPlan> {
        let inspection = inspect();
        let raw = std::fs::read_to_string(&inspection.patch_path).unwrap_or_default();
        let action = if matches!(mode, SwitchboardMode::Headroom | SwitchboardMode::Full) {
            ConfigPlanAction::ApplyManagedRouting
        } else {
            ConfigPlanAction::CleanupManagedRouting
        };
        let id = plan_id(&mode, inspection.version.as_deref(), &raw);
        let after = if matches!(action, ConfigPlanAction::ApplyManagedRouting) {
            format!(
                "Route dsh's native llm-deepseek adapter through {SWITCHBOARD_BASE_URL}; preserve its credential reference, model catalog, and all unrelated patch rows."
            )
        } else {
            "Remove only the marker-bounded AI Switchboard dsh patch; preserve all other Cordis patches."
                .to_string()
        };
        Ok(ConfigPlan {
            plan_id: id.clone(),
            client_id: DSH_ADAPTER_ID.to_string(),
            mode,
            action,
            diffs: vec![ConfigDiff {
                target: "$DSH_HOME/cordis.patch.yml".to_string(),
                before: if inspection.configured {
                    "Switchboard-managed dsh provider patch is present; unrelated patch values and secrets are not displayed."
                        .to_string()
                } else {
                    "No Switchboard-managed dsh provider patch is present; unrelated patch values and secrets are not displayed."
                        .to_string()
                },
                after,
                managed_boundary: "# >>> ai-switchboard:deepseek-harness >>> … # <<< ai-switchboard:deepseek-harness <<<"
                    .to_string(),
            }],
            confirmation_phrase: format!("APPLY EXPERIMENTAL DSH PLAN {id}"),
            reversible: true,
            evidence: inspection.evidence,
        })
    }

    fn apply(&self, plan: &ConfigPlan, consent: ConsentToken) -> Result<ApplyReceipt> {
        validate_plan_and_consent(self.id(), plan, &consent)?;
        if plan.action != ConfigPlanAction::ApplyManagedRouting {
            return Err(anyhow!("dsh cleanup plans must use cleanup_off_mode."));
        }
        let current_plan = self.plan(plan.mode.clone())?;
        if current_plan.plan_id != plan.plan_id {
            return Err(anyhow!(
                "The dsh version or patch changed after preview; review a new dry-run plan."
            ));
        }
        let inspection = inspect();
        if let Some(error) = readiness_error(&inspection.readiness) {
            return Err(error);
        }
        let (changed, backup) =
            upsert_managed_block(&inspection.patch_path, DSH_MANAGED_BLOCK_ID, DSH_PATCH_BODY)?;
        let verification = self.verify()?;
        if !verification.verified {
            let _ = remove_managed_block(&inspection.patch_path, DSH_MANAGED_BLOCK_ID);
            return Err(anyhow!(
                "dsh patch was applied but verification failed; the managed block was removed."
            ));
        }
        Ok(ApplyReceipt {
            client_id: DSH_ADAPTER_ID.to_string(),
            plan_id: plan.plan_id.clone(),
            applied_at: Utc::now().to_rfc3339(),
            changed_files: changed
                .then(|| inspection.patch_path.display().to_string())
                .into_iter()
                .collect(),
            backup_files: backup
                .into_iter()
                .map(|path| path.display().to_string())
                .collect(),
            already_configured: !changed,
            summary: if changed {
                "Configured dsh's native llm-deepseek baseURL through its supported home patch seam."
                    .to_string()
            } else {
                "The exact Switchboard-managed dsh provider patch was already present.".to_string()
            },
            verification,
        })
    }

    fn verify(&self) -> Result<VerificationReport> {
        let inspection = inspect();
        let reachable = proxy_reachable();
        let version_ok = inspection.version.as_deref() == Some(DSH_SUPPORTED_VERSION);
        let schema_ok = matches!(inspection.readiness, DshReadiness::Managed);
        let verified = inspection.installed && version_ok && schema_ok && inspection.configured;
        let mut checks = inspection.evidence;
        checks.push(format!(
            "Upstream version evidence: installed={}, expected={} at commit {}.",
            inspection.version.as_deref().unwrap_or("unknown"),
            DSH_SUPPORTED_VERSION,
            DSH_SUPPORTED_UPSTREAM_SHA
        ));
        checks.push(if reachable {
            "AI Switchboard loopback endpoint is reachable on 127.0.0.1:6767.".to_string()
        } else {
            "Managed dsh config is independently verifiable; the loopback endpoint is not currently reachable."
                .to_string()
        });
        let mut failures = Vec::new();
        if !version_ok {
            failures.push(
                "dsh version is unknown or outside the exact supported preview pin.".to_string(),
            );
        }
        if !schema_ok {
            failures.push(
                "dsh patch schema is ambiguous; guided mode is active and writes are disabled."
                    .to_string(),
            );
        }
        if !inspection.configured {
            failures.push("Switchboard-managed dsh provider patch is absent.".to_string());
        }
        Ok(VerificationReport {
            client_id: DSH_ADAPTER_ID.to_string(),
            verified,
            proxy_reachable: reachable,
            checks,
            failures,
        })
    }

    fn rollback(&self, receipt: &ApplyReceipt) -> Result<RollbackReport> {
        if receipt.client_id != DSH_ADAPTER_ID {
            return Err(anyhow!("Apply receipt belongs to a different adapter."));
        }
        let path = patch_path();
        if !receipt.changed_files.is_empty()
            && receipt.changed_files != vec![path.display().to_string()]
        {
            return Err(anyhow!("dsh receipt contains an unexpected changed path."));
        }
        let mut actions = Vec::new();
        if receipt.already_configured || receipt.changed_files.is_empty() {
            actions.push(
                "Apply made no change; preserved the pre-existing managed patch.".to_string(),
            );
        } else if let Some(backup) = receipt.backup_files.first().map(PathBuf::from) {
            let expected_prefix = format!("{}.headroom-backup-", path.display());
            if !backup.display().to_string().starts_with(&expected_prefix) || !backup.exists() {
                return Err(anyhow!(
                    "dsh rollback backup is missing or outside the expected sibling path."
                ));
            }
            std::fs::copy(&backup, &path)
                .with_context(|| format!("restoring dsh home patch backup {}", backup.display()))?;
            actions.push("Restored the exact pre-apply dsh home patch backup.".to_string());
        } else {
            remove_managed_block(&path, DSH_MANAGED_BLOCK_ID)?;
            if path.exists()
                && std::fs::read_to_string(&path)
                    .map(|raw| raw.trim().is_empty())
                    .unwrap_or(false)
            {
                std::fs::remove_file(&path)
                    .with_context(|| format!("removing empty dsh patch {}", path.display()))?;
            }
            actions.push(
                "Removed the managed block from the newly created dsh home patch.".to_string(),
            );
        }
        Ok(RollbackReport {
            client_id: DSH_ADAPTER_ID.to_string(),
            plan_id: receipt.plan_id.clone(),
            rolled_back: true,
            actions,
            preserved_backups: receipt.backup_files.clone(),
            verification: self.verify()?,
        })
    }

    fn cleanup_off_mode(&self) -> Result<CleanupReport> {
        let path = patch_path();
        let removed = remove_managed_block(&path, DSH_MANAGED_BLOCK_ID)?;
        if path.exists()
            && std::fs::read_to_string(&path)
                .map(|raw| raw.trim().is_empty())
                .unwrap_or(false)
        {
            std::fs::remove_file(&path)
                .with_context(|| format!("removing empty dsh patch {}", path.display()))?;
        }
        let verification = self.verify()?;
        if verification.verified {
            return Err(anyhow!(
                "dsh Off cleanup returned but the managed provider patch still verifies."
            ));
        }
        Ok(CleanupReport {
            client_id: DSH_ADAPTER_ID.to_string(),
            cleaned: true,
            actions: vec![if removed {
                "Removed only the Switchboard-owned dsh home patch block.".to_string()
            } else {
                "No Switchboard-owned dsh home patch block was present.".to_string()
            }],
            verification,
        })
    }

    fn footprint(&self) -> ManagedFootprint {
        let path = patch_path();
        ManagedFootprint {
            client_id: DSH_ADAPTER_ID.to_string(),
            secret_values_included: false,
            items: vec![ManagedFootprintItem {
                id: "deepseek-harness-home-patch".to_string(),
                category: "client-config".to_string(),
                path: "$DSH_HOME/cordis.patch.yml".to_string(),
                exists: path.exists(),
                managed: true,
                action: "Marker-bounded llm-deepseek baseURL route; dsh credentials, models, and upstream core are never managed."
                    .to_string(),
                reversible: true,
                backup_paths: Vec::new(),
                notes: vec![
                    format!("Exact supported dsh version: {DSH_SUPPORTED_VERSION}."),
                    "Unknown or breaking versions degrade to guided/no-write mode.".to_string(),
                ],
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::ffi::OsString;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    struct FixtureHome {
        _root: tempfile::TempDir,
        home: PathBuf,
        previous: Vec<(&'static str, Option<OsString>)>,
    }

    impl FixtureHome {
        fn new(version: &str) -> Self {
            let root = tempfile::tempdir().expect("fixture root");
            let home = root.path().join("home");
            let bin = root.path().join("bin");
            fs::create_dir_all(&home).expect("home");
            fs::create_dir_all(&bin).expect("bin");
            let executable = bin.join("dsh");
            fs::write(
                &executable,
                format!("#!/bin/sh\nprintf '%s\\n' '{version}'\n"),
            )
            .expect("fake dsh");
            #[cfg(unix)]
            {
                let mut permissions = fs::metadata(&executable).unwrap().permissions();
                permissions.set_mode(0o700);
                fs::set_permissions(&executable, permissions).unwrap();
            }
            let previous = ["HOME", "DSH_HOME", "PATH"]
                .into_iter()
                .map(|key| (key, std::env::var_os(key)))
                .collect::<Vec<_>>();
            std::env::set_var("HOME", &home);
            std::env::set_var("DSH_HOME", home.join(".dsh"));
            std::env::set_var("PATH", &bin);
            Self {
                _root: root,
                home,
                previous,
            }
        }

        fn patch(&self) -> PathBuf {
            self.home.join(".dsh").join(DSH_PATCH_FILE)
        }
    }

    impl Drop for FixtureHome {
        fn drop(&mut self) {
            for (key, value) in self.previous.drain(..) {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    #[test]
    #[serial]
    fn exact_preview_requires_exact_consent_and_does_not_write() {
        let fixture = FixtureHome::new(DSH_SUPPORTED_VERSION);
        let adapter = DeepSeekHarnessAdapter;
        let plan = adapter.plan(SwitchboardMode::Full).expect("plan");
        assert!(!fixture.patch().exists());
        assert!(ConsentToken::issue(&plan, "wrong").is_err());
        assert!(ConsentToken::issue(&plan, &plan.confirmation_phrase).is_ok());
        assert!(plan
            .evidence
            .iter()
            .any(|line| line.contains(DSH_SUPPORTED_UPSTREAM_SHA)));
    }

    #[test]
    #[serial]
    fn supported_version_apply_verify_rollback_preserves_user_patch_exactly() {
        let fixture = FixtureHome::new(DSH_SUPPORTED_VERSION);
        let patch = fixture.patch();
        fs::create_dir_all(patch.parent().unwrap()).unwrap();
        let original = "- id: user-plugin\n  config:\n    note: keep-me\n";
        fs::write(&patch, original).unwrap();
        let adapter = DeepSeekHarnessAdapter;
        let plan = adapter.plan(SwitchboardMode::Full).unwrap();
        let consent = ConsentToken::issue(&plan, &plan.confirmation_phrase).unwrap();
        let receipt = adapter.apply(&plan, consent).unwrap();
        let applied = fs::read_to_string(&patch).unwrap();
        assert!(applied.starts_with(original));
        assert!(applied.contains(SWITCHBOARD_BASE_URL));
        assert!(receipt.verification.verified);
        assert!(!receipt.backup_files.is_empty());
        let rollback = adapter.rollback(&receipt).unwrap();
        assert!(rollback.rolled_back);
        assert_eq!(fs::read_to_string(&patch).unwrap(), original);
    }

    #[test]
    #[serial]
    fn unsupported_version_is_guided_and_never_writes() {
        let fixture = FixtureHome::new("0.1.0-rc.6");
        let adapter = DeepSeekHarnessAdapter;
        let plan = adapter.plan(SwitchboardMode::Full).unwrap();
        assert!(plan
            .evidence
            .iter()
            .any(|line| line.contains("Guided mode")));
        let consent = ConsentToken::issue(&plan, &plan.confirmation_phrase).unwrap();
        let error = adapter.apply(&plan, consent).unwrap_err();
        assert!(error.to_string().contains("guided-only"));
        assert!(!fixture.patch().exists());
    }

    #[test]
    #[serial]
    fn hanging_version_probe_is_bounded_and_guided() {
        let fixture = FixtureHome::new(DSH_SUPPORTED_VERSION);
        let executable = fixture.home.parent().unwrap().join("bin/dsh");
        fs::write(&executable, "#!/bin/sh\nsleep 30\n").unwrap();
        let adapter = DeepSeekHarnessAdapter;
        let plan = adapter.plan(SwitchboardMode::Full).unwrap();
        assert!(plan.evidence.iter().any(|line| line.contains("failed safely")));
        let consent = ConsentToken::issue(&plan, &plan.confirmation_phrase).unwrap();
        assert!(adapter.apply(&plan, consent).is_err());
        assert!(!fixture.patch().exists());
    }

    #[test]
    fn version_parser_requires_one_explicit_unambiguous_candidate() {
        assert_eq!(
            parse_dsh_version("v0.1.0-rc.5\n", ""),
            Some("0.1.0-rc.5".to_string())
        );
        assert_eq!(
            parse_dsh_version("warning: dsh unavailable\n", "dsh version 0.1.0-rc.5\n"),
            Some("0.1.0-rc.5".to_string())
        );
        assert_eq!(parse_dsh_version("warning 0.1.0-rc.5\n", ""), None);
        assert_eq!(
            parse_dsh_version("0.1.0-rc.5\n0.1.0-rc.4\n", ""),
            None
        );
    }

    #[test]
    #[serial]
    fn unknown_or_conflicting_schema_is_guided_and_never_rewritten() {
        let fixture = FixtureHome::new(DSH_SUPPORTED_VERSION);
        let patch = fixture.patch();
        fs::create_dir_all(patch.parent().unwrap()).unwrap();
        let original = "llm-pi-ai:\n  providers:\n    ai-switchboard: user-owned\n";
        fs::write(&patch, original).unwrap();
        let adapter = DeepSeekHarnessAdapter;
        let plan = adapter.plan(SwitchboardMode::Full).unwrap();
        let consent = ConsentToken::issue(&plan, &plan.confirmation_phrase).unwrap();
        assert!(adapter.apply(&plan, consent).is_err());
        assert_eq!(fs::read_to_string(&patch).unwrap(), original);
    }

    #[test]
    #[serial]
    fn user_owned_native_base_url_is_guided_and_never_overwritten() {
        let fixture = FixtureHome::new(DSH_SUPPORTED_VERSION);
        let patch = fixture.patch();
        fs::create_dir_all(patch.parent().unwrap()).unwrap();
        let original = "- id: llm-deepseek\n  config:\n    baseURL: https://gateway.example/v1\n";
        fs::write(&patch, original).unwrap();
        let adapter = DeepSeekHarnessAdapter;
        let plan = adapter.plan(SwitchboardMode::Full).unwrap();
        let consent = ConsentToken::issue(&plan, &plan.confirmation_phrase).unwrap();
        let error = adapter.apply(&plan, consent).unwrap_err();
        assert!(error.to_string().contains("guided-only"));
        assert_eq!(fs::read_to_string(&patch).unwrap(), original);
    }

    #[test]
    #[serial]
    fn off_cleanup_removes_only_managed_block_and_keeps_credentials_untouched() {
        let fixture = FixtureHome::new(DSH_SUPPORTED_VERSION);
        let credentials = fixture.home.join(".dsh").join(".credentials.yaml");
        fs::create_dir_all(credentials.parent().unwrap()).unwrap();
        fs::write(&credentials, "deepseek: super-secret\n").unwrap();
        let adapter = DeepSeekHarnessAdapter;
        let plan = adapter.plan(SwitchboardMode::Full).unwrap();
        let consent = ConsentToken::issue(&plan, &plan.confirmation_phrase).unwrap();
        adapter.apply(&plan, consent).unwrap();
        adapter.cleanup_off_mode().unwrap();
        assert_eq!(
            fs::read_to_string(credentials).unwrap(),
            "deepseek: super-secret\n"
        );
        assert!(!fixture.patch().exists());
    }
}
