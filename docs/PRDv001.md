# Fawkes - Body Doubling Phoenix

## Product Requirements Document (PRD)

**Repo:** that-in-rust/fawkes-body-doubling-phoenix
**Stack:** Rust + Zed GPUI (native macOS GUI)
**Model:** Local LLM via Ollama (must fit in 16 GB RAM)
**Target Platform:** macOS 13+ (Ventura and up)

---

## 1. Problem Statement

People with ADHD, executive dysfunction, or general focus difficulties struggle to stay on task during study or deep-work sessions. Existing Pomodoro and focus timers require manual start/stop and provide no real feedback on what you're actually doing. There is no always-on companion that watches your screen, understands your activity, and gently nudges you back when you drift.

## 2. Product Vision

Fawkes is a macOS menu-bar companion that silently observes your screen activity, classifies what you're doing in real-time using a local vision-language model, maintains a timeline of your session, and intervenes with gentle nudges when it detects you've drifted from your stated focus task. Like a loyal phoenix, it's always there when you need it.

## 3. Core Concept

- **Menu-bar app** (not invisible/stealth -- macOS actively resists hidden screen capture)
- **Periodic screenshots** via Apple ScreenCaptureKit (requires Screen Recording permission)
- **Local VLM classification** via Ollama (no cloud, no data leaves the machine)
- **Activity timeline** -- continuous log of classified activities per session
- **Focus nudge** -- gentle alert when drift is detected from declared focus mode

## 4. Feasibility Assessment

| Dimension | Rating | Notes |
|-----------|--------|-------|
| MVP feasibility | 8/10 | ScreenCaptureKit + Ollama + simple classifier is well-proven |
| Polished production app | 6.5/10 | Edge cases in classification accuracy, battery impact, permission UX |
| Stealth/invisible mode | Not viable | macOS aggressively requires user consent for screen capture |

## 5. Functional Requirements

### 5.1 Screen Capture Engine

- Use Apple **ScreenCaptureKit** (WWDC 2022+) for high-performance capture
- Capture at configurable interval (default: every 30 seconds)
- Downscale screenshots to **512-768 px short side** before sending to VLM (latency optimization)
- Never store raw full-res screenshots -- only downscaled + classification result
- Respect Screen Recording permission lifecycle (prompt, denial, revocation)

### 5.2 Activity Classification (Local VLM)

- Use **Ollama** as the local inference runtime
- Model must fit comfortably in 16 GB RAM (alongside macOS overhead)
- Candidate models: `llama3.2-vision` (11B), `minicpm-v` (8B), `moondream2` (1.8B)
- Classification prompt returns: `{ activity_category, app_name, is_on_task, confidence }`
- Activity categories: `studying`, `coding`, `reading`, `writing`, `browsing`, `social_media`, `gaming`, `video`, `other`
- **Do NOT use the VLM as an OCR engine** -- it's unreliable for text extraction. Use macOS native APIs (Vision framework) for OCR if text content is needed.

### 5.3 Session Management

- User declares a focus session: "I'm going to study Rust for 2 hours"
- Fawkes sets the target activity category
- Tracks start time, duration, drift events, recovery events
- Session ends when user stops it or timer expires

### 5.4 Activity Timeline

- Persistent timeline of classified activities per session
- Stores: `{ timestamp, activity_category, app_name, is_on_task, confidence }`
- Visual timeline view (simple bar chart or horizontal strip)
- Daily/weekly summary: % time on-task vs off-task
- Export as JSON or CSV

### 5.5 Nudge System

- Triggers when `is_on_task == false` for N consecutive captures (default: 3, i.e., ~90 seconds of drift)
- Nudge delivery options:
  - macOS notification (silent, non-intrusive)
  - Subtle menu-bar icon change (e.g., color shift from green to amber to red)
  - Optional audio chime
- Nudge message is contextual: "You've been on YouTube for 5 minutes. Back to Rust?"
- Nudge frequency caps (max 1 per 5 minutes, configurable)

### 5.6 Menu Bar Interface

- Minimal menu-bar presence (small phoenix icon)
- Click to open: current session status, activity timeline, settings
- Status states:
  - **Idle** (grey) -- no active session
  - **Focused** (green) -- on-task
  - **Drifting** (amber) -- off-task but within tolerance
  - **Alert** (red) -- sustained drift, nudge triggered
- Session start/stop controls
- Quick-focus buttons: "Study mode", "Coding mode", "Reading mode"

### 5.7 Settings / Configuration

- Screenshot interval (15s / 30s / 60s / custom)
- Drift tolerance (consecutive off-task captures before nudge)
- Nudge style (notification / icon-only / audio)
- Focus categories (user-definable)
- Ollama model selection
- Data retention (7 days / 30 days / 90 days / forever)
- Auto-delete screenshots after classification (privacy)

## 6. Non-Functional Requirements

### 6.1 Privacy

- All processing is local. Zero network calls for classification.
- Screenshots are ephemeral -- deleted after classification unless user opts in to retention.
- Activity timeline stores only metadata, never raw screen content.
- Clear privacy disclosure on first launch explaining what's captured and how.

### 6.2 Performance

- Screenshot + classify cycle must complete in < 3 seconds on Apple Silicon M1+.
- CPU impact < 5% average during monitoring (spikes only during inference).
- Memory footprint < 500 MB for the app itself (Ollama process is separate).
- Battery impact must be minimal -- Ollama inference is bursty, not sustained.

### 6.3 Reliability

- Graceful handling of Ollama not running (prompt to start it).
- Graceful handling of permission denial (explain + link to System Settings).
- No data loss on crash -- timeline is append-only SQLite.
- Auto-recovery after sleep/wake cycles.

## 7. Technical Architecture

```
┌─────────────────────────────────────────────────┐
│                   Fawkes App                     │
│                 (Rust + GPUI)                    │
│                                                  │
│  ┌──────────┐  ┌───────────┐  ┌──────────────┐  │
│  │  Menu Bar │  │ Timeline  │  │   Settings   │  │
│  │    UI     │  │   View    │  │     Panel    │  │
│  └────┬─────┘  └─────┬─────┘  └──────┬───────┘  │
│       │              │               │           │
│  ┌────┴──────────────┴───────────────┴───────┐  │
│  │              Session Manager               │  │
│  │   (state machine: idle → focused → drift) │  │
│  └────────────────┬──────────────────────────┘  │
│                   │                              │
│  ┌────────────────┴──────────────────────────┐  │
│  │           Screen Capture Engine            │  │
│  │  (ScreenCaptureKit via objc FFI)          │  │
│  │  → downscale → classify → store metadata  │  │
│  └────────────────┬──────────────────────────┘  │
│                   │                              │
│  ┌────────────────┴──────────────────────────┐  │
│  │          Ollama VLM Client                │  │
│  │  (HTTP to localhost:11434)                │  │
│  │  Model: minicpm-v / llama3.2-vision       │  │
│  └───────────────────────────────────────────┘  │
│                                                  │
│  ┌───────────────────────────────────────────┐  │
│  │        SQLite Timeline Store               │  │
│  │  (append-only activity events)            │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
         │
         ▼
   Ollama Daemon (separate process)
   localhost:11434
```

### Key Technology Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| GUI framework | Zed GPUI | Native macOS, Rust-native, good performance |
| Screen capture | ScreenCaptureKit (SCK) | Apple's recommended API, high-performance, per-window capture |
| VLM inference | Ollama | Battle-tested local runtime, model management, HTTP API |
| Timeline storage | SQLite (rusqlite) | Append-only, zero-config, fast reads for timeline view |
| macOS interop | objc2 crate | Rust bindings for ScreenCaptureKit and NSStatusBar |
| Image processing | image crate | Downscale, convert to PNG/JPEG before VLM submission |
| Notifications | macOS UserNotifications | Native notification framework |

### Screenshot Processing Pipeline

```
ScreenCaptureKit
  → raw CGImage (full resolution)
  → downscale to 768px short side (image crate)
  → encode as base64 JPEG
  → POST to Ollama /api/generate with VLM model
  → parse JSON response { category, on_task, confidence }
  → append event to SQLite
  → delete downscaled image from memory
  → update session state machine
  → trigger nudge if drift threshold met
```

## 8. MVP Scope (v0.1)

- [ ] Menu-bar icon with session start/stop
- [ ] Screen Recording permission flow
- [ ] Screenshot capture at 30s intervals via ScreenCaptureKit
- [ ] Downscale + classify via Ollama (single model: minicpm-v or llama3.2-vision)
- [ ] Activity timeline stored in SQLite
- [ ] Drift detection (3 consecutive off-task classifications)
- [ ] macOS notification nudge
- [ ] Session summary at end

## 9. v0.2 Scope (Post-MVP)

- [ ] Visual timeline view in menu-bar popover
- [ ] Daily/weekly summary charts
- [ ] Multiple focus mode presets
- [ ] Configurable nudge style
- [ ] Auto-start with macOS (Login Items)
- [ ] Apple Shortcuts integration
- [ ] Export timeline data

## 10. v1.0 Scope (Production)

- [ ] Polished GPUI interface with animations
- [ ] Activity category training/customization
- [ ] Calendar integration (show focus sessions in Calendar.app)
- [ ] Focus score / streak tracking
- [ ] Multiple user profiles
- [ ] Comprehensive onboarding flow

## 11. Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| VLM misclassification | High | Medium | Confidence thresholding, user correction feedback loop |
| Ollama not installed/running | High | High | Clear setup guide, auto-detect, prompt to install |
| Screen Recording permission denied | Medium | High | Graceful UX explaining why, link to Settings |
| Battery drain from frequent inference | Medium | Medium | Configurable interval, pause on battery below threshold |
| GPUI immaturity for macOS APIs | Medium | Medium | Fallback to Tauri or Swift UI if GPUI blocks |
| macOS blocking background capture | Low | High | Stick to menu-bar app pattern, not background-only |

## 12. Success Metrics

- Classification accuracy > 85% for on-task vs off-task (user-reported)
- Screenshot-to-classification latency < 3 seconds on M1
- Zero data sent to external servers
- App memory < 500 MB (excluding Ollama)
- User retains the app for > 7 days (stickiness)

---

*Generated from ChatGPT conversation: https://chatgpt.com/share/69f44be5-092c-8322-a75a-77684d782497*
*Summarized and structured as PRD for the Fawkes project.*
