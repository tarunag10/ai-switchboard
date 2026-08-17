use std::env;
use std::fs;
use std::path::Path;

const ALWAYS_FORWARDED_ENV_VARS: &[&str] = &[
    "HEADROOM_UPDATER_PUBLIC_KEY",
    "HEADROOM_UPDATER_ENDPOINTS",
    "HEADROOM_UPDATER_STAGING_ENDPOINTS",
];

const REMOTE_SERVICE_ENV_VARS: &[&str] = &["HEADROOM_APTABASE_APP_KEY", "HEADROOM_SENTRY_DSN"];

fn main() {
    println!("cargo:rustc-check-cfg=cfg(headroom_remote_services)");
    println!("cargo:rerun-if-env-changed=HEADROOM_BUILD_FLAVOR");
    println!("cargo:rerun-if-env-changed=HEADROOM_REMOTE_SERVICES");

    let remote_services_enabled = configured_value("HEADROOM_REMOTE_SERVICES")
        .map(|value| is_truthy(&value))
        .unwrap_or(false)
        && configured_value("HEADROOM_BUILD_FLAVOR")
            .map(|value| value.trim() != "local-free")
            .unwrap_or(true);

    if remote_services_enabled {
        println!("cargo:rustc-cfg=headroom_remote_services");
    }

    for key in ALWAYS_FORWARDED_ENV_VARS.iter().chain(
        remote_services_enabled
            .then_some(REMOTE_SERVICE_ENV_VARS)
            .into_iter()
            .flatten(),
    ) {
        println!("cargo:rerun-if-env-changed={key}");
        if env::var_os(key).is_none() {
            if let Some(value) = env_file_value(key) {
                println!("cargo:rustc-env={key}={value}");
            }
        }
    }

    println!("cargo:rerun-if-changed=../.env");
    println!("cargo:rerun-if-changed=../.env.local");
    tauri_build::build()
}

fn configured_value(key: &str) -> Option<String> {
    env::var(key).ok().or_else(|| {
        ["../.env.local", "../.env"]
            .iter()
            .find_map(|path| read_env_file_value(Path::new(path), key))
    })
}

fn is_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn env_file_value(key: &str) -> Option<String> {
    ["../.env.local", "../.env"]
        .iter()
        .find_map(|path| read_env_file_value(Path::new(path), key))
}

fn read_env_file_value(path: &Path, key: &str) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let (name, raw_value) = trimmed.split_once('=')?;
        if name.trim() != key {
            continue;
        }

        return Some(parse_env_value(raw_value.trim()));
    }

    None
}

fn parse_env_value(raw_value: &str) -> String {
    let unquoted = match raw_value.as_bytes() {
        [quote, middle @ .., end] if (*quote == b'"' || *quote == b'\'') && quote == end => {
            String::from_utf8_lossy(middle).to_string()
        }
        _ => raw_value
            .split(" #")
            .next()
            .unwrap_or(raw_value)
            .trim()
            .to_string(),
    };
    unquoted
}
