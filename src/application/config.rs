use std::env;
use std::path::{Path, PathBuf};

use secrecy::SecretString;

use crate::cli::CliArgs;
use crate::core::error::ProbeError;
use crate::core::types::{ProbeAttemptCount, ProbeGoal, ProbeIntervalSecs};

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
        let openai_api_key = openai_api_key.ok_or(ProbeError::MissingOpenAiApiKey)?;
        let goal = ProbeGoal::try_new(args.goal)?;
        let interval = ProbeIntervalSecs::try_new(args.interval)?;
        let count = ProbeAttemptCount::try_new(args.count)?;

        Ok(Self {
            goal,
            interval,
            count,
            model: args.model,
            output_dir: args.output_dir,
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
    use std::path::PathBuf;

    use crate::cli::CliArgs;
    use crate::core::error::ProbeError;

    use super::ProbeRunConfig;

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
}
