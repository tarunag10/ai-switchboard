//! Compatibility facade for Workbench event contracts owned by `switchboard-core`.

pub const WORKBENCH_EVENT_SCHEMA_VERSION: u32 =
    switchboard_core::workbench::WORKBENCH_EVENT_SCHEMA_VERSION;
pub const MAX_EVENT_COUNT: usize = switchboard_core::workbench::MAX_EVENT_COUNT;

pub(crate) use switchboard_core::workbench::{
    new_event, transition_status, validate_event, validate_identifier,
};
pub use switchboard_core::workbench::{
    WorkbenchEvent, WorkbenchEventKind, WorkbenchSessionAction, WorkbenchSessionStatus,
};
