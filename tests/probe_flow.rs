use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use fawkes_probe::adapters::openai::OpenAiVisionClient;
use fawkes_probe::adapters::store::SqliteCaptureStore;
use fawkes_probe::application::config::ProbeRunConfig;
use fawkes_probe::application::service::ProbeRunService;
use fawkes_probe::application::traits::{
    CaptureStoreBehavior, ProbeTimeBehavior, ScreenCaptureBehavior, VisionClassifyBehavior,
};
use fawkes_probe::cli::CliArgs;
use fawkes_probe::core::error::ProbeError;
use fawkes_probe::core::types::{
    CaptureAssessment, ScreenCaptureFrame, TaskStatus, VisionProviderFailure, VisionRequest,
    VisionSuccess,
};
use mockito::{Matcher, Server};
use secrecy::SecretString;
use tempfile::TempDir;

#[derive(Clone)]
struct FakeScreenCapture {
    frame: ScreenCaptureFrame,
}

impl FakeScreenCapture {
    fn new() -> Self {
        let width = 1200;
        let height = 800;
        let rgba_bytes = vec![128; (width * height * 4) as usize];
        Self {
            frame: ScreenCaptureFrame {
                width,
                height,
                rgba_bytes,
                app_name: Some("Visual Studio Code".to_owned()),
                window_title: Some("main.rs".to_owned()),
            },
        }
    }
}

impl ScreenCaptureBehavior for FakeScreenCapture {
    fn preflight_capture_ready(&self) -> Result<(), ProbeError> {
        Ok(())
    }

    fn capture_active_screen_once(&self) -> Result<ScreenCaptureFrame, ProbeError> {
        Ok(self.frame.clone())
    }
}

#[derive(Default, Clone)]
struct FakeProbeTime {
    now_index: Rc<RefCell<i64>>,
    sleep_calls: Rc<RefCell<Vec<Duration>>>,
}

impl ProbeTimeBehavior for FakeProbeTime {
    fn current_time_utc(&self) -> DateTime<Utc> {
        let mut now_index = self.now_index.borrow_mut();
        let current = *now_index;
        *now_index += 30;
        Utc.timestamp_opt(1_747_950_000 + current, 0)
            .single()
            .expect("fixed timestamp")
    }

    fn sleep_probe_interval(&self, duration: Duration) {
        self.sleep_calls.borrow_mut().push(duration);
    }
}

#[derive(Clone)]
struct FakeVisionClient {
    responses: Rc<RefCell<Vec<Result<VisionSuccess, VisionProviderFailure>>>>,
    model_name: String,
}

impl FakeVisionClient {
    fn new(responses: Vec<Result<VisionSuccess, VisionProviderFailure>>) -> Self {
        Self {
            responses: Rc::new(RefCell::new(responses)),
            model_name: "gpt-4.1-mini".to_owned(),
        }
    }
}

impl VisionClassifyBehavior for FakeVisionClient {
    fn classify_remote_focus_frame(
        &self,
        _request: &VisionRequest,
    ) -> Result<VisionSuccess, VisionProviderFailure> {
        self.responses.borrow_mut().remove(0)
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

fn create_config(output_dir: PathBuf, count: u32) -> ProbeRunConfig {
    ProbeRunConfig::parse_probe_config_with_key(
        CliArgs {
            goal: "study Rust".to_owned(),
            interval: 5,
            count,
            model: "gpt-4.1-mini".to_owned(),
            output_dir,
        },
        Some("test-key".to_owned()),
    )
    .expect("valid config")
}

#[test]
fn test_req_rust_002_fake_capture_is_archived_once_and_downscaled() {
    let temp_dir = TempDir::new().expect("tempdir");
    let store =
        SqliteCaptureStore::open_local_probe_store(temp_dir.path().to_path_buf()).expect("store");
    store.ensure_store_ready().expect("ready");

    let frame = FakeScreenCapture::new()
        .capture_active_screen_once()
        .expect("frame");
    let run_id = fawkes_probe::core::types::RunIdentifier::generate_now_identifier();
    let archived = store
        .archive_probe_capture(&run_id, Utc::now(), 0, &frame)
        .expect("archived");

    assert!(archived.screenshot_path.exists());
    assert!(archived.jpeg_bytes.len() < frame.rgba_bytes.len());

    let image = image::load_from_memory(&archived.jpeg_bytes).expect("jpeg");
    assert!(image.width().min(image.height()) <= 768);
}

#[test]
fn test_req_rust_003_openai_request_uses_responses_shape_and_schema() {
    let mut server = Server::new();
    let mock = server
        .mock("POST", "/v1/responses")
        .match_header("authorization", "Bearer test-key")
        .match_body(Matcher::PartialJson(serde_json::json!({
            "model": "gpt-4.1-mini",
            "store": false,
            "text": {
                "format": {
                    "type": "json_schema",
                    "name": "capture_assessment",
                    "strict": true
                }
            }
        })))
        .with_status(200)
        .with_body(
            serde_json::json!({
                "output": [{
                    "type": "message",
                    "content": [{
                        "type": "output_text",
                        "text": "{\"activity_category\":\"coding\",\"task_status\":\"on_task\",\"confidence\":0.9,\"reason\":\"Rust code is visible.\"}"
                    }]
                }],
                "usage": {
                    "input_tokens": 100,
                    "output_tokens": 20,
                    "total_tokens": 120
                }
            })
            .to_string(),
        )
        .create();

    let client = OpenAiVisionClient::new(
        SecretString::new("test-key".to_owned().into_boxed_str()),
        "gpt-4.1-mini".to_owned(),
        Some(format!("{}/v1/responses", server.url())),
    )
    .expect("client");

    let request = VisionRequest {
        goal: fawkes_probe::core::types::ProbeGoal::try_new("study Rust").expect("goal"),
        model: "gpt-4.1-mini".to_owned(),
        jpeg_bytes: vec![0_u8; 16],
        app_name: Some("Visual Studio Code".to_owned()),
        window_title: Some("main.rs".to_owned()),
    };

    let success = client
        .classify_remote_focus_frame(&request)
        .expect("success");

    mock.assert();
    assert_eq!(success.assessment.task_status, TaskStatus::OnTask);
    assert_eq!(success.total_tokens, Some(120));
}

#[test]
fn test_req_rust_004_sqlite_rows_and_summary_match_attempts() {
    let temp_dir = TempDir::new().expect("tempdir");
    let config = create_config(temp_dir.path().to_path_buf(), 2);
    let store =
        SqliteCaptureStore::open_local_probe_store(temp_dir.path().to_path_buf()).expect("store");
    let service = ProbeRunService::new(
        FakeScreenCapture::new(),
        FakeVisionClient::new(vec![
            Ok(VisionSuccess {
                assessment: CaptureAssessment {
                    activity_category: fawkes_probe::core::types::ActivityCategory::Coding,
                    task_status: TaskStatus::OnTask,
                    confidence: 0.92,
                    reason: "Looks like coding".to_owned(),
                },
                raw_response_json: "{}".to_owned(),
                input_tokens: Some(11),
                output_tokens: Some(9),
                total_tokens: Some(20),
            }),
            Ok(VisionSuccess {
                assessment: CaptureAssessment {
                    activity_category: fawkes_probe::core::types::ActivityCategory::Browsing,
                    task_status: TaskStatus::OffTask,
                    confidence: 0.71,
                    reason: "Looks like unrelated browsing".to_owned(),
                },
                raw_response_json: "{}".to_owned(),
                input_tokens: Some(10),
                output_tokens: Some(8),
                total_tokens: Some(18),
            }),
        ]),
        store,
        FakeProbeTime::default(),
    );

    let summary = service.run_serial_probe_cycle(&config).expect("summary");
    assert_eq!(summary.on_task_count, 1);
    assert_eq!(summary.off_task_count, 1);
    assert_eq!(summary.error_count, 0);
    assert_eq!(summary.lines.len(), 2);
}

#[test]
fn test_req_rust_005_provider_failures_persist_and_loop_continues() {
    let temp_dir = TempDir::new().expect("tempdir");
    let config = create_config(temp_dir.path().to_path_buf(), 2);
    let store =
        SqliteCaptureStore::open_local_probe_store(temp_dir.path().to_path_buf()).expect("store");
    let service = ProbeRunService::new(
        FakeScreenCapture::new(),
        FakeVisionClient::new(vec![
            Err(VisionProviderFailure {
                error_message: "provider http failure: 429 Too Many Requests".to_owned(),
                status_code: Some(429),
                raw_body: Some("{\"error\":\"rate limited\"}".to_owned()),
            }),
            Err(VisionProviderFailure {
                error_message: "structured output parsing failed: missing field".to_owned(),
                status_code: None,
                raw_body: Some("{\"broken\":true}".to_owned()),
            }),
        ]),
        store,
        FakeProbeTime::default(),
    );

    let summary = service.run_serial_probe_cycle(&config).expect("summary");
    assert_eq!(summary.error_count, 2);
    assert!(summary.render_terminal_summary_report().contains("error"));
}

#[test]
fn test_req_rust_004_token_telemetry_nullability_is_preserved() {
    let success = VisionSuccess {
        assessment: CaptureAssessment {
            activity_category: fawkes_probe::core::types::ActivityCategory::Coding,
            task_status: TaskStatus::OnTask,
            confidence: 0.95,
            reason: "Coding".to_owned(),
        },
        raw_response_json: "{}".to_owned(),
        input_tokens: None,
        output_tokens: Some(4),
        total_tokens: None,
    };

    assert_eq!(success.input_tokens, None);
    assert_eq!(success.output_tokens, Some(4));
    assert_eq!(success.total_tokens, None);
}

#[test]
fn test_req_rust_002_stubbed_preprocessing_stays_fast() {
    let temp_dir = TempDir::new().expect("tempdir");
    let store =
        SqliteCaptureStore::open_local_probe_store(temp_dir.path().to_path_buf()).expect("store");
    store.ensure_store_ready().expect("ready");
    let frame = FakeScreenCapture::new()
        .capture_active_screen_once()
        .expect("frame");
    let run_id = fawkes_probe::core::types::RunIdentifier::generate_now_identifier();

    let mut samples = Vec::new();
    for attempt in 0..10 {
        let started_at = std::time::Instant::now();
        let _ = store
            .archive_probe_capture(&run_id, Utc::now(), attempt, &frame)
            .expect("archive");
        samples.push(started_at.elapsed().as_millis());
    }
    samples.sort_unstable();
    let median = samples[samples.len() / 2];
    assert!(median <= 1000, "median preprocessing too high: {median}ms");
}
