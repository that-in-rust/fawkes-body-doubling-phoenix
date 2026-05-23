# Fawkes Probe Journey 01

## Purpose

This note captures what we actually did to turn the Milestone 0A probe from a PRD into a working Rust CLI, including setup, implementation, verification, live experiments, and the main product learnings.

## Scope We Chose

We deliberately narrowed the first implementation to the smallest useful proof:

- OpenAI only
- full-screen capture only
- one current screenshot plus one declared goal per classification
- SQLite persistence
- terminal summary output

We explicitly deferred:

- GPUI or menu-bar UI
- Gemini or Vertex integration
- multi-provider routing
- nudges and notifications
- prior-capture memory in prompts
- workflow-level cost routing

## Environment And Auth Preparation

Before writing code, we amortized the setup work:

- cloned the Zed reference repo into `reference-repos/` for exploration only
- added top-level ignore rules so `reference-repos/` stays out of tracked code
- installed Google Cloud CLI locally
- completed Google Application Default Credentials setup for future Gemini work
- bound ADC quota project to `project-e86e4307-0fb5-43b0-a2f`
- confirmed Vertex AI reachability for later use
- configured local OpenAI access for the Rust CLI path

Important auth truth:

- OpenAI is unblocked through `OPENAI_API_KEY`
- Gemini is not using `GEMINI_API_KEY` in this environment
- Gemini is a later ADC-based path, not a Milestone 0 blocker

## Documentation Work

We created and refined the planning artifacts under `docs/prd01/`:

- `min01.md` for the short milestone direction
- `spec01.md` for the executable specification
- `journal.md` for setup and implementation notes

We also moved older top-level docs into `docs/archive/` so the active PRD area is cleaner.

## Rust Implementation

We built a new root Rust crate named `fawkes-probe`.

Core structure:

- `src/core/`
  - domain types
  - error types
  - prompt builder
  - summary reducer
- `src/application/`
  - validated runtime config
  - orchestration traits
  - serial run service
- `src/adapters/`
  - OpenAI Responses client
  - macOS screen capture
  - SQLite artifact store
  - time abstraction

Main behavior implemented:

- parse CLI args with `--goal`, `--interval`, `--count`, optional `--model`, optional `--output-dir`
- fail fast on missing `OPENAI_API_KEY`
- fail fast on invalid interval or count
- preflight screen capture readiness
- capture the full screen
- downscale the screenshot so the short side is at most `768px`
- encode as lossy JPEG at quality `75`
- compute SHA-256 for the archived JPEG
- send the image to OpenAI Responses with `detail: low`
- require structured JSON output
- persist one SQLite row per attempt
- print one line per attempt plus a final summary

## Artifact Layout

Runtime artifacts are stored locally under:

- `.fawkes_probe/fawkes_probe.sqlite`
- `.fawkes_probe/runs/<run_id>/captures/*.jpg`

These artifacts are intentionally ignored by git:

- `/.fawkes_probe/` is listed in `.gitignore`

That means probe screenshots and local SQLite runtime output stay available for debugging without polluting the repository.

## Compression And Cost Choices

The first implementation compresses captures in three meaningful ways:

1. downscale screen captures to a short side of `768px`
2. save them as JPEG with quality `75`
3. send them to OpenAI with `detail: low`

This makes the probe cheaper and faster while still preserving enough visual structure for classification.

## Test-Driven Delivery

We implemented the crate with a functional TDD path:

- unit tests for config validation, prompt boundaries, and summary reduction
- integration tests for capture archival, OpenAI request shape, SQLite persistence, recoverable provider failures, token telemetry handling, and preprocessing performance

The main test file is:

- `tests/probe_flow.rs`

## Verification Results

The automated verification gates passed:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `cargo build --all-targets --all-features`

We also removed the last production `expect` from the OpenAI adapter so the shipped code path stays aligned with the reliability bar.

## Live Smoke Runs

### Smoke Run 1

Goal:

- `study Rust`

Result:

- successful end-to-end run
- captured screen
- classified with OpenAI
- stored row in SQLite
- archived one downscaled JPEG
- printed summary

Run id:

- `019e5434-bd8f-7f71-9494-b70a8901805a`

### Smoke Run 2

Goal:

- `study Rust`

Result:

- successful repeat end-to-end run after the final reliability cleanup

Run id:

- `019e5436-bd98-74e0-9247-28106941da7d`

## Three-Minute Experiment

We then ran a 10-capture experiment over roughly three minutes with:

- goal `watch YouTube`
- interval `20` seconds
- count `10`

Run id:

- `019e543c-7cce-7752-a63e-df6a56530347`

Outcome:

- `9` on-task
- `1` off-task
- `0` ambiguous
- `0` errors
- average latency `2614 ms`

Important interpretation:

- the model repeatedly identified the screen as YouTube browsing or video activity
- later screenshot review confirmed that those captured images genuinely showed `youtube.com`
- this was not a hallucination from the classifier
- however, that experiment used the goal `watch YouTube`, so YouTube was treated as on-task by design

That revealed an important product truth:

- the same screenshot can be on-task or off-task depending entirely on the declared goal

## Main Product Learnings

1. The core loop works end to end on this machine.
2. Capturing, compressing, classifying, persisting, and summarizing is viable with a simple Rust CLI.
3. Saved screenshots are essential for debugging model mistakes and validating what the classifier really saw.
4. Goal phrasing matters a lot. A bad or overly permissive goal can make the system look smarter than it is.
5. YouTube-like browsing is a good ambiguity test because it can plausibly be either work or distraction depending on context.
6. The next meaningful product test is not a better UI. It is a stricter misclassification experiment, for example:
   - goal `study Rust`
   - actual activity `browse YouTube`

## Recommended Next Step

Run a fresh 10-capture probe where the declared goal is clearly non-YouTube, then compare:

- model classification
- confidence
- saved captures
- your own human judgment

That will tell us whether the probe is merely functional, or actually useful.
