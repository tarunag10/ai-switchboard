//! Deterministic, no-process Workbench controller.
//!
//! This module exercises the lifecycle and persistence boundary that a future
//! native executor must satisfy. It never resolves or launches a binary, opens
//! a shell, accepts a command/environment, contacts a provider, or exposes a
//! Tauri command. Stream bytes are classified transiently and discarded; only
//! bounded content-free counters are persisted.

mod controller;
mod receipt;
mod registry;
mod stream;

pub(crate) use controller::WorkbenchFakeProcessController;

#[cfg(test)]
mod tests;
