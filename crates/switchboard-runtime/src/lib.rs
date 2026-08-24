//! Platform-neutral runtime boundary for Switchboard.
//!
//! Implementations for macOS, Linux, and Windows belong outside this crate.
//! The default capability set is deliberately fail-closed: declaring a
//! capability does not grant permission to exercise it.

use std::time::{SystemTime, UNIX_EPOCH};

use switchboard_core::{ExecutionMode, HarnessStatus, HarnessSurface};

pub const RUNTIME_CONTRACT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeCapabilities {
    pub filesystem: bool,
    pub process_start: bool,
    pub provider_transport: bool,
    pub secret_store: bool,
}

impl RuntimeCapabilities {
    pub const fn fail_closed() -> Self {
        Self {
            filesystem: false,
            process_start: false,
            provider_transport: false,
            secret_store: false,
        }
    }
}

pub trait RuntimeClock: Send + Sync {
    fn unix_millis(&self) -> i64;
}

pub trait RuntimeAdapter: RuntimeClock + Send + Sync {
    fn contract_version(&self) -> u32 {
        RUNTIME_CONTRACT_VERSION
    }

    fn capabilities(&self) -> RuntimeCapabilities;

    fn harness_status(&self, surface: HarnessSurface) -> HarnessStatus {
        let capabilities = self.capabilities();
        HarnessStatus {
            contract_version: switchboard_core::CORE_CONTRACT_VERSION,
            surface,
            execution_mode: ExecutionMode::ObserveOnly,
            provider_traffic_enabled: capabilities.provider_transport,
            process_start_enabled: capabilities.process_start,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PortableRuntime;

impl RuntimeClock for PortableRuntime {
    fn unix_millis(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
            .unwrap_or(0)
    }
}

impl RuntimeAdapter for PortableRuntime {
    fn capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities::fail_closed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_runtime_is_fail_closed() {
        let runtime = PortableRuntime;
        let capabilities = runtime.capabilities();
        assert_eq!(capabilities, RuntimeCapabilities::fail_closed());
        assert!(runtime.unix_millis() > 0);

        let status = runtime.harness_status(HarnessSurface::Workbench);
        assert_eq!(status.execution_mode, ExecutionMode::ObserveOnly);
        assert!(!status.provider_traffic_enabled);
        assert!(!status.process_start_enabled);
    }
}
