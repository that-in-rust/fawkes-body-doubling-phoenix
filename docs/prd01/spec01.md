# Fawkes Probe Executable Specification

**Repo:** `that-in-rust/fawkes-body-doubling-phoenix`  
**Derived from:** [min01.md](/Users/neetipatni/Desktop/fawkes-body-doubling-phoenix/docs/prd01/min01.md)  
**Milestone:** `Milestone 0 - Fawkes Probe`  
**Runtime:** Rust CLI on macOS 13+ with Apple Silicon M2+  
**Primary Outcome:** Prove the end-to-end loop `capture -> downscale -> classify -> persist -> summarize` with OpenAI before any menu-bar, GPUI, or multi-provider work

## Scope

This specification covers a single Rust CLI binary named `fawkes_probe`.

Out of scope for this milestone:

- GPUI or menu-bar UI
- Notifications or nudges
- Session presets and timers beyond a fixed run count
- Background auto-start behavior
- Daily or weekly summaries
- Gemini or Vertex AI implementation in Milestone 0
- Multi-provider routing
- Retry economics and workflow-level cost optimization
- Prior-capture memory in provider prompts
- Local-only inference guarantees

## Cost And Latency Architecture Principles

This probe SHALL treat cost and latency as product behavior, not only backend implementation details.

- Keep the synchronous path minimal: only work that changes the next visible probe result belongs before that result is printed.
- Budget context explicitly: do not dump full run history or previous screenshots into each model call.
- Record enough latency and token usage to compare future provider choices, but do not block Milestone 0 on full economic instrumentation.

## OpenAI Tasks Needed

### Operator setup tasks

- [ ] Confirm the OpenAI project that will own probe usage.
- [ ] Create an OpenAI API key for that project.
- [ ] Store the key locally as `OPENAI_API_KEY` for probe execution.
- [ ] Set or review spend alerts before repeated screenshot runs.
- [ ] Pin the first OpenAI model name for the probe smoke test.
- [ ] Run one manual single-image OpenAI request outside Rust to verify the key, spend path, and model access.

### Engineering tasks

- [ ] Read `OPENAI_API_KEY` from the environment and fail fast when it is missing.
- [ ] Implement one schema-bound OpenAI Responses API image classification request for the declared focus goal.
- [ ] Persist OpenAI model name, HTTP status, latency, raw payload, and error details for each attempt.
- [ ] Treat OpenAI `401`, `403`, `429`, and `5xx` responses as recoverable per-iteration errors unless startup configuration is invalid.

## Executable Requirements

### REQ-CLI-001.0: Validate required probe arguments

**WHEN** the user invokes `fawkes_probe`  
**THEN** the system SHALL require `--goal`, `--interval`, and `--count` before starting a run  
**AND** SHALL reject `interval < 5` seconds and `count < 1`  
**SHALL** exit non-zero with a usage error and create no probe artifacts for invalid input

### REQ-OAI-001.0: Validate OpenAI credential availability

**WHEN** the user starts the probe  
**THEN** the system SHALL require a non-empty `OPENAI_API_KEY` before the first capture attempt  
**AND** SHALL exit non-zero with a remediation message naming the missing environment variable  
**SHALL** create no screenshot file and no SQLite capture row when the key is missing

### REQ-CAP-001.0: Fail fast on missing screen permission

**WHEN** the probe starts without macOS Screen Recording permission  
**THEN** the system SHALL stop before the first capture attempt  
**AND** SHALL print a remediation message that names System Settings and Screen Recording  
**SHALL** create no screenshot file and no SQLite capture row for that aborted run

### REQ-CAP-002.0: Capture one downscaled image per iteration

**WHEN** a capture iteration begins with permission granted  
**THEN** the system SHALL capture exactly one current screen image  
**AND** SHALL persist exactly one downscaled image under `.fawkes_probe/runs/<run_id>/captures/`  
**SHALL** ensure the stored image short side is less than or equal to `768` pixels and SHALL never persist the full-resolution source image

### REQ-ART-001.0: Store probe artifacts in a reviewable layout

**WHEN** the probe creates local artifacts  
**THEN** the system SHALL store the SQLite database at `.fawkes_probe/fawkes_probe.sqlite`  
**AND** SHALL store capture images under `.fawkes_probe/runs/<run_id>/captures/` using timestamp-based filenames  
**SHALL** make every persisted capture row traceable to a saved image by both relative path and SHA-256 hash

### REQ-CTX-001.0: Attach best-effort local activity context

**WHEN** a capture file is created  
**THEN** the system SHALL attempt to collect active app name and active window title metadata  
**AND** SHALL persist nullable fields for both values in the capture row  
**SHALL** continue the iteration when either metadata value cannot be obtained or the metadata collector is not implemented yet

### REQ-CTX-002.0: Enforce a bounded model context budget

**WHEN** the system prepares a provider request for one capture  
**THEN** the system SHALL include only the declared goal, the current downscaled image, output schema instructions, and current app or window metadata when available  
**AND** SHALL include no prior screenshots or prior capture summaries in Milestone 0  
**SHALL** exclude full run history and unrelated tool output from the provider request

### REQ-VLM-001.0: Request schema-bound focus classification

**WHEN** a downscaled image is ready for analysis  
**THEN** the system SHALL send the declared goal and the image to OpenAI  
**AND** SHALL require a JSON response with `activity_category`, `task_status`, `confidence`, and `reason` fields  
**SHALL** accept only `task_status` values `on_task`, `off_task`, or `ambiguous`

### REQ-OAI-002.0: Use OpenAI image input with schema-bound output

**WHEN** a downscaled image is ready for analysis  
**THEN** the system SHALL submit the goal text plus image input to the OpenAI Responses API  
**AND** SHALL require a JSON response shape equivalent to `activity_category`, `task_status`, `confidence`, and `reason`  
**SHALL** reject non-schema output as a per-iteration provider error

### REQ-VLM-002.0: Persist provider failures without losing the run

**WHEN** a provider call times out, returns malformed JSON, or omits a required field  
**THEN** the system SHALL persist a capture row with a non-null `error` field for that iteration  
**AND** SHALL record `latency_ms` plus raw provider payload when available  
**SHALL** continue remaining iterations unless the user interrupts the process or a local storage error occurs

### REQ-OAI-003.0: Persist OpenAI HTTP failure context

**WHEN** an OpenAI request returns HTTP `401`, `403`, `429`, or any `5xx` status  
**THEN** the system SHALL persist the HTTP status code, model name, and provider error body when available  
**AND** SHALL mark the iteration with a non-null `error` field  
**SHALL** continue remaining iterations for `429` and `5xx` responses and SHALL stop the run only for invalid startup configuration

### REQ-DB-001.0: Persist one row for every capture attempt

**WHEN** a capture attempt reaches the persistence step  
**THEN** the system SHALL insert exactly one row into a `captures` table  
**AND** SHALL persist at minimum `captured_at`, `goal`, `screenshot_path`, `screenshot_sha256`, `provider`, `model`, `activity_category`, `task_status`, `confidence`, `reason`, `latency_ms`, `raw_response_json`, and `error` columns  
**SHALL** store nullable values rather than synthetic defaults for unavailable classification fields

### REQ-MET-001.0: Persist route, usage, and retry telemetry

**WHEN** a capture attempt is persisted  
**THEN** the system SHALL persist nullable telemetry fields for `input_tokens`, `output_tokens`, `total_tokens`, and `estimated_cost_usd` when those values are available  
**AND** SHALL store provider-supplied usage metrics when the OpenAI API returns them  
**SHALL** store null values rather than inferred placeholders when a metric is unavailable

### REQ-RUN-001.0: Execute a serial probe loop with exact attempt count

**WHEN** the user starts a run with `--count N`  
**THEN** the system SHALL execute exactly `N` capture attempts unless a startup precondition fails or the user interrupts the process  
**AND** SHALL avoid overlapping capture-classify-persist operations  
**SHALL** wait the configured interval between completed attempts except after the final attempt

### REQ-RPT-001.0: Print per-capture output and final summary

**WHEN** the probe completes its final attempt  
**THEN** the system SHALL print one terminal line per attempt containing timestamp, category or error, `task_status`, confidence when available, and latency  
**AND** SHALL print a summary with counts for `on_task`, `off_task`, `ambiguous`, and `error` outcomes  
**SHALL** print average classification latency over successfully classified attempts

### REQ-RPT-002.0: Report workflow-level cost and route metrics

**WHEN** the probe prints its final summary  
**THEN** the system SHALL include token totals and estimated cost totals when those metrics are available  
**AND** SHALL include average total latency when that metric is available  
**SHALL** omit unavailable metrics rather than printing synthetic values

### REQ-NFR-001.0: Measure local preprocessing performance

**WHEN** the probe executes a ten-attempt run on Apple M2 hardware with provider calls stubbed out  
**THEN** the system SHALL record local preprocessing time for capture, downscale, hash, and file write for each iteration  
**AND** SHALL keep median local preprocessing time less than or equal to `1000 ms` across the run  
**SHALL** fail the performance verification when the threshold is exceeded

## Test Matrix

| req_id | test_id | type | assertion | target |
| --- | --- | --- | --- | --- |
| REQ-CLI-001.0 | TEST-UNIT-CLI-001 | unit | accepts valid `goal`, `interval`, and `count` inputs | correctness |
| REQ-CLI-001.0 | TEST-UNIT-CLI-002 | unit | rejects invalid interval and count before artifact creation | validation |
| REQ-OAI-001.0 | TEST-UNIT-OAI-001 | unit | refuses startup when `OPENAI_API_KEY` is absent | configuration |
| REQ-CAP-001.0 | TEST-INTEG-CAP-001 | integration | exits with permission guidance and creates no files or rows when screen permission is denied | permission |
| REQ-CAP-002.0 | TEST-INTEG-CAP-002 | integration | writes exactly one downscaled image per attempt and never writes full-resolution artifacts | capture |
| REQ-ART-001.0 | TEST-INTEG-ART-001 | integration | stores database and captures in `.fawkes_probe/` and matches row path to image hash | storage |
| REQ-CTX-001.0 | TEST-UNIT-CTX-001 | unit | persists nullable app and window metadata when collection is unavailable or skipped | resilience |
| REQ-CTX-002.0 | TEST-UNIT-CTX-002 | unit | builds provider context from current-state inputs only and excludes prior history blobs | token budget |
| REQ-VLM-001.0 | TEST-UNIT-VLM-001 | unit | accepts only schema-valid JSON with allowed `task_status` values | schema |
| REQ-OAI-002.0 | TEST-INTEG-OAI-002 | integration | sends goal plus image to OpenAI and rejects non-schema output as a captured provider error | provider contract |
| REQ-VLM-002.0 | TEST-INTEG-VLM-002 | integration | persists error rows and continues the run after timeout or malformed provider payload | fault tolerance |
| REQ-OAI-003.0 | TEST-INTEG-OAI-003 | integration | records HTTP status and error body for OpenAI `401`, `403`, `429`, and `5xx` responses | provider resilience |
| REQ-DB-001.0 | TEST-INTEG-DB-001 | integration | inserts exactly one `captures` row per attempt with required columns populated or null | persistence |
| REQ-MET-001.0 | TEST-INTEG-MET-001 | integration | stores token and cost telemetry with nulls for unavailable metrics | observability |
| REQ-RUN-001.0 | TEST-INTEG-RUN-001 | integration | executes exactly `N` non-overlapping attempts and skips post-run sleep | control flow |
| REQ-RPT-001.0 | TEST-INTEG-RPT-001 | integration | prints per-attempt lines and summary counts that match persisted rows | reporting |
| REQ-RPT-002.0 | TEST-INTEG-RPT-002 | integration | prints token and cost summaries when telemetry is present | workflow economics |
| REQ-NFR-001.0 | TEST-PERF-NFR-001 | performance | keeps median local preprocessing at or below `1000 ms` on M2 with stubbed provider | latency |

## TDD Plan

1. STUB
- Create failing tests for argument parsing, SQLite schema creation, provider schema validation, serial loop behavior, and terminal summary formatting.
- Add image fixtures and provider payload fixtures under `tests/fixtures/`.
- Add OpenAI fixtures for missing-key startup, Responses API success, rate-limit failure, and non-schema output failure.
- Add telemetry fixtures for usage-present and usage-absent provider responses.
- Stub these implementation seams with four-word names:
  - `parse_strict_probe_args`
  - `open_local_probe_store`
  - `capture_active_screen_once`
  - `resize_small_capture_image`
  - `collect_active_window_meta`
  - `build_bounded_prompt_context`
  - `classify_remote_focus_frame`
  - `persist_probe_capture_row`
  - `render_terminal_summary_report`
  - `run_serial_probe_cycle`

2. RED
- Run `cargo test` and confirm failures map to missing parser, capture, persistence, and reporting behaviors.
- Record the expected failure reason for each `TEST-*` case before implementation begins.

3. GREEN
- Implement CLI parsing and validation first.
- Implement OpenAI environment validation before any live provider call path.
- Implement SQLite initialization and row persistence second.
- Implement capture and downscale pipeline third.
- Implement provider adapter, bounded prompt construction, and strict JSON decoding fourth.
- Implement the serial run loop and summary rendering last.

4. REFACTOR
- Extract provider and capture seams behind traits so tests can run with stubs instead of live macOS or network dependencies.
- Replace duplicated row-building and summary-count logic with shared helpers.
- Keep all public behavior aligned to the `REQ-*` contracts while simplifying internals.

5. VERIFY
- Run `cargo test`.
- Run `cargo fmt --check`.
- Run `cargo clippy --all-targets --all-features -- -D warnings`.
- Run a manual ten-attempt smoke test with a real goal and inspect the database plus saved captures.
- Confirm every `REQ-*` ID is referenced by at least one `TEST-*` case.

## Quality Gates

- [ ] Every requirement has a stable `REQ-*` identifier.
- [ ] Every `REQ-*` identifier appears in the test matrix.
- [ ] `cargo test` passes.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes.
- [ ] No new `TODO`, `STUB`, or `FIXME` markers remain in committed code.
- [ ] No full-resolution screenshots are persisted during the smoke test.
- [ ] The smoke test creates exactly one database row and one saved image per attempt.
- [ ] A manual OpenAI one-image smoke test succeeds before the full ten-attempt probe run.
- [ ] Spend alerting is configured or confirmed on the chosen OpenAI project before repeated live runs.
- [ ] The final schema includes token and cost telemetry fields even when OpenAI leaves them null.
- [ ] The provider prompt builder is verified to exclude full run history and prior screenshot binaries.
- [ ] The performance gate for `REQ-NFR-001.0` includes recorded measurements, not narrative claims.
- [ ] Manual reviewer agreement is captured for at least ten real screenshots before promoting the probe to UI work.

## Open Questions

1. Should Milestone 0 capture the full display only, or should it prefer active-window capture when ScreenCaptureKit metadata is easy to obtain?
2. Which exact OpenAI model should be pinned first for screenshot classification?
3. Should downscaled screenshots persist by default for every probe run, or should retention require an explicit debug flag even in this early milestone?
4. Should `ambiguous` results count as neutral, weakly off-task, or user-review-only when later drift logic is added?
5. When Milestone 0 works, should the next addition be Gemini via ADC or richer app and window metadata first?
