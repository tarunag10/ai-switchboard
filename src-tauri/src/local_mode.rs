const HEADROOM_LOCAL_ONLY: Option<&str> = option_env!("HEADROOM_LOCAL_ONLY");
const HEADROOM_REMOTE_SERVICES: Option<&str> = option_env!("HEADROOM_REMOTE_SERVICES");
const HEADROOM_BUILD_FLAVOR: Option<&str> = option_env!("HEADROOM_BUILD_FLAVOR");

pub fn enabled() -> bool {
    std::env::var("HEADROOM_LOCAL_ONLY")
        .ok()
        .as_deref()
        .map(is_truthy)
        .unwrap_or_else(|| {
            if HEADROOM_LOCAL_ONLY.map(is_truthy).unwrap_or(false) {
                return true;
            }
            if std::env::var("HEADROOM_BUILD_FLAVOR")
                .ok()
                .as_deref()
                .map(is_local_free)
                .unwrap_or_else(|| HEADROOM_BUILD_FLAVOR.map(is_local_free).unwrap_or(false))
            {
                return true;
            }
            {
                !std::env::var("HEADROOM_REMOTE_SERVICES")
                    .ok()
                    .as_deref()
                    .map(is_truthy)
                    .unwrap_or_else(|| HEADROOM_REMOTE_SERVICES.map(is_truthy).unwrap_or(false))
            }
        })
}

fn is_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn is_local_free(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("local-free")
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    fn with_env(local: Option<&str>, remote: Option<&str>, f: impl FnOnce()) {
        let previous_local = std::env::var_os("HEADROOM_LOCAL_ONLY");
        let previous_remote = std::env::var_os("HEADROOM_REMOTE_SERVICES");
        let previous_flavor = std::env::var_os("HEADROOM_BUILD_FLAVOR");
        match local {
            Some(value) => std::env::set_var("HEADROOM_LOCAL_ONLY", value),
            None => std::env::remove_var("HEADROOM_LOCAL_ONLY"),
        }
        match remote {
            Some(value) => std::env::set_var("HEADROOM_REMOTE_SERVICES", value),
            None => std::env::remove_var("HEADROOM_REMOTE_SERVICES"),
        }
        f();
        match previous_local {
            Some(value) => std::env::set_var("HEADROOM_LOCAL_ONLY", value),
            None => std::env::remove_var("HEADROOM_LOCAL_ONLY"),
        }
        match previous_remote {
            Some(value) => std::env::set_var("HEADROOM_REMOTE_SERVICES", value),
            None => std::env::remove_var("HEADROOM_REMOTE_SERVICES"),
        }
        match previous_flavor {
            Some(value) => std::env::set_var("HEADROOM_BUILD_FLAVOR", value),
            None => std::env::remove_var("HEADROOM_BUILD_FLAVOR"),
        }
    }

    #[test]
    #[serial]
    fn defaults_to_local_only() {
        with_env(None, None, || assert!(super::enabled()));
    }

    #[test]
    #[serial]
    fn explicit_remote_services_disables_local_only() {
        with_env(None, Some("1"), || assert!(!super::enabled()));
    }

    #[test]
    #[serial]
    fn explicit_local_only_wins_over_remote_services() {
        with_env(Some("1"), Some("1"), || assert!(super::enabled()));
    }

    #[test]
    #[serial]
    fn local_free_build_flavor_wins_over_remote_services() {
        with_env(None, Some("1"), || {
            std::env::set_var("HEADROOM_BUILD_FLAVOR", "local-free");
            assert!(super::enabled());
        });
    }
}
