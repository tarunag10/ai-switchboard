use std::path::Path;
use std::process::Command;

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReleaseReadinessReportPayload {
    pub(crate) report_path: String,
    pub(crate) report: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReleaseEvidenceCommandResult {
    pub(crate) command_id: String,
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) summary_path: Option<String>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

struct ReleaseEvidenceCommandSpec {
    label: &'static str,
    command: &'static str,
    steps: &'static [(&'static str, &'static [&'static str])],
    summary_path: Option<&'static str>,
}

pub(crate) fn load_release_readiness_report_from(
    path: &Path,
) -> Result<ReleaseReadinessReportPayload, String> {
    let report_path = path.to_string_lossy().into_owned();
    match std::fs::read_to_string(path) {
        Ok(raw) => {
            let report = serde_json::from_str(&raw)
                .map_err(|err| format!("release readiness report is invalid JSON: {err}"))?;
            Ok(ReleaseReadinessReportPayload {
                report_path,
                report: Some(report),
            })
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Ok(ReleaseReadinessReportPayload {
                report_path,
                report: None,
            })
        }
        Err(err) => Err(format!("failed to read release readiness report: {err}")),
    }
}

#[tauri::command]
pub fn load_release_readiness_report() -> Result<ReleaseReadinessReportPayload, String> {
    let path = std::env::current_dir()
        .map_err(|err| err.to_string())?
        .join("dist/release-readiness-report.json");
    load_release_readiness_report_from(&path)
}

#[tauri::command]
pub fn refresh_release_readiness_report() -> Result<ReleaseReadinessReportPayload, String> {
    let cwd = std::env::current_dir().map_err(|err| err.to_string())?;
    let output = Command::new("npm")
        .args(["run", "release:ready", "--", "--json"])
        .current_dir(&cwd)
        .output()
        .map_err(|err| format!("failed to run npm run release:ready: {err}"))?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = [stdout, stderr]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        return Err(if detail.is_empty() {
            format!("npm run release:ready failed with status {}", output.status)
        } else {
            format!(
                "npm run release:ready failed with status {}:\n{}",
                output.status, detail
            )
        });
    }

    load_release_readiness_report_from(&cwd.join("dist/release-readiness-report.json"))
}

#[tauri::command]
pub fn run_release_evidence_command(
    command_id: String,
) -> Result<ReleaseEvidenceCommandResult, String> {
    const STATIC_PREFLIGHT_STEPS: &[(&str, &[&str])] = &[("npm", &["run", "smoke:preflight"])];
    const DESKTOP_VALIDATION_STEPS: &[(&str, &[&str])] = &[
        ("npm", &["run", "fmt:desktop"]),
        ("npm", &["run", "test:desktop"]),
    ];
    const LOCAL_INSTALLED_SMOKE_STEPS: &[(&str, &[&str])] =
        &[("npm", &["run", "smoke:installed:local"])];
    const LOCAL_MODE_RELAUNCH_SMOKE_STEPS: &[(&str, &[&str])] = &[(
        "npm",
        &["run", "smoke:mode-relaunch:local", "--", "--confirm"],
    )];
    const ROLLBACK_CENTER_VALIDATION_STEPS: &[(&str, &[&str])] =
        &[("npm", &["run", "smoke:rollback:local"])];
    const DOCTOR_REPAIR_VALIDATION_STEPS: &[(&str, &[&str])] =
        &[("npm", &["run", "smoke:doctor-repair:local"])];
    const UNINSTALL_VALIDATION_STEPS: &[(&str, &[&str])] =
        &[("npm", &["run", "smoke:uninstall:local"])];
    const REPO_INTELLIGENCE_VALIDATION_STEPS: &[(&str, &[&str])] =
        &[("npm", &["run", "smoke:repo-intelligence:local"])];
    const REPO_MEMORY_MCP_VALIDATION_STEPS: &[(&str, &[&str])] =
        &[("npm", &["run", "smoke:repo-memory-mcp:local"])];
    const LOCAL_ONLY_NETWORK_VALIDATION_STEPS: &[(&str, &[&str])] =
        &[("npm", &["run", "smoke:local-only:local"])];
    const LOCAL_DMG_BUILD_INSTALL_STEPS: &[(&str, &[&str])] =
        &[("npm", &["run", "build:mac:local-install"])];
    const RELEASE_REPORT_STEPS: &[(&str, &[&str])] = &[("npm", &["run", "release:report"])];

    let spec = match command_id.as_str() {
        "static-preflight" => ReleaseEvidenceCommandSpec {
            label: "Static smoke preflight",
            command: "npm run smoke:preflight",
            steps: STATIC_PREFLIGHT_STEPS,
            summary_path: Some("dist/smoke-preflight-summary.md"),
        },
        "desktop-validation" => ReleaseEvidenceCommandSpec {
            label: "Desktop validation",
            command: "npm run fmt:desktop && npm run test:desktop",
            steps: DESKTOP_VALIDATION_STEPS,
            summary_path: None,
        },
        "local-installed-smoke" => ReleaseEvidenceCommandSpec {
            label: "Local installed smoke",
            command: "npm run smoke:installed:local",
            steps: LOCAL_INSTALLED_SMOKE_STEPS,
            summary_path: Some("dist/local-installed-smoke-summary.md"),
        },
        "local-mode-relaunch-smoke" => ReleaseEvidenceCommandSpec {
            label: "Local mode relaunch smoke",
            command: "npm run smoke:mode-relaunch:local -- --confirm",
            steps: LOCAL_MODE_RELAUNCH_SMOKE_STEPS,
            summary_path: Some("dist/local-mode-relaunch-smoke-summary.md"),
        },
        "rollback-center-validation" => ReleaseEvidenceCommandSpec {
            label: "Rollback Center validation",
            command: "npm run smoke:rollback:local",
            steps: ROLLBACK_CENTER_VALIDATION_STEPS,
            summary_path: Some("dist/local-rollback-validation-summary.md"),
        },
        "doctor-repair-validation" => ReleaseEvidenceCommandSpec {
            label: "Doctor repair validation",
            command: "npm run smoke:doctor-repair:local",
            steps: DOCTOR_REPAIR_VALIDATION_STEPS,
            summary_path: Some("dist/local-doctor-repair-validation-summary.md"),
        },
        "uninstall-validation" => ReleaseEvidenceCommandSpec {
            label: "Uninstall dry-run validation",
            command: "npm run smoke:uninstall:local",
            steps: UNINSTALL_VALIDATION_STEPS,
            summary_path: Some("dist/local-uninstall-validation-summary.md"),
        },
        "repo-intelligence-validation" => ReleaseEvidenceCommandSpec {
            label: "Repo Intelligence validation",
            command: "npm run smoke:repo-intelligence:local",
            steps: REPO_INTELLIGENCE_VALIDATION_STEPS,
            summary_path: Some("dist/local-repo-intelligence-validation-summary.md"),
        },
        "repo-memory-mcp-validation" => ReleaseEvidenceCommandSpec {
            label: "Repo Memory MCP validation",
            command: "npm run smoke:repo-memory-mcp:local",
            steps: REPO_MEMORY_MCP_VALIDATION_STEPS,
            summary_path: Some("dist/local-repo-memory-mcp-validation-summary.md"),
        },
        "local-only-network-validation" => ReleaseEvidenceCommandSpec {
            label: "Local-only network validation",
            command: "npm run smoke:local-only:local",
            steps: LOCAL_ONLY_NETWORK_VALIDATION_STEPS,
            summary_path: Some("dist/local-only-network-validation-summary.md"),
        },
        "local-dmg-build-install" => ReleaseEvidenceCommandSpec {
            label: "Local DMG build/install",
            command: "npm run build:mac:local-install",
            steps: LOCAL_DMG_BUILD_INSTALL_STEPS,
            summary_path: Some("dist/local-installed-smoke-summary.md"),
        },
        "release-report" => ReleaseEvidenceCommandSpec {
            label: "Release readiness report",
            command: "npm run release:report",
            steps: RELEASE_REPORT_STEPS,
            summary_path: Some("dist/release-readiness-report.md"),
        },
        _ => {
            return Err(
                "Release evidence execution is currently enabled only for static-preflight, desktop-validation, local-dmg-build-install, local-installed-smoke, local-mode-relaunch-smoke, rollback-center-validation, doctor-repair-validation, uninstall-validation, repo-intelligence-validation, repo-memory-mcp-validation, local-only-network-validation, and release-report."
                    .to_string(),
            )
        }
    };

    let cwd = std::env::current_dir().map_err(|err| err.to_string())?;
    let mut combined_stdout = Vec::new();
    let mut combined_stderr = Vec::new();
    for (program, args) in spec.steps {
        let step_label = format!("{} {}", program, args.join(" "));
        let output = Command::new(program)
            .args(*args)
            .current_dir(&cwd)
            .output()
            .map_err(|err| format!("failed to run {step_label}: {err}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        combined_stdout.push(format!("$ {step_label}\n{stdout}"));
        if !stderr.trim().is_empty() {
            combined_stderr.push(format!("$ {step_label}\n{stderr}"));
        }
        if !output.status.success() {
            let detail = [stdout.trim(), stderr.trim()]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            return Err(if detail.is_empty() {
                format!("{step_label} failed with status {}", output.status)
            } else {
                format!(
                    "{step_label} failed with status {}:\n{}",
                    output.status, detail
                )
            });
        }
    }

    Ok(ReleaseEvidenceCommandResult {
        command_id,
        label: spec.label.to_string(),
        command: spec.command.to_string(),
        summary_path: spec.summary_path.map(str::to_string),
        stdout: combined_stdout.join("\n"),
        stderr: combined_stderr.join("\n"),
    })
}
