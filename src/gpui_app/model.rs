use crate::application::config::ProbeSessionRequest;
use crate::core::summary::format_plain_summary_sentence;
use crate::core::types::RunSummary;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayFormFields {
    pub task_text: String,
    pub interval_text: String,
    pub count_text: String,
}

impl Default for OverlayFormFields {
    fn default() -> Self {
        Self {
            task_text: String::new(),
            interval_text: "30".to_owned(),
            count_text: "6".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlaySessionSummary {
    pub run_id: String,
    pub on_task_count: usize,
    pub off_task_count: usize,
    pub ambiguous_count: usize,
    pub error_count: usize,
    pub average_latency_ms: Option<u128>,
    pub summary_sentence: String,
}

impl OverlaySessionSummary {
    pub fn from_run_summary(summary: &RunSummary) -> Self {
        Self {
            run_id: summary.run_id.clone(),
            on_task_count: summary.on_task_count,
            off_task_count: summary.off_task_count,
            ambiguous_count: summary.ambiguous_count,
            error_count: summary.error_count,
            average_latency_ms: summary.average_latency_ms,
            summary_sentence: format_plain_summary_sentence(summary),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlaySessionState {
    Editing,
    Running,
    Completed(OverlaySessionSummary),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayViewModel {
    pub inline_error: Option<String>,
    pub session_state: OverlaySessionState,
}

impl Default for OverlayViewModel {
    fn default() -> Self {
        Self {
            inline_error: None,
            session_state: OverlaySessionState::Editing,
        }
    }
}

impl OverlayViewModel {
    pub fn record_inline_error(&mut self, message: impl Into<String>) {
        self.inline_error = Some(message.into());
    }

    pub fn mark_running_session(&mut self) {
        self.inline_error = None;
        self.session_state = OverlaySessionState::Running;
    }

    pub fn record_completed_session(&mut self, summary: &RunSummary) {
        self.inline_error = None;
        self.session_state =
            OverlaySessionState::Completed(OverlaySessionSummary::from_run_summary(summary));
    }

    pub fn record_failed_session(&mut self, message: impl Into<String>) {
        self.inline_error = None;
        self.session_state = OverlaySessionState::Failed(message.into());
    }

    pub fn is_running_session(&self) -> bool {
        matches!(self.session_state, OverlaySessionState::Running)
    }
}

pub fn validate_session_form_fields(
    fields: &OverlayFormFields,
) -> Result<ProbeSessionRequest, String> {
    let trimmed_task = fields.task_text.trim();
    if trimmed_task.is_empty() {
        return Err("Task must not be empty.".to_owned());
    }

    let interval_secs = fields
        .interval_text
        .trim()
        .parse::<u64>()
        .map_err(|_| "Interval must be a whole number of seconds.".to_owned())?;
    if interval_secs < 5 {
        return Err("Interval must be at least 5 seconds.".to_owned());
    }

    let attempt_count = fields
        .count_text
        .trim()
        .parse::<u32>()
        .map_err(|_| "Count must be a whole number.".to_owned())?;
    if attempt_count < 1 {
        return Err("Count must be at least 1.".to_owned());
    }

    Ok(ProbeSessionRequest::build_overlay_request(
        trimmed_task,
        interval_secs,
        attempt_count,
    ))
}

#[cfg(test)]
mod tests {
    use crate::core::types::RunSummary;

    use super::{
        OverlayFormFields, OverlaySessionState, OverlayViewModel, validate_session_form_fields,
    };

    #[test]
    fn test_req_rust_103_validates_overlay_fields_inline() {
        let error = validate_session_form_fields(&OverlayFormFields::default())
            .expect_err("empty task should fail");
        assert_eq!(error, "Task must not be empty.");

        let error = validate_session_form_fields(&OverlayFormFields {
            task_text: "study Rust".to_owned(),
            interval_text: "four".to_owned(),
            count_text: "2".to_owned(),
        })
        .expect_err("non numeric interval should fail");
        assert_eq!(error, "Interval must be a whole number of seconds.");

        let error = validate_session_form_fields(&OverlayFormFields {
            task_text: "study Rust".to_owned(),
            interval_text: "5".to_owned(),
            count_text: "0".to_owned(),
        })
        .expect_err("count below one should fail");
        assert_eq!(error, "Count must be at least 1.");
    }

    #[test]
    fn test_req_rust_105_tracks_running_and_completed_states() {
        let mut model = OverlayViewModel::default();
        assert!(!model.is_running_session());

        model.mark_running_session();
        assert!(model.is_running_session());

        model.record_completed_session(&RunSummary {
            run_id: "run-123".to_owned(),
            lines: Vec::new(),
            on_task_count: 2,
            off_task_count: 1,
            ambiguous_count: 0,
            error_count: 0,
            average_latency_ms: Some(1234),
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
        });

        match &model.session_state {
            OverlaySessionState::Completed(summary) => {
                assert_eq!(summary.run_id, "run-123");
                assert_eq!(summary.on_task_count, 2);
            }
            other => panic!("unexpected session state: {other:?}"),
        }
    }
}
