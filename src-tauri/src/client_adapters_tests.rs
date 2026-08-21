    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::json;

    use crate::client_connectors::{
        planned_connector_has_implemented_setup, CONNECTOR_MANIFEST_JSON, PLANNED_CLIENT_SPECS,
        PLANNED_CONFIG_CREATION_STEPS, PLANNED_CONFIG_CREATION_STEP_IDS,
    };
    use crate::client_paths::{
        grok_config_path, planned_sidecar_routing_path, SWITCHBOARD_ROUTING_FILE,
    };
    use crate::client_provider_configs::HEADROOM_OPENAI_BASE_URL;
    use crate::models::{
        ClientConnectorSupportStatus, ClientHealth, ClientStatus, CodexThreadRetaggingMode,
        CodexThreadRetaggingSettings, ManagedRollbackExecutionStatus, SwitchboardMode,
    };

    use super::{
        build_headroom_markitdown_hook, build_headroom_rtk_hook, build_markitdown_codex_nudge,
        build_markitdown_office_nudge, claude_code_user_state_exists, claude_hook_present_in_value,
        codex_home, default_shell_targets_for_family, entry_contains_hook, find_on_path_entries,
        list_client_connectors, normalize_setup_state, normalized_setup_id, nvm_binary_candidates,
        parse_json_object, planned_switchboard_sidecar_matches, remove_managed_block,
        remove_pre_tool_use_markers, serialize_paths, shell_block_contains_in_files,
        shell_block_contains_text_in_files, shell_double_quote, strip_headroom_hook_from_settings,
        upsert_managed_block, write_file_if_changed, ClientSetupState,
    };
    use crate::client_connector_status::MANAGED_CLIENT_SPECS;
    use crate::client_footprint;
    use crate::client_paths::{zed_config_path, OPENCODE_CONFIG_FILE};
    use rusqlite::Connection;

    #[test]
    fn normalize_setup_state_keeps_codex_but_drops_legacy_codex_gui() {
        let state = ClientSetupState {
            configured_clients: BTreeMap::from([
                ("claude_code".into(), "2026-03-27T10:00:00Z".into()),
                ("codex_cli".into(), "2026-03-27T10:01:00Z".into()),
                ("codex_gui".into(), "2026-03-27T10:02:00Z".into()),
            ]),
            remembered_clients: BTreeMap::from([
                ("codex".into(), "2026-03-27T10:03:00Z".into()),
                ("claude_code".into(), "2026-03-27T10:04:00Z".into()),
            ]),
            managed_shell_files: BTreeMap::from([
                ("claude_code".into(), vec!["/Users/test/.zprofile".into()]),
                ("codex_cli".into(), vec!["/Users/test/.zshrc".into()]),
                ("codex_gui".into(), vec!["/Users/test/.zshrc".into()]),
            ]),
            remembered_shell_files: BTreeMap::from([
                ("codex".into(), vec!["/Users/test/.bash_profile".into()]),
                ("claude_code".into(), vec!["/Users/test/.bashrc".into()]),
            ]),
            rtk_disabled: false,
            switchboard_mode: Some(SwitchboardMode::Full),
            savings_mode: None,
        };

        let normalized = normalize_setup_state(state);

        // codex_cli stays configured; only the removed codex_gui id is stripped.
        assert!(normalized.configured_clients.contains_key("claude_code"));
        assert!(normalized.configured_clients.contains_key("codex_cli"));
        assert!(!normalized.configured_clients.contains_key("codex_gui"));

        assert!(normalized.remembered_clients.contains_key("claude_code"));
        assert!(normalized.remembered_clients.contains_key("codex"));
        assert_eq!(normalized.switchboard_mode, Some(SwitchboardMode::Full));

        assert!(normalized.managed_shell_files.contains_key("claude_code"));
        assert!(normalized.managed_shell_files.contains_key("codex_cli"));
        assert!(!normalized.managed_shell_files.contains_key("codex_gui"));

        assert!(normalized
            .remembered_shell_files
            .contains_key("claude_code"));
        assert!(normalized.remembered_shell_files.contains_key("codex"));
    }

    #[test]
    fn planned_connector_registry_tracks_popular_agent_tools() {
        let ids = PLANNED_CLIENT_SPECS
            .iter()
            .map(|spec| spec.id)
            .collect::<BTreeSet<_>>();

        assert_eq!(
            ids,
            BTreeSet::from([
                "aider",
                "amazon_q",
                "continue",
                "cursor",
                "gemini_cli",
                "goose",
                "grok_cli",
                "opencode",
                "qwen_code",
                "windsurf",
                "zed_ai",
            ])
        );
    }

    #[test]
    fn connector_registry_uses_manifest_owned_identity_and_status() {
        let detected_clients = vec![
            ClientStatus {
                id: "claude_code".into(),
                name: "Claude Code".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Healthy,
                notes: vec!["Claude config present".into()],
            },
            ClientStatus {
                id: "gemini_cli".into(),
                name: "Gemini CLI".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec!["Gemini binary present".into()],
            },
        ];
        let connectors = list_client_connectors(&detected_clients).expect("list connectors");
        let manifests = serde_json::from_str::<Vec<serde_json::Value>>(CONNECTOR_MANIFEST_JSON)
            .expect("valid connector manifest");
        let rust_ids = MANAGED_CLIENT_SPECS
            .iter()
            .map(|spec| spec.id)
            .chain(PLANNED_CLIENT_SPECS.iter().map(|spec| spec.id))
            .collect::<BTreeSet<_>>();

        for manifest in manifests {
            let id = manifest["id"].as_str().expect("manifest id");
            assert!(rust_ids.contains(id), "{id} missing from Rust registry");
            let connector = connectors
                .iter()
                .find(|connector| connector.client_id == id)
                .unwrap_or_else(|| panic!("{id} missing from connector status"));
            assert_eq!(connector.name, manifest["name"].as_str().unwrap());
            assert_eq!(connector.category, manifest["category"].as_str().unwrap());
            let expected_status = match manifest["support_status"].as_str().unwrap() {
                "managed" => ClientConnectorSupportStatus::Managed,
                _ => ClientConnectorSupportStatus::Planned,
            };
            assert_eq!(connector.support_status, expected_status);
        }
    }

    #[test]
    fn manifest_managed_connectors_have_implemented_setup_paths() {
        let manifests = serde_json::from_str::<Vec<serde_json::Value>>(CONNECTOR_MANIFEST_JSON)
            .expect("valid connector manifest");
        let managed_ids = manifests
            .iter()
            .filter(|manifest| manifest["support_status"].as_str() == Some("managed"))
            .map(|manifest| manifest["id"].as_str().expect("manifest id"))
            .collect::<BTreeSet<_>>();

        assert_eq!(
            managed_ids,
            BTreeSet::from([
                "aider",
                "continue",
                "claude_code",
                "codex",
                "gemini_cli",
                "goose",
                "grok_cli",
                "opencode",
                "qwen_code",
                "amazon_q",
                "windsurf",
                "zed_ai",
            ])
        );

        for id in managed_ids {
            if id == "goose" {
                continue;
            }
            let native_managed = MANAGED_CLIENT_SPECS.iter().any(|spec| spec.id == id);
            let promoted_planned = planned_connector_has_implemented_setup(id);
            assert!(
                native_managed || promoted_planned,
                "{id} is manifest-managed but has no apply/verify/repair setup path"
            );
        }
    }

    #[test]
    // lifecycle-intent: detect
    fn planned_connector_registry_includes_backend_detection_metadata() {
        for spec in PLANNED_CLIENT_SPECS {
            assert!(matches!(spec.category, "cli" | "editor" | "agent"));
            assert!(matches!(
                spec.setup_phase,
                "detect" | "guide" | "adapt" | "managed" | "managed mcp"
            ));
            assert!(
                !spec.detection_sources.is_empty(),
                "{} should have detection sources",
                spec.id
            );
            assert!(
                !spec.config_locations.is_empty(),
                "{} should have config locations",
                spec.id
            );
            if planned_connector_has_implemented_setup(spec.id) {
                assert!(
                    spec.setup_hint.contains("Managed")
                        || (spec.id == "goose" && spec.setup_hint.contains("Managed MCP")),
                    "{} should describe its managed setup lifecycle",
                    spec.id
                );
            } else {
                assert!(
                    spec.setup_hint.contains("Manual guide")
                        || spec.setup_hint.contains("Detection only"),
                    "{} should stay manual until reversible adapters exist",
                    spec.id
                );
            }
        }
        let gemini = PLANNED_CLIENT_SPECS
            .iter()
            .find(|spec| spec.id == "gemini_cli")
            .expect("Gemini spec");
        let gemini_copy = format!(
            "{} {}",
            gemini.setup_hint,
            gemini.automation_gates.join(" ")
        );
        assert!(gemini_copy.contains("sibling rollback backups"));
        assert!(!gemini_copy.contains("sidecar evidence"));
    }

    #[test]
    fn editor_settings_discovery_finds_user_settings_without_writing() {
        let root = unique_temp_dir("editor-settings-discovery");
        let cursor_root = root.join("Cursor");
        let windsurf_root = root.join("Windsurf");
        fs::create_dir_all(cursor_root.join("User")).expect("create cursor user");
        fs::create_dir_all(windsurf_root.join("profiles").join("User"))
            .expect("create windsurf profile");
        let cursor_settings = cursor_root.join("User").join("settings.json");
        let windsurf_settings = windsurf_root
            .join("profiles")
            .join("User")
            .join("settings.jsonc");
        fs::write(&cursor_settings, "{}").expect("write cursor settings");
        fs::write(&windsurf_settings, "{}").expect("write windsurf settings");

        let discovered =
            super::discover_editor_settings_files(&[cursor_root.clone(), windsurf_root.clone()]);

        assert!(discovered.contains(&cursor_settings));
        assert!(discovered.contains(&windsurf_settings));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[serial_test::serial]
    fn planned_connectors_are_detected_but_not_enabled_or_verified() {
        let _home = TestHome::new();
        let detected_clients = vec![
        ClientStatus {
            id: "gemini_cli".into(),
            name: "Gemini CLI".into(),
            installed: true,
            configured: false,
            health: ClientHealth::Attention,
            notes: vec![
                "Gemini binary: /opt/homebrew/bin/gemini".into(),
                "Gemini version: gemini 0.2.1".into(),
                "Gemini config surface: /Users/test/.gemini".into(),
                "Provider routing blocked until stable config surface, backup, verify, rollback, and Off mode cleanup exist.".into(),
            ],
        },
            ClientStatus {
                id: "opencode".into(),
                name: "OpenCode".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "OpenCode binary: /opt/homebrew/bin/opencode".into(),
                    "OpenCode version: opencode 1.0.0".into(),
                    "OpenCode config surface: /Users/test/.config/opencode".into(),
                    "Provider routing blocked until active config path, backup, verify, rollback, and Off mode cleanup exist.".into(),
                ],
            },
            ClientStatus {
                id: "grok_cli".into(),
                name: "Grok / xAI CLI".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "Grok / xAI binary: /opt/homebrew/bin/xai".into(),
                    "Grok / xAI version: xai 0.4.0".into(),
                    "Grok / xAI config surface: /Users/test/.config/xai".into(),
                    "Provider routing blocked until model/account guardrails, backup, verify, rollback, and Off mode cleanup exist.".into(),
                ],
            },
            ClientStatus {
                id: "cursor".into(),
                name: "Cursor".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "Cursor app: /Applications/Cursor.app".into(),
                    "Cursor profile settings: /Users/test/Library/Application Support/Cursor".into(),
                    "Settings routing blocked until profile settings parse, dry-run diff, backup, verify, rollback, and Off mode cleanup exist.".into(),
                ],
            },
            ClientStatus {
                id: "aider".into(),
                name: "Aider".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "Aider binary: /opt/homebrew/bin/aider".into(),
                    "Aider version: aider 0.84.0".into(),
                    "Aider config surface: /Users/test/.aider.conf.yml".into(),
                    "Provider routing blocked until reversible environment wrapper, backup, verify, rollback, and Off mode cleanup exist.".into(),
                ],
            },
            ClientStatus {
                id: "continue".into(),
                name: "Continue".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "Continue command: /opt/homebrew/bin/continue".into(),
                    "Continue config folder: /Users/test/.continue".into(),
                    "Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while provider choices remain manual.".into(),
                ],
            },
            ClientStatus {
                id: "goose".into(),
                name: "Goose".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "Goose binary: /opt/homebrew/bin/goose".into(),
                    "Goose version: goose 1.2.0".into(),
                    "Goose config surface: /Users/test/.config/goose".into(),
                    "Provider routing blocked until MCP handoff shape, backup, verify, rollback, and Off mode cleanup exist.".into(),
                ],
            },
            ClientStatus {
                id: "qwen_code".into(),
                name: "Qwen Code".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "Qwen Code binary: /opt/homebrew/bin/qwen-code".into(),
                    "Qwen Code version: qwen-code 0.9.0".into(),
                    "Qwen Code config surface: /Users/test/.qwen".into(),
                    "Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while model/account choices remain manual.".into(),
                ],
            },
            ClientStatus {
                id: "amazon_q".into(),
                name: "Amazon Q Developer CLI".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "Amazon Q binary: /opt/homebrew/bin/q".into(),
                    "Amazon Q version: q 1.11.0".into(),
                    "Amazon Q config surface: /Users/test/.aws/amazonq".into(),
                    "Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while AWS auth, provider, and workspace choices remain manual.".into(),
                ],
            },
            ClientStatus {
                id: "windsurf".into(),
                name: "Windsurf".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "Windsurf app: /Applications/Windsurf.app".into(),
                    "Windsurf settings: /Users/test/Library/Application Support/Windsurf"
                        .into(),
                    "Managed Windsurf settings routing uses settings parse, dry-run diff, backup, Doctor verification, rollback, and Off mode cleanup.".into(),
                ],
            },
            ClientStatus {
                id: "zed_ai".into(),
                name: "Zed AI".into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![
                    "Zed app: /Applications/Zed.app".into(),
                    "Zed assistant settings: /Users/test/.config/zed".into(),
                    "Managed Zed settings routing uses lossless settings parse, dry-run diff, backup, Doctor verification, rollback, and Off mode cleanup.".into(),
                ],
            },
        ];

        let connectors = list_client_connectors(&detected_clients).expect("list connectors");
        let planned = connectors
            .iter()
            .filter(|connector| connector.support_status == ClientConnectorSupportStatus::Planned)
            .collect::<Vec<_>>();

        assert_eq!(
            planned
                .iter()
                .map(|connector| connector.client_id.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["cursor"])
        );

        for connector in planned {
            assert!(!connector.enabled);
            assert!(!connector.verified);
            assert_eq!(connector.last_configured_at, None);
            assert!(!connector.category.is_empty());
            assert!(!connector.detection_sources.is_empty());
            assert!(!connector.detection_evidence.is_empty());
            assert!(!connector.config_locations.is_empty());
            assert_eq!(
                connector.config_creation_steps,
                PLANNED_CONFIG_CREATION_STEPS
                    .iter()
                    .map(|step| step.to_string())
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                connector
                    .config_creation_step_details
                    .iter()
                    .map(|step| step.id.as_str())
                    .collect::<Vec<_>>(),
                PLANNED_CONFIG_CREATION_STEP_IDS
            );
            assert!(connector
                .config_creation_step_details
                .iter()
                .all(|step| !step.label.is_empty()
                    && step.detail.len() > 30
                    && step.required_evidence.len() >= 2
                    && step
                        .required_evidence
                        .iter()
                        .all(|evidence| evidence.len() > 30)));
            let dry_run = connector
                .config_creation_step_details
                .iter()
                .find(|step| step.id == "dryRunDiff")
                .expect("gated connector dry-run step");
            let dry_run_copy =
                format!("{} {}", dry_run.detail, dry_run.required_evidence.join(" "));
            for snippet in [
                "target path",
                "before/after",
                "managed marker boundary",
                "rollback preview",
                "confirmation phrase",
            ] {
                assert!(dry_run_copy.contains(snippet));
            }
            let preview = connector
                .config_dry_run_preview
                .as_ref()
                .expect("gated connector dry-run preview");
            assert!(!preview.target.trim().is_empty());
            assert!(!preview.marker.trim().is_empty());
            assert!(!preview.backup_path.trim().is_empty());
            assert!(!preview.rollback_preview.trim().is_empty());
            assert!(!preview.confirmation_phrase.trim().is_empty());
            assert_eq!(
                preview.marker,
                format!("ai-switchboard:{}", connector.client_id)
            );
            assert!(preview.backup_path.ends_with(".ai-switchboard.bak"));
            assert!(preview.current_state.contains(&connector.name));
            assert!(preview.proposed_state.contains("Preview only"));
            assert!(preview.proposed_state.contains("no files are written"));
            assert!(preview.apply_blocked_reason.contains(&connector.name));
            if connector.client_id == "cursor" {
                assert!(preview
                    .apply_blocked_reason
                    .contains("does not document a stable on-disk"));
            } else {
                assert!(preview
                    .apply_blocked_reason
                    .contains("backup, verify, rollback, and Off cleanup"));
            }
            if connector.client_id == "cursor" {
                assert!(preview.rollback_preview.contains("No Cursor native write"));
                assert_eq!(preview.confirmation_phrase, "CURSOR NATIVE SCHEMA GATE");
            } else {
                assert!(preview.rollback_preview.contains("remove only"));
                assert_eq!(
                    preview.confirmation_phrase,
                    format!("APPLY {} CONFIG", connector.name.to_uppercase())
                );
            }
            assert!(preview.writes.is_empty());
            assert_eq!(connector.automation_path.len(), 7);
            assert_eq!(
                connector
                    .automation_path
                    .iter()
                    .map(|stage| stage.id.as_str())
                    .collect::<Vec<_>>(),
                PLANNED_CONFIG_CREATION_STEP_IDS
            );
            assert_eq!(connector.automation_path[0].status, "ready");
            assert_eq!(connector.automation_path[1].status, "ready");
            assert!(connector
                .automation_path
                .iter()
                .skip(2)
                .all(|stage| stage.status == "blocked"));
            assert!(connector.automation_path[1]
                .evidence
                .contains(&preview.confirmation_phrase));
        }

        let gemini = connectors
            .iter()
            .find(|connector| connector.client_id == "gemini_cli")
            .expect("gemini connector");
        assert_eq!(gemini.support_status, ClientConnectorSupportStatus::Managed);
        assert_eq!(gemini.setup_phase, "managed");
        assert!(gemini.config_creation_steps.is_empty());
        assert!(gemini.config_creation_step_details.is_empty());
        let gemini_preview = gemini
            .config_dry_run_preview
            .as_ref()
            .expect("gemini managed dry-run preview");
        assert!(gemini_preview.apply_blocked_reason.contains("read-only"));
        assert!(gemini_preview.writes.is_empty());
        assert!(gemini.automation_path.is_empty());
        assert!(!gemini.enabled);
        assert!(!gemini.verified);

        let opencode = connectors
            .iter()
            .find(|connector| connector.client_id == "opencode")
            .expect("opencode connector");
        assert_eq!(
            opencode.support_status,
            ClientConnectorSupportStatus::Managed
        );
        assert_eq!(opencode.setup_phase, "managed");
        assert!(opencode.config_creation_steps.is_empty());
        assert!(opencode.config_creation_step_details.is_empty());
        let opencode_preview = opencode
            .config_dry_run_preview
            .as_ref()
            .expect("opencode managed dry-run preview");
        assert!(opencode_preview.apply_blocked_reason.contains("read-only"));
        assert!(opencode_preview.writes.is_empty());
        assert!(opencode.automation_path.is_empty());
        assert!(!opencode.enabled);
        assert!(!opencode.verified);

        let managed = connectors
            .iter()
            .filter(|connector| connector.support_status == ClientConnectorSupportStatus::Managed)
            .collect::<Vec<_>>();
        assert!(managed
            .iter()
            .all(|connector| connector.config_creation_steps.is_empty()
                && connector.config_creation_step_details.is_empty()));

        for connector in connectors.iter() {
            let Some(preview) = connector.config_dry_run_preview.as_ref() else {
                continue;
            };
            assert!(!preview.target.trim().is_empty(), "{} preview target", connector.client_id);
            assert!(!preview.marker.trim().is_empty(), "{} preview marker", connector.client_id);
            assert!(!preview.backup_path.trim().is_empty(), "{} preview backup", connector.client_id);
            assert!(!preview.current_state.trim().is_empty(), "{} preview current state", connector.client_id);
            assert!(!preview.proposed_state.trim().is_empty(), "{} preview proposed state", connector.client_id);
            assert!(!preview.apply_blocked_reason.trim().is_empty(), "{} preview block reason", connector.client_id);
            assert!(!preview.rollback_preview.trim().is_empty(), "{} preview rollback", connector.client_id);
            assert!(!preview.confirmation_phrase.trim().is_empty(), "{} preview confirmation", connector.client_id);
            assert!(preview.writes.is_empty(), "{} preview must remain read-only", connector.client_id);
        }

        assert!(connectors.iter().any(|connector| {
            connector.client_id == "gemini_cli"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Gemini binary: /opt/homebrew/bin/gemini".to_string())
                && connector
                    .detection_evidence
                    .contains(&"Gemini version: gemini 0.2.1".to_string())
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "opencode"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"OpenCode binary: /opt/homebrew/bin/opencode".to_string())
                && connector
                    .detection_evidence
                    .contains(&"OpenCode version: opencode 1.0.0".to_string())
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "grok_cli"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Grok / xAI binary: /opt/homebrew/bin/xai".to_string())
                && connector
                    .detection_evidence
                    .contains(&"Grok / xAI version: xai 0.4.0".to_string())
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "cursor"
                && connector.support_status == ClientConnectorSupportStatus::Planned
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Cursor app: /Applications/Cursor.app".to_string())
                && connector.detection_evidence.contains(
                    &"Settings routing blocked until profile settings parse, dry-run diff, backup, verify, rollback, and Off mode cleanup exist."
                        .to_string()
                )
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "aider"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Aider binary: /opt/homebrew/bin/aider".to_string())
                && connector
                    .detection_evidence
                    .contains(&"Aider version: aider 0.84.0".to_string())
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "continue"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Continue command: /opt/homebrew/bin/continue".to_string())
                && connector.detection_evidence.contains(
                    &"Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while provider choices remain manual."
                        .to_string()
                )
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "goose"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Goose binary: /opt/homebrew/bin/goose".to_string())
                && connector
                    .detection_evidence
                    .contains(&"Goose version: goose 1.2.0".to_string())
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "qwen_code"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Qwen Code binary: /opt/homebrew/bin/qwen-code".to_string())
                && connector
                    .detection_evidence
                    .contains(&"Qwen Code version: qwen-code 0.9.0".to_string())
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "amazon_q"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Amazon Q binary: /opt/homebrew/bin/q".to_string())
                && connector
                    .detection_evidence
                    .contains(&"Amazon Q version: q 1.11.0".to_string())
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "windsurf"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Windsurf app: /Applications/Windsurf.app".to_string())
                && connector.detection_evidence.contains(
                    &"Managed Windsurf settings routing uses settings parse, dry-run diff, backup, Doctor verification, rollback, and Off mode cleanup."
                        .to_string()
                )
        }));
        assert!(connectors.iter().any(|connector| {
            connector.client_id == "zed_ai"
                && connector.support_status == ClientConnectorSupportStatus::Managed
                && connector.installed
                && connector
                    .detection_evidence
                    .contains(&"Zed app: /Applications/Zed.app".to_string())
                && connector.detection_evidence.contains(
                    &"Managed Zed settings routing uses lossless settings parse, dry-run diff, backup, Doctor verification, rollback, and Off mode cleanup."
                        .to_string()
                )
        }));
    }

    #[test]
    fn gemini_compatibility_evidence_reports_version_config_and_managed_routing() {
        let report = super::PlannedCliCompatibilityReport {
            label: "Gemini",
            binary_path: Some(PathBuf::from("/opt/homebrew/bin/gemini")),
            version: Some("gemini 0.2.1".to_string()),
            config_surfaces: vec![PathBuf::from("/Users/test/.gemini")],
            routing_blocker:
                "Managed shell/base-url routing uses Switchboard-owned shell blocks, sibling rollback backups, Doctor verification, rollback, and Off mode cleanup.",
        };

        let evidence = super::planned_cli_compatibility_evidence(&report).join(" ");

        assert!(evidence.contains("Gemini binary: /opt/homebrew/bin/gemini"));
        assert!(evidence.contains("Gemini version: gemini 0.2.1"));
        assert!(evidence.contains("Gemini config surface: /Users/test/.gemini"));
        assert!(evidence.contains("Managed shell/base-url routing"));
        assert!(evidence.contains("sibling rollback backups"));
        assert!(!evidence.contains("sidecar evidence"));
        assert!(evidence.contains("Doctor verification"));
        assert!(evidence.contains("backup"));
        assert!(evidence.contains("rollback"));
        assert!(evidence.contains("Off mode cleanup"));
    }

    #[test]
    // lifecycle-intent: detect
    fn opencode_compatibility_evidence_reports_version_config_and_managed_routing() {
        let report = super::PlannedCliCompatibilityReport {
            label: "OpenCode",
            binary_path: Some(PathBuf::from("/opt/homebrew/bin/opencode")),
            version: Some("opencode 1.0.0".to_string()),
            config_surfaces: vec![PathBuf::from("/Users/test/.config/opencode")],
            routing_blocker:
                "Managed provider routing uses the active OpenCode config path with backup, Doctor verification, rollback, and Off mode cleanup.",
        };

        let evidence = super::planned_cli_compatibility_evidence(&report).join(" ");

        assert!(evidence.contains("OpenCode binary: /opt/homebrew/bin/opencode"));
        assert!(evidence.contains("OpenCode version: opencode 1.0.0"));
        assert!(evidence.contains("OpenCode config surface: /Users/test/.config/opencode"));
        assert!(evidence.contains("Managed provider routing"));
        assert!(evidence.contains("active OpenCode config path"));
        assert!(evidence.contains("Doctor verification"));
        assert!(evidence.contains("backup"));
        assert!(evidence.contains("rollback"));
        assert!(evidence.contains("Off mode cleanup"));
    }

    #[test]
    // lifecycle-intent: detect
    fn grok_compatibility_evidence_reports_model_account_blocker() {
        let report = super::PlannedCliCompatibilityReport {
            label: "Grok / xAI",
            binary_path: Some(PathBuf::from("/opt/homebrew/bin/xai")),
            version: Some("xai 0.4.0".to_string()),
            config_surfaces: vec![PathBuf::from("/Users/test/.config/xai")],
            routing_blocker:
                "Provider routing blocked until model/account guardrails, backup, verify, rollback, and Off mode cleanup exist.",
        };

        let evidence = super::planned_cli_compatibility_evidence(&report).join(" ");

        assert!(evidence.contains("Grok / xAI binary: /opt/homebrew/bin/xai"));
        assert!(evidence.contains("Grok / xAI version: xai 0.4.0"));
        assert!(evidence.contains("Grok / xAI config surface: /Users/test/.config/xai"));
        assert!(evidence.contains("model/account guardrails"));
        assert!(evidence.contains("backup"));
        assert!(evidence.contains("verify"));
        assert!(evidence.contains("rollback"));
        assert!(evidence.contains("Off mode cleanup"));
    }

    #[test]
    // lifecycle-intent: detect
    fn aider_compatibility_evidence_reports_environment_wrapper_blocker() {
        let report = super::PlannedCliCompatibilityReport {
            label: "Aider",
            binary_path: Some(PathBuf::from("/opt/homebrew/bin/aider")),
            version: Some("aider 0.84.0".to_string()),
            config_surfaces: vec![PathBuf::from("/Users/test/.aider.conf.yml")],
            routing_blocker:
                "Provider routing blocked until reversible environment wrapper, backup, verify, rollback, and Off mode cleanup exist.",
        };

        let evidence = super::planned_cli_compatibility_evidence(&report).join(" ");

        assert!(evidence.contains("Aider binary: /opt/homebrew/bin/aider"));
        assert!(evidence.contains("Aider version: aider 0.84.0"));
        assert!(evidence.contains("Aider config surface: /Users/test/.aider.conf.yml"));
        assert!(evidence.contains("reversible environment wrapper"));
        assert!(evidence.contains("backup"));
        assert!(evidence.contains("verify"));
        assert!(evidence.contains("rollback"));
        assert!(evidence.contains("Off mode cleanup"));
    }

    #[test]
    // lifecycle-intent: detect
    fn goose_compatibility_evidence_reports_mcp_handoff_blocker() {
        let report = super::PlannedCliCompatibilityReport {
            label: "Goose",
            binary_path: Some(PathBuf::from("/opt/homebrew/bin/goose")),
            version: Some("goose 1.2.0".to_string()),
            config_surfaces: vec![PathBuf::from("/Users/test/.config/goose")],
            routing_blocker:
                "Provider routing blocked until MCP handoff shape, backup, verify, rollback, and Off mode cleanup exist.",
        };

        let evidence = super::planned_cli_compatibility_evidence(&report).join(" ");

        assert!(evidence.contains("Goose binary: /opt/homebrew/bin/goose"));
        assert!(evidence.contains("Goose version: goose 1.2.0"));
        assert!(evidence.contains("Goose config surface: /Users/test/.config/goose"));
        assert!(evidence.contains("MCP handoff shape"));
        assert!(evidence.contains("backup"));
        assert!(evidence.contains("verify"));
        assert!(evidence.contains("rollback"));
        assert!(evidence.contains("Off mode cleanup"));
    }

    #[test]
    fn qwen_compatibility_evidence_reports_managed_sidecar_lifecycle() {
        let report = super::PlannedCliCompatibilityReport {
            label: "Qwen Code",
            binary_path: Some(PathBuf::from("/opt/homebrew/bin/qwen-code")),
            version: Some("qwen-code 0.9.0".to_string()),
            config_surfaces: vec![PathBuf::from("/Users/test/.qwen")],
            routing_blocker:
                "Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while model/account choices remain manual.",
        };

        let evidence = super::planned_cli_compatibility_evidence(&report).join(" ");

        assert!(evidence.contains("Qwen Code binary: /opt/homebrew/bin/qwen-code"));
        assert!(evidence.contains("Qwen Code version: qwen-code 0.9.0"));
        assert!(evidence.contains("Qwen Code config surface: /Users/test/.qwen"));
        assert!(evidence.contains("Managed sidecar routing-intent setup"));
        assert!(evidence.contains("Switchboard-owned config marker"));
        assert!(evidence.contains("Doctor verification"));
        assert!(evidence.contains("rollback"));
        assert!(evidence.contains("Off mode cleanup"));
    }

    #[test]
    // lifecycle-intent: detect
    fn amazon_q_compatibility_evidence_reports_managed_sidecar_lifecycle() {
        let report = super::PlannedCliCompatibilityReport {
            label: "Amazon Q",
            binary_path: Some(PathBuf::from("/opt/homebrew/bin/q")),
            version: Some("q 1.11.0".to_string()),
            config_surfaces: vec![PathBuf::from("/Users/test/.aws/amazonq")],
            routing_blocker:
                "Managed sidecar routing-intent setup uses a Switchboard-owned config marker with Doctor verification, rollback, and Off mode cleanup while AWS auth, provider, and workspace choices remain manual.",
        };

        let evidence = super::planned_cli_compatibility_evidence(&report).join(" ");

        assert!(evidence.contains("Amazon Q binary: /opt/homebrew/bin/q"));
        assert!(evidence.contains("Amazon Q version: q 1.11.0"));
        assert!(evidence.contains("Amazon Q config surface: /Users/test/.aws/amazonq"));
        assert!(evidence.contains("Managed sidecar routing-intent setup"));
        assert!(evidence.contains("Switchboard-owned config marker"));
        assert!(evidence.contains("Doctor verification"));
        assert!(evidence.contains("rollback"));
        assert!(evidence.contains("Off mode cleanup"));
    }

    #[test]
    // lifecycle-intent: detect
    fn gemini_detection_reports_managed_routing_lifecycle() {
        let mut status = ClientStatus {
            id: "gemini_cli".into(),
            name: "Gemini CLI".into(),
            installed: true,
            configured: false,
            health: ClientHealth::Attention,
            notes: vec!["Detected at /opt/homebrew/bin/gemini".into()],
        };

        super::append_gemini_manual_routing_note(&mut status);

        let notes = status.notes.join(" ");
        assert!(notes.contains("Gemini routing is managed"));
        assert!(notes.contains("reversible shell/base-url exports"));
        assert!(notes.contains("Doctor verification"));
        assert!(notes.contains("backup"));
        assert!(notes.contains("rollback"));
        assert!(notes.contains("Off mode cleanup"));
    }

    #[test]
    fn parse_json_object_accepts_json5_but_rejects_non_objects() {
        let parsed = parse_json_object(
            "{ unquoted: 'value', trailing: true, }",
            Path::new("settings.json"),
        )
        .expect("json5 object should parse");
        assert_eq!(
            parsed.get("unquoted").and_then(|value| value.as_str()),
            Some("value")
        );
        assert_eq!(
            parsed.get("trailing").and_then(|value| value.as_bool()),
            Some(true)
        );

        let err =
            parse_json_object("[]", Path::new("settings.json")).expect_err("arrays are rejected");
        assert!(err
            .to_string()
            .contains("must contain a top-level JSON object"));
    }

    #[test]
    fn setup_aliases_map_to_current_primary_ids() {
        assert_eq!(normalized_setup_id("codex"), "codex_cli");
        assert_eq!(normalized_setup_id("codex_gui"), "codex_cli");
        assert_eq!(normalized_setup_id("vscode"), "claude_code");
        assert_eq!(normalized_setup_id("claude_code"), "claude_code");
    }

    #[test]
    fn shell_double_quote_escapes_shell_sensitive_characters() {
        let escaped = shell_double_quote("path with spaces/$HOME/\"quoted\"`cmd`\\tail");
        assert_eq!(
            escaped,
            "path with spaces/\\$HOME/\\\"quoted\\\"\\`cmd\\`\\\\tail"
        );
    }

    #[test]
    fn shell_targets_include_profile_and_rc_for_supported_shells() {
        let zsh_targets = default_shell_targets_for_family(crate::client_paths::ShellFamily::Zsh);
        let bash_targets = default_shell_targets_for_family(crate::client_paths::ShellFamily::Bash);

        assert!(zsh_targets.iter().any(|path| path.ends_with(".zprofile")));
        assert!(zsh_targets.iter().any(|path| path.ends_with(".zshrc")));
        assert!(bash_targets.iter().any(|path| {
            path.ends_with(".bash_profile")
                || path.ends_with(".bash_login")
                || path.ends_with(".profile")
        }));
        assert!(bash_targets.iter().any(|path| path.ends_with(".bashrc")));
    }

    #[test]
    fn serialize_paths_dedupes_repeated_entries() {
        let serialized = serialize_paths(&[
            PathBuf::from("/Users/test/.zprofile"),
            PathBuf::from("/Users/test/.zprofile"),
            PathBuf::from("/Users/test/.zshrc"),
        ]);

        assert_eq!(
            serialized,
            vec![
                "/Users/test/.zprofile".to_string(),
                "/Users/test/.zshrc".to_string()
            ]
        );
    }

    #[test]
    fn generated_rtk_hook_uses_escaped_paths_and_rewrite_reason() {
        let hook = build_headroom_rtk_hook(
            Path::new("/tmp/head room/bin/rtk"),
            Path::new("/tmp/head room/runtime/$python"),
        );

        assert!(hook.contains("HEADROOM_RTK=\"/tmp/head room/bin/rtk\""));
        assert!(hook.contains("HEADROOM_PYTHON=\"/tmp/head room/runtime/\\$python\""));
        assert!(hook.contains("Headroom RTK auto-rewrite"));
        assert!(hook.contains("\"updatedInput\": updated"));
    }

    #[test]
    fn generated_markitdown_hook_escapes_paths_and_redirects_read() {
        let hook = build_headroom_markitdown_hook(
            Path::new("/tmp/head room/venv/bin/markitdown"),
            Path::new("/tmp/head room/venv/bin/$python"),
        );

        assert!(hook.contains("HEADROOM_MARKITDOWN=\"/tmp/head room/venv/bin/markitdown\""));
        assert!(hook.contains("HEADROOM_PYTHON=\"/tmp/head room/venv/bin/\\$python\""));
        // Scoped to PDF only (Office is handled by the nudge, not the hook),
        // redirects via updatedInput, and fails open.
        assert!(hook.contains("ALLOWED = {\".pdf\"}"));
        assert!(!hook.contains(".docx"));
        assert!(hook.contains("updated[\"file_path\"] = out"));
        assert!(hook.contains("\"updatedInput\": updated"));
        assert!(hook.contains("Headroom MarkItDown conversion"));
        assert!(hook.contains("sys.exit(0)"));
    }

    #[test]
    fn disabling_markitdown_marker_leaves_rtk_hook_intact() {
        let root = unique_temp_dir("headroom-strip-markitdown");
        fs::create_dir_all(&root).expect("create root");
        let settings = root.join("settings.json");
        fs::write(
            &settings,
            serde_json::to_string_pretty(&json!({
                "hooks": {
                    "PreToolUse": [
                        { "matcher": "Bash", "hooks": [{ "type": "command", "command": "/h/headroom-rtk-rewrite.sh" }] },
                        { "matcher": "Read", "hooks": [{ "type": "command", "command": "/h/headroom-markitdown-read.sh" }] }
                    ]
                }
            }))
            .unwrap(),
        )
        .expect("write settings");

        let changed = remove_pre_tool_use_markers(&settings, &["headroom-markitdown-read.sh"])
            .expect("strip");
        assert!(changed);

        let after: serde_json::Value =
            serde_json::from_slice(&fs::read(&settings).unwrap()).unwrap();
        let entries = after["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entry_contains_hook(&entries[0], "headroom-rtk-rewrite.sh"));
    }

    #[test]
    fn markitdown_office_nudge_points_at_the_shim_and_skips_pdf() {
        let nudge = build_markitdown_office_nudge(Path::new("/h/bin/markitdown"));
        assert!(nudge.contains("/h/bin/markitdown <path>"));
        assert!(nudge.contains(".docx"));
        assert!(nudge.contains("PDFs are handled automatically"));
    }

    #[test]
    fn markitdown_codex_nudge_covers_pdf_and_office() {
        let nudge = build_markitdown_codex_nudge(Path::new("/h/bin/markitdown"));
        assert!(nudge.contains("/h/bin/markitdown <path>"));
        // Codex has no hook, so PDF is covered by the CLI nudge too.
        assert!(nudge.contains(".pdf"));
        assert!(nudge.contains(".docx"));
    }

    #[test]
    fn hook_detection_finds_nested_hook_commands() {
        let hook_path = "/Users/test/.claude/hooks/headroom-rtk-rewrite.sh";
        let content = json!({
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "bash",
                        "hooks": [
                            { "type": "command", "command": hook_path }
                        ]
                    }
                ]
            }
        });

        assert!(claude_hook_present_in_value(&content, hook_path));
        assert!(entry_contains_hook(
            &content["hooks"]["PreToolUse"][0],
            "headroom-rtk-rewrite.sh"
        ));
        assert!(!entry_contains_hook(
            &json!({ "hooks": [] }),
            "headroom-rtk-rewrite.sh"
        ));
    }

    #[test]
    fn nvm_binary_candidates_include_installed_versions() {
        let home = unique_temp_dir("headroom-nvm-detect");
        let version_bin = home
            .join(".nvm")
            .join("versions")
            .join("node")
            .join("v22.17.1")
            .join("bin");
        fs::create_dir_all(&version_bin).expect("create nvm bin");
        fs::write(version_bin.join("claude"), "").expect("write fake claude binary");

        let candidates = nvm_binary_candidates(&home, &["claude"]);

        assert!(candidates
            .iter()
            .any(|candidate| candidate == &version_bin.join("claude")));

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn path_lookup_scans_supplied_entries() {
        let home = unique_temp_dir("headroom-path-detect");
        let bin_dir = home.join("custom-bin");
        fs::create_dir_all(&bin_dir).expect("create custom bin");
        fs::write(bin_dir.join("claude"), "").expect("write fake claude binary");

        let detected = find_on_path_entries(vec![bin_dir.clone()], &["claude"]);

        assert_eq!(detected, Some(bin_dir.join("claude")));

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn claude_user_state_detection_accepts_settings_or_projects() {
        let home = unique_temp_dir("headroom-claude-home");
        let claude_root = home.join(".claude");
        fs::create_dir_all(&claude_root).expect("create claude root");
        assert!(!claude_code_user_state_exists(&home));

        fs::write(claude_root.join("settings.json"), "{}").expect("write settings");
        assert!(claude_code_user_state_exists(&home));

        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn managed_block_upsert_replaces_existing_block_without_duplication() {
        let root = unique_temp_dir("headroom-managed-block");
        fs::create_dir_all(&root).expect("create root");
        let path = root.join(".zshrc");
        fs::write(&path, "export PATH=/usr/bin\n").expect("write shell file");

        let first = upsert_managed_block(
            &path,
            "claude_code",
            "export ANTHROPIC_BASE_URL=http://127.0.0.1:6767",
        )
        .expect("insert managed block");
        assert!(first.0);
        assert!(first.1.is_some());

        upsert_managed_block(
            &path,
            "claude_code",
            "export ANTHROPIC_BASE_URL=http://127.0.0.1:6767\nexport HEADROOM=1",
        )
        .expect("replace managed block");

        let content = fs::read_to_string(&path).expect("read updated shell file");
        assert_eq!(content.matches("# >>> ai-switchboard:claude_code >>>").count(), 1);
        assert!(content.contains("export PATH=/usr/bin"));
        assert!(content.contains("export HEADROOM=1"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn remove_managed_block_keeps_surrounding_shell_content_intact() {
        let root = unique_temp_dir("headroom-remove-block");
        fs::create_dir_all(&root).expect("create root");
        let path = root.join(".zprofile");
        fs::write(
            &path,
            "export PATH=/usr/bin\n# >>> ai-switchboard:claude_code >>>\nexport ANTHROPIC_BASE_URL=http://127.0.0.1:6767\n# <<< ai-switchboard:claude_code <<<\nexport EDITOR=vim\n",
        )
        .expect("write shell file");

        let removed = remove_managed_block(&path, "claude_code").expect("remove managed block");

        assert!(removed);
        assert_eq!(
            fs::read_to_string(&path).expect("read cleaned shell file"),
            "export PATH=/usr/bin\nexport EDITOR=vim\n"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn shell_block_helpers_only_match_content_inside_the_named_block() {
        let root = unique_temp_dir("headroom-shell-match");
        fs::create_dir_all(&root).expect("create root");
        let path = root.join(".bashrc");
        fs::write(
            &path,
            "export ANTHROPIC_BASE_URL=https://example.com\n# >>> ai-switchboard:claude_code >>>\nexport ANTHROPIC_BASE_URL=http://127.0.0.1:6767\nexport PATH=/tmp/headroom:$PATH\n# <<< ai-switchboard:claude_code <<<\n",
        )
        .expect("write shell file");

        assert!(shell_block_contains_in_files(
            &[path.clone()],
            "claude_code",
            "ANTHROPIC_BASE_URL",
            "http://127.0.0.1:6767",
        )
        .expect("detect managed export"));
        assert!(
            shell_block_contains_text_in_files(&[path.clone()], "claude_code", "export PATH=",)
                .expect("detect managed text")
        );
        assert!(!shell_block_contains_in_files(
            &[path],
            "managed_rtk",
            "ANTHROPIC_BASE_URL",
            "http://127.0.0.1:6767",
        )
        .expect("ignore other block ids"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    // lifecycle-intent: backup
    fn write_file_if_changed_skips_backups_when_content_is_unchanged() {
        let root = unique_temp_dir("headroom-write-file");
        fs::create_dir_all(&root).expect("create root");
        let path = root.join("headroom-rtk-rewrite.sh");
        fs::write(&path, "#!/bin/sh\necho headroom\n").expect("write hook file");

        let changed = write_file_if_changed(&path, "#!/bin/sh\necho headroom\n", false)
            .expect("skip unchanged write");

        assert_eq!(changed, (false, None));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn managed_block_round_trip_preserves_realistic_zshrc_content() {
        let root = unique_temp_dir("headroom-zshrc-roundtrip");
        fs::create_dir_all(&root).expect("create root");
        let path = root.join(".zshrc");
        let original = r#"export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

# pnpm
export PNPM_HOME="/Users/test/Library/pnpm"
case ":$PATH:" in
  *":$PNPM_HOME:"*) ;;
  *) export PATH="$PNPM_HOME:$PATH" ;;
esac

export BUN_INSTALL="$HOME/.bun"
export PATH="$BUN_INSTALL/bin:$PATH"
"#;
        fs::write(&path, original).expect("write zshrc");

        upsert_managed_block(
            &path,
            "managed_rtk",
            "export PATH=\"/tmp/headroom/bin:$PATH\"",
        )
        .expect("add managed rtk block");
        upsert_managed_block(
            &path,
            "claude_code",
            "export ANTHROPIC_BASE_URL=http://127.0.0.1:6767",
        )
        .expect("add claude block");

        remove_managed_block(&path, "claude_code").expect("remove claude block");
        remove_managed_block(&path, "managed_rtk").expect("remove managed rtk block");

        let final_content = fs::read_to_string(&path).expect("read round-tripped zshrc");
        assert_eq!(final_content, original);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn updating_one_managed_block_does_not_touch_other_blocks_or_user_content() {
        let root = unique_temp_dir("headroom-multi-block-update");
        fs::create_dir_all(&root).expect("create root");
        let path = root.join(".zprofile");
        let original = r#"eval "$(/opt/homebrew/bin/brew shellenv)"

# >>> ai-switchboard:managed_rtk >>>
export PATH="/old/headroom/bin:$PATH"
# <<< ai-switchboard:managed_rtk <<<

# >>> ai-switchboard:claude_code >>>
export ANTHROPIC_BASE_URL=http://127.0.0.1:6767
# <<< ai-switchboard:claude_code <<<

eval "$(/opt/homebrew/bin/rbenv init - zsh)"
"#;
        fs::write(&path, original).expect("write zprofile");

        upsert_managed_block(
            &path,
            "managed_rtk",
            "export PATH=\"/new/headroom/bin:$PATH\"",
        )
        .expect("update managed rtk block");

        let updated = fs::read_to_string(&path).expect("read updated zprofile");
        assert!(updated.contains("eval \"$(/opt/homebrew/bin/brew shellenv)\""));
        assert!(updated.contains("eval \"$(/opt/homebrew/bin/rbenv init - zsh)\""));
        assert!(updated.contains("export PATH=\"/new/headroom/bin:$PATH\""));
        assert!(updated.contains("export ANTHROPIC_BASE_URL=http://127.0.0.1:6767"));
        assert_eq!(updated.matches("# >>> ai-switchboard:managed_rtk >>>").count(), 1);
        assert_eq!(updated.matches("# >>> ai-switchboard:claude_code >>>").count(), 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn removing_one_managed_block_leaves_other_managed_blocks_and_user_content() {
        let root = unique_temp_dir("headroom-remove-single-block");
        fs::create_dir_all(&root).expect("create root");
        let path = root.join(".zshrc");
        fs::write(
            &path,
            r#"export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

# >>> ai-switchboard:managed_rtk >>>
export PATH="/tmp/headroom/bin:$PATH"
# <<< ai-switchboard:managed_rtk <<<

# >>> ai-switchboard:claude_code >>>
export ANTHROPIC_BASE_URL=http://127.0.0.1:6767
# <<< ai-switchboard:claude_code <<<
"#,
        )
        .expect("write zshrc");

        remove_managed_block(&path, "claude_code").expect("remove claude block");

        let updated = fs::read_to_string(&path).expect("read cleaned zshrc");
        assert!(updated.contains("export NVM_DIR=\"$HOME/.nvm\""));
        assert!(updated.contains("[ -s \"$NVM_DIR/nvm.sh\" ] && \\. \"$NVM_DIR/nvm.sh\""));
        assert!(updated.contains("# >>> ai-switchboard:managed_rtk >>>"));
        assert!(updated.contains("export PATH=\"/tmp/headroom/bin:$PATH\""));
        assert!(!updated.contains("# >>> ai-switchboard:claude_code >>>"));

        let _ = fs::remove_dir_all(root);
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }

    #[test]
    fn strip_hook_returns_false_when_file_missing() {
        let root = unique_temp_dir("headroom-strip-missing");
        let settings = root.join("does-not-exist.json");
        let changed = strip_headroom_hook_from_settings(&settings).expect("strip should succeed");
        assert!(!changed, "missing file should report no change");
        assert!(!settings.exists(), "should not create the file");
    }

    #[test]
    fn strip_hook_removes_headroom_entry_and_leaves_other_entries() {
        let root = unique_temp_dir("headroom-strip-mixed");
        fs::create_dir_all(&root).expect("create root");
        let settings = root.join("settings.json");
        let content = json!({
            "env": { "SOME_KEY": "keep-me" },
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "/other/tool/script.sh" }
                        ]
                    },
                    {
                        "matcher": "Bash",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "/Users/test/.claude/hooks/headroom-rtk-rewrite.sh"
                            }
                        ]
                    }
                ]
            }
        });
        fs::write(&settings, serde_json::to_string_pretty(&content).unwrap())
            .expect("write settings");

        let changed = strip_headroom_hook_from_settings(&settings).expect("strip should succeed");
        assert!(changed, "should report change");

        let raw = fs::read_to_string(&settings).expect("read settings");
        let parsed: serde_json::Value = serde_json::from_str(&raw).expect("parse settings");
        let entries = parsed
            .get("hooks")
            .and_then(|v| v.get("PreToolUse"))
            .and_then(|v| v.as_array())
            .expect("PreToolUse preserved");
        assert_eq!(entries.len(), 1, "only the non-headroom entry remains");
        assert!(
            entry_contains_hook(&entries[0], "other/tool/script.sh"),
            "unrelated entry preserved"
        );
        assert_eq!(
            parsed.get("env").and_then(|v| v.get("SOME_KEY")),
            Some(&json!("keep-me")),
            "unrelated top-level keys untouched"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn strip_hook_drops_empty_pre_tool_use_and_hooks_keys() {
        let root = unique_temp_dir("headroom-strip-empty");
        fs::create_dir_all(&root).expect("create root");
        let settings = root.join("settings.json");
        let content = json!({
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "/path/to/headroom-rtk-rewrite.sh"
                            }
                        ]
                    }
                ]
            }
        });
        fs::write(&settings, serde_json::to_string_pretty(&content).unwrap())
            .expect("write settings");

        let changed = strip_headroom_hook_from_settings(&settings).expect("strip should succeed");
        assert!(changed);

        let raw = fs::read_to_string(&settings).expect("read settings");
        let parsed: serde_json::Value = serde_json::from_str(&raw).expect("parse settings");
        assert!(
            parsed.get("hooks").is_none(),
            "empty hooks object should be removed, got {parsed}"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn strip_hook_leaves_file_untouched_when_no_headroom_entry_present() {
        let root = unique_temp_dir("headroom-strip-noop");
        fs::create_dir_all(&root).expect("create root");
        let settings = root.join("settings.json");
        let original = serde_json::to_string_pretty(&json!({
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": "/unrelated.sh" }
                        ]
                    }
                ]
            }
        }))
        .unwrap();
        fs::write(&settings, &original).expect("write settings");

        let changed = strip_headroom_hook_from_settings(&settings).expect("strip should succeed");
        assert!(!changed, "should report no change");

        let after = fs::read_to_string(&settings).expect("read settings");
        assert_eq!(after, original, "file should be byte-identical");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn strip_hook_tolerates_empty_file() {
        let root = unique_temp_dir("headroom-strip-empty-file");
        fs::create_dir_all(&root).expect("create root");
        let settings = root.join("settings.json");
        fs::write(&settings, "").expect("write empty file");

        let changed = strip_headroom_hook_from_settings(&settings).expect("strip should succeed");
        assert!(!changed, "empty file should report no change");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hook_script_falls_through_when_rewritten_first_token_missing_from_path() {
        // The hook has an OR guard that exits 0 when the binaries are missing,
        // so we give it real paths and verify the PATH-resolution check kicks in
        // when `rtk rewrite` produces a command whose first token can't be
        // resolved. That's the regression-prone slice added this session.
        let root = unique_temp_dir("headroom-hook-bash");
        fs::create_dir_all(&root).expect("create root");

        // Fake rtk that always prepends a made-up binary name that won't be on PATH.
        let fake_rtk = root.join("fake-rtk");
        fs::write(
            &fake_rtk,
            "#!/usr/bin/env bash\nshift  # drop the 'rewrite' arg\necho \"__headroom_nonexistent_binary_xyzzy__ $*\"\n",
        )
        .expect("write fake rtk");
        fs::set_permissions(
            &fake_rtk,
            <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .expect("chmod rtk");

        // Use the real system python3 so the embedded Python snippets run.
        let system_python = PathBuf::from("/usr/bin/python3");
        assert!(system_python.exists(), "this test assumes /usr/bin/python3");

        let hook_body = build_headroom_rtk_hook(&fake_rtk, &system_python);
        let hook_path = root.join("hook.sh");
        fs::write(&hook_path, &hook_body).expect("write hook");
        fs::set_permissions(
            &hook_path,
            <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .expect("chmod hook");

        // Hook expects a JSON object on stdin with tool_input.command.
        let stdin = r#"{"tool_input":{"command":"git status"}}"#;
        let output = std::process::Command::new("bash")
            .arg(&hook_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .as_mut()
                    .unwrap()
                    .write_all(stdin.as_bytes())
                    .unwrap();
                child.wait_with_output()
            })
            .expect("run hook");

        assert!(output.status.success(), "hook should exit 0");
        assert!(
            output.stdout.is_empty(),
            "hook should emit no rewrite when first token isn't resolvable, got: {:?}",
            String::from_utf8_lossy(&output.stdout)
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hook_script_emits_rewrite_when_first_token_is_valid_absolute_path() {
        let root = unique_temp_dir("headroom-hook-bash-ok");
        fs::create_dir_all(&root).expect("create root");

        // Pick a binary that definitely exists on macOS/Linux test hosts.
        let real_binary = "/bin/echo";
        assert!(Path::new(real_binary).exists());

        // Fake rtk rewrites to use an absolute path that *does* exist.
        let fake_rtk = root.join("fake-rtk");
        fs::write(
            &fake_rtk,
            format!("#!/usr/bin/env bash\nshift\necho \"{real_binary} $*\"\n"),
        )
        .expect("write fake rtk");
        fs::set_permissions(
            &fake_rtk,
            <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .expect("chmod rtk");

        let system_python = PathBuf::from("/usr/bin/python3");
        let hook_body = build_headroom_rtk_hook(&fake_rtk, &system_python);
        let hook_path = root.join("hook.sh");
        fs::write(&hook_path, &hook_body).expect("write hook");
        fs::set_permissions(
            &hook_path,
            <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .expect("chmod hook");

        let stdin = r#"{"tool_input":{"command":"git status"}}"#;
        let output = std::process::Command::new("bash")
            .arg(&hook_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .as_mut()
                    .unwrap()
                    .write_all(stdin.as_bytes())
                    .unwrap();
                child.wait_with_output()
            })
            .expect("run hook");

        assert!(output.status.success(), "hook should exit 0");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(real_binary),
            "rewrite should be emitted when first token is a valid absolute path, got stdout: {stdout:?}, stderr: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            stdout.contains("Headroom RTK auto-rewrite"),
            "should be a rewrite hookSpecificOutput payload"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hook_script_pins_bare_rtk_token_to_managed_absolute_path() {
        let root = unique_temp_dir("headroom-hook-pin-rtk");
        fs::create_dir_all(&root).expect("create root");

        // Fake rtk emits a bare `rtk` leading token, like the real binary.
        // `rtk` is NOT on PATH here, so without pinning the rewrite would be a
        // "command not found" landmine and the defense-in-depth guard would
        // drop it. Pinning to the managed absolute path must keep the rewrite.
        let fake_rtk = root.join("rtk");
        fs::write(&fake_rtk, "#!/usr/bin/env bash\nshift\necho \"rtk $*\"\n")
            .expect("write fake rtk");
        fs::set_permissions(
            &fake_rtk,
            <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .expect("chmod rtk");

        let system_python = PathBuf::from("/usr/bin/python3");
        let hook_body = build_headroom_rtk_hook(&fake_rtk, &system_python);
        let hook_path = root.join("hook.sh");
        fs::write(&hook_path, &hook_body).expect("write hook");
        fs::set_permissions(
            &hook_path,
            <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .expect("chmod hook");

        let stdin = r#"{"tool_input":{"command":"git status"}}"#;
        let output = std::process::Command::new("bash")
            .arg(&hook_path)
            .env("PATH", "/usr/bin:/bin") // ensure bare `rtk` is unresolvable
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .as_mut()
                    .unwrap()
                    .write_all(stdin.as_bytes())
                    .unwrap();
                child.wait_with_output()
            })
            .expect("run hook");

        assert!(output.status.success(), "hook should exit 0");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Headroom RTK auto-rewrite"),
            "rewrite should survive when bare `rtk` is pinned to absolute path, got stdout: {stdout:?}, stderr: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            stdout.contains(&fake_rtk.to_string_lossy().replace('"', "\\\"")),
            "rewritten command should invoke the managed rtk by absolute path, got: {stdout:?}"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hook_script_emits_rewrite_even_when_rtk_rewrite_exits_nonzero() {
        let root = unique_temp_dir("headroom-hook-bash-nonzero");
        fs::create_dir_all(&root).expect("create root");

        let real_binary = "/bin/echo";
        assert!(Path::new(real_binary).exists());

        // Match the real rtk behavior we observed during smoke testing:
        // emit a rewrite, then exit non-zero. The hook's `|| true` should
        // still preserve the rewritten command.
        let fake_rtk = root.join("fake-rtk");
        fs::write(
            &fake_rtk,
            format!("#!/usr/bin/env bash\nshift\necho \"{real_binary} $*\"\nexit 3\n"),
        )
        .expect("write fake rtk");
        fs::set_permissions(
            &fake_rtk,
            <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .expect("chmod rtk");

        let system_python = PathBuf::from("/usr/bin/python3");
        let hook_body = build_headroom_rtk_hook(&fake_rtk, &system_python);
        let hook_path = root.join("hook.sh");
        fs::write(&hook_path, &hook_body).expect("write hook");
        fs::set_permissions(
            &hook_path,
            <fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .expect("chmod hook");

        let stdin = r#"{"tool_input":{"command":"git status"}}"#;
        let output = std::process::Command::new("bash")
            .arg(&hook_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .as_mut()
                    .unwrap()
                    .write_all(stdin.as_bytes())
                    .unwrap();
                child.wait_with_output()
            })
            .expect("run hook");

        assert!(output.status.success(), "hook should exit 0");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(real_binary),
            "rewrite output should survive non-zero RTK exit, got stdout: {stdout:?}, stderr: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            stdout.contains("Headroom RTK auto-rewrite"),
            "should still emit a rewrite hookSpecificOutput payload"
        );

        let _ = fs::remove_dir_all(root);
    }

    // ── Lifecycle integration tests ──────────────────────────────────────────
    //
    // These tests drive `apply_client_setup` / `verify_client_setup` /
    // `disable_client_setup` / `clear_client_setups` against a temp $HOME so we
    // catch regressions in the user-visible setup-then-teardown flow. Tests are
    // serialized via `serial_test` because they mutate process-wide env vars
    // (HOME, XDG_DATA_HOME, SHELL).

    /// RAII-style guard that snapshots HOME / XDG_DATA_HOME / SHELL, points
    /// them at a fresh tempdir, and restores them on drop. Used to keep
    /// lifecycle tests from touching the developer's real profile.
    static TEST_HOME_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

    struct TestHome {
        _lock: std::sync::MutexGuard<'static, ()>,
        _tmp: tempfile::TempDir,
        home: PathBuf,
        prev_home: Option<std::ffi::OsString>,
        prev_continue: Option<std::ffi::OsString>,
        prev_xdg: Option<std::ffi::OsString>,
        prev_shell: Option<std::ffi::OsString>,
        prev_codex: Option<std::ffi::OsString>,
        prev_goose_env: Vec<(&'static str, Option<std::ffi::OsString>)>,
    }

    impl TestHome {
        fn new() -> Self {
            let lock = TEST_HOME_LOCK
                .get_or_init(|| std::sync::Mutex::new(()))
                .lock()
                .expect("lock test home env");
            let tmp = tempfile::tempdir().expect("create temp home");
            let home = tmp.path().to_path_buf();
            let prev_home = std::env::var_os("HOME");
            let prev_continue = std::env::var_os("CONTINUE_PATH_ROOT");
            let prev_xdg = std::env::var_os("XDG_DATA_HOME");
            let prev_shell = std::env::var_os("SHELL");
            let prev_codex = std::env::var_os("CODEX_HOME");
            let goose_env_keys = [
                "GOOSE_PROVIDER",
                "OPENAI_HOST",
                "OPENAI_BASE_URL",
                "OPENAI_BASE_PATH",
                "ANTHROPIC_HOST",
            ];
            let prev_goose_env = goose_env_keys
                .iter()
                .map(|key| (*key, std::env::var_os(key)))
                .collect::<Vec<_>>();
            std::env::set_var("HOME", &home);
            std::env::set_var("CONTINUE_PATH_ROOT", home.join(".continue"));
            std::env::set_var("XDG_DATA_HOME", home.join(".local").join("share"));
            // Force a deterministic shell family so tests don't depend on the
            // dev's login shell.
            std::env::set_var("SHELL", "/bin/zsh");
            // Clear any real CODEX_HOME so codex_home() falls back to the temp
            // $HOME/.codex and the Codex tests stay hermetic on dev machines.
            std::env::remove_var("CODEX_HOME");
            // Goose treats endpoint environment variables as higher priority
            // than config.yaml. Clear them for fixture-home lifecycle tests so
            // the native adapter exercises its documented persisted schema;
            // the original values are restored by Drop.
            for (key, _) in &prev_goose_env {
                std::env::remove_var(key);
            }
            // Mirror what the app does at startup so write_setup_state has a
            // config dir to land in.
            crate::storage::ensure_data_dirs(&crate::storage::app_data_dir())
                .expect("ensure_data_dirs in test home");
            TestHome {
                _lock: lock,
                _tmp: tmp,
                home,
                prev_home,
                prev_continue,
                prev_xdg,
                prev_shell,
                prev_codex,
                prev_goose_env,
            }
        }

        fn path(&self) -> &Path {
            &self.home
        }
    }

    impl Drop for TestHome {
        fn drop(&mut self) {
            match self.prev_home.take() {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
            match self.prev_continue.take() {
                Some(v) => std::env::set_var("CONTINUE_PATH_ROOT", v),
                None => std::env::remove_var("CONTINUE_PATH_ROOT"),
            }
            match self.prev_xdg.take() {
                Some(v) => std::env::set_var("XDG_DATA_HOME", v),
                None => std::env::remove_var("XDG_DATA_HOME"),
            }
            match self.prev_shell.take() {
                Some(v) => std::env::set_var("SHELL", v),
                None => std::env::remove_var("SHELL"),
            }
            match self.prev_codex.take() {
                Some(v) => std::env::set_var("CODEX_HOME", v),
                None => std::env::remove_var("CODEX_HOME"),
            }
            for (key, value) in self.prev_goose_env.drain(..) {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    /// RTK is opt-in: its PATH block and Claude Code hook are only wired when the
    /// managed binary exists on disk. Drop a fake one at the default location so
    /// tests covering a fully-configured environment exercise the RTK wiring.
    fn seed_installed_rtk() {
        let rtk = super::default_headroom_rtk_path();
        fs::create_dir_all(rtk.parent().unwrap()).unwrap();
        fs::write(&rtk, "#!/bin/sh\n").unwrap();
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: preview,backup,apply,verify,off
    fn gemini_setup_writes_verifies_and_cleans_sidecar_only() {
        let home = TestHome::new();
        let sidecar = home.path().join(".gemini").join(SWITCHBOARD_ROUTING_FILE);
        fs::create_dir_all(sidecar.parent().unwrap()).expect("create gemini dir");
        fs::write(&sidecar, "# user note\nkeep this\n").expect("seed sidecar");

        let result = super::apply_client_setup("gemini_cli").expect("apply gemini setup");
        assert!(result.applied);
        assert!(!result.already_configured);
        assert!(result
            .changed_files
            .contains(&home.path().join(".zprofile").display().to_string()));
        assert!(result
            .changed_files
            .contains(&sidecar.display().to_string()));
        assert_eq!(result.backup_files.len(), 1);
        assert!(result.verification.verified);
        assert!(result.summary.contains("Switchboard sidecar written"));

        let content = fs::read_to_string(&sidecar).expect("read sidecar");
        assert!(content.contains("# user note\nkeep this"));
        assert!(content.contains("# >>> ai-switchboard:gemini_cli >>>"));
        assert!(content.contains(super::HEADROOM_OPENAI_BASE_URL));
        let shell_content = fs::read_to_string(home.path().join(".zprofile")).expect("read shell");
        assert!(shell_content.contains("GOOGLE_GEMINI_BASE_URL=http://127.0.0.1:6767"));
        assert!(shell_content.contains("GEMINI_BASE_URL=http://127.0.0.1:6767"));

        let drifted = content.replace(super::HEADROOM_OPENAI_BASE_URL, "http://127.0.0.1:1");
        fs::write(&sidecar, drifted).expect("drift gemini sidecar");
        let drift_verification =
            super::verify_client_setup("gemini_cli").expect("verify drifted gemini setup");
        assert!(!drift_verification.verified);
        let repaired = super::apply_client_setup("gemini_cli").expect("repair gemini setup");
        assert!(repaired.verification.verified);

        let detected_clients = vec![ClientStatus {
            id: "gemini_cli".into(),
            name: "Gemini CLI".into(),
            installed: true,
            configured: false,
            health: ClientHealth::Attention,
            notes: vec![
                "Gemini binary: /opt/homebrew/bin/gemini".into(),
                format!(
                    "Gemini config surface: {}",
                    home.path().join(".gemini").display()
                ),
            ],
        }];
        let connectors = list_client_connectors(&detected_clients).expect("list connectors");
        let gemini = connectors
            .iter()
            .find(|connector| connector.client_id == "gemini_cli")
            .expect("gemini connector");
        assert!(gemini.enabled);
        assert!(gemini.verified);
        assert!(gemini.last_configured_at.is_some());
        assert!(gemini
            .automation_path
            .iter()
            .all(|stage| stage.status == "ready"));

        super::disable_client_setup("gemini_cli").expect("disable gemini setup");
        let content = fs::read_to_string(&sidecar).expect("read cleaned sidecar");
        assert_eq!(content, "# user note\nkeep this\n");
        let shell_content = fs::read_to_string(home.path().join(".zprofile")).expect("read shell");
        assert!(!shell_content.contains("GOOGLE_GEMINI_BASE_URL"));
        let verification =
            super::verify_client_setup("gemini_cli").expect("verify cleaned gemini setup");
        assert!(!verification.verified);
        let cleaned_once = fs::read_to_string(&sidecar).expect("read gemini after first off");
        super::disable_client_setup("gemini_cli").expect("repeat disable gemini setup");
        assert_eq!(fs::read_to_string(&sidecar).expect("read gemini after second off"), cleaned_once);
    }

    #[test]
    #[serial_test::serial]
    fn gemini_verification_fails_closed_for_each_drifted_shell_export() {
        let home = TestHome::new();
        let sidecar = home.path().join(".gemini").join(SWITCHBOARD_ROUTING_FILE);
        fs::create_dir_all(sidecar.parent().unwrap()).expect("create gemini dir");
        fs::write(&sidecar, "# user note\nkeep this\n").expect("seed sidecar");
        let applied = super::apply_client_setup("gemini_cli").expect("apply gemini setup");
        let shell_paths = applied
            .changed_files
            .iter()
            .map(PathBuf::from)
            .filter(|path| path.file_name().is_some_and(|name| name != SWITCHBOARD_ROUTING_FILE))
            .collect::<Vec<_>>();
        assert!(!shell_paths.is_empty(), "apply should report Gemini shell targets");
        let exports = [
            ("GOOGLE_GEMINI_BASE_URL", "http://127.0.0.1:6767", "http://127.0.0.1:1"),
            ("GEMINI_BASE_URL", "http://127.0.0.1:6767", "http://127.0.0.1:2"),
            ("GEMINI_API_KEY", "headroom-local", "wrong-local-key"),
        ];
        for (key, expected, drifted) in exports {
            let expected_line = format!("export {key}={expected}");
            let drifted_line = format!("export {key}={drifted}");
            let mut drifted_target = false;
            for shell_path in &shell_paths {
                let shell = fs::read_to_string(shell_path).expect("read gemini shell");
                if shell.contains(&expected_line) {
                    fs::write(shell_path, shell.replace(&expected_line, &drifted_line))
                        .expect("write drifted gemini shell");
                    drifted_target = true;
                }
            }
            assert!(drifted_target, "missing {key} before drift");

            let verification =
                super::verify_client_setup("gemini_cli").expect("verify drifted gemini shell");
            assert!(!verification.verified, "verification should fail for {key}");
            let repaired = super::apply_client_setup("gemini_cli").expect("repair gemini shell");
            assert!(repaired.verification.verified, "repair should restore {key}");
        }
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: rollback
    fn gemini_managed_rollback_removes_shell_and_sidecar_blocks() {
        let home = TestHome::new();
        let sidecar = home.path().join(".gemini").join(SWITCHBOARD_ROUTING_FILE);
        fs::create_dir_all(sidecar.parent().unwrap()).expect("create gemini dir");
        fs::write(&sidecar, "# user note\nkeep this\n").expect("seed sidecar");

        super::apply_client_setup("gemini_cli").expect("apply gemini setup");

        let preview =
            super::preview_managed_rollback("gemini-routing").expect("preview gemini rollback");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.backup_path.is_none());
        assert!(preview.backup_exists);
        assert!(preview.marker_present);
        assert_eq!(
            preview.confirmation_phrase,
            "Restore ai-switchboard:gemini_cli for Gemini CLI routing"
        );

        let result = super::execute_managed_rollback(
            "gemini-routing",
            "",
            "Restore ai-switchboard:gemini_cli for Gemini CLI routing",
        )
        .expect("execute gemini rollback");
        assert_eq!(
            result.restored_from,
            "Switchboard-owned Gemini shell and sidecar blocks removed."
        );

        let content = fs::read_to_string(&sidecar).expect("read cleaned sidecar");
        assert_eq!(content, "# user note\nkeep this\n");
        let shell_content = fs::read_to_string(home.path().join(".zprofile")).expect("read shell");
        assert!(!shell_content.contains("GOOGLE_GEMINI_BASE_URL"));
    }

    #[test]
    #[serial_test::serial]
    fn sidecar_managed_rollback_removes_existing_cursor_sidecar_block_only() {
        let home = TestHome::new();
        let sidecar = home
            .path()
            .join("Library")
            .join("Application Support")
            .join("Cursor")
            .join(SWITCHBOARD_ROUTING_FILE);
        fs::create_dir_all(sidecar.parent().unwrap()).expect("create cursor dir");
        fs::write(&sidecar, "# cursor user note\nkeep this\n").expect("seed sidecar");

        super::configure_planned_switchboard_sidecar("cursor").expect("seed cursor sidecar");

        let preview =
            super::preview_managed_rollback("cursor-routing").expect("preview cursor rollback");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.backup_path.is_none());
        assert!(preview.backup_exists);
        assert!(preview.marker_present);
        assert_eq!(
            preview.confirmation_phrase,
            "Restore ai-switchboard:cursor for Cursor routing"
        );
        assert!(preview.proposed_action.contains("Cursor sidecar block"));
        assert!(preview
            .evidence
            .join(" ")
            .contains("Current sidecar must still contain"));

        let result = super::execute_managed_rollback(
            "cursor-routing",
            "",
            "Restore ai-switchboard:cursor for Cursor routing",
        )
        .expect("execute cursor rollback");
        assert_eq!(
            result.restored_from,
            "Switchboard-owned cursor sidecar block removed."
        );
        let safety_backup = result
            .safety_backup_path
            .as_ref()
            .expect("sidecar rollback reports safety backup");
        assert!(
            safety_backup.contains(".headroom-backup-"),
            "unexpected safety backup path: {safety_backup}"
        );
        assert!(std::path::Path::new(safety_backup).exists());
        assert!(result
            .verification
            .join(" ")
            .contains("fresh sidecar safety backup"));
        assert!(result
            .verification
            .join(" ")
            .contains("Relaunch-survival evidence"));

        let content = fs::read_to_string(&sidecar).expect("read cleaned sidecar");
        assert_eq!(content, "# cursor user note\nkeep this\n");
        assert!(!super::planned_switchboard_sidecar_matches("cursor")
            .expect("check cleaned cursor sidecar"));
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: preview,backup,apply,verify,rollback,off
    fn amazon_q_sidecar_lifecycle_applies_repairs_rolls_back_and_disables() {
        let home = TestHome::new();
        let sidecar = home
            .path()
            .join(".aws")
            .join("amazonq")
            .join(SWITCHBOARD_ROUTING_FILE);
        fs::create_dir_all(sidecar.parent().unwrap()).expect("create amazon q dir");
        fs::write(&sidecar, "# amazon q user note\nkeep this\n").expect("seed sidecar");

        let result = super::apply_client_setup("amazon_q").expect("apply amazon q setup");
        assert!(result.applied);
        assert!(!result.already_configured);
        assert_eq!(result.changed_files, vec![sidecar.display().to_string()]);
        assert_eq!(result.backup_files.len(), 1);
        assert!(result.verification.verified);
        assert!(result.summary.contains("Amazon Q Developer CLI"));

        let content = fs::read_to_string(&sidecar).expect("read amazon q sidecar");
        assert!(content.contains("# amazon q user note\nkeep this"));
        assert!(content.contains("# >>> ai-switchboard:amazon_q >>>"));
        assert!(content.contains(super::HEADROOM_OPENAI_BASE_URL));
        assert!(content.contains("Amazon Q Developer CLI routing-intent sidecar"));

        let connectors = list_client_connectors(&[ClientStatus {
            id: "amazon_q".into(),
            name: "Amazon Q Developer CLI".into(),
            installed: true,
            configured: false,
            health: ClientHealth::Attention,
            notes: vec![format!(
                "Amazon Q config surface: {}",
                sidecar.parent().unwrap().display()
            )],
        }])
        .expect("list connectors");
        let amazon_q = connectors
            .iter()
            .find(|connector| connector.client_id == "amazon_q")
            .expect("amazon q connector");
        assert_eq!(
            amazon_q.support_status,
            ClientConnectorSupportStatus::Managed
        );
        assert!(amazon_q.enabled);
        assert!(amazon_q.verified);
        assert!(amazon_q.config_creation_steps.is_empty());
        assert!(amazon_q.automation_path.is_empty());

        let drifted = content.replace(super::HEADROOM_OPENAI_BASE_URL, "http://127.0.0.1:1");
        fs::write(&sidecar, drifted).expect("drift amazon q sidecar");
        let verification =
            super::verify_client_setup("amazon_q").expect("verify drifted amazon q setup");
        assert!(!verification.verified);
        assert!(verification
            .failures
            .join(" ")
            .contains("Switchboard-managed Amazon Q Developer CLI sidecar was not found"));

        let repaired = super::apply_client_setup("amazon_q").expect("repair amazon q setup");
        assert!(repaired.verification.verified);
        assert!(fs::read_to_string(&sidecar)
            .expect("read repaired amazon q sidecar")
            .contains(super::HEADROOM_OPENAI_BASE_URL));

        let preview =
            super::preview_managed_rollback("amazon-q-routing").expect("preview amazon q rollback");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.marker_present);
        assert_eq!(
            preview.confirmation_phrase,
            "Restore ai-switchboard:amazon_q for Amazon Q Developer CLI routing"
        );

        let rollback = super::execute_managed_rollback(
            "amazon-q-routing",
            "",
            "Restore ai-switchboard:amazon_q for Amazon Q Developer CLI routing",
        )
        .expect("execute amazon q rollback");
        assert_eq!(
            rollback.restored_from,
            "Switchboard-owned amazon_q sidecar block removed."
        );
        assert_eq!(
            fs::read_to_string(&sidecar).expect("read rolled back amazon q sidecar"),
            "# amazon q user note\nkeep this\n"
        );

        super::apply_client_setup("amazon_q").expect("reapply amazon q setup");
        super::disable_client_setup("amazon_q").expect("disable amazon q setup");
        assert_eq!(
            fs::read_to_string(&sidecar).expect("read disabled amazon q sidecar"),
            "# amazon q user note\nkeep this\n"
        );
        let verification =
            super::verify_client_setup("amazon_q").expect("verify disabled amazon q setup");
        assert!(!verification.verified);
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: preview,backup,apply,verify,rollback,off
    fn aider_sidecar_lifecycle_applies_repairs_rolls_back_and_disables() {
        let home = TestHome::new();
        let config = home.path().join(".aider.conf.yml");
        let sidecar = home
            .path()
            .join(".config")
            .join("aider")
            .join(SWITCHBOARD_ROUTING_FILE);
        fs::create_dir_all(sidecar.parent().unwrap()).expect("create aider dir");
        fs::write(
            &config,
            "model: gpt-4o\n# user aider settings\n",
        )
        .expect("seed aider config");
        fs::write(&sidecar, "# aider user note\nkeep this\n").expect("seed sidecar");

        let prev_aider = std::env::var_os("AIDER_CONFIG_PATH");
        std::env::set_var("AIDER_CONFIG_PATH", &config);

        let result = super::apply_client_setup("aider").expect("apply aider setup");
        assert!(result.applied);
        assert!(!result.already_configured);
        assert_eq!(result.changed_files.len(), 2);
        assert!(result
            .changed_files
            .iter()
            .any(|path| path == &config.display().to_string()));
        assert!(result
            .changed_files
            .iter()
            .any(|path| path == &sidecar.display().to_string()));
        assert_eq!(result.backup_files.len(), 2);
        assert!(result.verification.verified);
        assert!(result.summary.contains("Aider"));

        let config_content = fs::read_to_string(&config).expect("read aider config");
        assert!(config_content.contains(crate::aider_provider_configs::AIDER_OPENAI_API_BASE_KEY));
        assert!(config_content.contains(super::HEADROOM_OPENAI_BASE_URL));
        assert!(crate::aider_provider_configs::aider_provider_config_matches()
            .expect("verify aider provider"));

        let content = fs::read_to_string(&sidecar).expect("read aider sidecar");
        assert!(content.contains("# aider user note\nkeep this"));
        assert!(content.contains("# >>> ai-switchboard:aider >>>"));
        assert!(content.contains(super::HEADROOM_OPENAI_BASE_URL));
        assert!(content.contains("Aider routing-intent sidecar"));

        let connectors = list_client_connectors(&[ClientStatus {
            id: "aider".into(),
            name: "Aider".into(),
            installed: true,
            configured: false,
            health: ClientHealth::Attention,
            notes: vec![format!(
                "Aider config surface: {}",
                sidecar.parent().unwrap().display()
            )],
        }])
        .expect("list connectors");
        let aider = connectors
            .iter()
            .find(|connector| connector.client_id == "aider")
            .expect("aider connector");
        assert_eq!(aider.support_status, ClientConnectorSupportStatus::Managed);
        assert!(aider.enabled);
        assert!(aider.verified);
        assert!(aider.config_creation_steps.is_empty());
        assert!(aider.automation_path.is_empty());

        let drifted = content.replace(super::HEADROOM_OPENAI_BASE_URL, "http://127.0.0.1:1");
        fs::write(&sidecar, drifted).expect("drift aider sidecar");
        let verification = super::verify_client_setup("aider").expect("verify drifted aider setup");
        assert!(!verification.verified);
        assert!(verification
            .failures
            .join(" ")
            .contains("Switchboard-managed Aider sidecar was not found"));

        let repaired = super::apply_client_setup("aider").expect("repair aider setup");
        assert!(repaired.verification.verified);
        assert!(fs::read_to_string(&sidecar)
            .expect("read repaired aider sidecar")
            .contains(super::HEADROOM_OPENAI_BASE_URL));

        let preview =
            super::preview_managed_rollback("aider-routing").expect("preview aider rollback");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.marker_present);
        assert_eq!(
            preview.confirmation_phrase,
            "Restore ai-switchboard:aider for Aider routing"
        );

        let rollback = super::execute_managed_rollback(
            "aider-routing",
            "",
            "Restore ai-switchboard:aider for Aider routing",
        )
        .expect("execute aider rollback");
        assert_eq!(
            rollback.restored_from,
            "Switchboard-owned aider sidecar block removed."
        );
        assert_eq!(
            fs::read_to_string(&sidecar).expect("read rolled back aider sidecar"),
            "# aider user note\nkeep this\n"
        );

        let provider_preview = super::preview_managed_config_apply("aider-provider-routing")
            .expect("preview aider provider apply");
        assert_eq!(provider_preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(provider_preview
            .confirmation_phrase
            .starts_with("Apply ai-switchboard:aider-provider to"));

        let provider_rollback =
            super::preview_managed_rollback("aider-provider-routing")
                .expect("preview aider provider rollback");
        assert_eq!(provider_rollback.status, ManagedRollbackExecutionStatus::Ready);
        assert_eq!(
            provider_rollback.confirmation_phrase,
            "Restore ai-switchboard:aider-provider for Aider provider routing"
        );

        super::apply_client_setup("aider").expect("reapply aider setup");
        super::disable_client_setup("aider").expect("disable aider setup");
        assert_eq!(
            fs::read_to_string(&sidecar).expect("read disabled aider sidecar"),
            "# aider user note\nkeep this\n"
        );
        assert!(!crate::aider_provider_configs::aider_provider_config_matches()
            .expect("aider provider removed"));
        let verification =
            super::verify_client_setup("aider").expect("verify disabled aider setup");
        assert!(!verification.verified);

        match prev_aider {
            Some(value) => std::env::set_var("AIDER_CONFIG_PATH", value),
            None => std::env::remove_var("AIDER_CONFIG_PATH"),
        }
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: preview,backup,apply,verify,rollback,off
    fn continue_sidecar_lifecycle_applies_repairs_rolls_back_and_disables() {
        let home = TestHome::new();
        let continue_dir = home.path().join(".continue");
        let sidecar = continue_dir.join(SWITCHBOARD_ROUTING_FILE);
        let config = continue_dir.join("config.yaml");
        fs::create_dir_all(&continue_dir).expect("create continue dir");
        fs::write(
            &config,
            "name: User Config\nversion: 1.0.0\nschema: v1\nmodels: []\n",
        )
        .expect("seed continue config");
        fs::write(&sidecar, "# continue user note\nkeep this\n").expect("seed sidecar");

        let result = super::apply_client_setup("continue").expect("apply continue setup");
        assert!(result.applied);
        assert!(!result.already_configured);
        assert_eq!(result.changed_files.len(), 2);
        assert!(result
            .changed_files
            .iter()
            .any(|path| path == &config.display().to_string()));
        assert!(result
            .changed_files
            .iter()
            .any(|path| path == &sidecar.display().to_string()));
        assert_eq!(result.backup_files.len(), 2);
        assert!(result.verification.verified);
        assert!(result.summary.contains("Continue"));

        let config_content = fs::read_to_string(&config).expect("read continue config");
        assert!(config_content.contains("AI Switchboard"));
        assert!(config_content.contains(super::HEADROOM_OPENAI_BASE_URL));
        assert!(crate::continue_provider_configs::continue_provider_config_matches()
            .expect("verify continue provider"));

        let content = fs::read_to_string(&sidecar).expect("read continue sidecar");
        assert!(content.contains("# continue user note\nkeep this"));
        assert!(content.contains("# >>> ai-switchboard:continue >>>"));
        assert!(content.contains(super::HEADROOM_OPENAI_BASE_URL));
        assert!(content.contains("Continue routing-intent sidecar"));

        let connectors = list_client_connectors(&[ClientStatus {
            id: "continue".into(),
            name: "Continue".into(),
            installed: true,
            configured: false,
            health: ClientHealth::Attention,
            notes: vec![format!(
                "Continue config folder: {}",
                sidecar.parent().unwrap().display()
            )],
        }])
        .expect("list connectors");
        let continue_connector = connectors
            .iter()
            .find(|connector| connector.client_id == "continue")
            .expect("continue connector");
        assert_eq!(
            continue_connector.support_status,
            ClientConnectorSupportStatus::Managed
        );
        assert!(continue_connector.enabled);
        assert!(continue_connector.verified);
        assert!(continue_connector.config_creation_steps.is_empty());
        assert!(continue_connector.automation_path.is_empty());

        let drifted = content.replace(super::HEADROOM_OPENAI_BASE_URL, "http://127.0.0.1:1");
        fs::write(&sidecar, drifted).expect("drift continue sidecar");
        let verification =
            super::verify_client_setup("continue").expect("verify drifted continue setup");
        assert!(!verification.verified);
        assert!(verification
            .failures
            .join(" ")
            .contains("Switchboard-managed Continue sidecar was not found"));

        let repaired = super::apply_client_setup("continue").expect("repair continue setup");
        assert!(repaired.verification.verified);
        assert!(fs::read_to_string(&sidecar)
            .expect("read repaired continue sidecar")
            .contains(super::HEADROOM_OPENAI_BASE_URL));

        let preview =
            super::preview_managed_rollback("continue-routing").expect("preview continue rollback");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.marker_present);
        assert_eq!(
            preview.confirmation_phrase,
            "Restore ai-switchboard:continue for Continue routing"
        );

        let rollback = super::execute_managed_rollback(
            "continue-routing",
            "",
            "Restore ai-switchboard:continue for Continue routing",
        )
        .expect("execute continue rollback");
        assert_eq!(
            rollback.restored_from,
            "Switchboard-owned continue sidecar block removed."
        );
        assert_eq!(
            fs::read_to_string(&sidecar).expect("read rolled back continue sidecar"),
            "# continue user note\nkeep this\n"
        );

        super::apply_client_setup("continue").expect("reapply continue setup");

        let provider_preview = super::preview_managed_config_apply("continue-provider-routing")
            .expect("preview continue provider apply");
        assert_eq!(provider_preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(provider_preview
            .confirmation_phrase
            .starts_with("Apply ai-switchboard:continue-provider to"));

        let provider_rollback =
            super::preview_managed_rollback("continue-provider-routing")
                .expect("preview continue provider rollback");
        assert_eq!(provider_rollback.status, ManagedRollbackExecutionStatus::Ready);
        assert_eq!(
            provider_rollback.confirmation_phrase,
            "Restore ai-switchboard:continue-provider for Continue provider routing"
        );

        super::disable_client_setup("continue").expect("disable continue setup");
        assert_eq!(
            fs::read_to_string(&sidecar).expect("read disabled continue sidecar"),
            "# continue user note\nkeep this\n"
        );
        assert!(!crate::continue_provider_configs::continue_provider_config_matches()
            .expect("continue provider removed"));
        let verification =
            super::verify_client_setup("continue").expect("verify disabled continue setup");
        assert!(!verification.verified);
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: apply,verify,off
    fn goose_native_provider_and_mcp_bridge_lifecycle_preserves_user_state() {
        let home = TestHome::new();
        let config = home
            .path()
            .join("Library")
            .join("Application Support")
            .join("Block")
            .join("goose")
            .join("config.yaml");
        fs::create_dir_all(config.parent().unwrap()).expect("create goose config dir");
        fs::write(
            &config,
            "active_provider: openai\nproviders:\n  openai:\n    enabled: true\n    model: gpt-4o\n    configured: true\nkeep: true\n",
        )
        .expect("seed goose config");

        let sidecar = planned_sidecar_routing_path("goose").expect("goose sidecar path");
        fs::create_dir_all(sidecar.parent().unwrap()).expect("create goose dir");
        fs::write(&sidecar, "# goose user note\nkeep this\n").expect("seed sidecar");

        let result = super::apply_client_setup("goose").expect("apply goose setup");
        assert!(result.applied);
        assert!(!result.already_configured);
        assert!(result.changed_files.contains(&config.display().to_string()));
        assert!(result
            .changed_files
            .contains(&sidecar.display().to_string()));
        assert_eq!(result.backup_files.len(), 2);
        assert!(result.verification.verified);
        assert!(result.summary.contains("Repo Memory MCP bridge"));
        assert!(result
            .summary
            .contains("credentials and account state remain manual"));
        assert!(result
            .next_steps
            .iter()
            .any(|step| step.contains("allowlisted provider endpoint fields")));

        let content = fs::read_to_string(&sidecar).expect("read goose sidecar");
        assert!(content.contains("# goose user note\nkeep this"));
        assert!(content.contains("# >>> ai-switchboard:goose >>>"));
        assert!(content.contains(super::HEADROOM_OPENAI_BASE_URL));
        assert!(content.contains("Repo Memory MCP bridge marker"));
        assert!(content.contains("allowlisted native endpoint routing"));
        assert!(content.contains(
            "account state, secrets, provider credentials, and model selection remain manual"
        ));

        let config_content = fs::read_to_string(&config).expect("read configured goose config");
        assert!(config_content.contains("active_provider: openai"));
        assert!(config_content.contains("model: gpt-4o"));
        assert!(config_content.contains("keep: true"));
        assert!(config_content.contains(super::HEADROOM_OPENAI_BASE_URL));
        assert!(config_content.contains("OPENAI_BASE_PATH: v1/chat/completions"));

        let preview =
            super::preview_managed_rollback("goose-routing").expect("preview goose rollback");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.marker_present);
        assert_eq!(
            preview.confirmation_phrase,
            "Restore ai-switchboard:goose for Goose MCP bridge"
        );

        let rollback = super::execute_managed_rollback(
            "goose-routing",
            "",
            "Restore ai-switchboard:goose for Goose MCP bridge",
        )
        .expect("execute goose rollback");
        assert_eq!(
            rollback.restored_from,
            "Switchboard-owned goose sidecar block removed."
        );
        assert_eq!(
            fs::read_to_string(&sidecar).expect("read rolled back goose sidecar"),
            "# goose user note\nkeep this\n"
        );

        super::apply_client_setup("goose").expect("reapply goose setup");
        super::disable_client_setup("goose").expect("disable goose setup");
        assert_eq!(
            fs::read_to_string(&sidecar).expect("read disabled goose sidecar"),
            "# goose user note\nkeep this\n"
        );
        let disabled_config = fs::read_to_string(&config).expect("read disabled goose config");
        assert!(disabled_config.contains("active_provider: openai"));
        assert!(disabled_config.contains("model: gpt-4o"));
        assert!(disabled_config.contains("keep: true"));
        assert!(!disabled_config.contains(super::HEADROOM_OPENAI_BASE_URL));
        let verification =
            super::verify_client_setup("goose").expect("verify disabled goose setup");
        assert!(!verification.verified);
    }

    #[test]
    #[serial_test::serial]
    fn goose_and_grok_sidecar_apply_requires_exact_current_confirmation_and_preserves_user_content()
    {
        let home = TestHome::new();
        for (record_id, client_id, owner) in [
            (
                super::GOOSE_SIDECAR_APPLY_RECORD_ID,
                "goose",
                super::GOOSE_SIDECAR_OWNER,
            ),
            (
                super::GROK_SIDECAR_APPLY_RECORD_ID,
                "grok_cli",
                super::GROK_SIDECAR_OWNER,
            ),
        ] {
            let sidecar = planned_sidecar_routing_path(client_id).expect("sidecar path");
            fs::create_dir_all(sidecar.parent().unwrap()).expect("create sidecar parent");
            fs::write(&sidecar, format!("# {client_id} user note\nkeep this\n"))
                .expect("seed user content");

            let preview = super::preview_managed_config_apply(record_id).expect("preview");
            assert!(preview.current_state.contains("keep this"));
            assert!(preview
                .proposed_state
                .contains(&format!("ai-switchboard:{client_id}")));
            assert!(preview
                .evidence
                .iter()
                .any(|item| item.contains("not allowlisted")));

            fs::write(&sidecar, "changed outside Switchboard\n").expect("create stale preview");
            assert!(
                super::execute_managed_config_apply(record_id, &preview.confirmation_phrase)
                    .is_err()
            );
            assert_eq!(
                fs::read_to_string(&sidecar).unwrap(),
                "changed outside Switchboard\n"
            );

            let preview = super::preview_managed_config_apply(record_id).expect("fresh preview");
            let applied =
                super::execute_managed_config_apply(record_id, &preview.confirmation_phrase)
                    .expect("apply sidecar");
            assert_eq!(applied.owner, owner);
            assert!(super::planned_switchboard_sidecar_matches(client_id).unwrap());
            assert!(applied
                .verification
                .iter()
                .any(|item| item.contains("credentials")));

            super::disable_client_setup(client_id).expect("off cleanup");
            let cleaned = fs::read_to_string(&sidecar).expect("read cleanup");
            assert!(cleaned.contains("changed outside Switchboard"));
            assert!(!super::planned_switchboard_sidecar_matches(client_id).unwrap());
        }
        assert!(!home
            .path()
            .join(".config")
            .join("xai")
            .join("auth.json")
            .exists());
    }

    #[test]
    #[serial_test::serial]
    fn promoted_editor_rollback_records_use_native_targets_not_sidecars() {
        let _home = TestHome::new();

        assert!(super::sidecar_rollback_target("windsurf-routing").is_none());
        assert!(super::sidecar_rollback_target("zed-ai-routing").is_none());

        let windsurf =
            super::preview_managed_rollback("windsurf-routing").expect("preview windsurf rollback");
        assert_eq!(windsurf.record_id, "windsurf-routing");
        assert_eq!(windsurf.marker, "ai-switchboard:windsurf");
        assert!(windsurf
            .target_path
            .ends_with("Library/Application Support/Windsurf/User/settings.json"));
        assert!(windsurf
            .proposed_action
            .contains("Restore the Windsurf settings"));

        let zed = super::preview_managed_rollback("zed-ai-routing").expect("preview zed rollback");
        assert_eq!(zed.record_id, "zed-ai-routing");
        assert_eq!(zed.marker, "ai-switchboard:zed");
        assert!(zed.target_path.ends_with(".config/zed/settings.json"));
        assert!(zed.proposed_action.contains("Restore the Zed settings"));
    }

    #[test]
    #[serial_test::serial]
    fn managed_rollback_undo_all_executes_ready_native_rows_only() {
        let home = TestHome::new();
        let gemini_sidecar = home.path().join(".gemini").join(SWITCHBOARD_ROUTING_FILE);
        fs::create_dir_all(gemini_sidecar.parent().unwrap()).expect("create gemini dir");
        fs::write(&gemini_sidecar, "# gemini user note\nkeep this\n").expect("seed gemini");
        let cursor_sidecar = home
            .path()
            .join("Library")
            .join("Application Support")
            .join("Cursor")
            .join(SWITCHBOARD_ROUTING_FILE);
        fs::create_dir_all(cursor_sidecar.parent().unwrap()).expect("create cursor dir");
        fs::write(&cursor_sidecar, "# cursor user note\nkeep this\n").expect("seed cursor");

        super::apply_client_setup("gemini_cli").expect("apply gemini setup");
        super::configure_planned_switchboard_sidecar("cursor").expect("seed cursor sidecar");

        let preview = super::preview_managed_rollback_undo_all();
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        let ready_ids = preview
            .ready
            .iter()
            .map(|row| row.record_id.as_str())
            .collect::<Vec<_>>();
        assert!(ready_ids.contains(&"gemini-routing"));
        assert!(ready_ids.contains(&"cursor-routing"));
        assert!(
            !preview.blocked.is_empty(),
            "unused native rows should remain blocked"
        );

        let result = super::execute_managed_rollback_undo_all(
            "Undo all ready Switchboard native rollback rows",
        )
        .expect("execute undo-all");
        let executed_ids = result
            .executed
            .iter()
            .map(|row| row.record_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(executed_ids, vec!["gemini-routing", "cursor-routing"]);
        let cursor_result = result
            .executed
            .iter()
            .find(|row| row.record_id == "cursor-routing")
            .expect("cursor rollback result");
        assert!(cursor_result.safety_backup_path.is_some());
        assert_eq!(
            fs::read_to_string(&gemini_sidecar).expect("read cleaned gemini"),
            "# gemini user note\nkeep this\n"
        );
        assert_eq!(
            fs::read_to_string(&cursor_sidecar).expect("read cleaned cursor"),
            "# cursor user note\nkeep this\n"
        );
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: apply,verify,off
    fn opencode_setup_writes_verifies_and_cleans_native_routing_only() {
        let home = TestHome::new();
        let sidecar = home
            .path()
            .join(".config")
            .join("opencode")
            .join(SWITCHBOARD_ROUTING_FILE);
        let config = home
            .path()
            .join(".config")
            .join("opencode")
            .join(OPENCODE_CONFIG_FILE);
        fs::create_dir_all(sidecar.parent().unwrap()).expect("create opencode dir");
        fs::write(&sidecar, "# opencode user note\nkeep this\n").expect("seed sidecar");
        fs::write(
            &config,
            r#"{"provider":{"custom":{"name":"Custom"}},"theme":"system"}"#,
        )
        .expect("seed opencode config");

        let result = super::apply_client_setup("opencode").expect("apply opencode setup");
        assert!(result.applied);
        assert!(!result.already_configured);
        assert!(result.changed_files.contains(&config.display().to_string()));
        assert!(result
            .changed_files
            .contains(&sidecar.display().to_string()));
        assert_eq!(result.backup_files.len(), 2);
        assert!(result.verification.verified);
        assert!(result
            .summary
            .contains("OpenCode Switchboard sidecar written"));

        let content = fs::read_to_string(&sidecar).expect("read sidecar");
        assert!(content.contains("# opencode user note\nkeep this"));
        assert!(content.contains("# >>> ai-switchboard:opencode >>>"));
        assert!(content.contains(super::HEADROOM_OPENAI_BASE_URL));
        let config_value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&config).expect("read config"))
                .expect("parse config");
        assert_eq!(config_value["theme"], "system");
        assert_eq!(config_value["provider"]["custom"]["name"], "Custom");
        assert_eq!(
            config_value["provider"]["headroom"]["options"]["baseURL"],
            super::HEADROOM_OPENAI_BASE_URL
        );

        let detected_clients = vec![ClientStatus {
            id: "opencode".into(),
            name: "OpenCode".into(),
            installed: true,
            configured: false,
            health: ClientHealth::Attention,
            notes: vec![
                "OpenCode binary: /opt/homebrew/bin/opencode".into(),
                format!(
                    "OpenCode config surface: {}",
                    home.path().join(".config").join("opencode").display()
                ),
            ],
        }];
        let connectors = list_client_connectors(&detected_clients).expect("list connectors");
        let opencode = connectors
            .iter()
            .find(|connector| connector.client_id == "opencode")
            .expect("opencode connector");
        assert!(opencode.enabled);
        assert!(opencode.verified);
        assert!(opencode.last_configured_at.is_some());
        assert!(opencode
            .automation_path
            .iter()
            .all(|stage| stage.status == "ready"));

        super::disable_client_setup("opencode").expect("disable opencode setup");
        let content = fs::read_to_string(&sidecar).expect("read cleaned sidecar");
        assert_eq!(content, "# opencode user note\nkeep this\n");
        let config_value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&config).expect("read config"))
                .expect("parse config");
        assert!(config_value["provider"]["headroom"].is_null());
        assert_eq!(config_value["provider"]["custom"]["name"], "Custom");
        let verification =
            super::verify_client_setup("opencode").expect("verify cleaned opencode setup");
        assert!(!verification.verified);
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: apply,verify,off
    fn windsurf_setup_writes_verifies_and_off_cleanup_removes_native_routing_only() {
        let home = TestHome::new();
        let windsurf_dir = home
            .path()
            .join("Library")
            .join("Application Support")
            .join("Windsurf")
            .join("User");
        fs::create_dir_all(&windsurf_dir).unwrap();
        let settings_json = windsurf_dir.join("settings.json");
        fs::write(
            &settings_json,
            r#"{"workbench.colorTheme":"Quiet Light","assistant":{"defaultModel":"claude-3-5-sonnet"}}"#,
        )
        .unwrap();

        let result = super::apply_client_setup("windsurf").expect("apply windsurf setup");
        assert!(result.applied);
        assert!(!result.already_configured);
        assert!(result
            .changed_files
            .contains(&settings_json.display().to_string()));
        assert_eq!(result.backup_files.len(), 1);
        assert!(result.verification.verified);

        let configured: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&settings_json).expect("read settings"))
                .expect("parse settings");
        assert_eq!(configured["workbench.colorTheme"], "Quiet Light");
        assert_eq!(configured["assistant"]["defaultModel"], "claude-3-5-sonnet");
        assert_eq!(
            configured["anthropic.baseUrl"],
            super::HEADROOM_ANTHROPIC_BASE_URL
        );
        assert!(configured
            .get(format!("// >>> {} >>>", super::WINDSURF_MARKER_PREFIX))
            .is_some());

        super::disable_client_setup("windsurf").expect("disable windsurf setup");
        let cleaned: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&settings_json).expect("read cleaned"))
                .expect("parse cleaned settings");
        assert_eq!(cleaned["workbench.colorTheme"], "Quiet Light");
        assert_eq!(cleaned["assistant"]["defaultModel"], "claude-3-5-sonnet");
        assert!(cleaned.get("anthropic.baseUrl").is_none());
        assert!(cleaned
            .get(format!("// >>> {} >>>", super::WINDSURF_MARKER_PREFIX))
            .is_none());
        assert!(cleaned
            .get(format!("// <<< {} <<<", super::WINDSURF_MARKER_PREFIX))
            .is_none());

        let verification =
            super::verify_client_setup("windsurf").expect("verify cleaned windsurf setup");
        assert!(!verification.verified);
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: apply,verify,off
    fn zed_setup_writes_verifies_and_off_cleanup_removes_native_routing_only() {
        let home = TestHome::new();
        let zed_dir = home.path().join(".config").join("zed");
        fs::create_dir_all(&zed_dir).unwrap();
        let settings_json = zed_dir.join("settings.json");
        fs::write(
            &settings_json,
            r#"{"theme":"One Dark","assistant":{"default_model":"claude-3-5-sonnet"}}"#,
        )
        .unwrap();

        let result = super::apply_client_setup("zed_ai").expect("apply zed setup");
        assert!(result.applied);
        assert!(!result.already_configured);
        assert!(result
            .changed_files
            .contains(&settings_json.display().to_string()));
        assert!(result.backup_files.len() == 1);
        assert!(result.verification.verified);

        let configured: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&settings_json).expect("read settings"))
                .expect("parse settings");
        assert_eq!(configured["theme"], "One Dark");
        assert_eq!(
            configured["assistant"]["default_model"],
            "claude-3-5-sonnet"
        );
        assert_eq!(
            configured["anthropic.baseUrl"],
            super::HEADROOM_ANTHROPIC_BASE_URL
        );
        assert!(configured
            .get(format!("// >>> {} >>>", super::ZED_MARKER_PREFIX))
            .is_some());

        super::disable_client_setup("zed_ai").expect("disable zed setup");
        let cleaned: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&settings_json).expect("read cleaned"))
                .expect("parse cleaned settings");
        assert_eq!(cleaned["theme"], "One Dark");
        assert_eq!(cleaned["assistant"]["default_model"], "claude-3-5-sonnet");
        assert!(cleaned.get("anthropic.baseUrl").is_none());
        assert!(cleaned
            .get(format!("// >>> {} >>>", super::ZED_MARKER_PREFIX))
            .is_none());
        assert!(cleaned
            .get(format!("// <<< {} <<<", super::ZED_MARKER_PREFIX))
            .is_none());

        let verification = super::verify_client_setup("zed_ai").expect("verify cleaned zed setup");
        assert!(!verification.verified);
    }

    #[test]
    #[serial_test::serial]
    fn grok_sidecar_setup_is_managed_and_never_reads_or_writes_xai_credentials() {
        let home = TestHome::new();
        let connectors = [("grok_cli", "Grok / xAI CLI")];

        for (client_id, name) in connectors {
            let sidecar = planned_sidecar_routing_path(client_id).expect("sidecar path available");
            fs::create_dir_all(sidecar.parent().unwrap()).expect("create sidecar parent");
            fs::write(&sidecar, format!("# {client_id} user note\nkeep this\n"))
                .expect("seed sidecar");

            let result = super::apply_client_setup(client_id).expect("apply managed sidecar setup");
            assert!(result.verification.verified);
            let content = fs::read_to_string(&sidecar).expect("read sidecar");
            assert!(content.contains(&format!("# {client_id} user note\nkeep this")));
            assert!(content.contains("routing-intent sidecar"));

            let detected_clients = vec![ClientStatus {
                id: client_id.into(),
                name: name.into(),
                installed: true,
                configured: false,
                health: ClientHealth::Attention,
                notes: vec![format!("{name} config surface: {}", sidecar.display())],
            }];
            let listed = list_client_connectors(&detected_clients).expect("list connectors");
            let connector = listed
                .iter()
                .find(|connector| connector.client_id == client_id)
                .unwrap_or_else(|| panic!("{client_id} connector listed"));
            assert_eq!(
                connector.support_status,
                ClientConnectorSupportStatus::Managed,
                "{client_id} sidecar lifecycle is safely managed"
            );
            assert!(connector.enabled, "{client_id} should be enabled");
            assert!(connector.verified, "{client_id} should be verified");
            assert!(connector.config_creation_steps.is_empty());
            let preview = connector
                .config_dry_run_preview
                .as_ref()
                .expect("managed connector dry-run preview");
            assert!(preview.apply_blocked_reason.contains("read-only"));
            assert!(preview.writes.is_empty());
            assert!(connector.automation_path.is_empty());
            super::disable_client_setup(client_id).expect("off cleanup");
        }

        assert!(
            !home.path().join(".aws").join("credentials").exists(),
            "Amazon Q sidecar setup must not create AWS credentials"
        );
    }

    fn read_settings_json(path: &Path) -> serde_json::Value {
        let raw = fs::read_to_string(path).expect("read settings.json");
        serde_json::from_str(&raw).expect("parse settings.json")
    }

    fn seed_caveman_clients_configured() {
        super::write_setup_state(&ClientSetupState {
            configured_clients: BTreeMap::from([
                ("claude_code".into(), "2026-06-27T00:00:00Z".into()),
                ("codex_cli".into(), "2026-06-27T00:00:01Z".into()),
            ]),
            remembered_clients: BTreeMap::new(),
            managed_shell_files: BTreeMap::new(),
            remembered_shell_files: BTreeMap::new(),
            rtk_disabled: false,
            switchboard_mode: None,
            savings_mode: None,
        })
        .expect("write setup state");
    }

    #[test]
    #[serial_test::serial]
    fn caveman_block_round_trips_for_configured_clients() {
        let home = TestHome::new();
        seed_caveman_clients_configured();

        super::enable_caveman_integration("scoped").expect("enable caveman");

        let claude =
            fs::read_to_string(home.path().join(".claude").join("CLAUDE.md")).expect("read claude");
        let codex =
            fs::read_to_string(home.path().join(".codex").join("AGENTS.md")).expect("read codex");
        assert!(claude.contains("ai-switchboard:caveman"));
        assert!(claude.contains("Switchboard Caveman, scoped"));
        assert!(codex.contains("ai-switchboard:caveman"));
        assert!(codex.contains("Switchboard Caveman, scoped"));
    }

    #[test]
    #[serial_test::serial]
    fn caveman_level_switch_rewrites_managed_body() {
        let home = TestHome::new();
        seed_caveman_clients_configured();

        super::enable_caveman_integration("scoped").expect("enable scoped");
        super::enable_caveman_integration("aggressive").expect("enable aggressive");

        let agents =
            fs::read_to_string(home.path().join(".codex").join("AGENTS.md")).expect("read codex");
        assert!(agents.contains("Switchboard Caveman, aggressive"));
        assert!(!agents.contains("Switchboard Caveman, scoped"));
    }

    #[test]
    #[serial_test::serial]
    fn caveman_integration_match_detects_stale_level_body() {
        let _home = TestHome::new();
        seed_caveman_clients_configured();

        super::enable_caveman_integration("scoped").expect("enable scoped");

        assert!(
            super::caveman_integration_matches_level("scoped").expect("check scoped"),
            "scoped body should match"
        );
        assert!(
            !super::caveman_integration_matches_level("compact_chinese")
                .expect("check compact chinese"),
            "compact Chinese should not match stale scoped body"
        );
    }

    #[test]
    #[serial_test::serial]
    fn caveman_compact_chinese_profile_is_internal_only() {
        let home = TestHome::new();
        seed_caveman_clients_configured();

        super::enable_caveman_integration("compact_chinese").expect("enable compact chinese");

        let agents =
            fs::read_to_string(home.path().join(".codex").join("AGENTS.md")).expect("read codex");
        assert!(agents.contains("Switchboard Caveman, compact Chinese experimental"));
        assert!(agents.contains("private internal planning notes"));
        assert!(agents.contains("user-visible replies"));
        assert!(agents.contains("legal, safety"));
        assert!(agents.contains("debugging"));
        assert!(agents.contains("release-readiness"));
        assert!(agents.contains("Never translate code"));
    }

    #[test]
    #[serial_test::serial]
    fn caveman_disable_and_full_cleanup_remove_managed_blocks() {
        let home = TestHome::new();
        seed_caveman_clients_configured();

        super::enable_caveman_integration("scoped").expect("enable caveman");
        assert!(super::disable_caveman_integration().expect("disable caveman"));
        let claude_path = home.path().join(".claude").join("CLAUDE.md");
        assert!(!fs::read_to_string(&claude_path)
            .expect("read claude")
            .contains("ai-switchboard:caveman"));

        super::enable_caveman_integration("scoped").expect("enable again");
        super::perform_full_cleanup();
        let codex_path = home.path().join(".codex").join("AGENTS.md");
        assert!(!fs::read_to_string(codex_path)
            .expect("read codex")
            .contains("ai-switchboard:caveman"));
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: preview,apply,verify
    fn apply_then_verify_claude_code_writes_expected_files() {
        let home = TestHome::new();
        // Seed an empty zshrc/zshenv so the shell-block writers have files to
        // edit and don't depend on the dev's real shell config layout.
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        fs::create_dir_all(home.path().join(".claude")).unwrap();
        fs::write(
            home.path().join(".claude").join("settings.json"),
            r#"{"hooks": {}}"#,
        )
        .unwrap();
        seed_installed_rtk();

        let result = super::apply_client_setup("claude_code").expect("apply_client_setup succeeds");
        assert!(result.applied);
        assert_eq!(result.client_id, "claude_code");

        // Hook script and settings.json hook entry must be present.
        let hook_path = home
            .path()
            .join(".claude")
            .join("hooks")
            .join("headroom-rtk-rewrite.sh");
        assert!(hook_path.exists(), "hook script written to disk");
        let hook_contents = fs::read_to_string(&hook_path).unwrap();
        assert!(
            hook_contents.starts_with("#!/usr/bin/env bash"),
            "hook has expected shebang"
        );

        let settings = read_settings_json(&home.path().join(".claude").join("settings.json"));
        assert_eq!(
            settings["env"]["ANTHROPIC_BASE_URL"].as_str(),
            Some("http://127.0.0.1:6767"),
            "claude settings.json points env at headroom proxy"
        );
        let pre_tool_use = &settings["hooks"]["PreToolUse"];
        assert!(
            pre_tool_use.is_array() && !pre_tool_use.as_array().unwrap().is_empty(),
            "PreToolUse hook entry exists, got: {settings}"
        );

        // Shell block in zshenv (or whichever profile the writer chose) should
        // export ANTHROPIC_BASE_URL pointing at the loopback proxy.
        let zshrc = fs::read_to_string(home.path().join(".zshrc")).unwrap();
        let zshenv = fs::read_to_string(home.path().join(".zshenv")).unwrap();
        let combined = format!("{zshrc}\n{zshenv}");
        assert!(
            combined.contains("ANTHROPIC_BASE_URL=http://127.0.0.1:6767"),
            "ANTHROPIC_BASE_URL exported from a managed shell block, got:\n{combined}"
        );

        // verify_client_setup should report all the configured checks.
        // Proxy reachability is reported via `proxy_reachable` only, so a
        // missing proxy in the test environment no longer flips `verified`.
        let verification =
            super::verify_client_setup("claude_code").expect("verify_client_setup succeeds");
        assert_eq!(verification.client_id, "claude_code");
        assert!(
            verification
                .checks
                .iter()
                .any(|c| c.contains("ANTHROPIC_BASE_URL")),
            "verification reports the env check, got: {:?}",
            verification.checks
        );
        assert!(
            verification
                .checks
                .iter()
                .any(|c| c.contains("RTK Claude hook")),
            "verification reports the hook check, got: {:?}",
            verification.checks
        );
    }

    #[test]
    #[serial_test::serial]
    fn verify_claude_code_passes_when_rtk_deliberately_disabled() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        fs::create_dir_all(home.path().join(".claude")).unwrap();
        fs::write(
            home.path().join(".claude").join("settings.json"),
            r#"{"hooks": {}}"#,
        )
        .unwrap();

        super::apply_client_setup("claude_code").expect("apply_client_setup succeeds");

        // User turns RTK off: this strips the RTK PATH block + hook but leaves
        // ANTHROPIC_BASE_URL routing intact, and persists the opt-out.
        super::set_rtk_enabled(false, home.path(), home.path()).expect("disable RTK");

        let hook_path = home
            .path()
            .join(".claude")
            .join("hooks")
            .join("headroom-rtk-rewrite.sh");
        assert!(!hook_path.exists(), "RTK hook removed when RTK disabled");

        // Routing config is still present, so Claude Code must verify green
        // even though the RTK pieces are gone.
        let verification =
            super::verify_client_setup("claude_code").expect("verify_client_setup succeeds");
        assert!(
            verification.verified,
            "claude_code verifies on routing alone when RTK is disabled, failures: {:?}",
            verification.failures
        );
        assert!(
            verification.failures.iter().all(|f| !f.contains("RTK")),
            "no RTK failures reported when RTK is disabled, got: {:?}",
            verification.failures
        );
    }

    #[test]
    #[serial_test::serial]
    fn verify_claude_code_passes_when_rtk_not_installed() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        fs::create_dir_all(home.path().join(".claude")).unwrap();
        fs::write(
            home.path().join(".claude").join("settings.json"),
            r#"{"hooks": {}}"#,
        )
        .unwrap();

        // Clean install with RTK auto-install removed: routing is configured but
        // the managed RTK binary was never dropped on disk and the user never
        // toggled RTK off (rtk_disabled stays false). Claude Code must still
        // verify green on routing alone.
        super::apply_client_setup("claude_code").expect("apply_client_setup succeeds");

        assert!(
            !super::default_headroom_rtk_path().exists(),
            "RTK binary must be absent for this test"
        );
        let state = super::load_setup_state();
        assert!(
            !state.rtk_disabled,
            "rtk_disabled stays false when untoggled"
        );

        let verification =
            super::verify_client_setup("claude_code").expect("verify_client_setup succeeds");
        assert!(
            verification.verified,
            "claude_code verifies on routing alone when RTK isn't installed, failures: {:?}",
            verification.failures
        );
        assert!(
            verification.failures.iter().all(|f| !f.contains("RTK")),
            "no RTK failures reported when RTK isn't installed, got: {:?}",
            verification.failures
        );
    }

    #[test]
    #[serial_test::serial]
    fn ensure_rtk_integrations_writes_codex_nudge_and_disable_removes_it() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        fs::create_dir_all(home.path().join(".claude")).unwrap();
        fs::write(home.path().join(".claude").join("settings.json"), "{}").unwrap();

        // Mark Codex as a configured client so the AGENTS.md nudge path runs.
        let mut state = super::load_setup_state();
        state
            .configured_clients
            .insert("codex_cli".into(), "now".into());
        super::write_setup_state(&state).unwrap();

        // Fake managed rtk + python binaries so the binary-present guard passes.
        let bin_dir = home.path().join("managed-bin");
        fs::create_dir_all(&bin_dir).unwrap();
        let rtk = bin_dir.join("rtk");
        fs::write(&rtk, "#!/bin/sh\n").unwrap();
        let python = bin_dir.join("python3");
        fs::write(&python, "#!/bin/sh\n").unwrap();

        super::ensure_rtk_integrations(&rtk, &python).expect("ensure_rtk_integrations");

        let agents = home.path().join(".codex").join("AGENTS.md");
        let body = fs::read_to_string(&agents).expect("AGENTS.md written");
        assert!(
            body.contains("Headroom RTK"),
            "nudge heading present: {body}"
        );
        assert!(
            body.contains(&rtk.display().to_string()),
            "nudge references the managed rtk path: {body}"
        );

        // Disabling RTK must remove the managed block.
        super::set_rtk_enabled(false, &rtk, &python).expect("disable rtk");
        let after = fs::read_to_string(&agents).unwrap_or_default();
        assert!(
            !after.contains("Headroom RTK"),
            "nudge removed on disable: {after}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn apply_claude_code_is_byte_idempotent() {
        // Regression: a second apply used to add blank-line padding between
        // managed blocks, so byte-exact idempotency now holds and is
        // asserted here.
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        seed_installed_rtk();

        super::apply_client_setup("claude_code").expect("first apply");
        let zshrc_after_first = fs::read_to_string(home.path().join(".zshrc")).unwrap();
        let zshenv_after_first = fs::read_to_string(home.path().join(".zshenv")).unwrap();
        let settings_after_first =
            fs::read_to_string(home.path().join(".claude").join("settings.json")).unwrap();
        let hook_after_first = fs::read_to_string(
            home.path()
                .join(".claude")
                .join("hooks")
                .join("headroom-rtk-rewrite.sh"),
        )
        .unwrap();

        super::apply_client_setup("claude_code").expect("second apply");
        let zshrc_after_second = fs::read_to_string(home.path().join(".zshrc")).unwrap();
        let zshenv_after_second = fs::read_to_string(home.path().join(".zshenv")).unwrap();
        let settings_after_second =
            fs::read_to_string(home.path().join(".claude").join("settings.json")).unwrap();
        let hook_after_second = fs::read_to_string(
            home.path()
                .join(".claude")
                .join("hooks")
                .join("headroom-rtk-rewrite.sh"),
        )
        .unwrap();

        assert_eq!(zshrc_after_first, zshrc_after_second, "zshrc byte-stable");
        assert_eq!(
            zshenv_after_first, zshenv_after_second,
            "zshenv byte-stable"
        );
        assert_eq!(
            settings_after_first, settings_after_second,
            "settings.json byte-stable"
        );
        assert_eq!(
            hook_after_first, hook_after_second,
            "hook script byte-stable"
        );

        // Sanity: each managed block still appears exactly once.
        let combined = format!("{zshrc_after_second}\n{zshenv_after_second}");
        assert_eq!(
            combined.matches("# >>> ai-switchboard:claude_code >>>").count(),
            1
        );
        assert_eq!(
            combined.matches("# >>> ai-switchboard:managed_rtk >>>").count(),
            1
        );
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: rollback,off
    fn disable_then_clear_claude_code_removes_traces() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        seed_installed_rtk();

        super::apply_client_setup("claude_code").expect("apply");
        let hook_path = home
            .path()
            .join(".claude")
            .join("hooks")
            .join("headroom-rtk-rewrite.sh");
        assert!(hook_path.exists(), "hook present after apply");

        super::disable_client_setup("claude_code").expect("disable");

        // Hook script removed.
        assert!(!hook_path.exists(), "hook removed after disable");

        // Shell blocks removed.
        let zshrc = fs::read_to_string(home.path().join(".zshrc")).unwrap();
        let zshenv = fs::read_to_string(home.path().join(".zshenv")).unwrap();
        let combined = format!("{zshrc}\n{zshenv}");
        assert!(
            !combined.contains("ANTHROPIC_BASE_URL=http://127.0.0.1:6767"),
            "ANTHROPIC_BASE_URL export removed, got:\n{combined}"
        );

        // settings.json no longer points env at the proxy and no longer carries
        // the Headroom hook entry.
        let settings = read_settings_json(&home.path().join(".claude").join("settings.json"));
        assert!(
            settings["env"]["ANTHROPIC_BASE_URL"].is_null(),
            "ANTHROPIC_BASE_URL stripped from settings.json env, got: {settings}"
        );
        let still_has_headroom_hook =
            claude_hook_present_in_value(&settings, "headroom-rtk-rewrite.sh");
        assert!(
            !still_has_headroom_hook,
            "Headroom hook entry stripped from settings.json, got: {settings}"
        );

        // clear_client_setups runs disable across all clients without error,
        // and the setup state file is left without a `claude_code` entry.
        super::clear_client_setups().expect("clear");
        let post = super::load_setup_state();
        assert!(
            post.configured_clients.get("claude_code").is_none(),
            "claude_code dropped from configured_clients, got: {:?}",
            post.configured_clients
        );
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: preview,apply,verify,off
    fn apply_then_verify_then_disable_codex_round_trip() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();

        let result = super::apply_client_setup("codex").expect("apply_client_setup succeeds");
        assert!(result.applied);
        assert_eq!(result.client_id, "codex");

        // Managed provider block lands in ~/.codex/config.toml.
        let config_toml = home.path().join(".codex").join("config.toml");
        let toml = fs::read_to_string(&config_toml).expect("codex config.toml written");
        assert!(
            toml.contains("# >>> ai-switchboard:codex_cli >>>"),
            "managed marker present, got:\n{toml}"
        );
        assert!(
            toml.contains("model_provider = \"headroom\""),
            "model_provider set, got:\n{toml}"
        );
        assert!(
            toml.contains("base_url = \"http://127.0.0.1:6767/v1\""),
            "provider base_url points at proxy, got:\n{toml}"
        );
        // No ~/.codex/auth.json in this test ⇒ not ChatGPT-OAuth ⇒ the flag is
        // omitted (it would force an OpenAI OAuth login for API-key users, #406).
        assert!(
            !toml.contains("requires_openai_auth"),
            "requires_openai_auth must NOT be written without ChatGPT auth, got:\n{toml}"
        );

        // OPENAI_BASE_URL exported from a managed shell block.
        let zshrc = fs::read_to_string(home.path().join(".zshrc")).unwrap();
        let zshenv = fs::read_to_string(home.path().join(".zshenv")).unwrap();
        let combined = format!("{zshrc}\n{zshenv}");
        assert!(
            combined.contains("OPENAI_BASE_URL=http://127.0.0.1:6767/v1"),
            "OPENAI_BASE_URL exported from a managed shell block, got:\n{combined}"
        );

        // verify_client_setup reports the configured checks and passes.
        let verification =
            super::verify_client_setup("codex").expect("verify_client_setup succeeds");
        assert_eq!(verification.client_id, "codex");
        assert!(
            verification.failures.is_empty(),
            "no verification failures, got: {:?}",
            verification.failures
        );
        assert!(
            verification
                .checks
                .iter()
                .any(|c| c.contains("config.toml")),
            "verification reports the toml check, got: {:?}",
            verification.checks
        );

        // Disable strips both the toml block and the shell export.
        super::disable_client_setup("codex").expect("disable_client_setup succeeds");
        let toml_after = fs::read_to_string(&config_toml).unwrap_or_default();
        assert!(
            !toml_after.contains("# >>> ai-switchboard:codex_cli >>>"),
            "managed block removed on disable, got:\n{toml_after}"
        );
        let combined_after = format!(
            "{}\n{}",
            fs::read_to_string(home.path().join(".zshrc")).unwrap(),
            fs::read_to_string(home.path().join(".zshenv")).unwrap(),
        );
        assert!(
            !combined_after.contains("OPENAI_BASE_URL=http://127.0.0.1:6767/v1"),
            "shell export removed on disable, got:\n{combined_after}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn verify_codex_accepts_config_provider_without_shell_export() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        super::apply_client_setup("codex").expect("apply_client_setup succeeds");
        for shell_profile in [
            ".zshrc",
            ".zshenv",
            ".zprofile",
            ".bashrc",
            ".bash_profile",
            ".profile",
        ] {
            fs::write(home.path().join(shell_profile), "# user shell profile\n").unwrap();
        }

        let verification =
            super::verify_client_setup("codex").expect("verify_client_setup succeeds");
        assert!(
            verification.failures.is_empty(),
            "config-provider-only routing should pass, got: {:?}",
            verification.failures
        );
        assert!(
            verification
                .checks
                .iter()
                .any(|check| check.contains("config.toml provider routing is active")),
            "verification should report config-only routing evidence, got: {:?}",
            verification.checks
        );
    }

    #[test]
    #[serial_test::serial]
    fn apply_codex_is_byte_idempotent() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();

        super::apply_client_setup("codex").expect("first apply");
        let config_toml = home.path().join(".codex").join("config.toml");
        let toml_first = fs::read_to_string(&config_toml).unwrap();
        let zshenv_first = fs::read_to_string(home.path().join(".zshenv")).unwrap();

        super::apply_client_setup("codex").expect("second apply");
        let toml_second = fs::read_to_string(&config_toml).unwrap();
        let zshenv_second = fs::read_to_string(home.path().join(".zshenv")).unwrap();

        assert_eq!(toml_first, toml_second, "config.toml byte-stable");
        assert_eq!(zshenv_first, zshenv_second, "zshenv byte-stable");
        assert_eq!(
            toml_second.matches("# >>> ai-switchboard:codex_cli >>>").count(),
            1,
            "managed block appears exactly once"
        );
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: backup,rollback
    fn managed_rollback_preview_and_execute_restores_codex_backup() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        let config_toml = codex_dir.join("config.toml");
        let original = "model = \"gpt-5\"\n[profiles.default]\napproval_policy = \"never\"\n";
        fs::write(&config_toml, original).unwrap();

        super::apply_client_setup("codex").expect("apply codex");
        let preview = super::preview_managed_rollback("codex-routing").expect("preview rollback");

        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.marker_present);
        assert!(preview.backup_exists);
        assert_eq!(
            preview.confirmation_phrase,
            "Restore ai-switchboard:codex_cli for Codex routing"
        );
        let backup_path = preview.backup_path.expect("backup path");

        let result = super::execute_managed_rollback(
            "codex-routing",
            &backup_path,
            "Restore ai-switchboard:codex_cli for Codex routing",
        )
        .expect("execute rollback");

        assert_eq!(result.record_id, "codex-routing");
        assert_eq!(result.restored_from, backup_path);
        assert!(
            result.safety_backup_path.is_some(),
            "fresh safety backup is created before restore"
        );
        assert_eq!(fs::read_to_string(&config_toml).unwrap(), original);
    }

    #[test]
    #[serial_test::serial]
    fn managed_rollback_rejects_backup_outside_codex_config_directory() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(codex_dir.join("config.toml"), "model = \"gpt-5\"\n").unwrap();
        super::apply_client_setup("codex").expect("apply codex");
        let wrong_backup = home.path().join("config.toml.headroom-backup-wrong");
        fs::write(&wrong_backup, "model = \"gpt-4\"\n").unwrap();

        let err = super::execute_managed_rollback(
            "codex-routing",
            wrong_backup.to_str().unwrap(),
            "Restore ai-switchboard:codex_cli for Codex routing",
        )
        .expect_err("wrong backup must be rejected");

        assert!(
            err.to_string().contains("must live next to"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn managed_rollback_rejects_missing_codex_marker() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        fs::write(home.path().join(".zshenv"), "# user zshenv\n").unwrap();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        let config_toml = codex_dir.join("config.toml");
        fs::write(&config_toml, "model = \"gpt-5\"\n").unwrap();
        super::apply_client_setup("codex").expect("apply codex");
        let preview = super::preview_managed_rollback("codex-routing").expect("preview");
        let backup_path = preview.backup_path.expect("backup");
        fs::write(&config_toml, "model = \"gpt-5\"\n").unwrap();

        let err = super::execute_managed_rollback(
            "codex-routing",
            &backup_path,
            "Restore ai-switchboard:codex_cli for Codex routing",
        )
        .expect_err("missing marker must be rejected");

        assert!(
            err.to_string().contains("marker is missing"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: rollback
    fn managed_rollback_preview_and_execute_restores_opencode_backup() {
        let home = TestHome::new();
        let opencode_dir = home.path().join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        let config_json = opencode_dir.join("opencode.json");
        let original = serde_json::json!({
            "provider": {
                "openai": {
                    "npm": "@ai-sdk/openai",
                    "name": "OpenAI",
                    "options": {
                        "baseURL": "https://api.openai.com/v1"
                    }
                }
            },
            "theme": "system"
        });
        fs::write(
            &config_json,
            serde_json::to_vec_pretty(&original).expect("serialize original opencode"),
        )
        .unwrap();

        super::apply_client_setup("opencode").expect("apply opencode");
        let preview =
            super::preview_managed_rollback("opencode-routing").expect("preview rollback");

        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.marker_present);
        assert!(preview.backup_exists);
        assert!(preview
            .evidence
            .join(" ")
            .contains("Relaunch-survival evidence"));
        assert_eq!(
            preview.confirmation_phrase,
            "Restore ai-switchboard:opencode for OpenCode routing"
        );
        let backup_path = preview.backup_path.expect("backup path");

        let result = super::execute_managed_rollback(
            "opencode-routing",
            &backup_path,
            "Restore ai-switchboard:opencode for OpenCode routing",
        )
        .expect("execute rollback");

        assert_eq!(result.record_id, "opencode-routing");
        assert_eq!(result.restored_from, backup_path);
        assert!(
            result.safety_backup_path.is_some(),
            "fresh safety backup is created before restore"
        );
        assert!(result
            .verification
            .join(" ")
            .contains("Relaunch-survival evidence"));
        let restored: serde_json::Value =
            serde_json::from_slice(&fs::read(&config_json).unwrap()).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: preview,backup
    fn managed_config_apply_preview_and_execute_promotes_opencode_safely() {
        let home = TestHome::new();
        let opencode_dir = home.path().join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        let config_json = opencode_dir.join("opencode.json");
        let original = serde_json::json!({
            "provider": {
                "openai": {
                    "name": "OpenAI",
                    "options": {
                        "baseURL": "https://api.openai.com/v1"
                    }
                }
            },
            "theme": "system"
        });
        fs::write(
            &config_json,
            serde_json::to_vec_pretty(&original).expect("serialize original opencode"),
        )
        .unwrap();

        let preview =
            super::preview_managed_config_apply("opencode-routing").expect("preview apply");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.confirmation_phrase.starts_with(&format!(
            "Apply ai-switchboard:opencode to {} after reviewing ",
            config_json.display()
        )));
        assert!(preview.current_state.contains("OpenAI"));
        assert!(preview.proposed_state.contains("AI Switchboard"));
        assert!(preview.proposed_state.contains("\"theme\": \"system\""));
        assert!(preview.rollback_preview.contains("Rollback Center"));

        let result =
            super::execute_managed_config_apply("opencode-routing", &preview.confirmation_phrase)
                .expect("execute apply");
        assert!(result.changed);
        assert!(result.backup_path.is_some());
        assert!(result
            .verification
            .join(" ")
            .contains("provider.headroom matches"));
        assert!(super::opencode_provider_config_matches().expect("verify opencode"));

        let applied: serde_json::Value =
            serde_json::from_slice(&fs::read(&config_json).unwrap()).unwrap();
        assert_eq!(applied["theme"], "system");
        assert_eq!(
            applied["provider"]["openai"],
            original["provider"]["openai"]
        );
        assert_eq!(
            applied["provider"]["headroom"]["options"]["baseURL"],
            super::HEADROOM_OPENAI_BASE_URL
        );

        let rollback = super::execute_managed_rollback(
            "opencode-routing",
            result.backup_path.as_deref().expect("backup"),
            "Restore ai-switchboard:opencode for OpenCode routing",
        )
        .expect("rollback applied config");
        assert_eq!(rollback.record_id, "opencode-routing");
        let restored: serde_json::Value =
            serde_json::from_slice(&fs::read(&config_json).unwrap()).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    #[serial_test::serial]
    fn cursor_sidecar_apply_is_profile_aware_and_preserves_user_owned_files() {
        let home = TestHome::new();
        let cursor_root = home
            .path()
            .join("Library")
            .join("Application Support")
            .join("Cursor");
        let profile_settings = cursor_root
            .join("User")
            .join("profiles")
            .join("work")
            .join("settings.json");
        fs::create_dir_all(profile_settings.parent().unwrap()).unwrap();
        let profile_contents = r#"{"cursor.model":"user-selected","token":"must-not-read"}"#;
        fs::write(&profile_settings, profile_contents).unwrap();

        let status = super::detect_cursor_client();
        assert!(status.installed);
        assert!(status
            .notes
            .iter()
            .any(|note| note.contains("User/profiles/work/settings.json")));
        assert_eq!(
            fs::read_to_string(&profile_settings).unwrap(),
            profile_contents
        );

        let sidecar = cursor_root.join(super::SWITCHBOARD_ROUTING_FILE);
        fs::write(&sidecar, "# user-owned cursor note\nkeep this\n").unwrap();
        let preview = super::preview_managed_config_apply(super::CURSOR_SIDECAR_APPLY_RECORD_ID)
            .expect("preview Cursor sidecar apply");
        assert_eq!(preview.record_id, super::CURSOR_SIDECAR_APPLY_RECORD_ID);
        assert_eq!(preview.target_path, sidecar.display().to_string());
        assert!(preview.current_state.contains("user-owned cursor note"));
        assert!(preview.proposed_state.contains("ai-switchboard:cursor"));
        assert!(!preview.proposed_state.contains("user-selected"));
        assert!(preview.confirmation_phrase.contains("after reviewing"));
        assert_eq!(
            fs::read_to_string(&sidecar).unwrap(),
            "# user-owned cursor note\nkeep this\n"
        );

        let result = super::execute_managed_config_apply(
            super::CURSOR_SIDECAR_APPLY_RECORD_ID,
            &preview.confirmation_phrase,
        )
        .expect("apply Cursor sidecar");
        assert!(result.changed);
        assert!(result.backup_path.is_some());
        assert!(super::planned_switchboard_sidecar_matches("cursor").unwrap());
        assert_eq!(
            fs::read_to_string(&profile_settings).unwrap(),
            profile_contents
        );

        let rollback = super::execute_managed_rollback(
            "cursor-routing",
            "",
            "Restore ai-switchboard:cursor for Cursor routing",
        )
        .expect("remove only Cursor sidecar block");
        assert!(rollback.safety_backup_path.is_some());
        assert_eq!(
            fs::read_to_string(&sidecar).unwrap(),
            "# user-owned cursor note\nkeep this\n"
        );
        assert_eq!(
            fs::read_to_string(&profile_settings).unwrap(),
            profile_contents
        );
    }

    #[test]
    #[serial_test::serial]
    fn cursor_sidecar_apply_rejects_stale_or_wrong_confirmation() {
        let home = TestHome::new();
        let sidecar = home
            .path()
            .join("Library")
            .join("Application Support")
            .join("Cursor")
            .join(super::SWITCHBOARD_ROUTING_FILE);
        fs::create_dir_all(sidecar.parent().unwrap()).unwrap();
        fs::write(&sidecar, "first\n").unwrap();
        let preview = super::preview_managed_config_apply(super::CURSOR_SIDECAR_APPLY_RECORD_ID)
            .expect("preview Cursor sidecar");
        fs::write(&sidecar, "changed outside Switchboard\n").unwrap();

        let err = super::execute_managed_config_apply(
            super::CURSOR_SIDECAR_APPLY_RECORD_ID,
            &preview.confirmation_phrase,
        )
        .expect_err("stale confirmation must be rejected");
        assert!(err.to_string().contains("confirmation phrase"));
        assert_eq!(
            fs::read_to_string(&sidecar).unwrap(),
            "changed outside Switchboard\n"
        );
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: preview,backup,rollback
    fn managed_config_apply_preview_and_execute_promotes_zed_rollback_safely() {
        let home = TestHome::new();
        let zed_dir = home.path().join(".config").join("zed");
        fs::create_dir_all(&zed_dir).unwrap();
        let settings_json = zed_dir.join("settings.json");
        let original = serde_json::json!({
            "theme": "One Dark",
            "assistant": { "default_model": "claude-3-5-sonnet" }
        });
        fs::write(
            &settings_json,
            serde_json::to_vec_pretty(&original).unwrap(),
        )
        .unwrap();

        let preview =
            super::preview_managed_config_apply("zed-ai-routing").expect("preview zed apply");
        assert_eq!(preview.record_id, "zed-ai-routing");
        assert!(preview.target_path.ends_with(".config/zed/settings.json"));
        assert!(preview.current_state.contains("One Dark"));
        assert!(preview.proposed_state.contains("anthropic.baseUrl"));
        assert!(preview.rollback_preview.contains("Rollback Center"));

        let result =
            super::execute_managed_config_apply("zed-ai-routing", &preview.confirmation_phrase)
                .expect("execute zed apply");
        assert!(result.changed);
        assert!(result.backup_path.is_some());
        assert!(super::zed_provider_config_matches().expect("verify zed"));

        let applied: serde_json::Value =
            serde_json::from_slice(&fs::read(&settings_json).unwrap()).unwrap();
        assert_eq!(applied["theme"], "One Dark");
        assert_eq!(applied["assistant"]["default_model"], "claude-3-5-sonnet");
        assert_eq!(
            applied["anthropic.baseUrl"],
            super::HEADROOM_ANTHROPIC_BASE_URL
        );

        let rollback_preview =
            super::preview_managed_rollback("zed-ai-routing").expect("preview zed rollback");
        assert_eq!(rollback_preview.record_id, "zed-ai-routing");
        assert_eq!(rollback_preview.marker, "ai-switchboard:zed");
        assert!(rollback_preview.backup_path.is_some());
        assert!(rollback_preview
            .proposed_action
            .contains("Restore the Zed settings"));

        let rollback = super::execute_managed_rollback(
            "zed-ai-routing",
            result.backup_path.as_deref().expect("backup"),
            "Restore ai-switchboard:zed for Zed routing",
        )
        .expect("rollback applied zed config");
        assert_eq!(rollback.record_id, "zed-ai-routing");
        let restored: serde_json::Value =
            serde_json::from_slice(&fs::read(&settings_json).unwrap()).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: preview,backup,rollback
    fn managed_config_apply_preview_and_execute_promotes_windsurf_rollback_safely() {
        let home = TestHome::new();
        let windsurf_dir = home
            .path()
            .join("Library")
            .join("Application Support")
            .join("Windsurf")
            .join("User");
        fs::create_dir_all(&windsurf_dir).unwrap();
        let settings_json = windsurf_dir.join("settings.json");
        let original = serde_json::json!({
            "workbench.colorTheme": "Quiet Light",
            "assistant": { "defaultModel": "claude-3-5-sonnet" }
        });
        fs::write(
            &settings_json,
            serde_json::to_vec_pretty(&original).unwrap(),
        )
        .unwrap();

        let preview = super::preview_managed_config_apply("windsurf-routing")
            .expect("preview windsurf apply");
        assert_eq!(preview.record_id, "windsurf-routing");
        assert!(preview
            .target_path
            .ends_with("Application Support/Windsurf/User/settings.json"));
        assert!(preview.current_state.contains("Quiet Light"));
        assert!(preview.proposed_state.contains("anthropic.baseUrl"));
        assert!(preview.rollback_preview.contains("Rollback Center"));

        let result =
            super::execute_managed_config_apply("windsurf-routing", &preview.confirmation_phrase)
                .expect("execute windsurf apply");
        assert!(result.changed);
        assert!(result.backup_path.is_some());
        assert!(super::windsurf_provider_config_matches().expect("verify windsurf"));

        let applied: serde_json::Value =
            serde_json::from_slice(&fs::read(&settings_json).unwrap()).unwrap();
        assert_eq!(applied["workbench.colorTheme"], "Quiet Light");
        assert_eq!(applied["assistant"]["defaultModel"], "claude-3-5-sonnet");
        assert_eq!(
            applied["anthropic.baseUrl"],
            super::HEADROOM_ANTHROPIC_BASE_URL
        );

        let rollback_preview =
            super::preview_managed_rollback("windsurf-routing").expect("preview windsurf rollback");
        assert_eq!(rollback_preview.record_id, "windsurf-routing");
        assert_eq!(rollback_preview.marker, "ai-switchboard:windsurf");
        assert!(rollback_preview.backup_path.is_some());
        assert!(rollback_preview
            .proposed_action
            .contains("Restore the Windsurf settings"));

        let rollback = super::execute_managed_rollback(
            "windsurf-routing",
            result.backup_path.as_deref().expect("backup"),
            "Restore ai-switchboard:windsurf for Windsurf routing",
        )
        .expect("rollback applied windsurf config");
        assert_eq!(rollback.record_id, "windsurf-routing");
        let restored: serde_json::Value =
            serde_json::from_slice(&fs::read(&settings_json).unwrap()).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    #[serial_test::serial]
    fn managed_config_apply_rejects_wrong_confirmation_for_opencode() {
        let _home = TestHome::new();
        let err = super::execute_managed_config_apply("opencode-routing", "Apply OpenCode")
            .expect_err("wrong confirmation must be rejected");
        assert!(
            err.to_string().contains("confirmation phrase"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn managed_config_apply_rejects_opencode_drift_after_preview() {
        let home = TestHome::new();
        let opencode_dir = home.path().join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        let config_json = opencode_dir.join("opencode.json");
        fs::write(&config_json, r#"{"provider":{},"theme":"system"}"#).unwrap();

        let preview =
            super::preview_managed_config_apply("opencode-routing").expect("preview apply");
        fs::write(&config_json, r#"{"provider":{},"theme":"midnight"}"#).unwrap();

        let err =
            super::execute_managed_config_apply("opencode-routing", &preview.confirmation_phrase)
                .expect_err("post-preview drift must be rejected");

        assert!(
            err.to_string().contains("confirmation phrase"),
            "unexpected error: {err:#}"
        );
        let current: serde_json::Value =
            serde_json::from_slice(&fs::read(&config_json).unwrap()).unwrap();
        assert_eq!(current["theme"], "midnight");
        assert!(current["provider"].as_object().unwrap().is_empty());
    }

    #[test]
    #[serial_test::serial]
    fn managed_rollback_rejects_backup_outside_opencode_config_directory() {
        let home = TestHome::new();
        let opencode_dir = home.path().join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        fs::write(opencode_dir.join("opencode.json"), "{}").unwrap();
        super::apply_client_setup("opencode").expect("apply opencode");
        let wrong_backup = home.path().join("opencode.json.headroom-backup-wrong");
        fs::write(&wrong_backup, "{}").unwrap();

        let err = super::execute_managed_rollback(
            "opencode-routing",
            wrong_backup.to_str().unwrap(),
            "Restore ai-switchboard:opencode for OpenCode routing",
        )
        .expect_err("wrong backup must be rejected");

        assert!(
            err.to_string().contains("must live next to"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn managed_rollback_rejects_backup_outside_promoted_editor_config_directories() {
        let home = TestHome::new();

        let windsurf_dir = home
            .path()
            .join("Library")
            .join("Application Support")
            .join("Windsurf")
            .join("User");
        fs::create_dir_all(&windsurf_dir).unwrap();
        fs::write(windsurf_dir.join("settings.json"), "{}").unwrap();
        super::apply_client_setup("windsurf").expect("apply windsurf");
        let wrong_windsurf_backup = home.path().join("settings.json.headroom-backup-wrong");
        fs::write(&wrong_windsurf_backup, "{}").unwrap();

        let windsurf_err = super::execute_managed_rollback(
            "windsurf-routing",
            wrong_windsurf_backup.to_str().unwrap(),
            "Restore ai-switchboard:windsurf for Windsurf routing",
        )
        .expect_err("wrong Windsurf backup must be rejected");

        assert!(
            windsurf_err.to_string().contains("must live next to"),
            "unexpected error: {windsurf_err:#}"
        );

        let zed_dir = home.path().join(".config").join("zed");
        fs::create_dir_all(&zed_dir).unwrap();
        fs::write(zed_dir.join("settings.json"), "{}").unwrap();
        super::apply_client_setup("zed_ai").expect("apply zed");
        let wrong_zed_backup = home.path().join("settings.json.headroom-backup-wrong");
        fs::write(&wrong_zed_backup, "{}").unwrap();

        let zed_preview =
            super::preview_managed_rollback("zed-ai-routing").expect("preview zed rollback");
        assert!(zed_preview
            .evidence
            .contains(&"Allowlisted rollback execution row: zed-ai-routing.".to_string()));

        let zed_err = super::execute_managed_rollback(
            "zed-ai-routing",
            wrong_zed_backup.to_str().unwrap(),
            "Restore ai-switchboard:zed for Zed routing",
        )
        .expect_err("wrong Zed backup must be rejected");

        assert!(
            zed_err.to_string().contains("must live next to"),
            "unexpected error: {zed_err:#}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn managed_rollback_rejects_missing_opencode_provider() {
        let home = TestHome::new();
        let opencode_dir = home.path().join(".config").join("opencode");
        fs::create_dir_all(&opencode_dir).unwrap();
        let config_json = opencode_dir.join("opencode.json");
        fs::write(&config_json, "{}").unwrap();
        super::apply_client_setup("opencode").expect("apply opencode");
        let preview = super::preview_managed_rollback("opencode-routing").expect("preview");
        let backup_path = preview.backup_path.expect("backup");
        fs::write(&config_json, "{}").unwrap();

        let err = super::execute_managed_rollback(
            "opencode-routing",
            &backup_path,
            "Restore ai-switchboard:opencode for OpenCode routing",
        )
        .expect_err("missing provider must be rejected");

        assert!(
            err.to_string().contains("marker is missing"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn apply_codex_emits_requires_openai_auth_for_chatgpt_users() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(
            codex_dir.join("auth.json"),
            "{\"auth_mode\":\"chatgpt\",\"tokens\":{\"account_id\":\"acct_123\"}}",
        )
        .unwrap();

        super::apply_client_setup("codex").expect("apply_client_setup succeeds");
        let toml = fs::read_to_string(codex_dir.join("config.toml")).unwrap();
        assert!(
            toml.contains("requires_openai_auth = true"),
            "ChatGPT-OAuth users need the flag for the account menu, got:\n{toml}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn apply_codex_omits_requires_openai_auth_for_api_key_users() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(
            codex_dir.join("auth.json"),
            "{\"auth_mode\":\"apikey\",\"OPENAI_API_KEY\":\"sk-test\"}",
        )
        .unwrap();

        super::apply_client_setup("codex").expect("apply_client_setup succeeds");
        let toml = fs::read_to_string(codex_dir.join("config.toml")).unwrap();
        assert!(
            !toml.contains("requires_openai_auth"),
            "API-key users must not be forced into an OpenAI OAuth login (#406), got:\n{toml}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn apply_codex_replaces_unmarked_legacy_headroom_provider_table() {
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(
            codex_dir.join("config.toml"),
            "model_provider = \"headroom\"\n\
model = \"gpt-5.5\"\n\n\
[model_providers.headroom]\n\
name = \"OpenAI via old Headroom proxy\"\n\
base_url = \"http://127.0.0.1:8787/v1\"\n\
supports_websockets = true\n\n\
[features]\n\
js_repl = false\n",
        )
        .unwrap();

        super::apply_client_setup("codex").expect("apply_client_setup repairs stale provider");
        let toml = fs::read_to_string(codex_dir.join("config.toml")).unwrap();
        let parsed: toml::Value = toml.parse().expect("repaired config parses");

        assert_eq!(
            toml.matches("[model_providers.headroom]").count(),
            1,
            "stale provider table should be replaced, got:\n{toml}"
        );
        assert!(
            !toml.contains("127.0.0.1:8787"),
            "stale proxy port should be removed, got:\n{toml}"
        );
        assert_eq!(
            parsed
                .get("model_providers")
                .and_then(|providers| providers.get("headroom"))
                .and_then(|headroom| headroom.get("base_url"))
                .and_then(|value| value.as_str()),
            Some(super::HEADROOM_OPENAI_BASE_URL),
            "managed provider should point at current Headroom proxy, got:\n{toml}"
        );
        assert_eq!(
            parsed
                .get("features")
                .and_then(|features| features.get("js_repl"))
                .and_then(|value| value.as_bool()),
            Some(false),
            "unrelated user tables should be preserved, got:\n{toml}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn apply_codex_keeps_root_keys_at_root_scope_when_config_ends_in_a_table() {
        // Regression for the `invalid type: string "headroom", expected a
        // boolean in features` error: a config whose last table is `[features]`
        // (boolean-only values) used to absorb the appended root keys.
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        let config_toml = codex_dir.join("config.toml");
        fs::write(
            &config_toml,
            "model = \"gpt-5.4\"\n\n[features]\njs_repl = false\n",
        )
        .unwrap();

        super::apply_client_setup("codex").expect("apply succeeds");

        let raw = fs::read_to_string(&config_toml).unwrap();
        let parsed: toml::Value = raw
            .parse()
            .unwrap_or_else(|e| panic!("valid toml: {e}\n{raw}"));

        assert_eq!(
            parsed.get("model_provider").and_then(|v| v.as_str()),
            Some("headroom"),
            "model_provider must resolve at root scope, got:\n{raw}"
        );
        assert!(
            parsed
                .get("features")
                .and_then(|f| f.get("model_provider"))
                .is_none(),
            "model_provider must not leak into [features], got:\n{raw}"
        );
        assert_eq!(
            parsed
                .get("model_providers")
                .and_then(|m| m.get("headroom"))
                .and_then(|h| h.get("base_url"))
                .and_then(|v| v.as_str()),
            Some(super::HEADROOM_OPENAI_BASE_URL),
            "provider table base_url points at the proxy, got:\n{raw}"
        );
        // The user's own content survives untouched.
        assert_eq!(
            parsed.get("model").and_then(|v| v.as_str()),
            Some("gpt-5.4"),
            "existing root key preserved, got:\n{raw}"
        );
        assert_eq!(
            parsed
                .get("features")
                .and_then(|f| f.get("js_repl"))
                .and_then(|v| v.as_bool()),
            Some(false),
            "existing [features] table preserved, got:\n{raw}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn apply_codex_repairs_a_previously_corrupted_features_block() {
        // A machine upgraded mid-bug: the old single block sits at end-of-file,
        // its root keys absorbed into [features]. Re-applying must repair it so
        // the file parses and the keys resolve at root scope.
        let home = TestHome::new();
        fs::write(home.path().join(".zshrc"), "# user zshrc\n").unwrap();
        let codex_dir = home.path().join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        let config_toml = codex_dir.join("config.toml");
        fs::write(
            &config_toml,
            "[features]\njs_repl = false\n\
             # >>> ai-switchboard:codex_cli >>>\n\
             model_provider = \"headroom\"\n\
             openai_base_url = \"http://127.0.0.1:6767/v1\"\n\n\
             [model_providers.headroom]\n\
             name = \"Headroom persistent proxy\"\n\
             base_url = \"http://127.0.0.1:6767/v1\"\n\
             supports_websockets = true\n\
             # <<< ai-switchboard:codex_cli <<<\n",
        )
        .unwrap();

        // The corrupted file is invalid against Codex's schema, but still parses
        // as TOML with the key wrongly nested under [features].
        let before: toml::Value = fs::read_to_string(&config_toml).unwrap().parse().unwrap();
        assert_eq!(
            before
                .get("features")
                .and_then(|f| f.get("model_provider"))
                .and_then(|v| v.as_str()),
            Some("headroom"),
            "precondition: corruption present"
        );

        super::apply_client_setup("codex").expect("re-apply repairs config");

        let after: toml::Value = fs::read_to_string(&config_toml).unwrap().parse().unwrap();
        assert_eq!(
            after.get("model_provider").and_then(|v| v.as_str()),
            Some("headroom")
        );
        assert!(after
            .get("features")
            .and_then(|f| f.get("model_provider"))
            .is_none());
    }

    #[test]
    #[serial_test::serial]
    fn write_setup_state_publishes_atomically() {
        let _home = TestHome::new();
        let mut state = super::ClientSetupState::default();
        state
            .configured_clients
            .insert("claude_code".into(), "2026-01-01T00:00:00+00:00".into());
        super::write_setup_state(&state).expect("write");

        let path = super::setup_state_path();
        assert!(path.exists(), "setup state file written");

        // The sibling .tmp file must not be left behind after a successful
        // publish — its presence would mean the rename step never happened.
        let tmp = {
            let mut s = path.clone().into_os_string();
            s.push(".tmp");
            std::path::PathBuf::from(s)
        };
        assert!(!tmp.exists(), "tmp file cleaned up by rename, got: {tmp:?}");

        // Round-trip survives.
        let reloaded = super::load_setup_state();
        assert!(reloaded.configured_clients.contains_key("claude_code"));
    }

    #[test]
    #[serial_test::serial]
    fn load_setup_state_falls_back_to_default_on_corrupt_file() {
        let _home = TestHome::new();
        let path = super::setup_state_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // Simulate a torn / partial write that would have happened with the
        // pre-fix non-atomic writer. The retry path inside load_setup_state
        // re-reads after a short backoff and, when the file is still bad,
        // logs a warning and returns the default rather than panicking.
        std::fs::write(&path, b"{ not json").unwrap();

        let state = super::load_setup_state();
        assert!(state.configured_clients.is_empty());
        assert!(state.remembered_clients.is_empty());
    }

    fn seed_codex_threads_db(path: &Path, rows: &[(&str, &str)]) {
        let conn = Connection::open(path).unwrap();
        conn.execute(
            "CREATE TABLE threads (id TEXT PRIMARY KEY, model_provider TEXT NOT NULL)",
            [],
        )
        .unwrap();
        for (id, provider) in rows {
            conn.execute(
                "INSERT INTO threads (id, model_provider) VALUES (?, ?)",
                [id, provider],
            )
            .unwrap();
        }
    }

    fn provider_count(path: &Path, provider: &str) -> i64 {
        let conn = Connection::open(path).unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM threads WHERE model_provider = ?1",
            [provider],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn enable_codex_retagging() {
        crate::codex_threads::set_codex_thread_retagging_settings(CodexThreadRetaggingSettings {
            codex_thread_retagging: CodexThreadRetaggingMode::Enabled,
        })
        .unwrap();
    }

    #[test]
    fn retag_one_codex_db_moves_only_matching_provider() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("state_5.sqlite");
        seed_codex_threads_db(
            &db,
            &[
                ("a", "openai"),
                ("b", "openai"),
                ("c", "headroom"),
                ("d", "anthropic"),
            ],
        );

        let moved = crate::codex_threads::retag_one_codex_db(&db, "openai", "headroom").unwrap();
        assert_eq!(moved, 2);
        assert_eq!(provider_count(&db, "openai"), 0);
        assert_eq!(provider_count(&db, "headroom"), 3);
        // Third-party providers are untouched.
        assert_eq!(provider_count(&db, "anthropic"), 1);

        // Reverse direction round-trips only the headroom rows.
        let back = crate::codex_threads::retag_one_codex_db(&db, "headroom", "openai").unwrap();
        assert_eq!(back, 3);
        assert_eq!(provider_count(&db, "headroom"), 0);
        assert_eq!(provider_count(&db, "openai"), 3);
        assert_eq!(provider_count(&db, "anthropic"), 1);
    }

    #[test]
    fn retag_one_codex_db_noop_without_threads_table() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("state_5.sqlite");
        // Open creates an empty DB with no `threads` table.
        Connection::open(&db).unwrap();
        assert_eq!(
            crate::codex_threads::retag_one_codex_db(&db, "openai", "headroom").unwrap(),
            0
        );
    }

    #[test]
    #[serial_test::serial]
    fn retag_codex_thread_providers_silent_when_no_store() {
        let _home = TestHome::new();
        // No ~/.codex stores exist under the temp home: must not panic.
        crate::codex_threads::retag_codex_thread_providers("openai", "headroom");
    }

    #[test]
    #[serial_test::serial]
    fn codex_sqlite_store_expected_gates_on_sqlite_dir_not_config() {
        let home = TestHome::new();
        let codex = home.path().join(".codex");
        // CLI-only / pre-sqlite Codex: config + sessions but no sqlite/ store.
        std::fs::create_dir_all(codex.join("sessions")).unwrap();
        std::fs::write(codex.join("config.toml"), "").unwrap();
        assert!(
            !crate::codex_threads::codex_sqlite_store_expected(),
            "config/sessions alone must not trigger the moved-store warning"
        );
        // CLI store renamed loose in codex_home (version no longer parses) ->
        // expected, so the relocation gets flagged.
        std::fs::write(codex.join("state_5x.sqlite"), "").unwrap();
        assert!(crate::codex_threads::codex_sqlite_store_expected());
        std::fs::remove_file(codex.join("state_5x.sqlite")).unwrap();
        // GUI store dir present -> a missing state_<N>.sqlite is worth flagging.
        std::fs::create_dir_all(codex.join("sqlite")).unwrap();
        assert!(crate::codex_threads::codex_sqlite_store_expected());
    }

    #[test]
    #[serial_test::serial]
    fn retag_codex_threads_to_headroom_pulls_native_threads_back() {
        // Reproduces the app-update restart path: the quit handler left threads
        // tagged `openai`; launch must retag them back to `headroom`.
        let home = TestHome::new();
        let db = home.path().join(".codex").join("state_5.sqlite");
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        seed_codex_threads_db(&db, &[("a", "openai"), ("b", "openai"), ("c", "anthropic")]);
        enable_codex_retagging();

        crate::codex_threads::retag_codex_threads_to_headroom();

        assert_eq!(provider_count(&db, "headroom"), 2);
        assert_eq!(provider_count(&db, "openai"), 0);
        // Third-party threads are untouched.
        assert_eq!(provider_count(&db, "anthropic"), 1);
    }

    #[test]
    fn codex_store_version_parses_state_filename() {
        assert_eq!(
            crate::codex_threads::codex_store_version(Path::new("/x/state_5.sqlite")),
            Some(5)
        );
        assert_eq!(
            crate::codex_threads::codex_store_version(Path::new("/x/state_42.sqlite")),
            Some(42)
        );
        assert_eq!(
            crate::codex_threads::codex_store_version(Path::new("/x/config.toml")),
            None
        );
        assert_eq!(
            crate::codex_threads::codex_store_version(Path::new("/x/state_.sqlite")),
            None
        );
        assert_eq!(
            crate::codex_threads::codex_store_version(Path::new("/x/state_x.sqlite")),
            None
        );
        assert_eq!(
            crate::codex_threads::codex_store_version(Path::new("/x/state_5.db")),
            None
        );
    }

    #[test]
    #[serial_test::serial]
    fn codex_home_honors_env_else_default() {
        let home = TestHome::new();
        // TestHome clears CODEX_HOME, so we fall back to $HOME/.codex.
        assert_eq!(codex_home(), home.path().join(".codex"));

        let custom = home.path().join("custom-codex");
        std::env::set_var("CODEX_HOME", &custom);
        assert_eq!(codex_home(), custom);

        // An empty value is ignored (treated as unset).
        std::env::set_var("CODEX_HOME", "");
        assert_eq!(codex_home(), home.path().join(".codex"));
    }

    #[test]
    #[serial_test::serial]
    fn discover_codex_state_dbs_finds_versioned_stores() {
        let home = TestHome::new();
        let codex = home.path().join(".codex");
        std::fs::create_dir_all(codex.join("sqlite")).unwrap();
        // GUI store under sqlite/, CLI store at the root, on different versions.
        std::fs::File::create(codex.join("sqlite").join("state_6.sqlite")).unwrap();
        std::fs::File::create(codex.join("state_5.sqlite")).unwrap();
        // A non-store file in the same dir must be ignored.
        std::fs::File::create(codex.join("config.toml")).unwrap();

        let versions: BTreeSet<u32> = crate::codex_threads::discover_codex_state_dbs()
            .into_iter()
            .map(|(_, v)| v)
            .collect();
        assert_eq!(versions, BTreeSet::from([5, 6]));
    }

    #[test]
    #[serial_test::serial]
    fn retag_handles_unknown_store_version() {
        // Future-proofing: a Codex store-version bump (here state_99) must not
        // write until the schema version is verified.
        let home = TestHome::new();
        let db = home.path().join(".codex").join("state_99.sqlite");
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        seed_codex_threads_db(&db, &[("a", "openai"), ("b", "openai"), ("c", "anthropic")]);
        enable_codex_retagging();

        let report = crate::codex_threads::retag_codex_thread_providers("openai", "headroom");

        assert_eq!(report.reports.len(), 1);
        assert_eq!(report.reports[0].rows_changed, 0);
        assert!(report.reports[0]
            .skipped_reason
            .as_deref()
            .unwrap_or_default()
            .contains("unknown Codex store version"));
        assert_eq!(provider_count(&db, "headroom"), 0);
        assert_eq!(provider_count(&db, "openai"), 2);
        assert_eq!(provider_count(&db, "anthropic"), 1);
    }

    #[test]
    #[serial_test::serial]
    fn codex_retagging_defaults_to_ask_and_does_not_write() {
        let home = TestHome::new();
        let db = home.path().join(".codex").join("state_5.sqlite");
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        seed_codex_threads_db(&db, &[("a", "openai"), ("b", "openai")]);

        let settings = crate::codex_threads::get_codex_thread_retagging_settings();
        assert_eq!(
            settings.codex_thread_retagging,
            CodexThreadRetaggingMode::Ask
        );
        let report = crate::codex_threads::retag_codex_thread_providers("openai", "headroom");

        assert_eq!(report.mode, CodexThreadRetaggingMode::Ask);
        assert!(report.reports.is_empty());
        assert_eq!(provider_count(&db, "openai"), 2);
        assert!(!crate::codex_threads::codex_retagging_settings_path().exists());
    }

    #[test]
    #[serial_test::serial]
    fn enabled_codex_retagging_creates_backup_and_can_restore() {
        let home = TestHome::new();
        let db = home.path().join(".codex").join("state_5.sqlite");
        std::fs::create_dir_all(db.parent().unwrap()).unwrap();
        seed_codex_threads_db(&db, &[("a", "openai"), ("b", "openai"), ("c", "anthropic")]);
        enable_codex_retagging();

        let report = crate::codex_threads::retag_codex_thread_providers("openai", "headroom");

        assert_eq!(report.mode, CodexThreadRetaggingMode::Enabled);
        assert_eq!(report.reports.len(), 1);
        let backup = report.reports[0].backup_path.as_ref().expect("backup path");
        assert!(Path::new(backup).exists());
        assert_eq!(report.reports[0].rows_changed, 2);
        assert_eq!(provider_count(&db, "headroom"), 2);

        let restored = crate::codex_threads::restore_codex_thread_db_backup(backup).unwrap();
        assert_eq!(restored.restored_path, db.display().to_string());
        assert_eq!(provider_count(&db, "openai"), 2);
        assert_eq!(provider_count(&db, "headroom"), 0);
        assert_eq!(provider_count(&db, "anthropic"), 1);
    }

    #[test]
    #[serial_test::serial]
    fn managed_footprint_report_is_redacted_and_lists_core_surfaces() {
        let home = TestHome::new();
        std::fs::create_dir_all(home.path().join(".codex")).unwrap();
        std::fs::write(
            home.path().join(".codex").join("config.toml"),
            "secret = true",
        )
        .unwrap();

        let report = client_footprint::get_managed_footprint();
        let ids = report
            .items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<BTreeSet<_>>();

        assert!(ids.contains("app-storage"));
        assert!(ids.contains("legacy-storage"));
        assert!(ids.contains("codex-config"));
        assert!(ids.contains("claude-settings"));
        assert!(ids.contains("launch-agent"));
        assert!(ids.contains("keychain-mac-ai-switchboard"));
        assert!(ids.contains("gemini_cli-sidecar"));

        let serialized = serde_json::to_string(&report).unwrap();
        assert!(!serialized.contains("secret = true"));
        assert!(!serialized.contains("sk-"));
        assert!(serialized.contains("Keychain service: mac-ai-switchboard"));
        assert!(serialized.contains("*.headroom-backup-*"));
        assert!(!serialized.contains("*.headroom.bak"));
    }

    #[test]
    #[serial_test::serial]
    fn managed_footprint_marks_existing_paths_without_reading_values() {
        let home = TestHome::new();
        let sidecar = home.path().join(".gemini").join(SWITCHBOARD_ROUTING_FILE);
        std::fs::create_dir_all(sidecar.parent().unwrap()).unwrap();
        std::fs::write(&sidecar, "token = sk-test").unwrap();

        let report = client_footprint::get_managed_footprint();
        let gemini = report
            .items
            .iter()
            .find(|item| item.id == "gemini_cli-sidecar")
            .expect("gemini footprint");

        assert!(gemini.exists);
        assert!(gemini.managed);
        assert!(gemini.reversible);
        assert!(!serde_json::to_string(&report).unwrap().contains("sk-test"));
    }

    #[test]
    #[serial_test::serial]
    fn uninstall_dry_run_lists_current_cleanup_targets() {
        let home = TestHome::new();
        let current_storage = home
            .path()
            .join("Library")
            .join("Application Support")
            .join("Mac AI Switchboard");
        std::fs::create_dir_all(&current_storage).unwrap();

        let report = client_footprint::uninstall_dry_run_report();
        let serialized = serde_json::to_string(&report).unwrap();

        assert!(serialized.contains("Mac AI Switchboard"));
        assert!(serialized.contains(client_footprint::APP_BUNDLE_ID));
        assert!(serialized.contains("keychain://com.tarunagarwal.mac-ai-switchboard.account"));
        assert!(serialized.contains("User repositories and source files are never deleted."));
        assert!(!serialized.contains("session-token="));

        let app_storage = report
            .targets
            .iter()
            .find(|target| target.id == "app-support-current")
            .expect("current app storage target");
        assert!(app_storage.exists);
        assert!(app_storage.managed);
        assert!(app_storage.requires_confirmation);
    }

    #[test]
    fn zed_config_path_returns_user_home_config_json() {
        let path = zed_config_path();
        assert!(path.to_string_lossy().contains(".config"));
        assert!(path.to_string_lossy().contains("zed"));
        assert!(path.to_string_lossy().ends_with("settings.json"));
    }

    #[test]
    fn zed_config_backup_pattern_matches_timestamped_backups() {
        let pattern = super::zed_config_backup_pattern();
        assert!(pattern.contains("settings.json"));
        assert!(pattern.contains("headroom-backup-"));
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: preview,backup,apply,verify,rollback,off
    fn grok_native_endpoint_setup_preserves_config_and_supports_rollback_and_off_cleanup() {
        let home = TestHome::new();
        let config = grok_config_path();
        fs::create_dir_all(config.parent().unwrap()).expect("create Grok config parent");
        let original = "[cli]\nauto_update = false\n\n[models]\ndefault = \"grok-build\"\n\n[model.grok-build]\ncontext_window = 128000\n";
        fs::write(&config, original).expect("seed Grok config");

        let preview =
            super::preview_managed_config_apply("grok-routing").expect("native Grok preview");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert!(preview.target_path.ends_with(".grok/config.toml"));
        assert!(preview.proposed_state.contains("[endpoints]"));
        assert!(preview
            .proposed_state
            .contains(super::GROK_HEADROOM_BASE_URL));
        assert!(preview
            .evidence
            .iter()
            .any(|item| item.contains("models_base_url")));

        let applied = super::execute_managed_config_apply(
            super::GROK_ROLLBACK_RECORD_ID,
            &preview.confirmation_phrase,
        )
        .expect("apply native Grok endpoint");
        assert!(applied.changed);
        assert!(super::grok_provider_config_matches().expect("verify native Grok"));
        let configured = fs::read_to_string(&config).expect("read configured Grok");
        assert!(configured.contains("auto_update = false"));
        assert!(configured.contains("default = \"grok-build\""));
        assert!(configured.contains("context_window = 128000"));
        assert!(configured.contains(super::GROK_HEADROOM_BASE_URL));
        assert!(!configured.contains("auth.json"));

        let rollback = super::preview_managed_rollback(super::GROK_ROLLBACK_RECORD_ID)
            .expect("native Grok rollback preview");
        assert_eq!(rollback.status, ManagedRollbackExecutionStatus::Ready);
        let backup = rollback.backup_path.clone().expect("Grok backup path");
        let rollback_result = super::execute_managed_rollback(
            super::GROK_ROLLBACK_RECORD_ID,
            &backup,
            &rollback.confirmation_phrase,
        )
        .expect("rollback native Grok");
        assert_eq!(rollback_result.record_id, super::GROK_ROLLBACK_RECORD_ID);
        assert_eq!(fs::read_to_string(&config).unwrap(), original);

        // Re-apply through the normal connector path to cover sidecar parity,
        // then Off cleanup must remove only Switchboard-owned artifacts.
        super::apply_client_setup("grok_cli").expect("apply Grok connector");
        assert!(
            super::verify_client_setup("grok_cli")
                .expect("verify Grok connector")
                .verified
        );
        super::disable_client_setup("grok_cli").expect("disable Grok connector");
        assert_eq!(fs::read_to_string(&config).unwrap(), original);
        assert!(!super::grok_provider_config_matches().unwrap());
        assert!(!planned_switchboard_sidecar_matches("grok_cli").unwrap());

        drop(home);
    }

    #[test]
    #[serial_test::serial]
    // lifecycle-intent: preview,backup,rollback
    fn goose_native_endpoint_apply_exposes_rollback_center_restore() {
        let home = TestHome::new();
        let config = crate::goose_provider_configs::goose_config_path();
        fs::create_dir_all(config.parent().unwrap()).expect("create Goose config dir");
        let original = "active_provider: openai\nproviders:\n  openai:\n    enabled: true\n    model: gpt-4o\n    configured: true\nkeep: true\n";
        fs::write(&config, original).expect("seed Goose config");

        let preview = super::preview_managed_config_apply(
            crate::goose_provider_configs::GOOSE_NATIVE_APPLY_RECORD_ID,
        )
        .expect("native Goose preview");
        assert_eq!(preview.status, ManagedRollbackExecutionStatus::Ready);
        assert_eq!(preview.target_path, config.display().to_string());
        assert!(preview.proposed_state.contains(HEADROOM_OPENAI_BASE_URL));
        assert!(preview.evidence.iter().any(|item| {
            let item = item.to_ascii_lowercase();
            item.contains("allowlisted") && item.contains("endpoint")
        }));

        let applied = super::execute_managed_config_apply(
            crate::goose_provider_configs::GOOSE_NATIVE_APPLY_RECORD_ID,
            &preview.confirmation_phrase,
        )
        .expect("apply native Goose endpoint");
        assert!(applied.changed);
        let backup = applied.backup_path.clone().expect("Goose backup path");
        assert!(crate::goose_provider_configs::goose_provider_config_matches().unwrap());
        let configured = fs::read_to_string(&config).expect("read configured Goose");
        assert!(configured.contains("active_provider: openai"));
        assert!(configured.contains("model: gpt-4o"));
        assert!(configured.contains("keep: true"));
        assert!(configured.contains(HEADROOM_OPENAI_BASE_URL));
        assert!(!configured.contains("secrets.yaml"));

        let rollback_preview = super::preview_managed_rollback(
            crate::goose_provider_configs::GOOSE_NATIVE_APPLY_RECORD_ID,
        )
        .expect("native Goose rollback preview");
        assert_eq!(
            rollback_preview.status,
            ManagedRollbackExecutionStatus::Ready
        );
        assert!(rollback_preview.marker_present);
        assert!(rollback_preview.backup_exists);
        let rollback = super::execute_managed_rollback(
            crate::goose_provider_configs::GOOSE_NATIVE_APPLY_RECORD_ID,
            &backup,
            &rollback_preview.confirmation_phrase,
        )
        .expect("rollback native Goose endpoint");
        assert_eq!(
            rollback.record_id,
            crate::goose_provider_configs::GOOSE_NATIVE_APPLY_RECORD_ID
        );
        assert_eq!(fs::read_to_string(&config).unwrap(), original);
        assert!(!crate::goose_provider_configs::goose_provider_config_matches().unwrap());

        drop(home);
    }

    #[test]
    // lifecycle-intent: detect,preview
    fn cursor_connector_has_fixture_home_dry_run_preview() {
        let detected_clients = Vec::new();
        let connectors = super::list_client_connectors(&detected_clients).expect("list connectors");
        let cursor = connectors
            .iter()
            .find(|connector| connector.client_id == "cursor")
            .expect("cursor connector");

        assert_eq!(cursor.support_status, ClientConnectorSupportStatus::Planned);
        assert_eq!(cursor.automation_gates.len(), 7);
        assert!(cursor
            .config_creation_step_details
            .iter()
            .any(|step| step.detail.contains("state.vscdb")));
        assert!(cursor
            .config_locations
            .iter()
            .any(|location| location.contains("Cursor/User/settings.json")));
        assert!(cursor
            .config_locations
            .iter()
            .any(|location| location.contains("Cursor/User/profiles/*/settings.json")));
        assert!(!cursor
            .config_locations
            .iter()
            .any(|location| location.contains("globalStorage")));

        let preview = cursor
            .config_dry_run_preview
            .as_ref()
            .expect("cursor dry-run preview");
        assert!(preview.target.contains("Cursor/User/settings.json"));
        assert!(preview.marker.contains("cursor"));
        assert!(preview.rollback_preview.contains("Switchboard-owned"));
        assert!(preview
            .apply_blocked_reason
            .contains("does not document a stable on-disk"));
        assert_eq!(preview.confirmation_phrase, "CURSOR NATIVE SCHEMA GATE");
    }

    #[test]
    fn grok_connector_exposes_documented_native_endpoint_and_credential_boundary() {
        let detected_clients = Vec::new();
        let connectors = super::list_client_connectors(&detected_clients).expect("list connectors");
        let grok = connectors
            .iter()
            .find(|connector| connector.client_id == "grok_cli")
            .expect("grok connector");

        assert_eq!(grok.support_status, ClientConnectorSupportStatus::Managed);
        assert!(!grok.enabled);
        assert!(grok
            .config_locations
            .iter()
            .any(|location| location.contains(".config/xai")));
        assert!(grok
            .config_locations
            .iter()
            .any(|location| location.contains(".grok/config.toml")));
        assert!(grok
            .automation_gates
            .iter()
            .any(|gate| gate.contains("models_base_url")));
        assert!(grok
            .automation_gates
            .iter()
            .any(|gate| gate.contains("XAI_API_KEY")));
        let grok_preview = grok
            .config_dry_run_preview
            .as_ref()
            .expect("grok managed dry-run preview");
        assert!(grok_preview.apply_blocked_reason.contains("read-only"));
        assert!(grok_preview.writes.is_empty());
        assert!(grok.config_creation_step_details.is_empty());
    }

    #[test]
    // lifecycle-intent: detect
    fn continue_connector_exposes_managed_native_and_sidecar_paths() {
        let detected_clients = Vec::new();
        let connectors = super::list_client_connectors(&detected_clients).expect("list connectors");
        let continue_connector = connectors
            .iter()
            .find(|connector| connector.client_id == "continue")
            .expect("continue connector");

        assert_eq!(
            continue_connector.support_status,
            ClientConnectorSupportStatus::Managed
        );
        assert!(!continue_connector.enabled);
        assert!(!continue_connector.verified);
        assert!(continue_connector
            .config_locations
            .iter()
            .any(|location| location.contains(".continue")));
        assert!(continue_connector.config_creation_step_details.is_empty());
        let continue_preview = continue_connector
            .config_dry_run_preview
            .as_ref()
            .expect("continue managed dry-run preview");
        assert!(continue_preview.apply_blocked_reason.contains("read-only"));
        assert!(continue_preview.writes.is_empty());
        assert!(continue_connector.automation_path.is_empty());
    }

    #[test]
    // lifecycle-intent: detect
    fn qwen_connector_exposes_managed_sidecar_without_provider_writes() {
        let _home = TestHome::new();
        let detected_clients = Vec::new();
        let connectors = super::list_client_connectors(&detected_clients).expect("list connectors");
        let qwen = connectors
            .iter()
            .find(|connector| connector.client_id == "qwen_code")
            .expect("qwen connector");

        assert_eq!(qwen.support_status, ClientConnectorSupportStatus::Managed);
        assert!(!qwen.enabled);
        assert!(!qwen.verified);
        assert!(qwen
            .config_locations
            .iter()
            .any(|location| location.contains(".qwen")));
        assert!(qwen
            .config_locations
            .iter()
            .any(|location| location.contains(".config/qwen")));
        assert!(qwen.config_creation_step_details.is_empty());

        let qwen_preview = qwen
            .config_dry_run_preview
            .as_ref()
            .expect("qwen managed dry-run preview");
        assert!(qwen_preview.apply_blocked_reason.contains("read-only"));
        assert!(qwen_preview.writes.is_empty());
    }

    #[test]
    // lifecycle-intent: preview,backup,apply,verify,rollback,off
    fn qwen_connector_applies_and_disables_switchboard_owned_sidecar_only() {
        let _home = TestHome::new();
        let routing_path = planned_sidecar_routing_path("qwen_code").expect("qwen sidecar path");
        std::fs::create_dir_all(routing_path.parent().expect("qwen sidecar parent"))
            .expect("create qwen config directory");
        std::fs::write(&routing_path, "# user-owned qwen note\nkeep this\n")
            .expect("seed qwen sidecar");

        let result = super::apply_client_setup("qwen_code").expect("apply qwen sidecar");
        assert!(result.applied);
        assert!(!result.already_configured);
        assert_eq!(result.client_id, "qwen_code");
        assert_eq!(result.backup_files.len(), 1);
        assert!(result
            .changed_files
            .iter()
            .any(|path| path.contains("ai-switchboard-routing.md")));

        let body = std::fs::read_to_string(&routing_path).expect("read qwen sidecar");
        assert!(body.contains("# user-owned qwen note\nkeep this"));
        assert!(body.contains("ai-switchboard:qwen_code"));
        assert!(body.contains("reversible Qwen Code routing-intent sidecar"));
        assert!(
            super::verify_client_setup("qwen_code")
                .expect("verify qwen sidecar")
                .verified
        );

        let rollback = super::preview_managed_rollback("qwen-code-routing")
            .expect("preview qwen rollback");
        assert_eq!(rollback.status, ManagedRollbackExecutionStatus::Ready);
        assert!(rollback.marker_present);
        let rollback_result = super::execute_managed_rollback(
            "qwen-code-routing",
            "",
            "Restore ai-switchboard:qwen_code for Qwen Code routing",
        )
        .expect("execute qwen rollback");
        assert_eq!(rollback_result.record_id, "qwen-code-routing");
        assert_eq!(
            std::fs::read_to_string(&routing_path).expect("read rolled back qwen sidecar"),
            "# user-owned qwen note\nkeep this\n"
        );

        super::apply_client_setup("qwen_code").expect("reapply qwen sidecar");
        super::disable_client_setup("qwen_code").expect("disable qwen sidecar");
        assert!(
            !super::verify_client_setup("qwen_code")
                .expect("verify removed qwen sidecar")
                .verified
        );
        assert!(routing_path.exists());
        let cleaned = std::fs::read_to_string(&routing_path).expect("read cleaned qwen sidecar");
        assert!(cleaned.contains("# user-owned qwen note\nkeep this"));
        assert!(!cleaned.contains("ai-switchboard:qwen_code"));
        let drifted = cleaned.replace("# user-owned qwen note", "# user-owned qwen note\nhttp://127.0.0.1:1");
        std::fs::write(&routing_path, drifted).expect("drift qwen sidecar");
        assert!(!super::verify_client_setup("qwen_code")
            .expect("verify drifted qwen sidecar")
            .verified);
        assert!(super::apply_client_setup("qwen_code")
            .expect("repair qwen sidecar")
            .verification
            .verified);
        super::disable_client_setup("qwen_code").expect("disable repaired qwen sidecar");
        let repaired_cleaned = std::fs::read_to_string(&routing_path).expect("read repaired qwen sidecar");
        super::disable_client_setup("qwen_code").expect("repeat disable qwen sidecar");
        assert_eq!(std::fs::read_to_string(&routing_path).expect("read repeated qwen off"), repaired_cleaned);
    }

    #[test]
    fn qwen_verification_fails_closed_for_missing_and_malformed_sidecars() {
        let _home = TestHome::new();
        let routing_path = planned_sidecar_routing_path("qwen_code").expect("qwen sidecar path");
        std::fs::create_dir_all(routing_path.parent().expect("qwen sidecar parent"))
            .expect("create qwen config directory");
        std::fs::write(&routing_path, "# user-owned qwen note\nkeep this\n")
            .expect("seed qwen sidecar");

        super::apply_client_setup("qwen_code").expect("apply qwen sidecar");
        std::fs::remove_file(&routing_path).expect("remove qwen sidecar");
        assert!(!super::verify_client_setup("qwen_code")
            .expect("verify missing qwen sidecar")
            .verified);

        std::fs::write(
            &routing_path,
            "# user-owned qwen note\n# >>> ai-switchboard:qwen_code >>>\npartial marker\n",
        )
        .expect("write malformed qwen sidecar");
        assert!(!super::verify_client_setup("qwen_code")
            .expect("verify malformed qwen sidecar")
            .verified);
        assert!(super::apply_client_setup("qwen_code")
            .expect("repair malformed qwen sidecar")
            .verification
            .verified);
    }
