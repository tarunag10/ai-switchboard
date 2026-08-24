//! Fallible Tauri adapter from the shared runtime clock to Workbench UTC time.

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Utc};
use switchboard_runtime::RuntimeClock;

pub(crate) fn utc_from_runtime_clock<C>(clock: &C) -> Result<DateTime<Utc>>
where
    C: RuntimeClock + ?Sized,
{
    let unix_millis = clock.try_unix_millis()?;
    if unix_millis < 0 {
        bail!("Workbench runtime clock must not precede the Unix epoch");
    }
    DateTime::<Utc>::from_timestamp_millis(unix_millis)
        .ok_or_else(|| anyhow!("Workbench runtime clock is outside the supported range"))
}
