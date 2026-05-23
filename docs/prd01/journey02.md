# Fawkes Overlay Journey 02

## Purpose

This note captures the latest phase of the project: turning the first working GPUI overlay into a tighter, more testable experience with a fixed 15-second capture cadence, count-only controls, better summary UX, and visible per-capture explanations.

## What changed in this phase

The overlay moved from a more general form to a sharper probe runner:

- kept the working CLI path unchanged
- kept the overlay as a second Rust binary
- removed editable interval input from the overlay
- fixed overlay cadence at `15 seconds`
- kept default count at `6`
- added visible UI copy so users know the cadence is fixed
- added scrollable post-run summary lines in the format:
  - `timestamp - description - verdict`
- kept those lines backed by the same `RunSummary` that is built from SQLite-loaded rows

## Why this is a good space

This is now a better product-testing shape than the earlier overlay:

- less user confusion about timing
- fewer controls to misconfigure
- clearer explanation of what the model thought happened
- better debugging because the UI summary now reflects actual stored capture results
- easier qualitative review because the post-run output reads like a short timeline instead of only aggregate counters

## Implementation commentary

The implementation stayed disciplined:

- no CLI behavior change
- no second persistence path
- no new provider abstraction work
- no extra database reads just for the overlay
- no loss of the existing `.fawkes_probe/` artifact layout

The main architectural decision that paid off here was keeping the overlay on top of the existing library path instead of treating it like a separate product. That let us refine the UI without rewriting capture, classification, persistence, or summary logic.

## Verification commentary

The overlay refinement was completed with the same reliability bar as the probe itself:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `cargo build --all-targets --all-features`

We also launched the overlay from the built tree with the real local environment and verified that it stayed up cleanly.

## Latest 20 stored results

Source:

- `.fawkes_probe/fawkes_probe.sqlite`
- ordered by newest first

Timestamps below are stored in UTC exactly as recorded by the probe.

1. `2026-05-23T11:23:27.548721+00:00`  
   `ambiguous`, `other`, confidence `0.30`, run `019e5491-ec78-72f0-954f-7e8759d15880`  
   `No code editor window or coding activity visible in the screenshot; only Fawkes Overlay is open.`

2. `2026-05-23T11:23:08.416243+00:00`  
   `off_task`, `browsing`, confidence `0.95`, run `019e5491-ec78-72f0-954f-7e8759d15880`  
   `User is actively using WhatsApp web for messaging, not using a code editor to study.`

3. `2026-05-23T11:22:49.386533+00:00`  
   `off_task`, `browsing`, confidence `0.95`, run `019e5491-ec78-72f0-954f-7e8759d15880`  
   `User is viewing an event management webpage, not using a code editor or studying.`

4. `2026-05-23T11:22:29.965321+00:00`  
   `off_task`, `browsing`, confidence `0.90`, run `019e5491-ec78-72f0-954f-7e8759d15880`  
   `User is viewing a website about Wandler Labs, not using a code editor for study.`

5. `2026-05-23T11:22:10.641476+00:00`  
   `off_task`, `video`, confidence `0.90`, run `019e5491-ec78-72f0-954f-7e8759d15880`  
   `User is on YouTube homepage with video thumbnails, not using code editor for studying.`

6. `2026-05-23T11:21:49.944478+00:00`  
   `on_task`, `studying`, confidence `0.95`, run `019e5491-ec78-72f0-954f-7e8759d15880`  
   `Overlay summary shows continuous interaction with code editor and documentation aligned with study goal.`

7. `2026-05-23T11:21:10.863219+00:00`  
   `on_task`, `studying`, confidence `0.90`, run `019e548f-3485-77f1-a17e-6fb1b5d9a0bc`  
   `User is reading technical documentation and code in a code editor environment, aligned with studying code.`

8. `2026-05-23T11:20:31.512534+00:00`  
   `on_task`, `coding`, confidence `0.95`, run `019e548f-3485-77f1-a17e-6fb1b5d9a0bc`  
   `The user is interacting with code editor interface, working on code commits, and reviewing code documentation related to a project.`

9. `2026-05-23T11:19:53.053252+00:00`  
   `on_task`, `studying`, confidence `0.90`, run `019e548f-3485-77f1-a17e-6fb1b5d9a0bc`  
   `User is working with code and related documentation in an editor and chat interface aligned with a study goal via a code editor.`

10. `2026-05-23T11:19:32.181891+00:00`  
   `on_task`, `studying`, confidence `0.90`, run `019e548f-3485-77f1-a17e-6fb1b5d9a0bc`  
   `The user is viewing and interacting with code repository content and documentation, consistent with studying via code editor.`

11. `2026-05-23T11:19:11.984465+00:00`  
   `on_task`, `coding`, confidence `0.90`, run `019e548f-3485-77f1-a17e-6fb1b5d9a0bc`  
   `User is interacting with code-related content and files (Rust source files, markdown specs) within a code editor environment, matching the stated focus on studying via code editor.`

12. `2026-05-23T11:18:51.781511+00:00`  
   `ambiguous`, `other`, confidence `0.30`, run `019e548f-3485-77f1-a17e-6fb1b5d9a0bc`  
   `Screenshot shows a monitoring tool with task label but no visible code editor or study activity on screen.`

13. `2026-05-23T10:48:26.027678+00:00`  
   `ambiguous`, `browsing`, confidence `0.40`, run `019e5470-733e-7451-ad38-5de9aad40238`  
   `User is on a Google search page, no clear sign of active studying or related tasks.`

14. `2026-05-23T10:47:36.568956+00:00`  
   `off_task`, `social_media`, confidence `0.90`, run `019e5470-733e-7451-ad38-5de9aad40238`  
   `User is browsing a social media feed (X/Twitter) which does not align with the stated study goal.`

15. `2026-05-23T10:47:02.066082+00:00`  
   `on_task`, `studying`, confidence `0.90`, run `019e5470-733e-7451-ad38-5de9aad40238`  
   `The user is in 'Developer mode' on ChatGPT with a focus on Codex, indicating study or exploration of that tool.`

16. `2026-05-23T10:46:26.586234+00:00`  
   `on_task`, `studying`, confidence `0.90`, run `019e5470-733e-7451-ad38-5de9aad40238`  
   `User is reading technical specifications and studying code related to 'Codex' with command line and documentation visible.`

17. `2026-05-23T10:45:51.925074+00:00`  
   `on_task`, `studying`, confidence `0.90`, run `019e5470-733e-7451-ad38-5de9aad40238`  
   `User is reading technical documentation and code related to Codex project in the screenshot.`

18. `2026-05-23T10:45:16.222408+00:00`  
   `ambiguous`, `other`, confidence `0.50`, run `019e5470-733e-7451-ad38-5de9aad40238`  
   `No clear evidence of studying or coding activity visible; overlay suggests session running but content not shown`

19. `2026-05-23T09:52:18.376644+00:00`  
   `on_task`, `browsing`, confidence `0.80`, run `019e543c-7cce-7752-a63e-df6a56530347`  
   `User is on the YouTube homepage, browsing videos but not currently playing any video.`

20. `2026-05-23T09:51:53.899082+00:00`  
   `on_task`, `browsing`, confidence `0.90`, run `019e543c-7cce-7752-a63e-df6a56530347`  
   `User is on the YouTube homepage browsing video thumbnails, no video currently playing.`

## Read on the latest data

The last 20 stored results are already useful:

- the system is clearly separating strongly on-task coding/studying moments from obviously off-task browsing and social use
- the overlay itself can still generate ambiguous frames when it dominates the screen
- the model explanations are now detailed enough that a user can sanity-check them without opening SQLite directly
- the mixed run around `019e5491-ec78-72f0-954f-7e8759d15880` is especially valuable because it contains both on-task and off-task transitions in a short window

## Recommended next step

Use the current overlay for a few more intentional sessions with sharply different goals, then compare:

- what the UI summary says
- what SQLite stored
- what the screenshots actually show
- whether the probe feels fair or annoying in edge cases

That is the right next filter before adding anything larger than this UI.
