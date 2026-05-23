use crate::adapters::openai::OpenAiVisionClient;
use crate::adapters::screen::XcapScreenCapture;
use crate::adapters::store::SqliteCaptureStore;
use crate::adapters::time::SystemProbeTime;
use crate::application::config::ProbeRunConfig;
use crate::application::service::ProbeRunService;
use crate::application::traits::ScreenCaptureBehavior;
use crate::core::error::ProbeError;
use crate::core::types::RunSummary;

pub trait ProbeSessionLaunchBehavior: Send + Sync {
    fn preflight_probe_session(&self, config: &ProbeRunConfig) -> Result<(), ProbeError>;
    fn launch_blocking_probe_session(
        &self,
        config: ProbeRunConfig,
    ) -> Result<RunSummary, ProbeError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LiveProbeSessionLauncher;

impl ProbeSessionLaunchBehavior for LiveProbeSessionLauncher {
    fn preflight_probe_session(&self, _config: &ProbeRunConfig) -> Result<(), ProbeError> {
        XcapScreenCapture.preflight_capture_ready()
    }

    fn launch_blocking_probe_session(
        &self,
        config: ProbeRunConfig,
    ) -> Result<RunSummary, ProbeError> {
        let store = SqliteCaptureStore::open_local_probe_store(config.output_dir().to_path_buf())?;
        let screen = XcapScreenCapture;
        let vision = OpenAiVisionClient::new(
            config.openai_api_key().clone(),
            config.model_name().to_owned(),
            None,
        )?;
        let time = SystemProbeTime;
        let service = ProbeRunService::new(screen, vision, store, time);
        service.run_serial_probe_cycle(&config)
    }
}
