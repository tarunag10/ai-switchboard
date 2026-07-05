use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

pub(super) fn script_path(script_name: &str) -> Result<PathBuf> {
    for candidate in script_candidates(script_name) {
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    bail!("repo-memory script {script_name} is missing from dev scripts and bundled resources")
}

pub(super) fn script_candidates(script_name: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("scripts").join(script_name));
        if let Some(parent) = cwd.parent() {
            candidates.push(parent.join("scripts").join(script_name));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(mac_os_dir) = exe.parent() {
            if let Some(contents_dir) = mac_os_dir.parent() {
                let resources_dir = contents_dir.join("Resources");
                candidates.push(resources_dir.join("_up_").join("scripts").join(script_name));
                candidates.push(resources_dir.join("scripts").join(script_name));
                candidates.push(resources_dir.join(script_name));
            }
        }
        if let Some(exe_dir) = exe.parent() {
            candidates.push(exe_dir.join("scripts").join(script_name));
        }
    }
    candidates
}

pub(super) fn command_available_on_path(command: &str) -> bool {
    resolve_command_path(command).is_some()
}

pub(super) fn resolve_command_path(command: &str) -> Option<PathBuf> {
    fn is_executable_file(path: &Path) -> bool {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }

    if command.contains(std::path::MAIN_SEPARATOR) {
        let path = PathBuf::from(command);
        return is_executable_file(&path).then_some(path);
    }
    let path_candidates = std::env::var_os("PATH")
        .map(|path_var| std::env::split_paths(&path_var).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .chain(
            [
                "/opt/homebrew/bin",
                "/usr/local/bin",
                "/usr/bin",
                "/bin",
                "/opt/local/bin",
            ]
            .into_iter()
            .map(PathBuf::from),
        );
    path_candidates
        .map(|dir| dir.join(command))
        .find(|candidate| is_executable_file(candidate))
}

/// Claude Code >=2.x stores user-scope MCP servers in `~/.claude.json` under
/// `mcpServers.<name>`. The legacy `~/.claude/mcp.json` path written by our
/// Python CLI's fallback branch is ignored. Reading the file Claude Code
/// actually reads is the only reliable way to confirm the registration
/// landed where `/mcp` and `claude mcp list` will see it.
pub(super) fn claude_code_has_headroom_mcp_server() -> bool {
    claude_code_has_mcp_server("headroom")
}

pub(super) fn claude_code_has_mcp_server(name: &str) -> bool {
    let Some(home) = dirs::home_dir() else {
        return false;
    };
    let path = home.join(".claude.json");
    let Ok(bytes) = std::fs::read(&path) else {
        return false;
    };
    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
        return false;
    };
    value.get("mcpServers").and_then(|v| v.get(name)).is_some()
}

/// Writes the headroom MCP server entry directly to `~/.claude.json`.
/// Used when `claude mcp add` is unavailable (e.g. bare GUI PATH). Preserves
/// all existing keys; only merges `mcpServers.headroom`.
pub(super) fn write_headroom_to_claude_json(entrypoint: &Path, proxy_url: &str) -> Result<()> {
    write_mcp_server_to_claude_json(
        "headroom",
        json!({
            "command": entrypoint,
            "args": ["mcp", "serve"],
            "env": { "HEADROOM_PROXY_URL": proxy_url },
        }),
    )
}

pub(super) fn write_mcp_server_to_claude_json(name: &str, server: Value) -> Result<()> {
    let Some(home) = dirs::home_dir() else {
        anyhow::bail!("home directory not available");
    };
    let path = home.join(".claude.json");
    let mut config: Value = if path.exists() {
        std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_else(|| json!({}))
    } else {
        json!({})
    };
    let root = config
        .as_object_mut()
        .context("~/.claude.json root is not JSON object")?;
    root.entry("mcpServers")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .context("~/.claude.json mcpServers is not JSON object")?
        .insert(name.into(), server);
    std::fs::write(&path, serde_json::to_vec_pretty(&config)?)
        .with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use serde_json::{json, Value};

    use super::{command_available_on_path, script_candidates, write_mcp_server_to_claude_json};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "{}-{}",
            prefix,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    struct TestHome {
        root: PathBuf,
        previous_home: Option<std::ffi::OsString>,
    }

    impl TestHome {
        fn new(prefix: &str) -> Self {
            let root = unique_temp_dir(prefix);
            fs::create_dir_all(&root).expect("create home");
            let previous_home = std::env::var_os("HOME");
            std::env::set_var("HOME", &root);
            Self {
                root,
                previous_home,
            }
        }
    }

    impl Drop for TestHome {
        fn drop(&mut self) {
            match self.previous_home.take() {
                Some(home) => std::env::set_var("HOME", home),
                None => std::env::remove_var("HOME"),
            }
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    #[serial_test::serial]
    fn mcp_server_merge_preserves_existing_claude_json() {
        let home = TestHome::new("repo-memory-mcp-home");
        let claude_json = home.root.join(".claude.json");
        fs::write(
            &claude_json,
            br#"{"theme":"dark","mcpServers":{"headroom":{"command":"headroom"}}}"#,
        )
        .expect("write claude json");

        write_mcp_server_to_claude_json(
            "repo-memory",
            json!({"command":"node","args":["repo-intelligence.mjs","--mcp-serve"]}),
        )
        .expect("write repo memory mcp");

        let value: Value =
            serde_json::from_slice(&fs::read(&claude_json).expect("read claude json"))
                .expect("parse claude json");
        assert_eq!(value["theme"], "dark");
        assert!(value["mcpServers"]["headroom"].is_object());
        assert!(value["mcpServers"]["repo-memory"]["command"]
            .as_str()
            .expect("repo-memory command")
            .ends_with("node"));
    }

    #[test]
    fn command_available_on_path_detects_missing_commands() {
        assert!(!command_available_on_path(
            "mac-ai-switchboard-definitely-missing-command"
        ));
    }

    #[test]
    fn command_available_on_path_accepts_absolute_node_path() {
        for candidate in [
            "/usr/local/bin/node",
            "/opt/homebrew/bin/node",
            "/usr/bin/node",
        ] {
            if std::path::Path::new(candidate).exists() {
                assert!(command_available_on_path(candidate));
                return;
            }
        }
    }

    #[test]
    fn repo_memory_script_candidates_include_dev_and_resource_paths() {
        let candidates = script_candidates("repo-intelligence.mjs");
        assert!(
            candidates
                .iter()
                .any(|path| path.ends_with("scripts/repo-intelligence.mjs")),
            "dev script path should be a candidate"
        );
        assert!(
            candidates
                .iter()
                .any(|path| path.display().to_string().contains("Resources")),
            "bundled resource path should be a candidate"
        );
    }
}
