**Repo:** that-in-rust/fawkes-body-doubling-phoenix
**Stack:** Rust + Zed GPUI (native macOS GUI)
**Model:** Gemini APIs
**Target Platform:** Apple M2 onwards

---

## 1. Problem Statement

People with ADHD, executive dysfunction, or general focus difficulties struggle to stay on task during study or deep-work sessions. Existing Pomodoro and focus timers require manual start/stop and provide no real feedback on what you're actually doing. There is no always-on companion that watches your screen, understands your activity, and gently nudges you back when you drift.

## 2. Product Vision

Fawkes is a macOS menu-bar companion that silently observes your screen activity, classifies what you're doing in real-time using a vision-language model, maintains a timeline of your session, and intervenes with gentle nudges when it detects you've drifted from your stated focus task. Like a loyal phoenix, it's always there when you need it.

## 3. Core Concept

- **Menu-bar app** (not invisible/stealth -- macOS actively resists hidden screen capture)
- **Periodic screenshots** via Apple ScreenCaptureKit (requires Screen Recording permission)
- **Activity timeline** -- continuous log of classified activities per session
- **Focus nudge** -- gentle alert when drift is detected from declared focus mode
