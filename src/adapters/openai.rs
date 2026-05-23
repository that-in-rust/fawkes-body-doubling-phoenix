use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use secrecy::{ExposeSecret, SecretString};
use serde_json::{Value, json};

use crate::application::traits::VisionClassifyBehavior;
use crate::core::prompt::{
    build_assessment_json_schema, build_bounded_prompt_context, build_schema_prompt_input,
};
use crate::core::types::{CaptureAssessment, VisionProviderFailure, VisionRequest, VisionSuccess};

const DEFAULT_OPENAI_RESPONSES_URL: &str = "https://api.openai.com/v1/responses";

pub struct OpenAiVisionClient {
    api_key: SecretString,
    model_name: String,
    responses_url: String,
    client: Client,
}

impl OpenAiVisionClient {
    pub fn new(
        api_key: SecretString,
        model_name: String,
        responses_url: Option<String>,
    ) -> Result<Self, crate::core::error::ProbeError> {
        let client = Client::builder().timeout(Duration::from_secs(60)).build()?;
        Ok(Self {
            api_key,
            model_name,
            responses_url: responses_url.unwrap_or_else(|| DEFAULT_OPENAI_RESPONSES_URL.to_owned()),
            client,
        })
    }

    fn build_openai_request_body(&self, request: &VisionRequest) -> Value {
        let prompt = build_bounded_prompt_context(&build_schema_prompt_input(
            &request.goal,
            request.app_name.as_deref(),
            request.window_title.as_deref(),
        ));
        let data_url = format!(
            "data:image/jpeg;base64,{}",
            BASE64_STANDARD.encode(&request.jpeg_bytes)
        );

        json!({
            "model": request.model,
            "store": false,
            "input": [{
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": prompt
                    },
                    {
                        "type": "input_image",
                        "image_url": data_url,
                        "detail": "low"
                    }
                ]
            }],
            "text": {
                "format": {
                    "type": "json_schema",
                    "name": "capture_assessment",
                    "strict": true,
                    "schema": build_assessment_json_schema()
                }
            }
        })
    }

    fn parse_response_usage(value: &Value) -> (Option<u64>, Option<u64>, Option<u64>) {
        let input_tokens = value
            .get("usage")
            .and_then(|usage| usage.get("input_tokens"))
            .and_then(Value::as_u64);
        let output_tokens = value
            .get("usage")
            .and_then(|usage| usage.get("output_tokens"))
            .and_then(Value::as_u64);
        let total_tokens = value
            .get("usage")
            .and_then(|usage| usage.get("total_tokens"))
            .and_then(Value::as_u64);

        (input_tokens, output_tokens, total_tokens)
    }

    fn extract_output_text(value: &Value) -> Option<String> {
        value
            .get("output")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find_map(|item| {
                item.get("content")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .find_map(|content| {
                        let is_output_text = content
                            .get("type")
                            .and_then(Value::as_str)
                            .map(|kind| kind == "output_text")
                            .unwrap_or(false);
                        if is_output_text {
                            content
                                .get("text")
                                .and_then(Value::as_str)
                                .map(ToOwned::to_owned)
                        } else {
                            None
                        }
                    })
            })
            .or_else(|| {
                value
                    .get("output_text")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
    }

    fn parse_assessment_json(
        &self,
        raw_response_json: &str,
    ) -> Result<VisionSuccess, VisionProviderFailure> {
        let response_value: Value =
            serde_json::from_str(raw_response_json).map_err(|error| VisionProviderFailure {
                error_message: format!("response JSON could not be parsed: {error}"),
                status_code: None,
                raw_body: Some(raw_response_json.to_owned()),
            })?;

        let output_text =
            Self::extract_output_text(&response_value).ok_or_else(|| VisionProviderFailure {
                error_message: "provider response contained no structured output text".to_owned(),
                status_code: None,
                raw_body: Some(raw_response_json.to_owned()),
            })?;

        let assessment: CaptureAssessment =
            serde_json::from_str(&output_text).map_err(|error| VisionProviderFailure {
                error_message: format!("structured output parsing failed: {error}"),
                status_code: None,
                raw_body: Some(raw_response_json.to_owned()),
            })?;

        let (input_tokens, output_tokens, total_tokens) =
            Self::parse_response_usage(&response_value);

        Ok(VisionSuccess {
            assessment,
            raw_response_json: raw_response_json.to_owned(),
            input_tokens,
            output_tokens,
            total_tokens,
        })
    }
}

impl VisionClassifyBehavior for OpenAiVisionClient {
    fn classify_remote_focus_frame(
        &self,
        request: &VisionRequest,
    ) -> Result<VisionSuccess, VisionProviderFailure> {
        let body = self.build_openai_request_body(request);
        let response = self
            .client
            .post(&self.responses_url)
            .bearer_auth(self.api_key.expose_secret())
            .json(&body)
            .send()
            .map_err(|error| VisionProviderFailure {
                error_message: format!("http request failed: {error}"),
                status_code: None,
                raw_body: None,
            })?;

        let status = response.status();
        let raw_body = response.text().map_err(|error| VisionProviderFailure {
            error_message: format!("response body could not be read: {error}"),
            status_code: Some(status.as_u16()),
            raw_body: None,
        })?;

        if status != StatusCode::OK {
            return Err(VisionProviderFailure {
                error_message: format!("provider http failure: {status}"),
                status_code: Some(status.as_u16()),
                raw_body: Some(raw_body),
            });
        }

        self.parse_assessment_json(&raw_body)
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}
