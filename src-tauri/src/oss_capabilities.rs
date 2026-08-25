//! Compatibility facade for the OSS capability registry owned by `switchboard-core`.

pub(crate) use switchboard_core::oss_registry::OssCapabilityRegistry;

pub(crate) fn registry() -> OssCapabilityRegistry {
    switchboard_core::oss_registry::registry()
}
