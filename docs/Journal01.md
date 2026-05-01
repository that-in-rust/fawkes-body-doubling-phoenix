# Fawkes - Development Journal

---

## Entry 01 | 2026-05-01 | Stack Pivot Exploration: React + Spring Boot + Tauri + Postgres

### The Idea

I want to learn the basics of **React** and **Spring Boot (Java)**. Rather than building a throwaway tutorial app, can I integrate those learnings into Fawkes? This journal entry explores whether we can restructure the architecture so that:

1. The frontend becomes a **React** app (learning React)
2. The backend becomes a **Spring Boot** service (learning Java/Spring)
3. The desktop wrapper is **Tauri** (Rust, which I already know)
4. The data layer uses **PostgreSQL** (instead of SQLite from the original PRD)

### Why This Makes Sense for Fawkes

The original PRD (PRDv001.md) specified Rust + Zed GPUI + SQLite + Ollama. That's a clean native stack but it has drawbacks:

- **GPUI is immature** for macOS system-level APIs (menu bar, ScreenCaptureKit, notifications). The PRD itself flagged this as a risk.
- **SQLite is fine for local-only** but if Fawkes ever becomes multi-device or wants a web dashboard, you need a real database anyway.
- **React** gives us a rich timeline/dashboard UI far faster than building custom GPUI widgets.
- **Spring Boot** gives a clean REST API layer, which decouples the capture engine from the presentation layer.

### Proposed Architecture

```
┌─────────────────────────────────────────────────────┐
│                 Tauri Desktop Shell                  │
│            (Rust -- native macOS layer)              │
│                                                     │
│  ┌───────────────────────────────────────────────┐  │
│  │           React Frontend (webview)            │  │
│  │                                               │  │
│  │  - Session controls (start/stop/pause)        │  │
│  │  - Live activity timeline (D3/Recharts)       │  │
│  │  - Daily/weekly summary dashboards            │  │
│  │  - Settings panel                             │  │
│  │  - Focus mode selector                        │  │
│  └───────────────────┬───────────────────────────┘  │
│                      │ HTTP/REST                     │
│  ┌───────────────────┴───────────────────────────┐  │
│  │     Tauri Rust Backend (screen capture)        │  │
│  │                                               │  │
│  │  - ScreenCaptureKit (via objc FFI)            │  │
│  │  - Screenshot downscaling (image crate)       │  │
│  │  - Ollama VLM classification (HTTP client)    │  │
│  │  - Menu bar icon (system_tray via Tauri)      │  │
│  │  - macOS notifications                        │  │
│  └───────────────────┬───────────────────────────┘  │
│                      │                               │
└──────────────────────┼───────────────────────────────┘
                       │
          ┌────────────┴────────────┐
          │                         │
          ▼                         ▼
┌──────────────────┐    ┌──────────────────┐
│  Spring Boot API │    │     Ollama       │
│  (Java 21+)      │    │  localhost:11434 │
│                  │    │  (VLM inference) │
│  - Activity CRUD │    └──────────────────┘
│  - Session mgmt  │
│  - Timeline query│
│  - Summary stats │
│  - Nudge logic   │
│                  │
│  ┌────────────┐  │
│  │ PostgreSQL  │  │
│  │            │  │
│  │ - sessions │  │
│  │ - events   │  │
│  │ - settings │  │
│  │ - summaries│  │
│  └────────────┘  │
└──────────────────┘
```

### How the Layers Map to Learning Goals

| Layer | Technology | What I Learn | Fawkes Role |
|-------|-----------|--------------|-------------|
| Desktop shell | Tauri (Rust) | Tauri plugin system, system tray, menu bar, native permissions | Screen capture, macOS integration, app lifecycle |
| Frontend | React + TypeScript | Components, hooks, state management, charting libraries | Timeline view, session dashboard, settings UI |
| API layer | Spring Boot (Java) | REST controllers, JPA/Hibernate, service layers, migrations | Business logic, session management, query engine |
| Database | PostgreSQL | Schema design, indexes, time-series queries, migrations | Persistent activity timeline, session history |
| ML inference | Ollama (existing) | Nothing new -- just HTTP client calls | Screenshot classification |

### Is PostgreSQL the Right Choice?

**Yes, for this use case.** Here's why:

1. **Time-series queries are first-class.** Fawkes writes activity events every 30 seconds. PostgreSQL handles time-range queries, window functions, and aggregations natively. SQLite can do it but the queries are uglier and slower at scale.

2. **Spring Boot + JPA + PostgreSQL is the most idiomatic Java stack.** Every Spring tutorial, every production Java system, uses this combo. If the goal is learning Spring Boot "properly," PostgreSQL is the standard pairing.

3. **Postgres app on macOS is trivial.** `brew install postgresql@17 && brew services start postgresql@17`. Zero config for local dev. No Docker needed for MVP.

4. **Future-proof.** If Fawkes ever gets a web dashboard, mobile companion, or multi-device sync, you already have a proper database. SQLite would need a migration path.

**Alternative considered:** SQLite (from original PRD). Simpler, zero-config, embedded. But it doesn't teach me anything about real database management, and it becomes a bottleneck if Fawkes scales beyond single-machine. The whole point of this exercise is learning -- so pick the stack that teaches the most.

**Alternative considered:** Supabase (hosted Postgres). Overkill for local-only MVP. Adds network latency to every write. Revisit if we go multi-device.

### Why Tauri Instead of GPUI

The original PRD specified Zed's GPUI framework. After reviewing the Tauri stack research (see `TauriAppsOSS/docs/control-tower/tauri-stack/`):

1. **Tauri's system tray support is production-ready.** Menu bar icon, click handlers, popover windows -- all built-in via `tauri-plugin-system-tray`. GPUI has none of this.

2. **Tauri wraps a webview** -- which means React renders natively inside the app. No custom widget building needed.

3. **ScreenCaptureKit interop** is still Rust (via `objc2` crate) running in the Tauri backend. The webview doesn't need to touch it.

4. **Tauri v2 has mobile support** -- if Fawkes ever becomes a macOS + iOS companion, the same frontend code works.

5. **Tauri apps are tiny.** The `tauri-apps/tauri` repo digest shows active work on packaging, DMG bundling, and installer polish. Ship-ready.

**Trade-off:** We lose the "100% Rust" purity of GPUI. We gain: shipping faster, learning React, and having a real frontend instead of building UI primitives from scratch.

### The Data Flow

```
Every 30 seconds:
  1. Tauri Rust backend captures screen via ScreenCaptureKit
  2. Downscale to 768px, encode as JPEG
  3. POST to Ollama localhost:11434 → classification JSON
  4. POST to Spring Boot API: POST /api/events { session_id, timestamp, category, app, on_task, confidence }
  5. Spring Boot writes to PostgreSQL
  6. Spring Boot evaluates drift logic → returns { should_nudge: true/false }
  7. If nudge: Tauri fires macOS notification
  8. React frontend polls GET /api/sessions/{id}/timeline → updates dashboard
```

### What I'd Build First (Learning Order)

**Phase 1: Spring Boot + Postgres backend** (Week 1-2)
- Schema: `sessions`, `activity_events`, `focus_profiles`
- REST API: CRUD for sessions, event ingestion, timeline queries
- Drift detection logic in Java
- Learn: Spring Boot starters, JPA entities, repositories, service layer, `@RestController`, Flyway migrations

**Phase 2: React frontend** (Week 3-4)
- Tauri project scaffold (`npm create tauri-app`)
- React app inside Tauri's webview
- Timeline component (activity feed with color-coded bars)
- Session start/stop controls
- Settings form
- Learn: React hooks (`useState`, `useEffect`), component composition, fetching data, Tailwind CSS

**Phase 3: Tauri Rust backend** (Week 5-6)
- ScreenCaptureKit integration via `objc2`
- Ollama HTTP client in Rust
- System tray icon with status colors
- Glue: Rust backend talks to Spring Boot API
- Learn: Tauri commands (`#[tauri::command]`), event system between Rust and webview

### Open Questions

- [ ] Should Spring Boot run as a separate process or embedded in the Tauri app? (Separate is cleaner for learning, embedded is simpler for distribution.)
- [ ] Should the React frontend talk to Spring Boot directly or go through Tauri's command system? (Direct is simpler and more realistic for learning REST.)
- [ ] Which React charting library? Recharts (simple) vs D3 (more learning, more control) vs Visx (middle ground).
- [ ] Postgres schema: flat events table vs partitioned by date? (Flat is fine for MVP.)
- [ ] Authentication: none for local-only? Or add Spring Security to learn that too?

---

*Next entry: TBD -- likely after Phase 1 schema design or after first Spring Boot endpoint is running.*
