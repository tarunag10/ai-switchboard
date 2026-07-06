use crate::models::BootstrapProgress;
use crate::tool_manager::BootstrapStepUpdate;

use super::AppState;

impl AppState {
    pub fn bootstrap_progress(&self) -> BootstrapProgress {
        self.bootstrap_progress.lock().clone()
    }

    pub fn begin_bootstrap(&self) -> Result<(), String> {
        let python_installed = self.tool_manager.python_runtime_installed();
        let mut progress = self.bootstrap_progress.lock();
        let (next, result) = begin_bootstrap_transition(&progress, python_installed);
        *progress = next;
        result
    }

    pub fn update_bootstrap_step(&self, step: BootstrapStepUpdate) {
        let mut progress = self.bootstrap_progress.lock();
        *progress = apply_bootstrap_step(&progress, step);
    }

    pub fn mark_bootstrap_proxy_starting(&self) {
        let mut progress = self.bootstrap_progress.lock();
        *progress = BootstrapProgress {
            running: true,
            complete: false,
            failed: false,
            current_step: "Starting Headroom engine".into(),
            message:
                "Starting the Headroom engine for the first time (this can take ~1-2 minutes)…"
                    .into(),
            current_step_eta_seconds: 45,
            overall_percent: 95,
        };
    }

    pub fn mark_bootstrap_complete(&self) {
        let mut progress = self.bootstrap_progress.lock();
        *progress = bootstrap_complete_state();
    }

    pub fn mark_bootstrap_failed<S: Into<String>>(&self, message: S) {
        let mut progress = self.bootstrap_progress.lock();
        *progress = bootstrap_failed_state(&progress, message.into());
    }
}

fn begin_bootstrap_transition(
    current: &BootstrapProgress,
    python_installed: bool,
) -> (BootstrapProgress, Result<(), String>) {
    if python_installed {
        return (
            BootstrapProgress {
                running: false,
                complete: true,
                failed: false,
                current_step: "Install complete".into(),
                message: "Managed runtime already installed.".into(),
                current_step_eta_seconds: 0,
                overall_percent: 100,
            },
            Ok(()),
        );
    }
    if current.running {
        return (current.clone(), Err("Bootstrap is already running.".into()));
    }
    (
        BootstrapProgress {
            running: true,
            complete: false,
            failed: false,
            current_step: "Preparing install".into(),
            message: "Initializing installer workflow.".into(),
            current_step_eta_seconds: 3,
            overall_percent: 2,
        },
        Ok(()),
    )
}

fn apply_bootstrap_step(
    _current: &BootstrapProgress,
    step: BootstrapStepUpdate,
) -> BootstrapProgress {
    BootstrapProgress {
        running: true,
        complete: false,
        failed: false,
        current_step: step.step.into(),
        message: step.message,
        current_step_eta_seconds: step.eta_seconds,
        overall_percent: step.percent,
    }
}

fn bootstrap_complete_state() -> BootstrapProgress {
    BootstrapProgress {
        running: false,
        complete: true,
        failed: false,
        current_step: "Install complete".into(),
        message: "AI Switchboard for Mac is ready.".into(),
        current_step_eta_seconds: 0,
        overall_percent: 100,
    }
}

fn bootstrap_failed_state(current: &BootstrapProgress, message: String) -> BootstrapProgress {
    BootstrapProgress {
        running: false,
        complete: false,
        failed: true,
        current_step: "Install failed".into(),
        message,
        current_step_eta_seconds: 0,
        overall_percent: current.overall_percent.max(1),
    }
}

#[cfg(test)]
mod tests {
    use crate::models::BootstrapProgress;
    use crate::tool_manager::BootstrapStepUpdate;

    use super::{
        apply_bootstrap_step, begin_bootstrap_transition, bootstrap_complete_state,
        bootstrap_failed_state,
    };

    fn idle_progress() -> BootstrapProgress {
        BootstrapProgress {
            running: false,
            complete: false,
            failed: false,
            current_step: String::new(),
            message: String::new(),
            current_step_eta_seconds: 0,
            overall_percent: 0,
        }
    }

    #[test]
    fn begin_bootstrap_skips_install_when_python_already_installed() {
        let (next, result) = begin_bootstrap_transition(&idle_progress(), true);
        assert!(result.is_ok());
        assert!(next.complete);
        assert!(!next.running);
        assert!(!next.failed);
        assert_eq!(next.overall_percent, 100);
    }

    #[test]
    fn begin_bootstrap_starts_when_python_missing() {
        let (next, result) = begin_bootstrap_transition(&idle_progress(), false);
        assert!(result.is_ok());
        assert!(next.running);
        assert!(!next.complete);
        assert!(!next.failed);
        assert_eq!(next.overall_percent, 2);
    }

    #[test]
    fn begin_bootstrap_rejects_reentry_while_running() {
        let running = BootstrapProgress {
            running: true,
            overall_percent: 42,
            ..idle_progress()
        };
        let (next, result) = begin_bootstrap_transition(&running, false);
        assert!(result.is_err());
        // State is preserved when re-entry is rejected.
        assert_eq!(next.overall_percent, 42);
        assert!(next.running);
    }

    #[test]
    fn begin_bootstrap_after_failure_restarts_cleanly() {
        let failed = BootstrapProgress {
            failed: true,
            overall_percent: 50,
            message: "boom".into(),
            ..idle_progress()
        };
        let (next, result) = begin_bootstrap_transition(&failed, false);
        assert!(result.is_ok());
        assert!(next.running);
        assert!(!next.failed);
        assert_eq!(next.overall_percent, 2);
    }

    #[test]
    fn apply_step_normalizes_into_running_state() {
        let failed = BootstrapProgress {
            failed: true,
            ..idle_progress()
        };
        let next = apply_bootstrap_step(
            &failed,
            BootstrapStepUpdate {
                step: "Downloading Python",
                message: "Fetching runtime".into(),
                eta_seconds: 30,
                percent: 40,
            },
        );
        assert!(next.running);
        assert!(!next.failed);
        assert!(!next.complete);
        assert_eq!(next.current_step, "Downloading Python");
        assert_eq!(next.overall_percent, 40);
        assert_eq!(next.current_step_eta_seconds, 30);
    }

    #[test]
    fn complete_state_pins_to_full_progress() {
        let next = bootstrap_complete_state();
        assert!(next.complete);
        assert!(!next.running);
        assert!(!next.failed);
        assert_eq!(next.overall_percent, 100);
    }

    #[test]
    fn failed_state_preserves_current_percent_with_min_of_one() {
        let current = BootstrapProgress {
            running: true,
            overall_percent: 72,
            ..idle_progress()
        };
        let next = bootstrap_failed_state(&current, "download error".into());
        assert!(next.failed);
        assert!(!next.running);
        assert!(!next.complete);
        assert_eq!(next.overall_percent, 72);
        assert_eq!(next.message, "download error");
    }

    #[test]
    fn failed_state_floors_zero_percent_to_one() {
        let next = bootstrap_failed_state(&idle_progress(), "early failure".into());
        assert_eq!(next.overall_percent, 1);
        assert!(next.failed);
    }
}
