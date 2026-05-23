use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::error::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeGoal(String);

impl ProbeGoal {
    pub fn try_new(input: impl AsRef<str>) -> Result<Self, ProbeError> {
        let trimmed = input.as_ref().trim();
        if trimmed.is_empty() {
            return Err(ProbeError::EmptyProbeGoal);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeIntervalSecs(u64);

impl ProbeIntervalSecs {
    pub fn try_new(seconds: u64) -> Result<Self, ProbeError> {
        if seconds < 5 {
            return Err(ProbeError::InvalidProbeInterval);
        }
        Ok(Self(seconds))
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeAttemptCount(u32);

impl ProbeAttemptCount {
    pub fn try_new(count: u32) -> Result<Self, ProbeError> {
        if count < 1 {
            return Err(ProbeError::InvalidProbeAttemptCount);
        }
        Ok(Self(count))
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunIdentifier(String);

impl RunIdentifier {
    pub fn generate_now_identifier() -> Self {
        Self(Uuid::now_v7().to_string())
    }
}

impl Display for RunIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    OnTask,
    OffTask,
    Ambiguous,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OnTask => "on_task",
            Self::OffTask => "off_task",
            Self::Ambiguous => "ambiguous",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityCategory {
    Coding,
    Studying,
    Reading,
    Writing,
    Browsing,
    SocialMedia,
    Video,
    Gaming,
    Email,
    Other,
    Unknown,
}

impl ActivityCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Coding => "coding",
            Self::Studying => "studying",
            Self::Reading => "reading",
            Self::Writing => "writing",
            Self::Browsing => "browsing",
            Self::SocialMedia => "social_media",
            Self::Video => "video",
            Self::Gaming => "gaming",
            Self::Email => "email",
            Self::Other => "other",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureAssessment {
    pub activity_category: ActivityCategory,
    pub task_status: TaskStatus,
    pub confidence: f64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisionPromptInput<'a> {
    pub goal: &'a ProbeGoal,
    pub app_name: Option<&'a str>,
    pub window_title: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenCaptureFrame {
    pub width: u32,
    pub height: u32,
    pub rgba_bytes: Vec<u8>,
    pub app_name: Option<String>,
    pub window_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedCapture {
    pub screenshot_path: PathBuf,
    pub screenshot_sha256: String,
    pub jpeg_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisionRequest {
    pub goal: ProbeGoal,
    pub model: String,
    pub jpeg_bytes: Vec<u8>,
    pub app_name: Option<String>,
    pub window_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisionSuccess {
    pub assessment: CaptureAssessment,
    pub raw_response_json: String,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisionProviderFailure {
    pub error_message: String,
    pub status_code: Option<u16>,
    pub raw_body: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureRecord {
    pub run_id: String,
    pub captured_at: DateTime<Utc>,
    pub goal: String,
    pub screenshot_path: String,
    pub screenshot_sha256: String,
    pub provider: String,
    pub model: String,
    pub activity_category: Option<ActivityCategory>,
    pub task_status: Option<TaskStatus>,
    pub confidence: Option<f64>,
    pub reason: Option<String>,
    pub latency_ms: Option<u128>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub estimated_cost_usd: Option<f64>,
    pub raw_response_json: Option<String>,
    pub error: Option<String>,
    pub app_name: Option<String>,
    pub window_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureRecordSeed {
    pub run_id: String,
    pub captured_at: DateTime<Utc>,
    pub goal: String,
    pub screenshot_path: String,
    pub screenshot_sha256: String,
    pub provider: String,
    pub model: String,
    pub latency_ms: u128,
    pub app_name: Option<String>,
    pub window_title: Option<String>,
}

impl CaptureRecord {
    pub fn build_success_record(seed: CaptureRecordSeed, success: VisionSuccess) -> Self {
        Self {
            run_id: seed.run_id,
            captured_at: seed.captured_at,
            goal: seed.goal,
            screenshot_path: seed.screenshot_path,
            screenshot_sha256: seed.screenshot_sha256,
            provider: seed.provider,
            model: seed.model,
            activity_category: Some(success.assessment.activity_category),
            task_status: Some(success.assessment.task_status),
            confidence: Some(success.assessment.confidence),
            reason: Some(success.assessment.reason),
            latency_ms: Some(seed.latency_ms),
            input_tokens: success.input_tokens,
            output_tokens: success.output_tokens,
            total_tokens: success.total_tokens,
            estimated_cost_usd: None,
            raw_response_json: Some(success.raw_response_json),
            error: None,
            app_name: seed.app_name,
            window_title: seed.window_title,
        }
    }

    pub fn build_failure_record(seed: CaptureRecordSeed, failure: VisionProviderFailure) -> Self {
        let raw_response_json = failure.raw_body.as_ref().map(|body| {
            serde_json::json!({
                "http_status_code": failure.status_code,
                "raw_body": body,
            })
            .to_string()
        });

        Self {
            run_id: seed.run_id,
            captured_at: seed.captured_at,
            goal: seed.goal,
            screenshot_path: seed.screenshot_path,
            screenshot_sha256: seed.screenshot_sha256,
            provider: seed.provider,
            model: seed.model,
            activity_category: None,
            task_status: None,
            confidence: None,
            reason: None,
            latency_ms: Some(seed.latency_ms),
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            estimated_cost_usd: None,
            raw_response_json,
            error: Some(failure.error_message),
            app_name: seed.app_name,
            window_title: seed.window_title,
        }
    }

    #[cfg(test)]
    pub fn test_record(
        captured_at: DateTime<Utc>,
        task_status: Option<TaskStatus>,
        error: Option<String>,
    ) -> Self {
        Self {
            run_id: "test-run".to_owned(),
            captured_at,
            goal: "study Rust".to_owned(),
            screenshot_path: "/tmp/test.jpg".to_owned(),
            screenshot_sha256: "abc123".to_owned(),
            provider: "openai".to_owned(),
            model: "gpt-4.1-mini".to_owned(),
            activity_category: Some(ActivityCategory::Coding),
            task_status,
            confidence: Some(0.91),
            reason: Some("Looks like coding".to_owned()),
            latency_ms: Some(1200),
            input_tokens: Some(12),
            output_tokens: Some(7),
            total_tokens: Some(19),
            estimated_cost_usd: None,
            raw_response_json: Some("{}".to_owned()),
            error,
            app_name: None,
            window_title: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttemptSummaryLine {
    pub captured_at: String,
    pub task_status: Option<TaskStatus>,
    pub activity_category: Option<ActivityCategory>,
    pub confidence: Option<f64>,
    pub reason: Option<String>,
    pub latency_ms: Option<u128>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunSummary {
    pub run_id: String,
    pub lines: Vec<AttemptSummaryLine>,
    pub on_task_count: usize,
    pub off_task_count: usize,
    pub ambiguous_count: usize,
    pub error_count: usize,
    pub average_latency_ms: Option<u128>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone)]
pub struct OpenAiRuntimeConfig {
    api_key: SecretString,
    model_name: String,
}

impl OpenAiRuntimeConfig {
    pub fn new(api_key: SecretString, model_name: impl Into<String>) -> Self {
        Self {
            api_key,
            model_name: model_name.into(),
        }
    }

    pub fn api_key(&self) -> &SecretString {
        &self.api_key
    }

    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    pub fn exposed_api_key(&self) -> &str {
        self.api_key.expose_secret()
    }
}
