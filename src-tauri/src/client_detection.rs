use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cli_discovery;
use crate::client_paths::{
    dedupe_paths, grok_config_path, home_dir, opencode_config_path, planned_sidecar_routing_path,
    windsurf_config_path, zed_config_path, SWITCHBOARD_ROUTING_FILE,
};
use crate::client_provider_configs::HEADROOM_OPENAI_BASE_URL;
use crate::cursor_native::{assess_native_schema, evidence_lines as cursor_native_evidence};
use crate::models::{ClientHealth, ClientStatus};

pub(crate) fn codex_home() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".codex"))
}

pub(crate) fn detect_claude_code_client(configured: bool) -> ClientStatus {
    let executable = claude_code_candidate_paths()
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_on_path(&["claude", "claude-code"]));

    if let Some(path) = executable {
        return ClientStatus {
            id: "claude_code".into(),
            name: "Claude Code".into(),
            installed: true,
            configured,
            health: if configured {
                ClientHealth::Healthy
            } else {
                ClientHealth::Attention
            },
            notes: if configured {
                vec![
                    format!("Detected at {}", path.display()),
                    "Configured by Headroom.".into(),
                ]
            } else {
                vec![
                    format!("Detected at {}", path.display()),
                    "Route Claude Code through Headroom's localhost proxy so prompts stay lean."
                        .into(),
                ]
            },
        };
    }

    if claude_code_user_state_exists(&home_dir()) {
        return ClientStatus {
            id: "claude_code".into(),
            name: "Claude Code".into(),
            installed: true,
            configured,
            health: if configured {
                ClientHealth::Healthy
            } else {
                ClientHealth::Attention
            },
            notes: if configured {
                vec![
                    "Detected Claude Code data in ~/.claude.".into(),
                    "Configured by Headroom.".into(),
                ]
            } else {
                vec![
                    "Detected Claude Code data in ~/.claude.".into(),
                    "Claude Code appears to be installed, but Headroom could not resolve the CLI from its current launch PATH. This is common when Headroom starts outside your shell and Claude was installed via nvm or another user-local toolchain.".into(),
                ]
            },
        };
    }

    ClientStatus {
        id: "claude_code".into(),
        name: "Claude Code".into(),
        installed: false,
        configured: false,
        health: ClientHealth::NotDetected,
        notes: vec!["Not detected on this machine yet.".into()],
    }
}

fn claude_code_candidate_paths() -> Vec<PathBuf> {
    let home = home_dir();
    let binary_names = ["claude", "claude-code"];
    let mut candidates = vec![
        PathBuf::from("/usr/local/bin/claude"),
        PathBuf::from("/opt/homebrew/bin/claude"),
        PathBuf::from("/usr/local/bin/claude-code"),
        PathBuf::from("/opt/homebrew/bin/claude-code"),
    ];

    let user_bin_dirs = vec![
        home.join(".local").join("bin"),
        home.join("bin"),
        home.join(".npm-global").join("bin"),
        home.join(".yarn").join("bin"),
        home.join(".config")
            .join("yarn")
            .join("global")
            .join("node_modules")
            .join(".bin"),
        home.join(".volta").join("bin"),
        home.join(".bun").join("bin"),
        home.join(".asdf").join("shims"),
        home.join(".mise").join("shims"),
        home.join(".nodenv").join("shims"),
    ];

    candidates.extend(binary_candidates_in_dirs(&user_bin_dirs, &binary_names));
    candidates.extend(nvm_binary_candidates(&home, &binary_names));
    dedupe_paths(candidates)
}

fn binary_candidates_in_dirs(directories: &[PathBuf], binary_names: &[&str]) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for directory in directories {
        for binary_name in binary_names {
            candidates.push(directory.join(binary_name));
            if cfg!(windows) {
                for ext in windows_path_extensions() {
                    candidates.push(directory.join(format!("{binary_name}{ext}")));
                }
            }
        }
    }
    candidates
}

pub(crate) fn nvm_binary_candidates(home: &Path, binary_names: &[&str]) -> Vec<PathBuf> {
    let mut candidates = binary_candidates_in_dirs(
        &[home.join(".nvm").join("current").join("bin")],
        binary_names,
    );
    let versions_dir = home.join(".nvm").join("versions").join("node");
    let Ok(entries) = std::fs::read_dir(versions_dir) else {
        return candidates;
    };

    let mut version_bins = entries
        .flatten()
        .map(|entry| entry.path().join("bin"))
        .collect::<Vec<_>>();
    version_bins.sort();
    version_bins.reverse();
    candidates.extend(binary_candidates_in_dirs(&version_bins, binary_names));
    candidates
}

pub(crate) fn claude_code_user_state_exists(home: &Path) -> bool {
    let claude_root = home.join(".claude");
    claude_root.join("settings.json").exists()
        || claude_root.join("projects").exists()
        || claude_root.join("sessions").exists()
        || claude_root.join("statsig").exists()
}

pub(crate) fn detect_codex_client(configured: bool) -> ClientStatus {
    let executable = cli_discovery::detect_codex_cli();

    let detected = executable
        .as_ref()
        .map(|path| format!("Detected at {}", path.display()))
        .or_else(|| {
            codex_user_state_exists()
                .then(|| format!("Detected Codex data in {}.", codex_home().display()))
        });

    if let Some(detected_note) = detected {
        return ClientStatus {
            id: "codex".into(),
            name: "Codex".into(),
            installed: true,
            configured,
            health: if configured {
                ClientHealth::Healthy
            } else {
                ClientHealth::Attention
            },
            notes: if configured {
                vec![detected_note, "Configured by Headroom.".into()]
            } else {
                vec![
                    detected_note,
                    "Route Codex through Headroom's localhost proxy so prompts stay lean.".into(),
                ]
            },
        };
    }

    ClientStatus {
        id: "codex".into(),
        name: "Codex".into(),
        installed: false,
        configured: false,
        health: ClientHealth::NotDetected,
        notes: vec!["Not detected on this machine yet.".into()],
    }
}

fn codex_user_state_exists() -> bool {
    let codex_root = codex_home();
    codex_root.join("config.toml").exists()
        || codex_root.join("auth.json").exists()
        || codex_root.join("sessions").exists()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedCliCompatibilityReport {
    label: &'static str,
    binary_path: Option<PathBuf>,
    version: Option<String>,
    config_surfaces: Vec<PathBuf>,
    routing_blocker: &'static str,
}

fn read_cli_version(path: &Path) -> Option<String> {
    let output = Command::new(path).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let version = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    if version.is_empty() {
        None
    } else {
        Some(version)
    }
}

fn planned_cli_compatibility_report(
    label: &'static str,
    binary_path: Option<PathBuf>,
    config_candidates: &[PathBuf],
    routing_blocker: &'static str,
) -> PlannedCliCompatibilityReport {
    let config_surfaces = config_candidates
        .iter()
        .filter(|path| path.exists())
        .cloned()
        .collect::<Vec<_>>();
    let version = binary_path.as_deref().and_then(read_cli_version);

    PlannedCliCompatibilityReport {
        label,
        binary_path,
        version,
        config_surfaces,
        routing_blocker,
    }
}

pub(crate) fn planned_cli_compatibility_evidence(report: &PlannedCliCompatibilityReport) -> Vec<String> {
    let mut evidence = Vec::new();
    if let Some(path) = &report.binary_path {
        evidence.push(format!("{} binary: {}", report.label, path.display()));
    }
    evidence.push(match &report.version {
        Some(version) => format!("{} version: {version}", report.label),
        None => format!("{} version: unavailable from --version.", report.label),
    });
    if report.config_surfaces.is_empty() {
        evidence.push(format!(
            "{} config surface: none detected yet.",
            report.label
        ));
    } else {
        evidence.push(format!(
            "{} config surface: {}",
            report.label,
            report
                .config_surfaces
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    evidence.push(report.routing_blocker.to_string());
    evidence
}

/// Detect Gemini CLI without mutating config. The compatibility report is
/// surfaced as planned-connector evidence while routing remains manual.
pub(crate) fn detect_gemini_cli_client() -> ClientStatus {
    let executable = common_cli_candidate_paths(&["gemini"])
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_on_path(&["gemini"]));
    let config_candidates = [
        home_dir().join(".gemini"),
        home_dir().join(".config").join("gemini"),
    ];
    let report = planned_cli_compatibility_report(
        "Gemini",
        executable.clone(),
        &config_candidates,
        "Managed shell/base-url routing uses Switchboard-owned shell blocks, sibling rollback backups, Doctor verification, rollback, and Off mode cleanup.",
    );
    let installed = executable.is_some() || !report.config_surfaces.is_empty();
    let mut notes = if installed {
        planned_cli_compatibility_evidence(&report)
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Switchboard can manage Gemini CLI shell/base-url routing while keeping account and model choices user-owned."
                .into(),
        );
    }

    let mut status = ClientStatus {
        id: "gemini_cli".into(),
        name: "Gemini CLI".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    };
    append_gemini_manual_routing_note(&mut status);
    status
}

pub(crate) fn append_gemini_manual_routing_note(status: &mut ClientStatus) {
    if status.installed {
        status.notes.push(
            "Gemini routing is managed through reversible shell/base-url exports with backup, Doctor verification, rollback evidence, and Off mode cleanup."
                .into(),
        );
    }
}

pub(crate) fn detect_opencode_client() -> ClientStatus {
    let executable = common_cli_candidate_paths(&["opencode", "open-code"])
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_on_path(&["opencode", "open-code"]));
    let config_candidates = [
        home_dir().join(".opencode"),
        home_dir().join(".config").join("opencode"),
    ];
    let report = planned_cli_compatibility_report(
        "OpenCode",
        executable.clone(),
        &config_candidates,
        "Managed provider routing uses the active OpenCode config path with backup, Doctor verification, rollback, and Off mode cleanup.",
    );
    let installed = executable.is_some() || !report.config_surfaces.is_empty();
    let mut notes = if installed {
        planned_cli_compatibility_evidence(&report)
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Switchboard can manage OpenCode provider routing with backup, verification, rollback, and Off mode cleanup."
                .into(),
        );
    }

    ClientStatus {
        id: "opencode".into(),
        name: "OpenCode".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

pub(crate) fn detect_cursor_client() -> ClientStatus {
    let app_path = PathBuf::from("/Applications/Cursor.app");
    let command_path = find_on_path(&["cursor"]);
    let profile_candidates = [home_dir()
        .join("Library")
        .join("Application Support")
        .join("Cursor")];
    let profile_surfaces = profile_candidates
        .iter()
        .filter(|path| path.exists())
        .cloned()
        .collect::<Vec<_>>();
    let settings_files = discover_editor_settings_files(&profile_surfaces);
    let native_schema = assess_native_schema(&home_dir());
    let installed = app_path.exists() || command_path.is_some() || !profile_surfaces.is_empty();
    let mut notes = if installed {
        let mut evidence = Vec::new();
        if app_path.exists() {
            evidence.push(format!("Cursor app: {}", app_path.display()));
        } else if let Some(path) = command_path {
            evidence.push(format!("Cursor app: command {}", path.display()));
        }
        if profile_surfaces.is_empty() {
            evidence.push("Cursor profile settings: none detected yet.".into());
        } else {
            evidence.push(format!(
                "Cursor profile settings: {}",
                profile_surfaces
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if settings_files.is_empty() {
            evidence.push("Cursor settings files: none detected yet.".into());
        } else {
            evidence.push(format!(
                "Cursor settings files: {}",
                settings_files
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        evidence.push(
            "Cursor settings routing remains blocked because its provider schema is not allowlisted; only the isolated Switchboard sidecar can be applied with preview, exact consent, backup, verification, rollback, and Off cleanup.".into(),
        );
        evidence.extend(cursor_native_evidence(&native_schema));
        evidence
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Cursor provider/account/model settings remain manual; Switchboard can safely manage only its isolated routing-intent sidecar."
                .into(),
        );
    }

    ClientStatus {
        id: "cursor".into(),
        name: "Cursor".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

pub(crate) fn detect_grok_cli_client() -> ClientStatus {
    let executable = common_cli_candidate_paths(&["grok", "xai"])
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_on_path(&["grok", "xai"]));
    let config_candidates = [home_dir().join(".config").join("xai")];
    let report = planned_cli_compatibility_report(
        "Grok / xAI",
        executable.clone(),
        &config_candidates,
        "Provider routing blocked until model/account guardrails, backup, verify, rollback, and Off mode cleanup exist.",
    );
    let installed = executable.is_some() || !report.config_surfaces.is_empty();
    let mut notes = if installed {
        planned_cli_compatibility_evidence(&report)
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Switchboard can safely manage only its isolated routing-intent sidecar; xAI provider, model, credentials, and account settings remain manual."
                .into(),
        );
    }

    ClientStatus {
        id: "grok_cli".into(),
        name: "Grok / xAI CLI".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

pub(crate) fn detect_aider_client() -> ClientStatus {
    let executable = common_cli_candidate_paths(&["aider"])
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_on_path(&["aider"]));
    let config_candidates = [
        home_dir().join(".aider.conf.yml"),
        home_dir().join(".config").join("aider"),
    ];
    let report = planned_cli_compatibility_report(
        "Aider",
        executable.clone(),
        &config_candidates,
        "Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while provider config remains manual.",
    );
    let installed = executable.is_some() || !report.config_surfaces.is_empty();
    let mut notes = if installed {
        planned_cli_compatibility_evidence(&report)
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Switchboard can manage the Aider routing-intent sidecar while keeping provider config manual."
                .into(),
        );
    }

    ClientStatus {
        id: "aider".into(),
        name: "Aider".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

pub(crate) fn detect_continue_client() -> ClientStatus {
    let command_path = find_on_path(&["continue"]);
    let config_candidates = [
        home_dir().join(".continue"),
        home_dir().join(".config").join("continue"),
    ];
    let config_surfaces = config_candidates
        .iter()
        .filter(|path| path.exists())
        .cloned()
        .collect::<Vec<_>>();
    let installed = command_path.is_some() || !config_surfaces.is_empty();
    let mut notes = if installed {
        let mut evidence = Vec::new();
        if let Some(path) = command_path {
            evidence.push(format!("Continue command: {}", path.display()));
        }
        if config_surfaces.is_empty() {
            evidence.push("Continue config folder: none detected yet.".into());
        } else {
            evidence.push(format!(
                "Continue config folder: {}",
                config_surfaces
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        evidence.push(
            "Settings routing blocked until multi-provider parse, dry-run diff, backup, verify, rollback, and Off mode cleanup exist."
                .into(),
        );
        evidence
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
                "Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while provider choices remain manual."
                    .into(),
            );
    }

    ClientStatus {
        id: "continue".into(),
        name: "Continue".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

pub(crate) fn detect_goose_client() -> ClientStatus {
    let executable = common_cli_candidate_paths(&["goose"])
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_on_path(&["goose"]));
    let config_candidates = [home_dir().join(".config").join("goose")];
    let report = planned_cli_compatibility_report(
        "Goose",
        executable.clone(),
        &config_candidates,
        "Managed Switchboard-owned routing-intent sidecar and read-only Repo Memory MCP bridge use exact confirmation, Doctor verification, rollback, and Off mode cleanup while provider routing remains manual.",
    );
    let installed = executable.is_some() || !report.config_surfaces.is_empty();
    let mut notes = if installed {
        planned_cli_compatibility_evidence(&report)
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Switchboard can manage its isolated Goose routing-intent sidecar and read-only Repo Memory MCP bridge; provider, model, credentials, and account settings remain manual."
                .into(),
        );
    }

    ClientStatus {
        id: "goose".into(),
        name: "Goose".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

pub(crate) fn detect_qwen_code_client() -> ClientStatus {
    let executable = common_cli_candidate_paths(&["qwen", "qwen-code"])
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_on_path(&["qwen", "qwen-code"]));
    let config_candidates = [
        home_dir().join(".qwen"),
        home_dir().join(".config").join("qwen"),
    ];
    let report = planned_cli_compatibility_report(
        "Qwen Code",
        executable.clone(),
        &config_candidates,
        "Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while model/account choices remain manual.",
    );
    let installed = executable.is_some() || !report.config_surfaces.is_empty();
    let mut notes = if installed {
        planned_cli_compatibility_evidence(&report)
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Switchboard can manage the Qwen Code routing-intent sidecar while keeping model and account setup manual."
                .into(),
        );
    }

    ClientStatus {
        id: "qwen_code".into(),
        name: "Qwen Code".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

pub(crate) fn detect_amazon_q_client() -> ClientStatus {
    let executable = common_cli_candidate_paths(&["q"])
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_on_path(&["q"]));
    let config_candidates = [
        home_dir().join(".aws").join("amazonq"),
        home_dir().join(".config").join("amazon-q"),
    ];
    let report = planned_cli_compatibility_report(
        "Amazon Q",
        executable.clone(),
        &config_candidates,
        "Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while AWS auth, provider, and workspace choices remain manual.",
    );
    let installed = executable.is_some() || !report.config_surfaces.is_empty();
    let mut notes = if installed {
        planned_cli_compatibility_evidence(&report)
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Switchboard can manage the Amazon Q routing-intent sidecar while keeping AWS and Amazon Q account state manual."
                .into(),
        );
    }

    ClientStatus {
        id: "amazon_q".into(),
        name: "Amazon Q Developer CLI".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

pub(crate) fn detect_windsurf_client() -> ClientStatus {
    let app_path = PathBuf::from("/Applications/Windsurf.app");
    let command_path = find_on_path(&["windsurf"]);
    let settings_candidates = [home_dir()
        .join("Library")
        .join("Application Support")
        .join("Windsurf")];
    let settings_surfaces = settings_candidates
        .iter()
        .filter(|path| path.exists())
        .cloned()
        .collect::<Vec<_>>();
    let settings_files = discover_editor_settings_files(&settings_surfaces);
    let installed = app_path.exists() || command_path.is_some() || !settings_surfaces.is_empty();
    let mut notes = if installed {
        let mut evidence = Vec::new();
        if app_path.exists() {
            evidence.push(format!("Windsurf app: {}", app_path.display()));
        } else if let Some(path) = command_path {
            evidence.push(format!("Windsurf app: command {}", path.display()));
        }
        if settings_surfaces.is_empty() {
            evidence.push("Windsurf settings: none detected yet.".into());
        } else {
            evidence.push(format!(
                "Windsurf settings: {}",
                settings_surfaces
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if settings_files.is_empty() {
            evidence.push("Windsurf settings files: none detected yet.".into());
        } else {
            evidence.push(format!(
                "Windsurf settings files: {}",
                settings_files
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        evidence.push(
            "Managed Windsurf settings routing uses settings parse, dry-run diff, backup, Doctor verification, rollback, and Off mode cleanup."
                .into(),
        );
        evidence
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Switchboard can manage Windsurf editor settings routing with backup, verification, rollback, and Off mode cleanup."
                .into(),
        );
    }

    ClientStatus {
        id: "windsurf".into(),
        name: "Windsurf".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

pub(crate) fn detect_zed_ai_client() -> ClientStatus {
    let app_path = PathBuf::from("/Applications/Zed.app");
    let command_path = find_on_path(&["zed"]);
    let settings_candidates = [
        home_dir().join(".config").join("zed"),
        home_dir()
            .join("Library")
            .join("Application Support")
            .join("Zed"),
    ];
    let settings_surfaces = settings_candidates
        .iter()
        .filter(|path| path.exists())
        .cloned()
        .collect::<Vec<_>>();
    let installed = app_path.exists() || command_path.is_some() || !settings_surfaces.is_empty();
    let mut notes = if installed {
        let mut evidence = Vec::new();
        if app_path.exists() {
            evidence.push(format!("Zed app: {}", app_path.display()));
        } else if let Some(path) = command_path {
            evidence.push(format!("Zed app: command {}", path.display()));
        }
        if settings_surfaces.is_empty() {
            evidence.push("Zed assistant settings: none detected yet.".into());
        } else {
            evidence.push(format!(
                "Zed assistant settings: {}",
                settings_surfaces
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        evidence.push(
            "Managed Zed settings routing uses lossless settings parse, dry-run diff, backup, Doctor verification, rollback, and Off mode cleanup."
                .into(),
        );
        evidence
    } else {
        vec!["Not detected on machine yet.".into()]
    };
    if installed {
        notes.push(
            "Detected. Switchboard can manage Zed assistant settings routing with backup, verification, rollback, and Off mode cleanup."
                .into(),
        );
    }

    ClientStatus {
        id: "zed_ai".into(),
        name: "Zed AI".into(),
        installed,
        configured: false,
        health: if installed {
            ClientHealth::Attention
        } else {
            ClientHealth::NotDetected
        },
        notes,
    }
}

fn detect_planned_client(
    id: &str,
    name: &str,
    binary_names: &[&str],
    state_paths: &[PathBuf],
    planned_note: &str,
) -> ClientStatus {
    let executable = common_cli_candidate_paths(binary_names)
        .into_iter()
        .find(|path| path.exists())
        .or_else(|| find_on_path(binary_names));
    let detected = executable
        .as_ref()
        .map(|path| format!("Detected at {}", path.display()))
        .or_else(|| {
            state_paths
                .iter()
                .find(|path| path.exists())
                .map(|path| format!("Detected data at {}.", path.display()))
        });

    if let Some(detected_note) = detected {
        return ClientStatus {
            id: id.into(),
            name: name.into(),
            installed: true,
            configured: false,
            health: ClientHealth::Attention,
            notes: vec![detected_note, planned_note.into()],
        };
    }

    ClientStatus {
        id: id.into(),
        name: name.into(),
        installed: false,
        configured: false,
        health: ClientHealth::NotDetected,
        notes: vec!["Not detected on machine yet.".into()],
    }
}

fn common_cli_candidate_paths(binary_names: &[&str]) -> Vec<PathBuf> {
    let home = home_dir();
    let mut directories = vec![
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/opt/homebrew/bin"),
    ];
    directories.extend([
        home.join(".local").join("bin"),
        home.join(".cargo").join("bin"),
        home.join("bin"),
        home.join(".npm-global").join("bin"),
        home.join(".yarn").join("bin"),
        home.join(".volta").join("bin"),
        home.join(".bun").join("bin"),
        home.join(".asdf").join("shims"),
        home.join(".mise").join("shims"),
        home.join(".nodenv").join("shims"),
    ]);

    let mut paths = binary_candidates_in_dirs(&directories, binary_names);
    paths.extend(nvm_binary_candidates(&home, binary_names));
    dedupe_paths(paths)
}

pub(crate) fn detect_codex_cli() -> Option<PathBuf> {
    cli_discovery::detect_codex_cli()
}

/// True once the user has signed in to Codex with their ChatGPT account — the
/// OAuth token lands in `~/.codex/auth.json`. Required for the keyless
/// `codex exec` analysis backend.
pub(crate) fn codex_logged_in() -> bool {
    codex_home().join("auth.json").is_file()
}

pub(crate) fn discover_editor_settings_files(profile_roots: &[PathBuf]) -> Vec<PathBuf> {
    let relative_candidates = [
        PathBuf::from("User").join("settings.json"),
        PathBuf::from("User").join("settings.jsonc"),
        PathBuf::from("settings.json"),
        PathBuf::from("settings.jsonc"),
        PathBuf::from("profiles").join("User").join("settings.json"),
        PathBuf::from("profiles")
            .join("User")
            .join("settings.jsonc"),
    ];
    let mut candidates = Vec::new();
    for root in profile_roots {
        for relative in &relative_candidates {
            let path = root.join(relative);
            if path.is_file() {
                candidates.push(path);
            }
        }
        // Cursor profiles are stored beneath User/profiles/<profile-id>.  Only
        // inspect the well-known settings filenames: globalStorage and state
        // databases are intentionally never traversed or read.
        let profiles_dir = root.join("User").join("profiles");
        if let Ok(entries) = std::fs::read_dir(profiles_dir) {
            for entry in entries.flatten() {
                let profile = entry.path();
                if !profile.is_dir() {
                    continue;
                }
                for name in ["settings.json", "settings.jsonc"] {
                    let path = profile.join(name);
                    if path.is_file() {
                        candidates.push(path);
                    }
                }
            }
        }
    }
    dedupe_paths(candidates)
}

fn find_on_path(binary_names: &[&str]) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    find_on_path_entries(std::env::split_paths(&path_var), binary_names)
}

pub(crate) fn find_on_path_entries<I>(path_entries: I, binary_names: &[&str]) -> Option<PathBuf>
where
    I: IntoIterator<Item = PathBuf>,
{
    for entry in path_entries {
        for binary_name in binary_names {
            let candidate = entry.join(binary_name);
            if candidate.exists() {
                return Some(candidate);
            }

            if cfg!(windows) {
                for ext in windows_path_extensions() {
                    let with_ext = entry.join(format!("{binary_name}{ext}"));
                    if with_ext.exists() {
                        return Some(with_ext);
                    }
                }
            }
        }
    }

    None
}

fn windows_path_extensions() -> Vec<String> {
    std::env::var_os("PATHEXT")
        .unwrap_or_else(|| OsStr::new(".COM;.EXE;.BAT;.CMD").to_os_string())
        .to_string_lossy()
        .split(';')
        .filter(|value| !value.is_empty())
        .map(|value| {
            if value.starts_with('.') {
                value.to_string()
            } else {
                format!(".{value}")
            }
        })
        .collect()
}
