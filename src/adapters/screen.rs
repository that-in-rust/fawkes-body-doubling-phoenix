use xcap::Monitor;

use crate::application::traits::ScreenCaptureBehavior;
use crate::core::error::ProbeError;
use crate::core::types::ScreenCaptureFrame;

#[derive(Debug, Default, Clone, Copy)]
pub struct XcapScreenCapture;

impl XcapScreenCapture {
    fn find_primary_monitor(&self) -> Result<Monitor, ProbeError> {
        let monitors = Monitor::all()
            .map_err(|error| ProbeError::ScreenCapturePreflight(error.to_string()))?;

        monitors
            .into_iter()
            .find(|monitor| monitor.is_primary().unwrap_or(false))
            .or_else(|| {
                Monitor::all()
                    .ok()
                    .and_then(|mut monitors| monitors.drain(..).next())
            })
            .ok_or_else(|| ProbeError::ScreenCapturePreflight("no active monitor found".to_owned()))
    }
}

impl ScreenCaptureBehavior for XcapScreenCapture {
    fn preflight_capture_ready(&self) -> Result<(), ProbeError> {
        let _ = self.find_primary_monitor()?;
        Ok(())
    }

    fn capture_active_screen_once(&self) -> Result<ScreenCaptureFrame, ProbeError> {
        let monitor = self.find_primary_monitor()?;
        let image = monitor
            .capture_image()
            .map_err(|error| ProbeError::ScreenCaptureFailure(error.to_string()))?;

        Ok(ScreenCaptureFrame {
            width: image.width(),
            height: image.height(),
            rgba_bytes: image.into_vec(),
            app_name: None,
            window_title: None,
        })
    }
}
