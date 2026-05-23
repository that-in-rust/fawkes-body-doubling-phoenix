use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::core::error::ProbeError;
use crate::core::types::{
    ArchivedCapture, CaptureRecord, RunIdentifier, ScreenCaptureFrame, VisionProviderFailure,
    VisionRequest, VisionSuccess,
};

pub trait ScreenCaptureBehavior {
    fn preflight_capture_ready(&self) -> Result<(), ProbeError>;
    fn capture_active_screen_once(&self) -> Result<ScreenCaptureFrame, ProbeError>;
}

pub trait VisionClassifyBehavior {
    fn classify_remote_focus_frame(
        &self,
        request: &VisionRequest,
    ) -> Result<VisionSuccess, VisionProviderFailure>;

    fn model_name(&self) -> &str;
}

pub trait CaptureStoreBehavior {
    fn ensure_store_ready(&self) -> Result<(), ProbeError>;
    fn archive_probe_capture(
        &self,
        run_id: &RunIdentifier,
        captured_at: DateTime<Utc>,
        attempt_index: u32,
        frame: &ScreenCaptureFrame,
    ) -> Result<ArchivedCapture, ProbeError>;
    fn persist_probe_capture_row(&self, record: &CaptureRecord) -> Result<(), ProbeError>;
    fn load_run_capture_rows(&self, run_id: &str) -> Result<Vec<CaptureRecord>, ProbeError>;
}

pub trait ProbeTimeBehavior {
    fn current_time_utc(&self) -> DateTime<Utc>;
    fn sleep_probe_interval(&self, duration: Duration);
}
