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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn hf_cache_grew_returns_true_only_on_growth() {
        // First observation after the cache dir appeared. Empty dir
        // doesn't count as activity (HF created the dir but hasn't
        // started downloading yet).
        assert!(!hf_cache_grew(None, 0));
        // First observation with content — counts as growth.
        assert!(hf_cache_grew(None, 100));
        // Strictly grew.
        assert!(hf_cache_grew(Some(100), 200));
        // Unchanged.
        assert!(!hf_cache_grew(Some(100), 100));
        // Shrunk (HF cache pruning during boot — rare, but the function
        // shouldn't lie and call this growth).
        assert!(!hf_cache_grew(Some(200), 100));
    }

    #[test]
    fn parse_ps_cpu_time_handles_macos_formats() {
        // MM:SS.ss (most common — processes under an hour of CPU)
        assert_eq!(parse_ps_cpu_time("0:00.05"), Some(0));
        assert_eq!(parse_ps_cpu_time("0:42.13"), Some(42));
        assert_eq!(parse_ps_cpu_time("12:34.99"), Some(12 * 60 + 34));
        // HH:MM:SS (longer-lived processes)
        assert_eq!(parse_ps_cpu_time("1:23:45"), Some(3600 + 23 * 60 + 45));
        // D-HH:MM:SS (multi-day uptime)
        assert_eq!(
            parse_ps_cpu_time("2-01:23:45"),
            Some(2 * 86400 + 3600 + 23 * 60 + 45)
        );
        // Whitespace tolerated (ps emits a trailing newline)
        assert_eq!(parse_ps_cpu_time("  0:42.13\n"), Some(42));
        // Bad input returns None rather than panicking.
        assert_eq!(parse_ps_cpu_time(""), None);
        assert_eq!(parse_ps_cpu_time("   "), None);
        assert_eq!(parse_ps_cpu_time("not-a-time"), None);
        assert_eq!(parse_ps_cpu_time("1:2:3:4"), None);
    }

    #[test]
    fn cpu_time_advanced_detects_growth_only() {
        // Strictly grew → activity.
        assert!(cpu_time_advanced(Some(3), Some(5)));
        // First observation with non-zero CPU → activity (process was
        // already burning cycles before we started polling).
        assert!(cpu_time_advanced(None, Some(5)));
        // First observation with zero CPU → not yet doing work.
        assert!(!cpu_time_advanced(None, Some(0)));
        // Unchanged (whole-second resolution; sub-second growth is
        // dropped by the parser, so equal seconds means "no second
        // elapsed of CPU time").
        assert!(!cpu_time_advanced(Some(5), Some(5)));
        // ps stopped reporting (process gone) — not activity.
        assert!(!cpu_time_advanced(Some(5), None));
        // Both None — process never tracked or never observed.
        assert!(!cpu_time_advanced(None, None));
    }

    #[test]
    fn tcp_port_accepts_connection_true_when_listener_bound() {
        use std::net::TcpListener;
        use std::time::Duration;

        // Bind to an ephemeral port; OS picks an unused one.
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
        let addr = listener.local_addr().expect("local_addr");

        assert!(tcp_port_accepts_connection(addr, Duration::from_secs(1)));

        // The listener never accept()s — but the kernel still completes
        // the connect, which is the whole point: an alive-but-busy
        // proxy whose event loop is held still passes this check.
        drop(listener);
    }

    #[test]
    fn tcp_port_accepts_connection_false_when_no_listener() {
        use std::net::{SocketAddr, TcpListener};
        use std::time::Duration;

        // Bind to grab a port, then drop the listener so nothing is
        // listening on it. The OS can hand that freed port to another
        // process between drop() and connect_timeout(), so retry with
        // fresh ephemeral ports until one stays closed long enough to
        // observe. If every attempt across N tries shows accepted, the
        // function is genuinely broken.
        for _ in 0..16 {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
            let addr: SocketAddr = listener.local_addr().expect("local_addr");
            drop(listener);

            if !tcp_port_accepts_connection(addr, Duration::from_millis(200)) {
                return;
            }
        }
        panic!("tcp_port_accepts_connection returned true on 16 freshly-released ephemeral ports");
    }

    #[test]
    fn total_dir_size_bytes_returns_zero_for_missing_path() {
        let missing =
            std::env::temp_dir().join(format!("headroom-no-such-{}", uuid::Uuid::new_v4()));
        assert_eq!(total_dir_size_bytes(&missing, 1000), 0);
    }

    #[test]
    fn total_dir_size_bytes_sums_files_recursively() {
        let id = uuid::Uuid::new_v4();
        let root = std::env::temp_dir().join(format!("headroom-hf-test-{id}"));
        fs::create_dir_all(root.join("subdir/deeper")).expect("mkdir");
        fs::write(root.join("a.bin"), vec![0u8; 100]).expect("write a");
        fs::write(root.join("subdir/b.bin"), vec![0u8; 200]).expect("write b");
        fs::write(root.join("subdir/deeper/c.bin"), vec![0u8; 50]).expect("write c");

        assert_eq!(total_dir_size_bytes(&root, 1000), 350);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn total_dir_size_bytes_skips_symlinks_to_avoid_double_count() {
        // HF hub layout: snapshots/<rev>/<file> is a symlink into blobs/<sha>.
        // Counting both would overstate. We count only real files.
        let id = uuid::Uuid::new_v4();
        let root = std::env::temp_dir().join(format!("headroom-hf-symlink-test-{id}"));
        fs::create_dir_all(root.join("blobs")).expect("mkdir blobs");
        fs::create_dir_all(root.join("snapshots")).expect("mkdir snapshots");
        fs::write(root.join("blobs/file1"), vec![0u8; 500]).expect("write blob");

        let symlink_target = root.join("blobs/file1");
        let symlink_path = root.join("snapshots/file1");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&symlink_target, &symlink_path).expect("symlink");

        // 500 bytes (the blob), not 1000 (blob + symlink content).
        assert_eq!(total_dir_size_bytes(&root, 1000), 500);

        let _ = fs::remove_dir_all(&root);
    }
}
