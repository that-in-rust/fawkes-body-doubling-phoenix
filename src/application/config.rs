use std::env;
use std::path::{Path, PathBuf};

use secrecy::SecretString;

use crate::cli::CliArgs;
use crate::core::error::ProbeError;
use crate::core::types::{ProbeAttemptCount, ProbeGoal, ProbeIntervalSecs};

const DEFAULT_OPENAI_MODEL: &str = "gpt-4.1-mini";
const DEFAULT_OUTPUT_DIR: &str = ".fawkes_probe";

#[derive(Debug, Clone)]
pub struct ProbeSessionRequest {
    goal_text: String,
    interval_secs: u64,
    attempt_count: u32,
    model_name: String,
    output_dir: PathBuf,
}

impl ProbeSessionRequest {
    pub fn new(
        goal_text: impl Into<String>,
        interval_secs: u64,
        attempt_count: u32,
        model_name: impl Into<String>,
        output_dir: PathBuf,
    ) -> Self {
        Self {
            goal_text: goal_text.into(),
            interval_secs,
            attempt_count,
            model_name: model_name.into(),
            output_dir,
        }
    }

    pub fn build_overlay_request(
        goal_text: impl Into<String>,
        interval_secs: u64,
        attempt_count: u32,
    ) -> Self {
        Self::new(
            goal_text,
            interval_secs,
            attempt_count,
            DEFAULT_OPENAI_MODEL,
            PathBuf::from(DEFAULT_OUTPUT_DIR),
        )
    }
}

#[derive(Debug, Clone)]
pub struct ProbeRunConfig {
    goal: ProbeGoal,
    interval: ProbeIntervalSecs,
    count: ProbeAttemptCount,
    model: String,
    output_dir: PathBuf,
    openai_api_key: SecretString,
}

impl ProbeRunConfig {
    pub fn parse_strict_probe_args(args: CliArgs) -> Result<Self, ProbeError> {
        let openai_api_key =
            env::var("OPENAI_API_KEY").map_err(|_| ProbeError::MissingOpenAiApiKey)?;
        Self::parse_probe_config_with_key(args, Some(openai_api_key))
    }

    pub fn parse_probe_config_with_key(
        args: CliArgs,
        openai_api_key: Option<String>,
    ) -> Result<Self, ProbeError> {
        Self::build_programmatic_config_with_key(
            ProbeSessionRequest::new(
                args.goal,
                args.interval,
                args.count,
                args.model,
                args.output_dir,
            ),
            openai_api_key,
        )
    }

    pub fn build_programmatic_probe_config(
        request: ProbeSessionRequest,
    ) -> Result<Self, ProbeError> {
        let openai_api_key =
            env::var("OPENAI_API_KEY").map_err(|_| ProbeError::MissingOpenAiApiKey)?;
        Self::build_programmatic_config_with_key(request, Some(openai_api_key))
    }

    pub fn build_programmatic_config_with_key(
        request: ProbeSessionRequest,
        openai_api_key: Option<String>,
    ) -> Result<Self, ProbeError> {
        let openai_api_key = openai_api_key.ok_or(ProbeError::MissingOpenAiApiKey)?;
        let goal = ProbeGoal::try_new(&request.goal_text)?;
        let interval = ProbeIntervalSecs::try_new(request.interval_secs)?;
        let count = ProbeAttemptCount::try_new(request.attempt_count)?;

        Ok(Self {
            goal,
            interval,
            count,
            model: request.model_name,
            output_dir: request.output_dir,
            openai_api_key: SecretString::new(openai_api_key.into_boxed_str()),
        })
    }

    pub fn goal(&self) -> &ProbeGoal {
        &self.goal
    }

    pub fn interval(&self) -> ProbeIntervalSecs {
        self.interval
    }

    pub fn count(&self) -> ProbeAttemptCount {
        self.count
    }

    pub fn model_name(&self) -> &str {
        &self.model
    }

    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    pub fn openai_api_key(&self) -> &SecretString {
        &self.openai_api_key
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::cli::CliArgs;
    use crate::core::error::ProbeError;

    use super::{ProbeRunConfig, ProbeSessionRequest};

    fn create_test_cli_args(interval: u64, count: u32) -> CliArgs {
        CliArgs {
            goal: "study Rust".to_owned(),
            interval,
            count,
            model: "gpt-4.1-mini".to_owned(),
            output_dir: PathBuf::from(".fawkes_probe"),
        }
    }

    #[test]
    fn test_req_rust_001_accepts_valid_args_and_key() {
        let config = ProbeRunConfig::parse_probe_config_with_key(
            create_test_cli_args(5, 1),
            Some("test-key".to_owned()),
        )
        .expect("valid config");
        assert_eq!(config.interval().as_u64(), 5);
        assert_eq!(config.count().as_u32(), 1);
        assert_eq!(config.model_name(), "gpt-4.1-mini");
    }

    #[test]
    fn test_req_rust_001_rejects_invalid_args_and_missing_key() {
        let error = ProbeRunConfig::parse_probe_config_with_key(create_test_cli_args(1, 0), None)
            .expect_err("missing key should fail first");
        assert!(matches!(error, ProbeError::MissingOpenAiApiKey));

        let error = ProbeRunConfig::parse_probe_config_with_key(
            create_test_cli_args(4, 1),
            Some("test-key".to_owned()),
        )
        .expect_err("interval below minimum should fail");
        assert!(matches!(error, ProbeError::InvalidProbeInterval));

        let error = ProbeRunConfig::parse_probe_config_with_key(
            create_test_cli_args(5, 0),
            Some("test-key".to_owned()),
        )
        .expect_err("count below minimum should fail");
        assert!(matches!(error, ProbeError::InvalidProbeAttemptCount));
    }

    #[test]
    fn test_req_rust_104_builds_programmatic_probe_config() {
        let config = ProbeRunConfig::build_programmatic_config_with_key(
            ProbeSessionRequest::build_overlay_request("study Rust", 10, 3),
            Some("test-key".to_owned()),
        )
        .expect("programmatic config should build");

        assert_eq!(config.goal().as_str(), "study Rust");
        assert_eq!(config.interval().as_u64(), 10);
        assert_eq!(config.count().as_u32(), 3);
        assert_eq!(config.model_name(), "gpt-4.1-mini");
        assert_eq!(config.output_dir(), Path::new(".fawkes_probe"));
    }
}
