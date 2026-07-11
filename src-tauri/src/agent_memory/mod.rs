//! Local inspection and explicitly confirmed compaction for agent memory.
//!
//! Durable writes are restricted to Switchboard-owned managed blocks. Raw
//! instruction text never leaves this module through command responses.

mod dedup;
mod discovery;
mod preview;
mod rollback;
mod secret_scan;
mod session;

pub use discovery::{get_snapshot, AgentMemorySnapshot, AgentMemoryTarget};
pub use preview::{preview_compaction, AgentMemoryCompactionPreview};
pub use rollback::{
    apply_compaction, rollback_compaction, AgentMemoryCompactionApplyRequest,
    AgentMemoryCompactionReceipt,
};
pub use session::build_session_manifest;
