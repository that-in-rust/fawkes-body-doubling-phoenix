# Fawkes GPUI Spike Executable Specification

**Repo:** `that-in-rust/fawkes-body-doubling-phoenix`  
**Derived from:** [spec01.md](/Users/neetipatni/Desktop/fawkes-body-doubling-phoenix/docs/prd01/spec01.md), [min01.md](/Users/neetipatni/Desktop/fawkes-body-doubling-phoenix/docs/prd01/min01.md)  
**Milestone:** `Milestone 0B - Minimal GPUI Count Spike`  
**Runtime:** Rust + Zed GPUI on macOS 13+ with Apple Silicon  
**Primary Outcome:** Add a super-minimal native macOS GPUI window that starts a count-based focus probe and shows a post-session summary while reusing the existing probe library in-process

## Scope

This specification covers one minimal GPUI app window with:

- one single-line task input
- one interval input in seconds
- one capture-count input
- one `Start` action
- one session summary view after the run ends

Out of scope for this milestone:

- menu-bar integration
- settings panels
- historical timeline browsing
- nudges during the session
- Gemini or Vertex AI
- multiple windows for different flows
- dissociation verdict heuristics
- packaging polish beyond a runnable macOS GUI binary
- replacing the existing CLI proof harness during development
- hard guarantees about appearing above OS-reserved alerts or every fullscreen context

## Implementation Context

This spec is grounded in the local Zed GPUI reference checkout:

- GPUI apps open windows with `App::open_window(...)` as shown in [hello_world.rs](/Users/neetipatni/Desktop/fawkes-body-doubling-phoenix/reference-repos/zed/crates/gpui/examples/hello_world.rs:92)
- GPUI exposes `WindowKind::Floating` and `WindowKind::PopUp` in [platform.rs](/Users/neetipatni/Desktop/fawkes-body-doubling-phoenix/reference-repos/zed/crates/gpui/src/platform.rs:1576)
- GPUI macOS maps `WindowKind::PopUp` to a higher native window level and `canJoinAllSpaces`-style behavior in [window.rs](/Users/neetipatni/Desktop/fawkes-body-doubling-phoenix/reference-repos/zed/crates/gpui_macos/src/window.rs:908)
- GPUI text input is available, but the local reference shape is a custom input pattern rather than a trivial built-in one-liner, as shown in [input.rs](/Users/neetipatni/Desktop/fawkes-body-doubling-phoenix/reference-repos/zed/crates/gpui/examples/input.rs:36)

The app SHALL reuse the existing probe library logic directly and SHALL not shell out to the CLI binary during normal app execution.

## Executable Requirements

### REQ-UI-001.0: Render the minimal session form

**WHEN** the app launches successfully  
**THEN** the system SHALL open exactly one primary GPUI window with a single-line task field, an interval-in-seconds field, a count field, and a `Start` button  
**AND** SHALL render all controls without requiring any secondary dialog before the user can begin a session  
**SHALL** keep the initial UI limited to one visible form and no historical panels

### REQ-VAL-001.0: Validate session form inputs before starting

**WHEN** the user presses `Start`  
**THEN** the system SHALL reject an empty task, `interval_seconds < 5`, or `count < 1` before any capture starts  
**AND** SHALL show the validation failure inline in the same window without crashing or closing the app  
**SHALL** create no screenshots and no SQLite capture rows when validation fails

### REQ-APP-001.0: Validate runtime prerequisites in-app

**WHEN** the user starts a valid session  
**THEN** the system SHALL validate `OPENAI_API_KEY` availability and screen-capture readiness before the first timed iteration  
**AND** SHALL show remediation text in the app window when either prerequisite is missing  
**SHALL** create no screenshots and no SQLite capture rows when startup prerequisites fail

### REQ-WIN-001.0: Open the app as a minimal floating window

**WHEN** the app opens its primary window on macOS  
**THEN** the system SHALL request a GPUI window kind equivalent to `WindowKind::Floating` for the first implementation  
**AND** SHALL keep the window focused and usable as the single app surface for starting and observing a session  
**SHALL** avoid depending on popup-specific or all-spaces behavior in this first GPUI spike

### REQ-SES-001.0: Derive a count-based session from interval and count

**WHEN** the user starts a valid session with `interval_seconds` and `count`  
**THEN** the system SHALL run exactly `count` capture-classify-persist cycles at the chosen interval without overlapping attempts  
**AND** SHALL stop after the final scheduled attempt completes  
**SHALL** avoid inventing a separate minutes-based duration control in this milestone

### REQ-SES-002.0: Lock the form while a session is running

**WHEN** a session begins  
**THEN** the system SHALL disable editing of the task, interval, and count inputs until the session completes or fails  
**AND** SHALL replace or disable the `Start` button so the user cannot start a second overlapping session  
**SHALL** show a visible running-state indicator in the same window

### REQ-SES-003.0: Reuse the existing probe pipeline in-process

**WHEN** the app executes a running session  
**THEN** the system SHALL call the existing Rust capture, downscale, classify, persist, and summary logic directly through library interfaces  
**AND** SHALL continue storing runtime artifacts under `.fawkes_probe/` using the same SQLite and per-run capture layout as the CLI milestone  
**SHALL** avoid spawning a child `fawkes_probe` process during the normal GUI run path

### REQ-RPT-001.0: Show a session summary when the run ends

**WHEN** the final scheduled attempt finishes  
**THEN** the system SHALL replace the running state with a summary view in the same window  
**AND** SHALL show at minimum `on_task`, `off_task`, `ambiguous`, `error`, and average latency counts for that run  
**SHALL** include the run identifier so the user can trace the session back to SQLite and saved captures

### REQ-RPT-002.0: Show a plain-language summary sentence

**WHEN** the summary is rendered  
**THEN** the system SHALL show one short human-readable sentence that names the total `on_task` and `off_task` counts for the run  
**AND** SHALL avoid provider jargon such as `task_status` or `structured output` in the end-user summary sentence  
**SHALL** remain understandable without opening SQLite or reading logs

### REQ-ERR-001.0: Surface recoverable provider failures in-session

**WHEN** one or more provider requests fail during a running session  
**THEN** the system SHALL continue later attempts when the failure is recoverable under the existing probe rules  
**AND** SHALL include the resulting `error` count in the final summary view  
**SHALL** avoid crashing the window or closing the app because of one recoverable iteration failure

### REQ-BIN-001.0: Produce a user-runnable GPUI app binary

**WHEN** the milestone is built in development mode  
**THEN** the system SHALL produce one directly runnable GPUI macOS executable target for the UI flow in addition to the existing CLI path  
**AND** SHALL keep the UI path implemented in Rust without Electron, Tauri, or webview dependencies  
**SHALL** preserve the existing library reuse path so the same core logic is shared between CLI verification and GUI execution

## Test Matrix

| req_id | test_id | type | assertion | target |
| --- | --- | --- | --- | --- |
| REQ-UI-001.0 | TEST-GPUI-UNIT-001 | unit | launches one root view with task, interval, count, and start controls visible | rendering |
| REQ-VAL-001.0 | TEST-GPUI-UNIT-002 | unit | rejects empty task and invalid interval or count values before session creation | validation |
| REQ-APP-001.0 | TEST-GPUI-INTEG-003 | integration | missing `OPENAI_API_KEY` or screen permission shows inline remediation and creates no artifacts | startup safety |
| REQ-WIN-001.0 | TEST-GPUI-INTEG-004 | integration | primary window requests floating-window behavior on macOS | overlay |
| REQ-SES-001.0 | TEST-GPUI-UNIT-006 | unit | count-based session launcher runs the requested number of attempts without overlapping starts | control flow |
| REQ-SES-002.0 | TEST-GPUI-UNIT-007 | unit | running state disables inputs and prevents overlapping sessions | control flow |
| REQ-SES-003.0 | TEST-GPUI-INTEG-008 | integration | GUI session calls library probe services directly and persists rows under `.fawkes_probe/` | reuse |
| REQ-RPT-001.0 | TEST-GPUI-INTEG-009 | integration | completed session swaps to a summary view with counts and run id | reporting |
| REQ-RPT-002.0 | TEST-GPUI-UNIT-010 | unit | summary explanation names on-task and off-task totals in plain language | usability |
| REQ-ERR-001.0 | TEST-GPUI-INTEG-012 | integration | recoverable provider failures increment error count without terminating the window | resilience |
| REQ-BIN-001.0 | TEST-GPUI-BUILD-013 | build | GUI target builds successfully as a Rust GPUI executable without webview dependencies | packaging |

## TDD Plan

1. STUB
- Add a new GPUI app module and failing tests for form rendering, validation, count-based launch behavior, summary rendering, and plain-language summary formatting.
- Add a fake in-process session runner so GPUI tests do not need real screen capture or network calls.
- Add a platform window configuration seam so macOS window kind requests can be asserted without visual inspection alone.

2. RED
- Run the GPUI test suite and confirm failures for missing root view, missing validated form model, missing count-based session controller, and missing summary reducer wiring.
- Record which failures belong to view rendering versus probe-service integration.

3. GREEN
- Implement the minimal form view first.
- Implement validated form state and inline error messages second.
- Implement count-based session derivation and a single running-state view third.
- Reuse the existing probe library through an in-process session service fourth.
- Implement the summary view last.

4. REFACTOR
- Keep new symbols to four-word names where practical, such as:
  - `render_minimal_session_form`
  - `validate_session_form_fields`
  - `derive_session_attempt_schedule`
  - `start_overlay_probe_session`
  - `render_running_session_state`
  - `render_session_summary_panel`
  - `format_plain_summary_sentence`
  - `open_floating_probe_window`
- Keep GPUI view state separate from probe execution services.
- Keep macOS window-configuration logic isolated so later menu-bar or packaging work does not leak into session logic.

5. VERIFY
- Run `cargo fmt --all -- --check`.
- Run `cargo clippy --all-targets --all-features -- -D warnings`.
- Run `cargo test --all-targets --all-features`.
- Run a manual macOS smoke test that confirms:
  - the window opens
  - the window behaves like a floating app window
  - the form starts a real timed session
  - the summary appears at the end with raw counts

## Quality Gates

- [ ] Every new requirement has a stable `REQ-*` identifier.
- [ ] Every `REQ-*` identifier appears in the test matrix.
- [ ] The GPUI app reuses library probe services instead of spawning the CLI binary.
- [ ] `.fawkes_probe/` remains the only runtime artifact directory for screenshots and SQLite session data.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes.
- [ ] `cargo test --all-targets --all-features` passes.
- [ ] No new `TODO`, `STUB`, or `FIXME` markers remain in committed app code.
- [ ] No unmeasured claims are made about “always foreground” or all-spaces behavior in the first GUI milestone.
- [ ] The manual smoke test verifies that the session summary includes raw counts and a plain-language sentence.

## Open Questions

1. Should the GUI milestone ship as a second binary during development while keeping the CLI harness, or should we replace the existing binary entrypoint immediately?
2. Should the session begin with an immediate first capture, or should the first capture wait one full interval after pressing `Start`?
3. Do we want the floating window to be dismissible during a session, or must it remain present until the summary is shown?
4. Should the summary show the per-capture descriptions inline, or remain limited to aggregate counts plus one sentence in this milestone?
5. After the floating-window spike works, do we still want a second pass that experiments with `WindowKind::PopUp` for stronger overlay behavior?
