use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProbeError {
    #[error("OPENAI_API_KEY is missing. Set it in your shell before running the probe.")]
    MissingOpenAiApiKey,

    #[error("goal must not be empty")]
    EmptyProbeGoal,

    #[error("interval must be at least 5 seconds")]
    InvalidProbeInterval,

    #[error("count must be at least 1")]
    InvalidProbeAttemptCount,

    #[error(
        "screen capture preflight failed. Check macOS Screen Recording permission and try again: {0}"
    )]
    ScreenCapturePreflight(String),

    #[error("screen capture failed: {0}")]
    ScreenCaptureFailure(String),

    #[error("image processing failed: {0}")]
    ImageProcessing(String),

    #[error("artifact I/O failed at {path}: {source}")]
    ArtifactIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("sqlite store failed: {0}")]
    SqliteStore(#[from] rusqlite::Error),

    #[error("json serialization or parsing failed: {0}")]
    JsonFailure(#[from] serde_json::Error),

    #[error("http client failed: {0}")]
    HttpFailure(#[from] reqwest::Error),

    #[error("provider responded with http {status_code}: {message}")]
    ProviderHttp {
        status_code: u16,
        message: String,
        raw_body: Option<String>,
    },

    #[error("provider output did not match the required schema: {0}")]
    ProviderSchema(String),

    #[error("provider refused or returned no structured output: {0}")]
    ProviderOutputMissing(String),
}
