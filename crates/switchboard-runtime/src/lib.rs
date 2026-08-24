//! Platform-neutral runtime boundary for Switchboard.
//!
//! Implementations for macOS, Linux, and Windows belong outside this crate.
//! The default capability set is deliberately fail-closed: declaring a
//! capability does not grant permission to exercise it.

pub mod executable_search;

use std::fmt;
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

    fn try_unix_millis(&self) -> Result<i64, RuntimeClockError> {
        Ok(self.unix_millis())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeClockError {
    PreEpoch,
    Failed(&'static str),
}

impl fmt::Display for RuntimeClockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeClockError::PreEpoch => f.write_str("system clock is before the Unix epoch"),
            RuntimeClockError::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for RuntimeClockError {}

/// Deterministic runtime clock for tests and contract checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedClock {
    unix_millis: i64,
}

impl FixedClock {
    pub const fn new(unix_millis: i64) -> Self {
        Self {
            unix_millis: if unix_millis < 0 { 0 } else { unix_millis },
        }
    }
}

impl RuntimeClock for FixedClock {
    fn unix_millis(&self) -> i64 {
        self.unix_millis
    }
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

fn checked_unix_millis(time: SystemTime) -> Result<i64, RuntimeClockError> {
    let duration = time
        .duration_since(UNIX_EPOCH)
        .map_err(|_| RuntimeClockError::PreEpoch)?;
    Ok(duration.as_millis().min(i64::MAX as u128) as i64)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PortableRuntime;

impl RuntimeClock for PortableRuntime {
    fn unix_millis(&self) -> i64 {
        self.try_unix_millis().unwrap_or(0)
    }

    fn try_unix_millis(&self) -> Result<i64, RuntimeClockError> {
        checked_unix_millis(SystemTime::now())
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
    use std::time::Duration;

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

    #[test]
    fn fixed_clock_returns_exact_value() {
        let clock = FixedClock::new(1_725_000_123_456);
        assert_eq!(clock.unix_millis(), 1_725_000_123_456);
        assert_eq!(clock.try_unix_millis(), Ok(1_725_000_123_456));
        assert_eq!(
            <FixedClock as RuntimeClock>::unix_millis(&clock),
            1_725_000_123_456
        );
    }

    #[test]
    fn fixed_clock_is_rollback_safe_and_non_negative() {
        let clock = FixedClock::new(-42);
        assert_eq!(clock.unix_millis(), 0);
        assert_eq!(clock.unix_millis(), 0);
        assert_eq!(clock.try_unix_millis(), Ok(0));
        let stable = FixedClock::new(987_654_321);
        assert_eq!(stable.unix_millis(), 987_654_321);
        assert_eq!(stable.unix_millis(), 987_654_321);
        assert_eq!(stable.try_unix_millis(), Ok(987_654_321));
    }

    #[test]
    fn portable_runtime_errors_on_pre_epoch_system_time() {
        let instant = UNIX_EPOCH
            .checked_sub(Duration::from_millis(1))
            .expect("pre-epoch instant");
        assert_eq!(
            checked_unix_millis(instant),
            Err(RuntimeClockError::PreEpoch)
        );
    }

    #[test]
    fn failing_clock_falls_back_to_epoch_zero_for_compatibility() {
        #[derive(Clone, Copy, Debug)]
        struct FailingClock;

        impl RuntimeClock for FailingClock {
            fn unix_millis(&self) -> i64 {
                0
            }

            fn try_unix_millis(&self) -> Result<i64, RuntimeClockError> {
                Err(RuntimeClockError::Failed("clock unavailable"))
            }
        }

        let clock = FailingClock;
        assert_eq!(
            clock.try_unix_millis(),
            Err(RuntimeClockError::Failed("clock unavailable"))
        );
        assert_eq!(clock.unix_millis(), 0);
    }
}
