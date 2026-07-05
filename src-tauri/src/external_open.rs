use std::path::Path;
use std::process::Command;

pub(crate) fn open_external_link(url: &str) -> Result<(), String> {
    let trimmed = validate_external_link_url(url)?;
    open_target(&trimmed, "External link")
}

pub(crate) fn open_local_path(path: &Path) -> Result<(), String> {
    open_target(
        path.to_str()
            .ok_or_else(|| "Local path is not valid UTF-8.".to_string())?,
        "Local path",
    )
}

fn open_target(target: &str, label: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .arg(target)
            .status()
            .map_err(|err| format!("Could not open {target}: {err}"))?;
        return status_result(label, status);
    }

    #[cfg(target_os = "linux")]
    {
        for opener in ["xdg-open", "gio", "kde-open5", "wslview"] {
            let mut command = Command::new(opener);
            if opener == "gio" {
                command.args(["open", target]);
            } else {
                command.arg(target);
            }
            match command.status() {
                Ok(status) if status.success() => return Ok(()),
                Ok(_) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(format!("Could not open {target} with {opener}: {err}")),
            }
        }
        return Err("No supported opener found. Install xdg-utils or use a direct link.".into());
    }

    #[cfg(target_os = "windows")]
    {
        let status = Command::new("cmd")
            .args(["/C", "start", "", target])
            .status()
            .map_err(|err| format!("Could not open {target}: {err}"))?;
        return status_result(label, status);
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = (target, label);
        Err("Opening links is not supported on this platform.".into())
    }
}

fn status_result(label: &str, status: std::process::ExitStatus) -> Result<(), String> {
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label} opener failed with status {status}."))
    }
}

pub(crate) fn validate_external_link_url(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("External link URL is empty.".into());
    }
    if trimmed.contains('\n') || trimmed.contains('\r') {
        return Err("External link URL cannot contain line breaks.".into());
    }
    if let Some(address) = trimmed.strip_prefix("mailto:") {
        if address.is_empty() || address.contains('?') || address.contains('/') {
            return Err("Only simple mailto links are supported.".into());
        }
        return Ok(trimmed.to_string());
    }

    let parsed =
        reqwest::Url::parse(trimmed).map_err(|_| "External link URL is invalid.".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Only http, https, and mailto links are supported.".into());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("External link URL cannot contain credentials.".into());
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "External link URL must include a host.".to_string())?;
    if is_blocked_external_link_host(host) {
        return Err("External link host is not allowed.".into());
    }
    Ok(trimmed.to_string())
}

fn is_blocked_external_link_host(host: &str) -> bool {
    let normalized = host.trim_end_matches('.').to_ascii_lowercase();
    if normalized == "localhost"
        || normalized == "localhost.localdomain"
        || normalized.ends_with(".localhost")
        || normalized.ends_with(".local")
    {
        return true;
    }
    match normalized.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(ip)) => {
            ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_unspecified()
        }
        Ok(std::net::IpAddr::V6(ip)) => {
            ip.is_loopback() || ip.is_unspecified() || ip.is_unique_local()
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::validate_external_link_url;

    #[test]
    fn external_link_validator_accepts_documented_public_links() {
        assert_eq!(
            validate_external_link_url(" https://github.com/tarunag10/mac-ai-switchboard/issues ",)
                .expect("issues link"),
            "https://github.com/tarunag10/mac-ai-switchboard/issues",
        );
        assert_eq!(
            validate_external_link_url("https://developers.openai.com/codex/cli")
                .expect("codex docs link"),
            "https://developers.openai.com/codex/cli",
        );
        assert_eq!(
            validate_external_link_url("mailto:hello@example.com").expect("mailto"),
            "mailto:hello@example.com",
        );
    }

    #[test]
    fn external_link_validator_rejects_local_and_unsafe_links() {
        for raw in [
            "file:///etc/passwd",
            "http://127.0.0.1:6767/stats",
            "http://localhost:6767",
            "http://10.0.0.1",
            "https://user:pass@example.com",
            "mailto:hello@example.com/path",
            "mailto:hello@example.com?subject=Injected",
            "https://example.com/\nhttps://evil.test/",
        ] {
            assert!(
                validate_external_link_url(raw).is_err(),
                "expected {raw:?} to be rejected",
            );
        }
    }
}
