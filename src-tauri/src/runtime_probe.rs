use std::process::Command;

pub(crate) fn probe_proxy_livez(client: &reqwest::blocking::Client) -> bool {
    let backend = crate::backend_port::get();
    let urls = [
        format!("http://127.0.0.1:{backend}/livez"),
        format!("http://127.0.0.1:{backend}/health"),
        "http://127.0.0.1:6767/livez".to_string(),
        "http://127.0.0.1:6767/health".to_string(),
    ];
    for url in &urls {
        if client
            .get(url)
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

/// HuggingFace hub cache path — where transformers/huggingface_hub write
/// downloaded model weights. HF respects ``$HF_HOME`` but we set neither
/// in the bundled runtime, so the default ``$HOME/.cache/huggingface/hub``
/// is what we observe. Returns None if we can't resolve a home dir or the
/// path doesn't exist yet (first-run pre-download).
pub(crate) fn hf_hub_cache_dir() -> Option<std::path::PathBuf> {
    let home = dirs::home_dir()?;
    let path = home.join(".cache").join("huggingface").join("hub");
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// Total byte size of every regular file under ``path``. Used as a
/// "is the proxy downloading models right now" signal: HF model
/// downloads land in this tree and grow it monotonically, even when
/// the python process is otherwise quiet (no log writes). Errors
/// during the walk are swallowed — a partial sum is still a useful
/// signal, and a zero sum just means we miss this tick of evidence.
///
/// Bounded by ``max_entries`` to keep cost predictable on a warm
/// cache that already has tens of thousands of files.
pub(crate) fn total_dir_size_bytes(path: &std::path::Path, max_entries: usize) -> u64 {
    let mut total: u64 = 0;
    let mut visited: usize = 0;
    let mut stack: Vec<std::path::PathBuf> = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if visited >= max_entries {
            break;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            visited += 1;
            if visited >= max_entries {
                break;
            }
            let file_type = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if file_type.is_symlink() {
                // HF cache uses symlinks under ``snapshots/`` pointing into
                // ``blobs/``. Counting the blobs is enough; following the
                // symlink would double-count.
                continue;
            }
            if file_type.is_dir() {
                stack.push(entry.path());
            } else if file_type.is_file() {
                if let Ok(meta) = entry.metadata() {
                    total = total.saturating_add(meta.len());
                }
            }
        }
    }
    total
}

/// Whether log mtime advanced since the last poll. Counts the
/// transition None → Some(t) (first observation after the proxy
/// began writing) as advancement; a Some → None transition does not
/// (logs don't disappear during a healthy boot).

pub(crate) fn hf_cache_grew(prev: Option<u64>, current: u64) -> bool {
    match prev {
        Some(p) => current > p,
        None => current > 0,
    }
}

/// Whether the proxy is bound to its loopback port. Activity-only
/// signal — does NOT imply reachability. The kernel still completes
/// `accept()` even when uvicorn's event loop is held by an in-flight
/// upstream call (e.g. a forwarded `POST /v1/messages` retrying
/// against a 429-ing Anthropic), so a successful TCP connect proves
/// the python process is alive and bound, even when no HTTP endpoint
/// (`/livez`, `/health`, `/stats`) answers within the probe window.
/// 1s timeout is enough for a localhost SYN/SYN-ACK and short enough
/// not to dominate the 500ms loop tick if the OS is mid-bind.
pub(crate) fn tcp_port_accepts_connection(
    addr: std::net::SocketAddr,
    timeout: std::time::Duration,
) -> bool {
    std::net::TcpStream::connect_timeout(&addr, timeout).is_ok()
}

/// Probe the proxy's loopback port with a 1s timeout. See
/// [`tcp_port_accepts_connection`] for semantics. The backend port is
/// normally 6768 but may have been switched to a fallback by `backend_port`.
pub(crate) fn proxy_port_accepts_connection() -> bool {
    let addr: std::net::SocketAddr = ([127, 0, 0, 1], crate::backend_port::get()).into();
    tcp_port_accepts_connection(addr, std::time::Duration::from_secs(1))
}

pub(crate) fn intercept_port_accepts_connection() -> bool {
    let addr: std::net::SocketAddr =
        ([127, 0, 0, 1], crate::proxy_intercept::INTERCEPT_PORT).into();
    tcp_port_accepts_connection(addr, std::time::Duration::from_millis(250))
}

/// Parse the `ps -p PID -o time=` accumulated CPU time format.
/// macOS `ps` emits this as `MM:SS.ss`, `HH:MM:SS`, or `D-HH:MM:SS`
/// depending on duration. Returns whole seconds; sub-second precision
/// is dropped (we only care about per-tick advancement, which is
/// always >=1s of CPU work to register).
pub(crate) fn parse_ps_cpu_time(raw: &str) -> Option<u64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (days, rest) = match trimmed.split_once('-') {
        Some((d, r)) => (d.parse::<u64>().ok()?, r),
        None => (0u64, trimmed),
    };
    let parts: Vec<&str> = rest.split(':').collect();
    let (h, m, s_raw) = match parts.as_slice() {
        [h, m, s] => (h.parse::<u64>().ok()?, m.parse::<u64>().ok()?, *s),
        [m, s] => (0u64, m.parse::<u64>().ok()?, *s),
        _ => return None,
    };
    // Drop fractional seconds.
    let s_whole = s_raw.split('.').next()?.parse::<u64>().ok()?;
    Some(days * 86400 + h * 3600 + m * 60 + s_whole)
}

/// Read accumulated CPU time (seconds) for ``pid`` via macOS `ps`.
/// Returns None if the process is gone or `ps` fails. Cheap enough
/// to call on a 500ms boot-validation tick — fork+exec of a tiny
/// system binary, no I/O beyond the kernel proc table.
pub(crate) fn tracked_process_cpu_time_secs(pid: u32) -> Option<u64> {
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "time="])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_ps_cpu_time(&String::from_utf8_lossy(&output.stdout))
}

/// Whether the tracked process's accumulated CPU time advanced since
/// the previous observation. Catches the "alive but silent" case —
/// e.g. ONNX graph compile, model load, any synchronous CPU-bound
/// work in the proxy's lifespan startup that produces no log writes,
/// no HF cache growth, and doesn't yet bind :6768. Treats the first
/// observation (None → Some(>0)) as growth so a process that's
/// already burned cycles before we started polling counts as active;
/// None → Some(0) is "just spawned, not yet doing work" and is NOT
/// growth (matches `hf_cache_grew` semantics).
pub(crate) fn cpu_time_advanced(prev: Option<u64>, current: Option<u64>) -> bool {
    match (prev, current) {
        (Some(p), Some(c)) => c > p,
        (None, Some(c)) => c > 0,
        _ => false,
    }
}
