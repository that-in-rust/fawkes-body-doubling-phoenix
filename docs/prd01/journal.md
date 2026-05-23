# Fawkes Probe Journal

## 2026-05-23

### What we already prepared

- Created the executable spec scaffold in [spec01.md](/Users/neetipatni/Desktop/fawkes-body-doubling-phoenix/docs/prd01/spec01.md).
- Added a top-level project direction focused on the non-UI probe loop before any menu-bar work.
- Installed Google Cloud CLI locally and verified `gcloud` is available.
- Completed Google Application Default Credentials setup for local development.
- Bound ADC quota project to Google Cloud project `project-e86e4307-0fb5-43b0-a2f`.
- Verified Vertex AI is reachable with ADC by listing available locations, including `global`.
- Set local Google environment variables:
  - `GOOGLE_CLOUD_PROJECT=project-e86e4307-0fb5-43b0-a2f`
  - `GOOGLE_CLOUD_LOCATION=global`
- Stored an OpenAI API key locally for shell use.

### What is amortized now

- We do not need to rediscover how Gemini auth works in this environment.
- We do not need to guess the real Google Cloud project ID.
- We do not need to install `gcloud` again.
- We do not need to decide whether OpenAI auth is browser-session based for the CLI path.

### Important implementation truth

- OpenAI is the simplest first provider for Milestone 0.
- Gemini is available later through ADC, not through `GEMINI_API_KEY`.
- The first real product proof is still:
  - capture one screenshot
  - downscale it
  - classify it against a declared goal
  - persist the result
  - print a small summary

### Scope correction

- The earlier spec grew wider than the first discovery loop needed.
- Multi-provider support, route abstraction, retry economics, and workflow cost analysis are valuable, but they are not required to prove the first product assumption.
- The next spec revision should be narrower:
  - OpenAI only
  - full-screen capture only
  - current screenshot plus current goal only
  - SQLite persistence
  - terminal summary

### Milestone 0A implementation result

- Implemented a root Rust CLI crate `fawkes-probe` with layered `core`, `application`, and `adapters` modules.
- Implemented full-screen capture, downscale, JPEG archival, SHA-256 hashing, OpenAI Responses classification, SQLite persistence, and terminal summaries.
- Archived captures now live under `.fawkes_probe/runs/<run_id>/captures/`.
- The SQLite database now lives at `.fawkes_probe/fawkes_probe.sqlite`.
- Automated verification passed:
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all-targets --all-features`
  - `cargo build --all-targets --all-features`
- A live `count=1` smoke test succeeded end to end with OpenAI using model `gpt-4.1-mini`.
- The first successful smoke-test run produced:
  - run id `019e5434-bd8f-7f71-9494-b70a8901805a`
  - one persisted SQLite row
  - one archived downscaled JPEG capture
  - one terminal summary with token and latency reporting
- In this local environment, the shell may require `source ~/.zshrc` before running the probe if `OPENAI_API_KEY` is not already present in the current process environment.

### Remaining unknowns

- Whether active app and window metadata is easy enough to include in Milestone 0.
- Whether screenshots should persist by default for debugging or only behind a debug flag.
- What human-agreement threshold we want to call "good enough" after the first ten-capture run.

### GPUI count-only spike result

- Added a second binary entrypoint at `src/bin/fawkes_overlay.rs` without changing the existing CLI behavior in `src/main.rs`.
- Built a super-minimal GPUI floating-window app with:
  - task input
  - interval-seconds input
  - count input
  - start button
  - running state
  - final summary state
- Reused the existing probe library in-process through a new session launcher seam instead of spawning the CLI binary.
- Added a programmatic config path so the GUI can build a validated `ProbeRunConfig` directly.
- Added a one-line plain-language summary formatter for the GUI result panel.
- Verified runtime artifacts still land in the existing ignored area:
  - `.fawkes_probe/fawkes_probe.sqlite`
  - `.fawkes_probe/runs/<run_id>/captures/`
- Automated verification passed with the GUI included:
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all-targets --all-features`
  - `cargo build --all-targets --all-features`
- Verified the overlay binary launches locally with:
  - `cargo run --bin fawkes_overlay`
- One local build truth mattered here:
  - GPUI needed the `runtime_shaders` feature on this machine because the system has Apple Command Line Tools but not the full Xcode `metal` shader compiler.
