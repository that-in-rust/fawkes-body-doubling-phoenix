# Fawkes GPUI Overlay Executable Specification

**Repo:** `that-in-rust/fawkes-body-doubling-phoenix`  
**Milestone:** `Milestone 0C - Minimal GPUI Fixed-Cadence Summary`  
**Runtime:** Rust + Zed GPUI on macOS 13+ with Apple Silicon  
**Primary Outcome:** Keep the working CLI unchanged, simplify the overlay UI to `task + count`, fix the capture cadence at 15 seconds, and show both aggregate summary counts and one-line per-capture descriptions after the run

## Scope

This specification covers one minimal GPUI app window with:

- one single-line task input
- one count input
- one visible helper note that the capture interval is fixed at 15 seconds
- one `Start` action
- one running state
- one post-run summary view with aggregate counts and one-line per-capture descriptions

Out of scope for this milestone:

- any CLI contract changes
- editable interval controls in the overlay
- time-in-minutes inputs
- menu-bar integration
- settings panels
- historical timeline browsing
- nudges during the session
- Gemini or Vertex AI
- popup/all-spaces behavior
- dissociation heuristics beyond the existing model classifications

## Implementation Context

This spec is grounded in the current codebase:

- the existing CLI path already works and must stay stable in [src/main.rs](/Users/neetipatni/Desktop/fawkes-body-doubling-phoenix/src/main.rs)
- the overlay currently renders `task`, `interval`, `count`, `Start`, and an aggregate summary in [src/gpui_app/mod.rs](/Users/neetipatni/Desktop/fawkes-body-doubling-phoenix/src/gpui_app/mod.rs)
- the overlay form state still includes `interval_text` and validates it in [src/gpui_app/model.rs](/Users/neetipatni/Desktop/fawkes-body-doubling-phoenix/src/gpui_app/model.rs)
- the programmatic overlay request path already exists in [src/application/config.rs](/Users/neetipatni/Desktop/fawkes-body-doubling-phoenix/src/application/config.rs)
- per-attempt summary lines already exist in [src/core/types.rs](/Users/neetipatni/Desktop/fawkes-body-doubling-phoenix/src/core/types.rs)
- per-attempt `reason` is already carried into the run summary in [src/core/summary.rs](/Users/neetipatni/Desktop/fawkes-body-doubling-phoenix/src/core/summary.rs)

The implementation goal here is therefore a UI refinement, not a new probe architecture.

## Executable Requirements

### REQ-CLI-101.0: Preserve the working CLI contract

**WHEN** this UI milestone is implemented  
**THEN** the system SHALL leave the existing CLI entrypoint and `--goal --interval --count` behavior unchanged  
**AND** SHALL avoid introducing new required CLI flags for the overlay flow  
**SHALL** keep the overlay as a second Rust binary that reuses library code in-process

### REQ-UI-102.0: Render the smallest useful fixed-cadence form

**WHEN** the overlay launches successfully  
**THEN** the system SHALL open one floating GPUI window with exactly one task field, one count field, and one `Start` button  
**AND** SHALL show visible copy that the capture interval is fixed at `15 seconds`  
**SHALL** avoid rendering an editable interval input in this milestone

### REQ-VAL-103.0: Validate only task and count inline

**WHEN** the user presses `Start`  
**THEN** the system SHALL reject an empty task or `count < 1` inline in the same window before any capture starts  
**AND** SHALL keep the validation failure visible without closing or crashing the app  
**SHALL** create no screenshots and no SQLite rows when inline validation fails

### REQ-APP-104.0: Validate startup prerequisites without changing the CLI

**WHEN** the user starts a valid overlay session  
**THEN** the system SHALL validate `OPENAI_API_KEY` availability and screen-capture readiness before the first timed capture  
**AND** SHALL show remediation text in the overlay window when either prerequisite is missing  
**SHALL** create no screenshots and no SQLite rows when startup prerequisites fail

### REQ-SES-105.0: Run overlay sessions at a fixed 15-second cadence

**WHEN** the user starts a valid session with `count = N`  
**THEN** the system SHALL run exactly `N` capture-classify-persist attempts at a fixed interval of `15 seconds` between attempts  
**AND** SHALL derive the overlay session duration from `count * 15 seconds` rather than from a separate interval input  
**SHALL** avoid exposing interval mutability in the overlay while leaving CLI interval behavior untouched

### REQ-SES-106.0: Keep the existing in-process probe path

**WHEN** the overlay runs a session  
**THEN** the system SHALL call the existing Rust probe library directly through programmatic config and launcher interfaces  
**AND** SHALL continue storing runtime artifacts under `.fawkes_probe/` using the existing SQLite and per-run capture layout  
**SHALL** avoid spawning the CLI binary as a child process during normal GUI execution

### REQ-SES-107.0: Lock editing during a running session

**WHEN** a session begins  
**THEN** the system SHALL disable further task and count editing until the session completes or fails  
**AND** SHALL prevent overlapping starts from the same window  
**SHALL** show a visible running-state indicator that the probe is capturing every 15 seconds

### REQ-RPT-108.0: Show aggregate summary data after the run

**WHEN** the final scheduled attempt finishes  
**THEN** the system SHALL render a summary panel in the same window showing `run_id`, `on_task`, `off_task`, `ambiguous`, `error`, and `avg_latency_ms`  
**AND** SHALL keep those fields traceable to the same run stored in SQLite  
**SHALL** replace the running-state panel with this summary in the same window

### REQ-RPT-109.0: Show one-line per-capture descriptions

**WHEN** a run summary is displayed  
**THEN** the system SHALL render one summary line per capture attempt using the captured classification context already stored in the run summary  
**AND** SHALL include a short description derived from the model `reason` when present, or a compact error description when that attempt failed  
**SHALL** preserve attempt ordering from the completed run

### REQ-RPT-110.0: Keep per-capture descriptions readable in one window

**WHEN** the number of summary lines exceeds the visible summary area  
**THEN** the system SHALL keep all description lines reachable in the same window through a scrollable region or an equivalent non-lossy layout  
**AND** SHALL avoid truncating the run to aggregate counts only  
**SHALL** keep each description line to one visual row when practical, using concise wording rather than multi-paragraph detail

### REQ-RPT-111.0: Keep the end-user explanation plain

**WHEN** the summary panel is shown  
**THEN** the system SHALL continue showing one plain-language overview sentence in addition to the per-capture lines  
**AND** SHALL avoid provider jargon such as `structured output` or `task_status` in that human-facing sentence  
**SHALL** remain understandable without opening SQLite or inspecting logs

### REQ-ERR-112.0: Preserve recoverable iteration behavior

**WHEN** one or more provider requests fail recoverably during the overlay session  
**THEN** the system SHALL preserve the existing continuation behavior for later attempts  
**AND** SHALL surface those failures through the final `error` count and per-attempt error descriptions  
**SHALL** avoid crashing or freezing the window because of one recoverable iteration failure

## Test Matrix

| req_id | test_id | type | assertion | target |
| --- | --- | --- | --- | --- |
| REQ-CLI-101.0 | TEST-GPUI-UNIT-001 | unit | CLI main path and existing `--goal --interval --count` contract remain unchanged | compatibility |
| REQ-UI-102.0 | TEST-GPUI-UNIT-002 | gpui unit | overlay form renders task, count, fixed-15-second note, and start button with no editable interval field | rendering |
| REQ-VAL-103.0 | TEST-GPUI-UNIT-003 | gpui unit | empty task and invalid count show inline validation and do not launch a session | validation |
| REQ-APP-104.0 | TEST-GPUI-INTEG-004 | integration | missing `OPENAI_API_KEY` or screen-preflight failure surfaces remediation and creates no artifacts | startup safety |
| REQ-SES-105.0 | TEST-GPUI-UNIT-005 | unit | overlay request builder uses fixed `15` seconds for every overlay session | cadence |
| REQ-SES-106.0 | TEST-GPUI-INTEG-006 | integration | GUI path calls the library runner directly and writes artifacts under `.fawkes_probe/` | reuse |
| REQ-SES-107.0 | TEST-GPUI-UNIT-007 | gpui unit | running state disables task and count inputs and blocks overlapping starts | control flow |
| REQ-RPT-108.0 | TEST-GPUI-INTEG-008 | integration | completed run swaps to aggregate summary counts with `run_id` and latency | reporting |
| REQ-RPT-109.0 | TEST-GPUI-UNIT-009 | unit | per-attempt summary view preserves run order and surfaces reason-or-error text for each capture | reporting |
| REQ-RPT-110.0 | TEST-GPUI-UNIT-010 | gpui unit | description list remains reachable when capture count exceeds the initial visible area | layout |
| REQ-RPT-111.0 | TEST-GPUI-UNIT-011 | unit | overview sentence remains plain-language while per-attempt lines add detail | usability |
| REQ-ERR-112.0 | TEST-GPUI-INTEG-012 | integration | recoverable provider failures still end in a completed summary with nonzero error count and error lines | resilience |

## TDD Plan

1. STUB
- Add failing GPUI tests for the reduced form shape: task, count, fixed 15-second note, and no editable interval field.
- Add failing model tests for inline count validation and fixed-cadence overlay request construction.
- Add failing summary tests for one-line per-attempt descriptions using existing `RunSummary.lines`.
- Add a failing UI test for a summary panel that preserves all attempt lines in a reachable layout.

2. RED
- Run the GPUI and unit tests and confirm failures for the old interval field, missing fixed-cadence note, missing per-attempt lines, and missing scroll-or-equivalent summary layout.
- Confirm the CLI tests still pass unchanged.

3. GREEN
- Remove interval editing from the overlay form model and view.
- Route the overlay through the fixed 15-second overlay request builder while leaving CLI interval parsing untouched.
- Extend the overlay summary view model to carry one-line attempt descriptions derived from `RunSummary.lines`.
- Render the aggregate summary plus the ordered description list in one window.
- Keep startup preflight, in-process execution, and runtime artifact locations unchanged.

4. REFACTOR
- Keep new symbol names to four words where practical, such as:
  - `build_overlay_attempt_descriptions`
  - `render_fixed_interval_note`
  - `render_attempt_description_list`
  - `format_attempt_reason_line`
  - `validate_overlay_count_field`
- Keep GPUI layout logic in `gpui_app`.
- Keep summary-line derivation in `core` or a UI-facing adapter layer rather than scattering formatting rules across the window render path.

5. VERIFY
- Run `cargo fmt --all -- --check`.
- Run `cargo clippy --all-targets --all-features -- -D warnings`.
- Run `cargo test --all-targets --all-features`.
- Run `cargo build --all-targets --all-features`.
- Run a manual GUI smoke test that confirms:
  - the overlay shows `task + count` only
  - the helper note says the interval is fixed at 15 seconds
  - the session runs at the fixed cadence
  - the final summary shows both aggregate counts and one-line per-capture descriptions

## Quality Gates

- [ ] Every requirement has a stable `REQ-*` identifier.
- [ ] Every `REQ-*` identifier appears in the test matrix.
- [ ] The CLI contract remains unchanged.
- [ ] The overlay no longer exposes an editable interval field.
- [ ] Overlay sessions always use a fixed 15-second cadence.
- [ ] The final summary shows aggregate counts and one-line per-capture descriptions.
- [ ] `.fawkes_probe/` remains the only runtime artifact directory for overlay runs.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes.
- [ ] `cargo test --all-targets --all-features` passes.
- [ ] `cargo build --all-targets --all-features` passes.
- [ ] No new `TODO`, `STUB`, or `FIXME` markers remain in committed implementation code.

## Open Questions

1. No blocking product questions remain for this UI slice.
2. The only implementation choice left open is whether the per-attempt list should use a scroll region or a larger auto-sized summary panel, as long as no attempt descriptions are lost.
