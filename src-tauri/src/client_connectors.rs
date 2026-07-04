#[derive(Debug, Clone, Copy)]
pub(crate) struct PlannedSidecarSpec {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) config_dir: &'static [&'static str],
}

pub(crate) const PLANNED_SIDECAR_SPECS: [PlannedSidecarSpec; 11] = [
    PlannedSidecarSpec {
        id: "gemini_cli",
        name: "Gemini CLI",
        config_dir: &[".gemini"],
    },
    PlannedSidecarSpec {
        id: "opencode",
        name: "OpenCode",
        config_dir: &[".config", "opencode"],
    },
    PlannedSidecarSpec {
        id: "cursor",
        name: "Cursor",
        config_dir: &["Library", "Application Support", "Cursor"],
    },
    PlannedSidecarSpec {
        id: "grok_cli",
        name: "Grok / xAI CLI",
        config_dir: &[".config", "xai"],
    },
    PlannedSidecarSpec {
        id: "aider",
        name: "Aider",
        config_dir: &[".config", "aider"],
    },
    PlannedSidecarSpec {
        id: "continue",
        name: "Continue",
        config_dir: &[".continue"],
    },
    PlannedSidecarSpec {
        id: "goose",
        name: "Goose",
        config_dir: &[".config", "goose"],
    },
    PlannedSidecarSpec {
        id: "qwen_code",
        name: "Qwen Code",
        config_dir: &[".qwen"],
    },
    PlannedSidecarSpec {
        id: "amazon_q",
        name: "Amazon Q Developer CLI",
        config_dir: &[".aws", "amazonq"],
    },
    PlannedSidecarSpec {
        id: "windsurf",
        name: "Windsurf",
        config_dir: &["Library", "Application Support", "Windsurf"],
    },
    PlannedSidecarSpec {
        id: "zed_ai",
        name: "Zed AI",
        config_dir: &[".config", "zed"],
    },
];

pub(crate) fn planned_sidecar_spec(client_id: &str) -> Option<&'static PlannedSidecarSpec> {
    let client_id = normalized_connector_id(client_id);
    PLANNED_SIDECAR_SPECS
        .iter()
        .find(|spec| spec.id == client_id)
}

fn normalized_connector_id(client_id: &str) -> &str {
    match client_id {
        "codex" | "codex_gui" => "codex_cli",
        "vscode" => "claude_code",
        other => other,
    }
}
