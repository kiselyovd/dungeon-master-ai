# The Bell Beneath the Water Implementation Plan

> For agentic workers: execute inline with `superpowers:executing-plans`; RED/GREEN before each implementation boundary. The user's instruction to continue to the end authorizes execution without repeated approval handoffs.

**Goal:** Make the authored adventure selectable and executable, with persistent facts, meaningful choices and reachable authored endings, as the next stage of complete solo and LAN/VPN co-op delivery.
**Architecture:** Pure adventure transitions in app-domain. Application command/port with campaign/session/revision/causation identity. SQLite applies the transition, reward, revision and operation receipt atomically; snapshots capture/restore adventure state. React renders public state and available intentions using existing presentation conventions. DM-only canon stays server-side.
**Tech Stack:** Existing Rust/SQLx/Axum/React/Valibot/i18next; no production dependency additions.
**Spec:** `.ai-factory/RESEARCH.md`: agreed canon, six beats, NPC knowledge, physical topology and three ending predicates.

## Original Request

Продолжай до конца

## Constraints and UI brief

- Continue in the attached foundation worktree; preserve original dirty source and existing mixed hunks.
- Keep dependency direction, locale catalogs, existing buttons/dialogs/tokens and exact server identity/revision checks.
- Bundled source selects `bell-beneath-water`; collaborative campaigns remain usable.
- Show authored scene description, discovered clues, available actions and outcomes within the existing game table. No new visual scale or dependencies.
- Guaranteed clues and safe archive access cannot be lost on a failed check. Peaceful and sealing resolutions are separate; combat cannot restore the ward.
- Gold reward: ten ordinary gold pieces for the confirmed rescue, once per authoritative timeline, tracked in the canonical inventory. No unverified XP award.
- Operation replay returns the exact prior result; changed payload with reused ID is rejected. A stale write makes no changes.
- Hidden NPC knowledge/canon must never enter public state or ordinary player events before discovery.
- Later mechanics/AI recovery, real model evaluation and party/network acceptance remain active work, not replaced by this stage.

## Tasks

### 1. Domain adventure
Files: `crates/app-domain/src/adventure.rs`, `crates/app-domain/src/lib.rs`, `crates/app-domain/tests/bell_adventure.rs`.
Interface: `AdventureState::new()`, `available_actions() -> Vec<Action>`, `apply(Action) -> Result<Transition, AdventureError>`; stable serializable location/fact/ending/action identifiers.
- [x] RED: reachable peaceful/sealed/unresolved endings, archive-before-warehouse, unavailable action rejected without mutation, reward once, ledger independent.
- [x] Implement graph, public discovered facts, requirements and resumable unresolved retreat. Run complete domain suite.

### 2. Authoritative durable command
Files: application models/campaign use case/repository port; `adapter-sqlite/src/adventure.rs`, additive `0003_campaign_adventures.sql`, campaign creation, state loader and snapshot capture/restore.
Interface: `AdventureRepository::act(AdventureCommand) -> CampaignSessionState`; command has exact campaign/session/character/revision/causation/action.
- [x] RED SQLite real campaigns: replay, conflict, ownership mismatch, unknown package, reward, rollback, current load and restore-old.
- [x] Persist bundled initialization and each transition/reward/receipt atomically. Include optional typed adventure in snapshot projection with backward-compatible deserialization.

### 3. HTTP and server context
Files: campaign HTTP service/routes, app-server composition and agent/context_builder; real HTTP integration tests.
- [x] RED POST adventure action contract. Enforce body schema/identity; return full confirmed projection.
- [x] Inject relevant authored canon/state into model context without publishing DM-only knowledge. Invalid actions return stable codes.

### 4. Product selection and play
Files: campaign creation, adventure API/feature, production App composition, shared projection/session restoration, EN/RU catalogs.
- [x] RED bundled create input and adventure action UI: pending/failure retains current state; late A response cannot replace B; confirmed state restores atomically.
- [x] Enable authored bundled choice, scene/action/fact/outcome presentation in existing styles. Persist/reload through server, never local fake progression.

### 5. Acceptance and continuation
- [x] Production build, type/style/architecture, focused and full relevant tests.
- [x] Browser production root: create bundled campaign, rescue, restore ward, finish; sealed and unresolved paths, reload and save/restore.
- [x] Real backend acceptance proves effects and secrets isolation; native UI proof and independent final review before claiming executable delivery.
- [x] Update docs/ledger with exact evidence, local scoped checkpoint and remaining full-goal requirements. Continue to mechanics/AI and LAN/VPN co-op.

## Delivery record

- 2026-10-06: domain tests RED (missing module) to four GREEN authored-route/invariant cases; full domain/application/SQLite suites passed.
- Real SQLite covers retry, changed causation payload, stale revision, wrong character/session, reward once, snapshot rewind and forced receipt-insert rollback of reward/state/revision.
- Real HTTP action route RED404 to GREEN; unknown package and unavailable intention rejected422. Restore delayed-write RED was corrected by allocating a fresh revision and returning a complete current_state (legacy current_revision fallback).
- Independent read-only review found two issues: pending adventure response interrupting a new chat turn; lost restore response losing request identity. Both verified RED and fixed with shared client mutation coordination and a retained restore request that survives saves-screen remount.
- Full gates passed: 1021 Vitest tests in146 files, Rust workspace, Clippy and static/type/style/architecture checks. Production build passed. Full production-browser suite41 passed, one existing vision-flow skip; three adventure cases used real HTTP/SQLite rather than simulated progression.
- Native UI created the bundled campaign and reached the wizard; real API confirmed the previously UI-created fixture hero. Native story actions reached peaceful ending at revision25, with one ten-gold reward. Full native restart retained hero, ending and provider config. Artifacts under .artifacts/bell-adventure. All owned native/backend/preview processes stopped.
- Original source32 baseline hashes unchanged. Scoped checkpoint excludes inherited dirty files and retains the earlier mixed foundation hunks in the attached worktree. The whole attached copy is the verified baseline.
- The full thread goal remains active: real AI-directed recoverable actions, complete solo mechanics/encounter/map, Bonsai acceptance and LAN/VPN party/reconnect/ownership/two-client proof remain required. No release, CI, publication or dependency addition performed.
- Proposed AGENTS rule: in PowerShell, pass rg file patterns via -g rather than wildcard path arguments; advance dependent shell/UI steps only after the prior operation completes successfully.
