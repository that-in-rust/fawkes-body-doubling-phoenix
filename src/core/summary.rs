use std::fmt::Write;

use crate::core::types::{AttemptSummaryLine, CaptureRecord, RunSummary, TaskStatus};

pub fn summarize_capture_records(run_id: &str, records: &[CaptureRecord]) -> RunSummary {
    let mut on_task_count = 0usize;
    let mut off_task_count = 0usize;
    let mut ambiguous_count = 0usize;
    let mut error_count = 0usize;
    let mut latency_samples = Vec::new();
    let mut input_tokens = 0u64;
    let mut output_tokens = 0u64;
    let mut total_tokens = 0u64;

    let lines = records
        .iter()
        .map(|record| {
            if record.error.is_some() {
                error_count += 1;
            } else {
                match record.task_status {
                    Some(TaskStatus::OnTask) => on_task_count += 1,
                    Some(TaskStatus::OffTask) => off_task_count += 1,
                    Some(TaskStatus::Ambiguous) => ambiguous_count += 1,
                    None => error_count += 1,
                }
                if let Some(latency_ms) = record.latency_ms {
                    latency_samples.push(latency_ms);
                }
            }

            input_tokens += record.input_tokens.unwrap_or_default();
            output_tokens += record.output_tokens.unwrap_or_default();
            total_tokens += record.total_tokens.unwrap_or_default();

            AttemptSummaryLine {
                captured_at: record.captured_at.to_rfc3339(),
                task_status: record.task_status,
                activity_category: record.activity_category,
                confidence: record.confidence,
                latency_ms: record.latency_ms,
                error: record.error.clone(),
            }
        })
        .collect();

    let average_latency_ms = if latency_samples.is_empty() {
        None
    } else {
        Some(latency_samples.iter().copied().sum::<u128>() / latency_samples.len() as u128)
    };

    RunSummary {
        run_id: run_id.to_owned(),
        lines,
        on_task_count,
        off_task_count,
        ambiguous_count,
        error_count,
        average_latency_ms,
        input_tokens,
        output_tokens,
        total_tokens,
    }
}

impl RunSummary {
    pub fn render_terminal_summary_report(&self) -> String {
        let mut output = String::new();

        for line in &self.lines {
            match (&line.task_status, &line.activity_category, &line.error) {
                (_, _, Some(error)) => {
                    let _ = writeln!(
                        output,
                        "[{}] error latency_ms={} detail={}",
                        line.captured_at,
                        line.latency_ms.unwrap_or_default(),
                        error
                    );
                }
                (Some(task_status), Some(category), None) => {
                    let confidence = line.confidence.unwrap_or(0.0);
                    let _ = writeln!(
                        output,
                        "[{}] {} category={} confidence={:.2} latency_ms={}",
                        line.captured_at,
                        task_status.as_str(),
                        category.as_str(),
                        confidence,
                        line.latency_ms.unwrap_or_default()
                    );
                }
                _ => {
                    let _ = writeln!(
                        output,
                        "[{}] error latency_ms={} detail=missing classification details",
                        line.captured_at,
                        line.latency_ms.unwrap_or_default()
                    );
                }
            }
        }

        let _ = writeln!(output, "\nSummary:");
        let _ = writeln!(output, "run_id: {}", self.run_id);
        let _ = writeln!(output, "on_task: {}", self.on_task_count);
        let _ = writeln!(output, "off_task: {}", self.off_task_count);
        let _ = writeln!(output, "ambiguous: {}", self.ambiguous_count);
        let _ = writeln!(output, "error: {}", self.error_count);

        if let Some(average_latency_ms) = self.average_latency_ms {
            let _ = writeln!(output, "avg_latency_ms: {average_latency_ms}");
        }

        if self.total_tokens > 0 {
            let _ = writeln!(output, "input_tokens: {}", self.input_tokens);
            let _ = writeln!(output, "output_tokens: {}", self.output_tokens);
            let _ = writeln!(output, "total_tokens: {}", self.total_tokens);
        }

        output
    }
}

pub fn format_plain_summary_sentence(summary: &RunSummary) -> String {
    if summary.lines.is_empty() {
        return "This run finished without any captured assessments.".to_owned();
    }

    if summary.error_count == summary.lines.len() {
        return "This run failed before it could assess your work reliably.".to_owned();
    }

    if summary.off_task_count > summary.on_task_count {
        return "This run drifted away from the task more often than it stayed on it.".to_owned();
    }

    if summary.on_task_count > 0 && summary.off_task_count == 0 && summary.error_count == 0 {
        return "This run stayed on task throughout the session.".to_owned();
    }

    if summary.ambiguous_count > 0 {
        return "This run stayed mostly on task, but a few moments were unclear.".to_owned();
    }

    if summary.error_count > 0 {
        return "This run mostly stayed on task, but a few checks failed.".to_owned();
    }

    "This run stayed mostly on task.".to_owned()
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::core::types::{ActivityCategory, CaptureRecord, RunIdentifier, TaskStatus};

    use super::{format_plain_summary_sentence, summarize_capture_records};

    #[test]
    fn test_req_rust_004_summary_counts_match_records() {
        let run_id = RunIdentifier::generate_now_identifier().to_string();
        let records = vec![
            CaptureRecord::test_record(Utc::now(), Some(TaskStatus::OnTask), None),
            CaptureRecord::test_record(Utc::now(), Some(TaskStatus::OffTask), None),
            CaptureRecord::test_record(Utc::now(), Some(TaskStatus::Ambiguous), None),
            CaptureRecord::test_record(Utc::now(), None, Some("provider error".to_owned())),
        ];

        let summary = summarize_capture_records(&run_id, &records);
        assert_eq!(summary.on_task_count, 1);
        assert_eq!(summary.off_task_count, 1);
        assert_eq!(summary.ambiguous_count, 1);
        assert_eq!(summary.error_count, 1);
        assert_eq!(
            summary.lines[0].activity_category,
            Some(ActivityCategory::Coding)
        );
    }

    #[test]
    fn test_req_rust_105_formats_plain_summary_sentence() {
        let run_id = RunIdentifier::generate_now_identifier().to_string();
        let records = vec![
            CaptureRecord::test_record(Utc::now(), Some(TaskStatus::OffTask), None),
            CaptureRecord::test_record(Utc::now(), Some(TaskStatus::OffTask), None),
            CaptureRecord::test_record(Utc::now(), Some(TaskStatus::OnTask), None),
        ];

        let summary = summarize_capture_records(&run_id, &records);
        let sentence = format_plain_summary_sentence(&summary);
        assert!(sentence.contains("drifted away from the task"));
    }
}
