use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, GenericImageView, RgbaImage, imageops::FilterType};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};

use crate::application::traits::CaptureStoreBehavior;
use crate::core::error::ProbeError;
use crate::core::types::{
    ActivityCategory, ArchivedCapture, CaptureRecord, RunIdentifier, ScreenCaptureFrame, TaskStatus,
};

pub struct SqliteCaptureStore {
    output_dir: PathBuf,
    db_path: PathBuf,
}

impl SqliteCaptureStore {
    pub fn open_local_probe_store(output_dir: PathBuf) -> Result<Self, ProbeError> {
        let db_path = output_dir.join("fawkes_probe.sqlite");
        Ok(Self {
            output_dir,
            db_path,
        })
    }

    fn open_connection(&self) -> Result<Connection, ProbeError> {
        Connection::open(&self.db_path).map_err(ProbeError::from)
    }

    fn create_captures_table(&self) -> Result<(), ProbeError> {
        let connection = self.open_connection()?;
        connection.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS captures (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                captured_at TEXT NOT NULL,
                goal TEXT NOT NULL,
                screenshot_path TEXT NOT NULL,
                screenshot_sha256 TEXT NOT NULL,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                activity_category TEXT,
                task_status TEXT,
                confidence REAL,
                reason TEXT,
                latency_ms INTEGER,
                input_tokens INTEGER,
                output_tokens INTEGER,
                total_tokens INTEGER,
                estimated_cost_usd REAL,
                raw_response_json TEXT,
                error TEXT,
                app_name TEXT,
                window_title TEXT
            );
            ",
        )?;
        Ok(())
    }

    fn create_run_capture_dir(&self, run_id: &RunIdentifier) -> Result<PathBuf, ProbeError> {
        let capture_dir = self
            .output_dir
            .join("runs")
            .join(run_id.to_string())
            .join("captures");
        fs::create_dir_all(&capture_dir).map_err(|source| ProbeError::ArtifactIo {
            path: capture_dir.clone(),
            source,
        })?;
        Ok(capture_dir)
    }

    fn resize_small_capture_image(
        &self,
        frame: &ScreenCaptureFrame,
    ) -> Result<DynamicImage, ProbeError> {
        let rgba = RgbaImage::from_raw(frame.width, frame.height, frame.rgba_bytes.clone())
            .ok_or_else(|| {
                ProbeError::ImageProcessing("invalid RGBA frame dimensions".to_owned())
            })?;
        let dynamic = DynamicImage::ImageRgba8(rgba);
        let (width, height) = dynamic.dimensions();
        let short_side = width.min(height);
        if short_side <= 768 {
            return Ok(dynamic);
        }

        let scale = 768.0 / short_side as f32;
        let resized_width = (width as f32 * scale).round() as u32;
        let resized_height = (height as f32 * scale).round() as u32;

        Ok(dynamic.resize_exact(resized_width, resized_height, FilterType::Triangle))
    }

    fn encode_capture_as_jpeg(&self, image: DynamicImage) -> Result<Vec<u8>, ProbeError> {
        let mut jpeg_bytes = Vec::new();
        let rgb = image.to_rgb8();
        let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_bytes, 75);
        encoder
            .encode(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ColorType::Rgb8.into(),
            )
            .map_err(|error| ProbeError::ImageProcessing(error.to_string()))?;
        Ok(jpeg_bytes)
    }

    fn compute_capture_sha256(&self, jpeg_bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(jpeg_bytes);
        format!("{:x}", hasher.finalize())
    }

    fn save_capture_bytes(
        &self,
        capture_dir: &Path,
        captured_at: DateTime<Utc>,
        attempt_index: u32,
        jpeg_bytes: &[u8],
    ) -> Result<PathBuf, ProbeError> {
        let filename = format!(
            "{}-attempt-{:03}.jpg",
            captured_at.format("%Y%m%dT%H%M%S%.3fZ"),
            attempt_index + 1
        );
        let path = capture_dir.join(filename);
        fs::write(&path, jpeg_bytes).map_err(|source| ProbeError::ArtifactIo {
            path: path.clone(),
            source,
        })?;
        Ok(path)
    }
}

impl CaptureStoreBehavior for SqliteCaptureStore {
    fn ensure_store_ready(&self) -> Result<(), ProbeError> {
        fs::create_dir_all(&self.output_dir).map_err(|source| ProbeError::ArtifactIo {
            path: self.output_dir.clone(),
            source,
        })?;
        self.create_captures_table()
    }

    fn archive_probe_capture(
        &self,
        run_id: &RunIdentifier,
        captured_at: DateTime<Utc>,
        attempt_index: u32,
        frame: &ScreenCaptureFrame,
    ) -> Result<ArchivedCapture, ProbeError> {
        let capture_dir = self.create_run_capture_dir(run_id)?;
        let downscaled_image = self.resize_small_capture_image(frame)?;
        let jpeg_bytes = self.encode_capture_as_jpeg(downscaled_image)?;
        let screenshot_sha256 = self.compute_capture_sha256(&jpeg_bytes);
        let screenshot_path =
            self.save_capture_bytes(&capture_dir, captured_at, attempt_index, &jpeg_bytes)?;

        Ok(ArchivedCapture {
            screenshot_path,
            screenshot_sha256,
            jpeg_bytes,
        })
    }

    fn persist_probe_capture_row(&self, record: &CaptureRecord) -> Result<(), ProbeError> {
        let connection = self.open_connection()?;
        connection.execute(
            "
            INSERT INTO captures (
                run_id,
                captured_at,
                goal,
                screenshot_path,
                screenshot_sha256,
                provider,
                model,
                activity_category,
                task_status,
                confidence,
                reason,
                latency_ms,
                input_tokens,
                output_tokens,
                total_tokens,
                estimated_cost_usd,
                raw_response_json,
                error,
                app_name,
                window_title
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
            ",
            params![
                record.run_id,
                record.captured_at.to_rfc3339(),
                record.goal,
                record.screenshot_path,
                record.screenshot_sha256,
                record.provider,
                record.model,
                record.activity_category.map(|value| value.as_str().to_owned()),
                record.task_status.map(|value| value.as_str().to_owned()),
                record.confidence,
                record.reason,
                record.latency_ms.map(|value| value as i64),
                record.input_tokens.map(|value| value as i64),
                record.output_tokens.map(|value| value as i64),
                record.total_tokens.map(|value| value as i64),
                record.estimated_cost_usd,
                record.raw_response_json,
                record.error,
                record.app_name,
                record.window_title,
            ],
        )?;
        Ok(())
    }

    fn load_run_capture_rows(&self, run_id: &str) -> Result<Vec<CaptureRecord>, ProbeError> {
        let connection = self.open_connection()?;
        let mut statement = connection.prepare(
            "
            SELECT
                run_id,
                captured_at,
                goal,
                screenshot_path,
                screenshot_sha256,
                provider,
                model,
                activity_category,
                task_status,
                confidence,
                reason,
                latency_ms,
                input_tokens,
                output_tokens,
                total_tokens,
                estimated_cost_usd,
                raw_response_json,
                error,
                app_name,
                window_title
            FROM captures
            WHERE run_id = ?1
            ORDER BY id ASC
            ",
        )?;

        let rows = statement.query_map([run_id], |row| {
            let activity_category = row
                .get::<_, Option<String>>(7)?
                .as_deref()
                .map(parse_category);
            let task_status = row
                .get::<_, Option<String>>(8)?
                .as_deref()
                .map(parse_status);
            Ok(CaptureRecord {
                run_id: row.get(0)?,
                captured_at: row.get::<_, String>(1)?.parse::<DateTime<Utc>>().map_err(
                    |error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            1,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    },
                )?,
                goal: row.get(2)?,
                screenshot_path: row.get(3)?,
                screenshot_sha256: row.get(4)?,
                provider: row.get(5)?,
                model: row.get(6)?,
                activity_category,
                task_status,
                confidence: row.get(9)?,
                reason: row.get(10)?,
                latency_ms: row.get::<_, Option<i64>>(11)?.map(|value| value as u128),
                input_tokens: row.get::<_, Option<i64>>(12)?.map(|value| value as u64),
                output_tokens: row.get::<_, Option<i64>>(13)?.map(|value| value as u64),
                total_tokens: row.get::<_, Option<i64>>(14)?.map(|value| value as u64),
                estimated_cost_usd: row.get(15)?,
                raw_response_json: row.get(16)?,
                error: row.get(17)?,
                app_name: row.get(18)?,
                window_title: row.get(19)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(ProbeError::from)
    }
}

fn parse_category(input: &str) -> ActivityCategory {
    match input {
        "coding" => ActivityCategory::Coding,
        "studying" => ActivityCategory::Studying,
        "reading" => ActivityCategory::Reading,
        "writing" => ActivityCategory::Writing,
        "browsing" => ActivityCategory::Browsing,
        "social_media" => ActivityCategory::SocialMedia,
        "video" => ActivityCategory::Video,
        "gaming" => ActivityCategory::Gaming,
        "email" => ActivityCategory::Email,
        "other" => ActivityCategory::Other,
        _ => ActivityCategory::Unknown,
    }
}

fn parse_status(input: &str) -> TaskStatus {
    match input {
        "on_task" => TaskStatus::OnTask,
        "off_task" => TaskStatus::OffTask,
        _ => TaskStatus::Ambiguous,
    }
}
