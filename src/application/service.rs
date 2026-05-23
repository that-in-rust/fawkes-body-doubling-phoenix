use std::time::{Duration, Instant};

use crate::application::config::ProbeRunConfig;
use crate::application::traits::{
    CaptureStoreBehavior, ProbeTimeBehavior, ScreenCaptureBehavior, VisionClassifyBehavior,
};
use crate::core::error::ProbeError;
use crate::core::summary::summarize_capture_records;
use crate::core::types::{CaptureRecord, CaptureRecordSeed, RunIdentifier, VisionRequest};

pub struct ProbeRunService<S, V, C, T> {
    screen_capture: S,
    vision_client: V,
    capture_store: C,
    probe_time: T,
}

impl<S, V, C, T> ProbeRunService<S, V, C, T>
where
    S: ScreenCaptureBehavior,
    V: VisionClassifyBehavior,
    C: CaptureStoreBehavior,
    T: ProbeTimeBehavior,
{
    pub fn new(screen_capture: S, vision_client: V, capture_store: C, probe_time: T) -> Self {
        Self {
            screen_capture,
            vision_client,
            capture_store,
            probe_time,
        }
    }

    pub fn run_serial_probe_cycle(
        &self,
        config: &ProbeRunConfig,
    ) -> Result<crate::core::types::RunSummary, ProbeError> {
        self.capture_store.ensure_store_ready()?;
        self.screen_capture.preflight_capture_ready()?;
        let run_id = RunIdentifier::generate_now_identifier();

        for attempt_index in 0..config.count().as_u32() {
            let captured_at = self.probe_time.current_time_utc();
            let frame = self.screen_capture.capture_active_screen_once()?;
            let archived_capture = self.capture_store.archive_probe_capture(
                &run_id,
                captured_at,
                attempt_index,
                &frame,
            )?;

            let request = VisionRequest {
                goal: config.goal().clone(),
                model: config.model_name().to_owned(),
                jpeg_bytes: archived_capture.jpeg_bytes.clone(),
                app_name: frame.app_name.clone(),
                window_title: frame.window_title.clone(),
            };

            let started_at = Instant::now();
            let classification_result = self.vision_client.classify_remote_focus_frame(&request);
            let record_seed = CaptureRecordSeed {
                run_id: run_id.to_string(),
                captured_at,
                goal: config.goal().as_str().to_owned(),
                screenshot_path: archived_capture.screenshot_path.display().to_string(),
                screenshot_sha256: archived_capture.screenshot_sha256.clone(),
                provider: "openai".to_owned(),
                model: self.vision_client.model_name().to_owned(),
                latency_ms: started_at.elapsed().as_millis(),
                app_name: frame.app_name.clone(),
                window_title: frame.window_title.clone(),
            };

            let record = match classification_result {
                Ok(success) => CaptureRecord::build_success_record(record_seed, success),
                Err(failure) => CaptureRecord::build_failure_record(record_seed, failure),
            };

            self.capture_store.persist_probe_capture_row(&record)?;

            if attempt_index + 1 < config.count().as_u32() {
                self.probe_time
                    .sleep_probe_interval(Duration::from_secs(config.interval().as_u64()));
            }
        }

        let records = self
            .capture_store
            .load_run_capture_rows(&run_id.to_string())?;
        Ok(summarize_capture_records(&run_id.to_string(), &records))
    }
}
