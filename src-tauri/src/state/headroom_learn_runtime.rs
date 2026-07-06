use std::path::Path;

use chrono::{DateTime, Utc};

use crate::claude_sessions::project_display_name;
use crate::models::{HeadroomLearnPrereqStatus, HeadroomLearnStatus};

use super::AppState;

#[derive(Debug, Clone)]
pub(crate) struct HeadroomLearnRuntimeState {
    running: bool,
    project_path: Option<String>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    success: Option<bool>,
    summary: String,
    error: Option<String>,
    output_tail: Vec<String>,
}

impl Default for HeadroomLearnRuntimeState {
    fn default() -> Self {
        Self {
            running: false,
            project_path: None,
            started_at: None,
            finished_at: None,
            success: None,
            summary: "Select a project to run headroom learn.".into(),
            error: None,
            output_tail: Vec::new(),
        }
    }
}

impl AppState {
    pub fn headroom_learn_prereq_status(&self) -> HeadroomLearnPrereqStatus {
        if let Some(cached) = self.cached_headroom_learn_prereq.lock().clone() {
            return cached;
        }
        let status = crate::learning_commands::detect_headroom_learn_prereq_status();
        *self.cached_headroom_learn_prereq.lock() = Some(status.clone());
        status
    }

    pub fn invalidate_headroom_learn_prereq_cache(&self) {
        *self.cached_headroom_learn_prereq.lock() = None;
    }

    pub fn begin_headroom_learn_run(&self, project_path: &str) -> Result<(), String> {
        if project_path.trim().is_empty() {
            return Err("Select a project before running headroom learn.".into());
        }
        if !self.tool_manager.python_runtime_installed() {
            return Err("Install Headroom runtime before running headroom learn.".into());
        }
        if !self.tool_manager.headroom_entrypoint().exists() {
            return Err("Headroom runtime is not available yet.".into());
        }
        let project = Path::new(project_path);
        if !project.exists() {
            return Err(format!(
                "Project path does not exist: {}",
                project.display()
            ));
        }
        if !project.is_dir() {
            return Err(format!(
                "Project path is not a directory: {}",
                project.display()
            ));
        }

        let mut state = self.headroom_learn_state.lock();
        if state.running {
            return Err("headroom learn is already running.".into());
        }

        state.running = true;
        state.project_path = Some(project_path.to_string());
        state.started_at = Some(Utc::now());
        state.finished_at = None;
        state.success = None;
        state.summary = format!(
            "Running headroom learn for {}.",
            project
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(project_path)
        );
        state.error = None;
        state.output_tail = Vec::new();
        Ok(())
    }

    pub fn complete_headroom_learn_run(
        &self,
        success: bool,
        summary: String,
        error: Option<String>,
        output_tail: Vec<String>,
    ) {
        let mut state = self.headroom_learn_state.lock();
        state.running = false;
        state.finished_at = Some(Utc::now());
        state.success = Some(success);
        state.summary = summary;
        state.error = error;
        state.output_tail = output_tail;
        drop(state);
        // A completed run rewrites CLAUDE.md / MEMORY.md and updates the learn
        // log's mtime, so the cached project list (which depends on both) is
        // now stale. Force a fresh scan on the next read.
        self.invalidate_claude_code_projects_cache();
    }

    pub fn headroom_learn_status(
        &self,
        selected_project_path: Option<&str>,
    ) -> HeadroomLearnStatus {
        let state = self.headroom_learn_state.lock().clone();

        let current_project_path = state.project_path.clone();
        let lookup_project_path = selected_project_path
            .map(|path| path.to_string())
            .or_else(|| current_project_path.clone());
        let project_display_name = current_project_path.as_deref().map(project_display_name);
        let last_run_at = lookup_project_path
            .as_deref()
            .and_then(|path| self.tool_manager.headroom_learn_last_run_at(path));
        let started_at = state.started_at.map(|value| value.to_rfc3339());
        let finished_at = state.finished_at.map(|value| value.to_rfc3339());
        let elapsed_seconds = if state.running {
            state
                .started_at
                .map(|started| (Utc::now() - started).num_seconds().max(0) as u64)
        } else {
            match (state.started_at, state.finished_at) {
                (Some(started), Some(finished)) => {
                    Some((finished - started).num_seconds().max(0) as u64)
                }
                _ => None,
            }
        };
        let progress_percent = if state.running {
            let elapsed = elapsed_seconds.unwrap_or(0) as f64;
            (8.0 + (1.0 - (-elapsed / 36.0).exp()) * 84.0).round() as u8
        } else if state.finished_at.is_some() {
            100
        } else {
            0
        };

        HeadroomLearnStatus {
            running: state.running,
            project_path: current_project_path,
            project_display_name,
            started_at,
            finished_at,
            elapsed_seconds,
            progress_percent,
            summary: state.summary,
            success: state.success,
            error: state.error,
            last_run_at,
            output_tail: state.output_tail,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::HeadroomLearnRuntimeState;

    #[test]
    fn default_state_prompts_for_project_selection() {
        let state = HeadroomLearnRuntimeState::default();

        assert!(!state.running);
        assert!(state.project_path.is_none());
        assert_eq!(state.summary, "Select a project to run headroom learn.");
        assert!(state.output_tail.is_empty());
    }

    #[test]
    fn elapsed_seconds_uses_finished_time_for_completed_runs() {
        let started = Utc::now() - Duration::seconds(12);
        let finished = started + Duration::seconds(7);
        let state = HeadroomLearnRuntimeState {
            running: false,
            project_path: Some("/tmp/example".into()),
            started_at: Some(started),
            finished_at: Some(finished),
            success: Some(true),
            summary: "done".into(),
            error: None,
            output_tail: Vec::new(),
        };

        let elapsed = match (state.started_at, state.finished_at) {
            (Some(started), Some(finished)) => (finished - started).num_seconds().max(0) as u64,
            _ => 0,
        };

        assert_eq!(elapsed, 7);
    }
}
