# Campaign Platform Master Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the approved SRD 5.2.1 campaign platform through six green vertical plans without losing the existing real-Tauri play path.

**Architecture:** The host backend remains the only game authority. Each phase moves behavior inward to `app-domain` or `app-application`, exposes it through narrow adapter contracts, and adds React projections only after backend contracts are green.

**Tech Stack:** Rust, Tokio, Axum, SQLx/SQLite, React 19, TypeScript, Zustand, Tauri 2, Bun, Vitest, Playwright, Python 3.12, FastAPI.

**Spec:** `docs/superpowers/specs/2026-08-13-campaign-platform-srd52-design.md`

## Global Constraints

- Target only SRD 5.2.1; remove 2014-rule compatibility instead of branching on rules version.
- Do not add a production dependency without explicit user approval.
- Tauri is the sole process owner; `dmai-server` requests inference through authenticated loopback runtime control.
- React submits commands and renders projections; it never commits combat facts optimistically.
- Default first-run locale is Russian; machine contracts remain English.
- Never log documents, prompts, chat text, hidden campaign knowledge, secrets, runtime tokens, or base64 media.
- Preserve unrelated dirty-worktree files and stage checkpoint paths explicitly.
- Use CodeGraph before text search when locating or understanding code.
- Report focused tests, full gates, browser tests, real Tauri CDP, packaged smoke, and live-model play as separate evidence.

---

## Rollout order

| Phase | Plan | Independently testable result |
|---|---|---|
| 1 | `2026-08-13-srd52-campaign-foundation-plan.md` | SRD 5.2.1-only aggregates/catalog, versioned campaign/session/event repositories, atomic saves, campaign home and creation shell |
| 2 | `2026-08-13-campaign-documents-dm-memory-plan.md` | Dedicated PDF/DOCX library, scoped retrieval, continuity ledger, proactive DM context, atomic first-world preparation |
| 3 | `2026-08-13-vtt-combat-window-shell-plan.md` | True map contain, fixed shell scrolling, reliable native chrome, expanded authoritative combat |
| 4 | `2026-08-13-zero-touch-runtime-packaging-plan.md` | One-command dev bootstrap, unified artifact manifest, supervised sidecars, packaged smoke |
| 5 | `2026-08-13-session-compute-protocol-plan.md` | Multiplayer-safe projections and in-memory session/compute protocols with complete-model workers |
| 6 | `2026-08-13-full-acceptance-documentation-plan.md` | Deterministic full-session browser/Tauri acceptance, live-model proof, updated docs and release evidence |

Do not start a later plan until the prior plan's checkpoint gate is green and committed. A later plan may add tests to earlier code, but it must not weaken earlier contracts.

## Specification coverage

| Design sections | Owning plan and tasks |
|---|---|
| 4-5: dependency direction and SRD 5.2.1 reset | Phase 1 Tasks 1, 2, 7; Phase 3 Tasks 2-4 |
| 6.1-6.2: routes and campaign creation | Phase 1 Tasks 2-6; Phase 2 Task 7 |
| 6.3: original bundled adventures | Phase 6 Task 1 |
| 7-8: documents, knowledge, continuity, DM behavior | Phase 2 Tasks 1-8 |
| 9: VTT sizing, map preparation, authoritative combat | Phase 3 Tasks 1-4 and 7 |
| 10: multiplayer-ready session protocol | Phase 5 Tasks 1, 2, 5, 6 |
| 11: distributed inference workers | Phase 5 Tasks 3, 4, 6 |
| 12: Tauri shell and window chrome | Phase 3 Tasks 5-7 |
| 13: zero-touch development and distribution | Phase 4 Tasks 1-7; Phase 6 Task 5 |
| 14: errors and privacy | Phase 2 Tasks 1-7; Phase 4 Tasks 4-6; Phase 5 Tasks 1-6 |
| 15: testing and acceptance | Every phase checkpoint; Phase 6 Tasks 2-5 and 7 |
| 16-17: documentation, cleanup, completion | Phase 6 Tasks 6 and 7 |

## Shared checkpoint protocol

- [ ] Before each phase, capture `git status --short` and `git diff --stat`; record unrelated files that must remain untouched.
- [ ] Read the design spec and the phase plan completely.
- [ ] Run the phase baseline commands recorded in that plan.
- [ ] Implement tasks in order using red-green-refactor and one scoped commit per task.
- [ ] Run `bun run architecture:check` after any dependency/import boundary change.
- [ ] Run `bun run gates` at the phase checkpoint.
- [ ] Run the browser or Tauri checks named in the phase plan; do not substitute one class of evidence for another.
- [ ] Stage only files named by the completed task; never use `git add .`.

## Cross-phase contract names

The plans use these stable names so later phases can depend on earlier work:

```rust
pub const GAME_SCHEMA_VERSION: u16 = 1;
pub const RULESET_ID: &str = "srd-5.2.1";

pub struct CampaignId(pub uuid::Uuid);
pub struct SessionId(pub uuid::Uuid);
pub struct Revision(pub u64);

pub struct CampaignSummary {
    pub id: CampaignId,
    pub title: String,
    pub updated_at: String,
    pub last_session_id: Option<SessionId>,
}

pub struct CommandEnvelope<T> {
    pub command_id: uuid::Uuid,
    pub expected_revision: Revision,
    pub payload: T,
}
```

```typescript
export type AppRoute =
  | { kind: 'campaign-home' }
  | { kind: 'campaign-create'; draftId: string }
  | { kind: 'character-create'; campaignId: string }
  | { kind: 'world-preparation'; campaignId: string }
  | { kind: 'game-table'; campaignId: string; sessionId: string };
```

Renaming one of these requires updating this master plan and every not-yet-executed phase plan in the same commit.

## Final completion gate

All six plans are complete only when:

```text
bun run architecture:check
bun run gates
python -m ruff check sidecar
python -m pytest sidecar/tests -q
bun run e2e
bun run e2e:tauri
```

are green in their applicable environments, a packaged clean-install WebView inspection smoke is recorded per target (WebView2 CDP on Windows and the WebKit inspector on Linux/macOS), and a separate live-model session exercises local LLM plus image/video generation or explicitly records unavailable hardware/provider prerequisites.
