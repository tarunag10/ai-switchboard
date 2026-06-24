const HEADROOM_LOCAL_ONLY: Option<&str> = option_env!("HEADROOM_LOCAL_ONLY");

pub fn enabled() -> bool {
    std::env::var("HEADROOM_LOCAL_ONLY")
        .ok()
        .as_deref()
        .map(is_truthy)
        .unwrap_or_else(|| HEADROOM_LOCAL_ONLY.map(is_truthy).unwrap_or(false))
}

fn is_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

