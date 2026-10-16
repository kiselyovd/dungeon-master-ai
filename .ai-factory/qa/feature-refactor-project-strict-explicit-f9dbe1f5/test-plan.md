## Test Plan: Character Creation and First Game Session

**Date:** 2026-08-12
**Branch / Version:** feature/refactor-project-strict-explicit-architecture
**Environment:** isolated local Tauri development profile, real WebView2 over CDP

---

### 1. Testing Goal

Verify that the full desktop vertical works after the architecture and visual refactor: Tauri infrastructure launches, a first-run user creates a complete character, and the first campaign action reaches a real local model and is persisted by the real backend.

---

### 2. Test Scope

**In Scope:**

- Real Tauri/WebView2 process and backend startup.
- First-run onboarding and advanced character wizard.
- PC state derivation and Tauri Store persistence.
- Local mistralrs runtime control and provider activation.
- Campaign/session identifier creation, `/agent/turn` SSE, rendered DM reply, and SQLite message persistence.

**Out of Scope:**

- Cloud providers and real third-party credentials.
- GPU image/video generation and live combat resolution.
- Installer and release bundle validation.

---

### 3. Test Types

| Type | Priority | Area |
|---|---|---|
| Functional | High | Character creation and first playable turn |
| Integration | High | Tauri, backend, local runtime, HTTP/SSE, SQLite |
| Regression | High | Production root, overlays, persistence |
| Negative | Medium | Startup/runtime error visibility |

---

### 4. Test Data

| Category | Data | Purpose |
|---|---|---|
| Character | `Aldric Emberward`, Fighter, Human, Acolyte, standard array | Complete deterministic PC projection |
| Model | `Qwen3-0.6B (ISQ Q4K, text-only)` selected from the real Local LLM UI | Smallest known-compatible local narration model |
| Player action | `I enter the Lantern & Stag tavern, greet the innkeeper, and ask about trouble near the old watchtower.` | First narrative turn |
| Persistence | `sqlite::memory:` plus isolated Tauri identifier | Prevent mutation of the user's normal profile |

---

### 5. Preconditions

- [ ] Local Tauri, WebView2, Rust, Bun, mistralrs binary, and Python sidecar environment are available.
- [ ] CDP is enabled on port 9222.
- [ ] The test uses an isolated Tauri identifier and in-memory SQLite.
- [ ] The staged mistralrs runtime can resolve the public Qwen3-0.6B model.

---

### 6. Acceptance Criteria

- [ ] All high-priority cases pass.
- [ ] The created PC has the selected identity and positive HP/HP max.
- [ ] A real local runtime becomes ready and `/settings/v2` succeeds.
- [ ] The first `/agent/turn` succeeds and a non-empty DM reply renders.
- [ ] The backend contains the test session messages and the WebView reports no uncaught exceptions.

---

### 7. Plan Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Local model startup is slow or incompatible with the staged runtime | High | Use the smallest already-downloaded model and capture runtime/backend diagnostics. |
| Existing user persistence contaminates first-run state | High | Override the Tauri identifier and use an in-memory database. |
| Media startup consumes excessive resources | Medium | Character/session acceptance does not require generation; record media state separately. |

### 8. Checklist

| Check | Priority |
|---|---|
| Real production shell and backend health | High |
| Full character wizard commit | High |
| Local provider configuration and runtime ready | High |
| First agent turn and persisted messages | High |
| Screenshot and uncaught exception capture | Medium |
