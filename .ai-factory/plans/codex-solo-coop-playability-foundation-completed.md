# Campaign Resume Foundation Implementation Plan

> For agentic workers: use `superpowers:executing-plans` for inline execution, or `superpowers:subagent-driven-development` if the user selects per-task delegation. Run each task's RED/GREEN checks before advancing.

Branch: `codex/solo-coop-playability`
Created: 2026-10-06
Status: foundation completed locally 2026-10-06; independent review findings fixed.

**Goal:** A player creates a hero, plays, saves, closes the app and continues the correct campaign without losing or mixing characters, messages or encounter state.
**Architecture:** Persist the current character at the application/repository boundary and read current campaign projections in one SQLite transaction. React commits a validated complete projection atomically. Explicitly loading a save changes current persisted state; ordinary Continue reads that current state and never silently rewinds to an older save.
**Tech Stack:** Existing Rust workspace, SQLx/SQLite, Axum, React/Zustand, Valibot, Bun/Vitest and Playwright/Tauri. No new production dependency.
**Spec:** `.ai-factory/RESEARCH.md`, approved direction and authored adventure.

## Original Request

Намана, погнали тада

## Settings

- Testing: yes, RED before implementation, full relevant suites before completion.
- Logging: verbose development diagnostics using IDs, transitions and codes only.
- Docs: yes, update repository guides at the final checkpoint through aif-docs.
- Execution preference proposed: inline implementation with one fresh final review.
- Local checkpoint commits only after checks pass; no push, merge, release or deploy.
- Existing dirty source files are copied as baseline into the managed worktree; they remain user-owned and must not be bundled into checkpoint commits.

## Research Context

Source: `.ai-factory/RESEARCH.md`; Updated: 2026-10-06; SHA256: dc332a4d8e2fde17aafd3535d6cb9d22fabf143fba3430cdfa76fa7c6afe7263

Finish a complete solo adventure and then 2-4 player LAN/VPN cooperative play on the same authoritative campaign system.
Use prepared canon, NPC knowledge and explicit consequence rules for Колокол под водой instead of relying on runtime world invention.
Fix campaign/character persistence and coherent restoration before implementing adventure and networking.
Evaluate the installed Bonsai 2 PTQ1_0 independently through an official compatible runtime; do not replace the current provider without real tool/narration acceptance.
Preserve unrelated changes, use current components/tokens, EN/RU catalogs, safe logs and no new production dependencies.

## Scope and subsequent subprojects

This executable plan delivers the campaign-resume foundation, not the entire cooperative game. The ordered continuation is:

1. This plan: durable hero, fresh snapshots, transactional resume, atomic client transitions.
2. Authored-adventure package: immutable canon, visible/hidden facts, scene graph, NPC knowledge, flags, topology, authored endings and playable solo encounter variants.
3. Recoverable AI turns and complete solo mechanics: server context, actual spell/attack actions, actor-token IDs, rest, interrupted-turn recovery and real complete solo acceptance.
4. Bonsai bounded benchmark and optional runtime integration after compatibility/quality evidence.
5. Party and LAN/VPN rooms: versioned participants/characters, ownership, scoped command deduplication, shared event delivery, reconnect and real two-device acceptance.

Each subsequent subproject needs a concrete written plan derived from the interfaces this foundation actually delivers. Do not invent unverified networking/runtime APIs here.

## Global Constraints

- Preserve inward dependencies and composition ownership from `.ai-factory/ARCHITECTURE.md`.
- Extend existing routes; do not rename existing save, campaign, combat or SSE contracts.
- Keep SQLite migrations additive; never edit an already-applied migration or wipe user databases.
- Resume returns current persisted state. It does not restore the newest manual save as an implicit side effect.
- Every character document belongs to one campaign; a global legacy PC cannot be copied into arbitrary campaigns.
- Character documents retain authored backstory/personality/spells as well as current mechanical fields; trust-critical derived combat values remain subject to domain validation.
- A new campaign with no persisted hero returns an explicit empty character projection and enters the existing wizard.
- Switching campaign clears prior transient UI; a failed activation leaves the previous coherent campaign active.
- No new visual tokens. Reuse CampaignHome, CharacterWizard, existing buttons, loading/error conventions and EN/RU strings.
- Do not log model output, prompts, private character biographies, secrets or full projection bodies.
- Browser mocks are evidence of presentation/contracts, never evidence of actual SQLite persistence or local-model quality.
- Generated image data stays out of persisted browser state; durable geometry/media-reference restoration is a separate adventure/tabletop contract and must not be claimed complete here.

## Compact UI brief

Keep the current campaign library and character review layout. Continue is busy while the full state loads; failure leaves the previous selection coherent and offers retry using the existing error treatment. The wizard's final action remains pending until the server confirms character persistence. On error keep the complete draft and show a localized message. No new screen or aesthetic direction is required.

## Review Focus

- Fresh/empty campaign after playing another: no old hero, equipment, NPC, scene or chat survives.
- A request for campaign A finishes after selecting B: neither response nor error/finally from A changes B.
- A save retry returns the original result, but a distinct save at unchanged gameplay revision captures new character/live projection content.
- Restore an older save, then close/reopen: Continue returns the restored state, not a newer historical save.
- Existing legacy local PC with incomplete server evidence: preserve local data and show recovery; never silently assign it to another campaign.

## Canonical interfaces for this plan

New application models live in `crates/app-application/src/models/campaign_state.rs`:

```rust
pub struct CampaignSessionState {
    pub campaign: CampaignDetail,
    pub session_id: SessionId,
    pub character_id: Option<Uuid>,
    pub projection: SnapshotProjection,
}

pub struct CommitCampaignCharacter {
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub character_id: Uuid,
    pub expected_revision: Revision,
    pub causation_id: String,
    pub character: serde_json::Value,
}
```

`CampaignDetail`, `SessionId`, `SnapshotProjection`, `Revision` use their existing models. The character payload uses the complete current frontend sheet document plus preserved narrative/spell fields. Validate identity, document fields, schema and ruleset in application code; do not add frontend dependencies to inner crates.

New port `CampaignStateRepository` in `ports/repositories.rs`:

```rust
async fn load_current(&self, campaign_id: CampaignId)
    -> Result<CampaignSessionState, RepositoryError>;
async fn commit_character(&self, command: CommitCampaignCharacter)
    -> Result<CampaignSessionState, RepositoryError>;
```

New HTTP surface:

- `GET /campaigns/{campaign_id}/state`: read-only complete current state.
- `PUT /campaigns/{campaign_id}/character`: versioned character commit with session identity, UUID character ID, expected revision and causation ID; returns the same state envelope.

Wire state envelope: `schema_version: 1`, `ruleset_id: 'srd-5.2.1'`, `campaign` metadata, `session_id`, nullable `character_id`, `projection` using existing message/combat/scene/journal/NPC shapes. The metadata's revision is authoritative for subsequent writes.

Typed frontend clients in `src/api/campaigns.ts`: `fetchCampaignState(campaignId, signal?)` and `commitCampaignCharacter(campaignId, request, signal?)`. Runtime envelope validation belongs in `src/api/contracts/campaignState.ts`.

UI projection conversion lives in `src/state/campaignProjection.ts`. Features call it through composition-owned adapters; they do not import sibling feature internals. `state.restoration.apply` accepts campaign metadata with the validated projection and updates all relevant state in one notification.

## Task 1: Establish a trustworthy browser and quality baseline

**Files:** modify `e2e/full-campaign.spec.ts`, `src/api/__tests__/agentEvents.test.ts`; record evidence in `.artifacts/solo-coop-foundation/` only.

- [x] Run the existing browser full-campaign test against a new production build and confirm the save request fails its 127.0.0.1-only interceptor while the app uses localhost.
- [x] Replace session/save interception patterns with host-independent exact-path patterns; keep real requests blocked by fixture fallback, rather than passing on a leaked connection failure.

```typescript
await page.route('**/sessions/**', existingSessionHandler);
await page.route('**/saves/**', existingSaveHandler);
```

`existingSessionHandler` and `existingSaveHandler` mean retain the current inline callback bodies when replacing only their route patterns; no new helper is required.

- [x] Format only the touched agentEvents test. Re-run `bunx biome ci .`, `bun run architecture:check`, `bun run test -- --maxWorkers=1`, `bun run build` and selected browser tests. Record pre-existing warnings separately.
- [x] Capture safe baseline hashes and stage only this task's hunks, excluding copied inherited changes.

## Task 2: Correct snapshot freshness and idempotence

**Files:** `crates/adapter-sqlite/src/saves.rs`, a new next-numbered migration under `crates/adapter-sqlite/migrations/`, `crates/app-application/src/models/snapshot.rs`, `src/api/saves.ts`, `src/features/saves/useSaves.ts`; tests in `crates/adapter-sqlite/tests/repository_contracts.rs`, `crates/app-application/tests/campaign_snapshots.rs`, `src/api/__tests__/saves.test.ts` and saves hook tests.

**Decision:** Keep immutable `(campaign_id, revision)` snapshots. For each new causation ID, snapshot creation atomically allocates the next campaign revision and records a checkpoint event. Same causation ID returns the original snapshot only after matching campaign/session identity. Rollback reverses revision/event/save changes together. Clients reconcile the returned revision; they must not keep using the pre-save revision.

- [x] Add a regression that saves hero A, changes HP/inventory and live messages, then creates a distinct save and asserts different snapshot IDs and new content. Add same-causation retry and cross-campaign causation misuse cases.

```rust
assert_ne!(first.snapshot.id, second.snapshot.id);
assert_eq!(second.snapshot.projection.character["hp"], 4);
assert_eq!(retry.snapshot.id, second.snapshot.id);
assert!(second.snapshot.revision.0 > first.snapshot.revision.0);
```

- [x] Run these tests to expected RED with the current same-revision reuse path.
- [x] Implement allocation/event insertion and client revision reconciliation. Do not silently ignore revision conflicts; return the existing stable conflict code.
- [x] Verify rollback when snapshot/save insert fails, legacy save reading and restore identity validation. Run the complete adapter-sqlite and campaign-snapshot suites.

## Task 3: Persist current character and read a coherent current campaign

**Files:** create `crates/app-application/src/models/campaign_state.rs`, `crates/app-application/src/campaigns/state.rs`, `crates/adapter-sqlite/src/campaign_state.rs`, next-numbered migration and `crates/adapter-sqlite/tests/campaign_state_contracts.rs`; modify models/campaigns module exports, repository ports, sqlite exports and save restore persistence.

**Persistence:** Add `campaign_characters` keyed by campaign ID, with stable UUID character ID, complete document JSON, schema/ruleset and updated timestamp; add campaign-scoped character-operation receipts. This is a singular current-hero store for the foundation. Later party migration converts it into one owned character without copying globals into multiple campaigns.

- [x] Build two real SQLite campaigns and test distinct heroes, messages, scenes, encounters, journals and NPCs. Assert load_current never mixes their projections.
- [x] Test no hero -> `{}` plus null character ID; invalid/mismatched document -> typed error; character commit retry -> original result; conflicting expected revision -> conflict.
- [x] Implement `load_current` as one transaction reading current persisted projections, not a frontend fan-out or newest-save selection. Implement character commit plus event/revision update transactionally.
- [x] Backfill complete saved hero documents only for their exact stored campaign/session. If an active restored snapshot exists, use its character; otherwise choose the latest complete eligible checkpoint. Missing/corrupt evidence remains uninitialized, never an invented hero.
- [x] Update explicit save restore so current character changes in the same transaction as other projections. Test restore-old -> load_current equals old restored character/live rows.
- [x] Exercise migration against existing snapshots and a rollback failure. Run `cargo test -p adapter-sqlite` and application campaign tests.

## Task 4: Expose state/character contracts through existing composition

**Files:** `crates/adapter-http/src/routes/campaigns.rs`, `crates/adapter-http/src/services.rs`, `crates/adapter-http/src/lib.rs`, `crates/app-server/src/http_services.rs`, relevant `crates/app-bootstrap/src/wiring.rs`; create `crates/app-server/tests/campaign_state.rs`.

- [x] HTTP regressions cover empty campaign, persisted hero, wrong campaign/session, unsupported ruleset, stale write and immutable GET behavior.
- [x] Add `CampaignHttpService` state and character methods; route them to the application use cases. No SQL, process control or concrete constructor enters adapter-http.
- [x] Verify exactly matching state envelopes for GET and PUT. Never return host settings or secrets in the projection.
- [x] Run HTTP integration tests and `bun run architecture:check`.

## Task 5: Parse and atomically apply campaign transitions

**Files:** create `src/api/contracts/campaignState.ts`, `src/state/campaignProjection.ts`; modify `src/api/campaigns.ts`, `src/state/restoration.ts`, `src/state/pc.ts`, `src/state/session.ts`, `src/features/saves/model/buildRestoredSession.ts`; extend restoration and API contract tests.

- [x] Add RED tests: old inventory/portrait/proficiencies disappear when new sheet omits them; empty character clears the hero; campaign metadata/revision and all slices change in one notification; malformed envelope changes nothing.

```typescript
expect(useStore.getState().pc.inventory).toEqual([]);
expect(useStore.getState().pc.portraitUrl).toBeNull();
expect(useStore.getState().chat.messages).toEqual([]);
expect(notifications).toBe(1);
```

- [x] Export a fresh empty-PC data factory; never reuse mutable default inventory/maps. Move shared projection validation/conversion out of the save feature so state and campaigns do not import sibling feature internals.
- [x] Abort the existing stream before committing replacement state. Clear draft, tool history, save selection/list/error, targeting/AoE, map image, reasoning and loading transients. Preserve settings and native runtime state.
- [x] Save restore and ordinary activation share atomic application, with operation-specific revision checks instead of pretending activation is a restore-by-save-ID.
- [x] Run state/API/save tests, TypeScript and architecture checks.

## Task 6: Guard activation and late asynchronous responses

**Files:** `src/features/campaigns/useCampaigns.ts`, `src/hooks/useSession.ts`, `src/features/saves/useSaves.ts`, `src/state/session.ts`; create campaign-hook tests and extend session/save-hook tests.

- [x] Deferred A/B regression: start A, start B, finish B, finish A; B remains active. Repeat with A rejection and its finally path. Failed B retains coherent A; no half-applied IDs.
- [x] Add a monotonic transition generation plus AbortController; every completion/error/loading commit checks the current generation.
- [x] Subscribe message hydration to actual campaign/session IDs. Backend empty history replaces stale previous history. Ignore obsolete session responses and obsolete save-restore responses.
- [x] Guard refresh/create/clear consistently. Clearing an active campaign is an atomic empty transition, not just pointer removal.
- [x] Verify active streaming cancellation cannot append to the next campaign.

## Task 7: Commit wizard character before entering play

**Files:** `src/components/wizard/ReviewTab.tsx`, `src/state/pc.ts`, `src/state/charCreation.ts`, composition/controller files under `src/app/`, `src/locales/en/wizard.json` and `src/locales/ru/wizard.json`; tests for wizard review and real production root.

- [x] Add RED test: character finalization waits for persisted PUT; failed PUT preserves full draft and remains in wizard; confirmed PUT applies returned projection and only then clears draft.
- [x] Persist narrative history, personality/bonds/flaws and selected spells alongside mechanical sheet data. Assign a stable UUID character ID; do not infer identity from display name.
- [x] Keep API/business orchestration in the feature/application controller; the view invokes an injected command and renders existing busy/error treatment.
- [x] Legacy global PC remains a recoverable local draft until exact active campaign identity is established and migration succeeds. Add tests for no pointer, mismatched pointer and failed migration; never delete the original on failure.
- [x] Use English/Russian locale catalogs and existing styles. Run component tests, full frontend suite, lint and production build.

## Task 8: Prove persisted continuation and record completion

**Files:** `e2e/fixtures/tauri-mock.ts`, `e2e/campaign-home.spec.ts`, new `e2e/campaign-resume.spec.ts`, production-root tests; document via `README.md`/existing testing guide and `CHANGELOG.md`.

- [x] Extend fixtures to provide complete state and durable character behavior; do not seed an unrelated global PC to make Continue pass.
- [x] Browser cases: campaign A -> B -> empty C; delayed old response; failed transition; hero creation -> reload; save -> mutate -> restore -> reload. Assert names, inventory, chat and scene visibly.
- [x] Real backend/Tauri isolated-profile check: create two campaigns, persist two distinct heroes, save twice after a state change, restore the first save, restart, Continue and inspect current persisted state. Do not use internal-store injection as runtime proof.
- [x] Run `bun run gates`, `bun run build`, `bun run e2e`; if tools cannot run, record exact blocker and keep task unchecked. Verify screenshots and explicit process cleanup.
- [x] Update docs and plan with exact commands/revision and separate deterministic browser, real backend, Tauri, model, CI and release evidence.
- [x] Fresh whole-branch review covers identity, transaction rollback, migration retention, state leakage, cancellation and save freshness; fix actionable findings before marking this foundation complete.

## Commit checkpoints

1. Tasks 1-2: `fix(saves): capture fresh revision-bound checkpoints`
2. Tasks 3-4: `feat(campaigns): persist characters and expose current state`
3. Tasks 5-7: `fix(campaigns): restore coherent state before entering play`
4. Task 8: `test(campaigns): prove persisted continuation and document recovery`

Before staging compare against `.artifacts/implementation-baseline.json`; use explicit owned paths/hunks. No broad formatting, no git add ., no publication.

## Execution record

- 2026-10-06: managed worktree created from 36ecc25; 32 current source/context files copied to preserve the actual audited baseline; shared node_modules exposed through a local junction without installation.
- 2026-10-06: read-only backend/frontend exploration confirmed character persistence, stale activation and snapshot freshness gaps.
- Tasks 1-8 complete in the isolated actual baseline. Full gates passed (1012 frontend tests plus Rust workspace checks), production build passed, Playwright 38 passed/1 existing skip, Tauri 8/8 plus real save/restore/cold-restart acceptance. Four independent review findings fixed with regression tests. See docs/CAMPAIGN_STATE.md and the local progress ledger for evidence and limitations.

- Commit ruling: use one integrated foundation checkpoint because snapshot allocation, current-character persistence and frontend revision consumption share interfaces and files; exclude every inherited source hunk. The checked state describes the verified worktree including its preserved user baseline, not an assertion that the base commit alone had no existing changes.
- Browser delayed/error transitions are exercised by deferred hook/root regressions; the real browser suite covers production-root navigation/reload and existing save flows. Real Tauri supplies the restore-old/cold-restart proof. Map geometry, per-action autosave, full mechanics, model inference, CI and co-op remain outside this foundation.
- Staging boundary: three owned hunks depend on pre-existing uncommitted source context (initial wizard wiring in App.tsx, the restore fixture character and formatting an inherited agent-event test). They remain in the attached verified worktree with `.artifacts/retained-owned-hunks.patch`; they are deliberately excluded from the local checkpoint to avoid committing inherited edits. The checkpoint alone is not the complete tested baseline. Keep the attached worktree for continuation.
