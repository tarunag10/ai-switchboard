//! Compatibility facade for Workbench sessions owned by `switchboard-core`.

pub(crate) use switchboard_core::workbench::{deterministic_fork_session_id, validate_digest};
pub use switchboard_core::workbench::{CreateWorkbenchSessionInput, WorkbenchSession};
