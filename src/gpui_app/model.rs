use chrono::{DateTime, Local};

use crate::application::config::ProbeSessionRequest;
use crate::core::summary::format_plain_summary_sentence;
use crate::core::types::{AttemptSummaryLine, RunSummary, TaskStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayFormFields {
    pub task_text: String,
    pub count_text: String,
}

impl Default for OverlayFormFields {
    fn default() -> Self {
        Self {
            task_text: String::new(),
            count_text: "6".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayAttemptDescription {
    pub timestamp_text: String,
    pub description_text: String,
    pub verdict_text: String,
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
    pub attempt_descriptions: Vec<OverlayAttemptDescription>,
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
            attempt_descriptions: build_overlay_attempt_descriptions(&summary.lines),
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
        attempt_count,
    ))
}

pub fn build_overlay_attempt_descriptions(
    lines: &[AttemptSummaryLine],
) -> Vec<OverlayAttemptDescription> {
    lines
        .iter()
        .map(|line| OverlayAttemptDescription {
            timestamp_text: format_attempt_timestamp_text(&line.captured_at),
            description_text: format_attempt_reason_text(line),
            verdict_text: format_attempt_verdict_text(line).to_owned(),
        })
        .collect()
}

pub fn format_attempt_display_line(description: &OverlayAttemptDescription) -> String {
    format!(
        "{} - {} - {}",
        description.timestamp_text, description.description_text, description.verdict_text
    )
}

fn format_attempt_timestamp_text(captured_at: &str) -> String {
    DateTime::parse_from_rfc3339(captured_at)
        .map(|timestamp| {
            timestamp
                .with_timezone(&Local)
                .format("%H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|_| captured_at.to_owned())
}

fn format_attempt_reason_text(line: &AttemptSummaryLine) -> String {
    line.reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            line.error
                .as_deref()
                .map(str::trim)
                .filter(|error| !error.is_empty())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| "No concise description returned".to_owned())
}

fn format_attempt_verdict_text(line: &AttemptSummaryLine) -> &'static str {
    if line.error.is_some() {
        return "Error";
    }

    match line.task_status {
        Some(TaskStatus::OnTask) => "On task",
        Some(TaskStatus::OffTask) => "Off task",
        Some(TaskStatus::Ambiguous) => "Ambiguous",
        None => "Error",
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::application::config::{OVERLAY_CAPTURE_INTERVAL_SECS, ProbeRunConfig};
    use crate::core::types::{ActivityCategory, AttemptSummaryLine, RunSummary, TaskStatus};

    use super::{
        OverlayFormFields, OverlaySessionState, OverlayViewModel,
        build_overlay_attempt_descriptions, format_attempt_display_line,
        validate_session_form_fields,
    };

    #[test]
    fn test_req_rust_202_overlay_defaults_match_fixed_form() {
        let defaults = OverlayFormFields::default();
        assert!(defaults.task_text.is_empty());
        assert_eq!(defaults.count_text, "6");
    }

    #[test]
    fn test_req_rust_203_validates_overlay_fields_inline() {
        let error = validate_session_form_fields(&OverlayFormFields::default())
            .expect_err("empty task should fail");
        assert_eq!(error, "Task must not be empty.");

        let error = validate_session_form_fields(&OverlayFormFields {
            task_text: "study Rust".to_owned(),
            count_text: "many".to_owned(),
        })
        .expect_err("non numeric count should fail");
        assert_eq!(error, "Count must be a whole number.");

        let error = validate_session_form_fields(&OverlayFormFields {
            task_text: "study Rust".to_owned(),
            count_text: "0".to_owned(),
        })
        .expect_err("count below one should fail");
        assert_eq!(error, "Count must be at least 1.");

        let request = validate_session_form_fields(&OverlayFormFields {
            task_text: "study Rust".to_owned(),
            count_text: "3".to_owned(),
        })
        .expect("valid overlay request");
        let config = ProbeRunConfig::build_programmatic_config_with_key(
            request,
            Some("test-key".to_owned()),
        )
        .expect("fixed cadence config");
        assert_eq!(config.interval().as_u64(), OVERLAY_CAPTURE_INTERVAL_SECS);
        assert_eq!(config.count().as_u32(), 3);
    }

    #[test]
    fn test_req_rust_205_tracks_running_completed_and_attempt_descriptions() {
        let mut model = OverlayViewModel::default();
        assert!(!model.is_running_session());

        model.mark_running_session();
        assert!(model.is_running_session());

        let summary_line = AttemptSummaryLine {
            captured_at: Utc::now().to_rfc3339(),
            task_status: Some(TaskStatus::OnTask),
            activity_category: Some(ActivityCategory::Coding),
            confidence: Some(0.92),
            reason: Some("Rust code editing is visible".to_owned()),
            latency_ms: Some(1234),
            error: None,
        };

        model.record_completed_session(&RunSummary {
            run_id: "run-123".to_owned(),
            lines: vec![summary_line],
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
                assert_eq!(summary.attempt_descriptions.len(), 1);
            }
            other => panic!("unexpected session state: {other:?}"),
        }
    }

    #[test]
    fn test_req_rust_206_formats_attempt_description_lines() {
        let descriptions = build_overlay_attempt_descriptions(&[
            AttemptSummaryLine {
                captured_at: "2026-05-23T09:52:18+00:00".to_owned(),
                task_status: Some(TaskStatus::OffTask),
                activity_category: Some(ActivityCategory::Browsing),
                confidence: Some(0.8),
                reason: Some("Browsing unrelated videos".to_owned()),
                latency_ms: Some(1500),
                error: None,
            },
            AttemptSummaryLine {
                captured_at: "not-a-timestamp".to_owned(),
                task_status: None,
                activity_category: None,
                confidence: None,
                reason: None,
                latency_ms: Some(1800),
                error: Some("provider http failure: 429 Too Many Requests".to_owned()),
            },
            AttemptSummaryLine {
                captured_at: "2026-05-23T09:52:48+00:00".to_owned(),
                task_status: Some(TaskStatus::Ambiguous),
                activity_category: Some(ActivityCategory::Unknown),
                confidence: Some(0.4),
                reason: None,
                latency_ms: Some(1700),
                error: None,
            },
        ]);

        assert_eq!(
            descriptions[0].description_text,
            "Browsing unrelated videos"
        );
        assert_eq!(descriptions[0].verdict_text, "Off task");
        assert!(descriptions[0].timestamp_text.contains(':'));

        assert_eq!(descriptions[1].timestamp_text, "not-a-timestamp");
        assert_eq!(
            descriptions[1].description_text,
            "provider http failure: 429 Too Many Requests"
        );
        assert_eq!(descriptions[1].verdict_text, "Error");

        assert_eq!(
            descriptions[2].description_text,
            "No concise description returned"
        );
        assert_eq!(
            format_attempt_display_line(&descriptions[0]),
            format!(
                "{} - Browsing unrelated videos - Off task",
                descriptions[0].timestamp_text
            )
        );
    }
}
