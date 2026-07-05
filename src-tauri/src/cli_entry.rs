use crate::{
    client_adapters, client_footprint, codex_threads, message_logging, storage, tool_manager,
};

pub(crate) fn handle_headless_cli(args: &[String]) -> Option<i32> {
    if args.iter().any(|arg| arg == "--print-managed-footprint") {
        match serde_json::to_string_pretty(&client_footprint::get_managed_footprint()) {
            Ok(report) => {
                println!("{report}");
                return Some(0);
            }
            Err(err) => {
                eprintln!("failed to build managed footprint report: {err}");
                return Some(1);
            }
        }
    }

    if args.iter().any(|arg| arg == "--uninstall-dry-run") {
        match serde_json::to_string_pretty(&client_footprint::uninstall_dry_run_report()) {
            Ok(report) => {
                println!("{report}");
                return Some(0);
            }
            Err(err) => {
                eprintln!("failed to build uninstall dry-run report: {err}");
                return Some(1);
            }
        }
    }

    if args.iter().any(|arg| arg == "--disable-routing") {
        match client_adapters::clear_client_setups() {
            Ok(()) => {
                println!("disabled managed routing");
                return Some(0);
            }
            Err(err) => {
                eprintln!("failed to disable managed routing: {err}");
                return Some(1);
            }
        }
    }

    if args.iter().any(|arg| arg == "--disable-rtk") {
        let runtime = tool_manager::ManagedRuntime::bootstrap_root(&storage::app_data_dir());
        match client_adapters::set_rtk_enabled(
            false,
            &runtime.bin_dir.join("rtk"),
            &runtime.venv_dir.join("bin").join("python"),
        ) {
            Ok(()) => {
                println!("disabled managed RTK integration");
                return Some(0);
            }
            Err(err) => {
                eprintln!("failed to disable RTK integration: {err}");
                return Some(1);
            }
        }
    }

    if args.iter().any(|arg| arg == "--disable-markitdown") {
        let runtime = tool_manager::ManagedRuntime::bootstrap_root(&storage::app_data_dir());
        match client_adapters::disable_markitdown_integration(&runtime.bin_dir.join("markitdown")) {
            Ok(changed) => {
                println!("disabled managed MarkItDown integration changed={changed}");
                return Some(0);
            }
            Err(err) => {
                eprintln!("failed to disable MarkItDown integration: {err}");
                return Some(1);
            }
        }
    }

    if args.iter().any(|arg| arg == "--disable-caveman") {
        match client_adapters::disable_caveman_integration() {
            Ok(changed) => {
                println!("disabled managed Caveman integration changed={changed}");
                return Some(0);
            }
            Err(err) => {
                eprintln!("failed to disable Caveman integration: {err}");
                return Some(1);
            }
        }
    }

    if args.iter().any(|arg| arg == "--uninstall-managed-config") {
        let removed = client_adapters::perform_full_cleanup();
        match serde_json::to_string_pretty(&removed) {
            Ok(report) => {
                println!("{report}");
                return Some(0);
            }
            Err(err) => {
                eprintln!("failed to serialize cleanup report: {err}");
                return Some(1);
            }
        }
    }

    if args.iter().any(|arg| arg == "--purge-logs") {
        let activity_facts = storage::config_file(&storage::app_data_dir(), "activity-facts.json");
        match serde_json::to_string_pretty(&message_logging::purge_message_logs(&activity_facts)) {
            Ok(report) => {
                println!("{report}");
                return Some(0);
            }
            Err(err) => {
                eprintln!("failed to serialize log purge report: {err}");
                return Some(1);
            }
        }
    }

    if args.iter().any(|arg| arg == "--doctor-reset") {
        let routing = client_adapters::clear_client_setups();
        let activity_facts = storage::config_file(&storage::app_data_dir(), "activity-facts.json");
        let purge = message_logging::purge_message_logs(&activity_facts);
        match routing {
            Ok(()) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&purge)
                        .unwrap_or_else(|_| "{\"logsPurged\":true}".to_string())
                );
                return Some(0);
            }
            Err(err) => {
                eprintln!("doctor reset partially failed while disabling routing: {err}");
                return Some(1);
            }
        }
    }

    if let Some(index) = args
        .iter()
        .position(|arg| arg == "--restore-codex-thread-db-backup")
    {
        let Some(path) = args.get(index + 1) else {
            eprintln!("missing path for --restore-codex-thread-db-backup");
            return Some(2);
        };
        match codex_threads::restore_codex_thread_db_backup(path) {
            Ok(result) => match serde_json::to_string_pretty(&result) {
                Ok(report) => {
                    println!("{report}");
                    return Some(0);
                }
                Err(err) => {
                    eprintln!("failed to serialize Codex restore result: {err}");
                    return Some(1);
                }
            },
            Err(err) => {
                eprintln!("failed to restore Codex thread DB backup: {err}");
                return Some(1);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::handle_headless_cli;

    #[test]
    fn ignores_non_headless_args() {
        let args = vec!["mac-ai-switchboard".to_string(), "--normal".to_string()];

        assert_eq!(handle_headless_cli(&args), None);
    }

    #[test]
    fn restore_codex_backup_requires_path() {
        let args = vec![
            "mac-ai-switchboard".to_string(),
            "--restore-codex-thread-db-backup".to_string(),
        ];

        assert_eq!(handle_headless_cli(&args), Some(2));
    }
}
