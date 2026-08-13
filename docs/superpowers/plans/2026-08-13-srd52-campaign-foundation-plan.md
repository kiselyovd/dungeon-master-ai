# SRD 5.2.1 Campaign Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the development-only 2014 data model with SRD 5.2.1 foundations and introduce a real campaign lifecycle that reaches an independently usable campaign home.

**Architecture:** Pure rules and identifiers live in `app-domain`; campaign lifecycle use cases and ports live in `app-application`; SQLite and HTTP adapt those contracts; React displays a route projection and never lazily invents campaign/session IDs.

**Tech Stack:** Rust, SQLx/SQLite, Axum, React, TypeScript, Zustand, Valibot, Vitest, Playwright.

**Spec:** `docs/superpowers/specs/2026-08-13-campaign-platform-srd52-design.md`

## Global Constraints

- Target only SRD 5.2.1 and schema version 1; delete 2014 compatibility paths.
- Do not add production dependencies.
- Preserve `/agent/turn`, existing SSE names, settings contracts, and Tauri backend readiness while campaign routes are added.
- Russian is the default first-run UI language.
- Stage task paths explicitly because the worktree already contains unrelated edits and untracked directories.

---

### Task 1: Pin the SRD 5.2.1 identity and replace development schema history

**Files:**
- Create: `crates/app-domain/src/ruleset.rs`
- Create: `crates/app-domain/tests/ruleset_tests.rs`
- Modify: `crates/app-domain/src/lib.rs`
- Replace: `crates/adapter-sqlite/migrations/0001_initial.sql`
- Delete: `crates/adapter-sqlite/migrations/0002_m3_journal_npc_srd.sql`
- Delete: `crates/adapter-sqlite/migrations/0003_m4_5_messages.sql`
- Delete: `crates/adapter-sqlite/migrations/0004_m5_save_metadata.sql`
- Delete: `crates/adapter-sqlite/migrations/0005_scenes.sql`
- Delete: `crates/adapter-sqlite/migrations/0006_token_resistances.sql`
- Delete: `crates/adapter-sqlite/migrations/0007_combat_projection_revision.sql`
- Modify: `crates/adapter-sqlite/src/pool.rs`
- Test: `crates/adapter-sqlite/tests/repository_contracts.rs`

**Interfaces:**
- Produces: `GAME_SCHEMA_VERSION: u16 = 1`, `RULESET_ID: &str = "srd-5.2.1"`.
- Produces tables: `campaigns`, `sessions`, `campaign_events`, `campaign_snapshots`, `messages`, `saves`, `scenes`, `combat`, `combat_tokens`, `journal_entries`, `npc_memory`, `srd_chunks`.

- [x] **Step 1: Write the failing ruleset identity test**

```rust
#[test]
fn production_ruleset_is_srd_521_only() {
    assert_eq!(app_domain::ruleset::RULESET_ID, "srd-5.2.1");
    assert_eq!(app_domain::ruleset::GAME_SCHEMA_VERSION, 1);
}
```

- [x] **Step 2: Run the focused test and verify the missing module failure**

Run: `cargo test -p app-domain --test ruleset_tests`
Expected: FAIL because `app_domain::ruleset` does not exist.

- [x] **Step 3: Add the pure ruleset constants and export the module**

```rust
pub const RULESET_ID: &str = "srd-5.2.1";
pub const GAME_SCHEMA_VERSION: u16 = 1;
```

- [x] **Step 4: Replace the migration chain with one development baseline**

The new `0001_initial.sql` must create every current production table plus:

```sql
CREATE TABLE campaigns (
  id TEXT PRIMARY KEY,
  schema_version INTEGER NOT NULL CHECK (schema_version = 1),
  ruleset_id TEXT NOT NULL CHECK (ruleset_id = 'srd-5.2.1'),
  title TEXT NOT NULL,
  premise TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE sessions (
  id TEXT PRIMARY KEY,
  campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  revision INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE campaign_events (
  campaign_id TEXT NOT NULL REFERENCES campaigns(id) ON DELETE CASCADE,
  revision INTEGER NOT NULL,
  event_id TEXT NOT NULL UNIQUE,
  kind TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  causation_id TEXT,
  created_at TEXT NOT NULL,
  PRIMARY KEY (campaign_id, revision)
);
```

Move the final shape of existing tables into this migration; do not keep SQL from superseded migrations in Rust startup code.

- [x] **Step 5: Add a fresh-database schema assertion**

In `repository_contracts.rs`, open an in-memory store and query `PRAGMA table_info(campaigns)` plus `sqlite_master`; assert all required tables exist and the ruleset constraint rejects `srd-5.1`.

- [x] **Step 6: Run schema and domain tests**

Run: `cargo test -p app-domain && cargo test -p adapter-sqlite --test repository_contracts`
Expected: PASS.

- [x] **Step 7: Commit**

```bash
git add crates/app-domain/src/ruleset.rs crates/app-domain/src/lib.rs crates/app-domain/tests/ruleset_tests.rs crates/adapter-sqlite/migrations/0001_initial.sql crates/adapter-sqlite/migrations/0002_m3_journal_npc_srd.sql crates/adapter-sqlite/migrations/0003_m4_5_messages.sql crates/adapter-sqlite/migrations/0004_m5_save_metadata.sql crates/adapter-sqlite/migrations/0005_scenes.sql crates/adapter-sqlite/migrations/0006_token_resistances.sql crates/adapter-sqlite/migrations/0007_combat_projection_revision.sql crates/adapter-sqlite/src/pool.rs crates/adapter-sqlite/tests/repository_contracts.rs
git commit -m "feat(domain): reset storage for SRD 5.2.1"
```

### Task 2: Introduce campaign/session/event application contracts

**Files:**
- Create: `crates/app-application/src/models/identity.rs`
- Create: `crates/app-application/src/models/session.rs`
- Create: `crates/app-application/src/campaigns/mod.rs`
- Create: `crates/app-application/src/campaigns/create.rs`
- Create: `crates/app-application/src/campaigns/list.rs`
- Modify: `crates/app-application/src/models/mod.rs`
- Modify: `crates/app-application/src/lib.rs`
- Modify: `crates/app-application/src/ports/repositories.rs`
- Test: `crates/app-application/tests/campaign_workflow.rs`

**Interfaces:**
- Produces: `CampaignId`, `SessionId`, `Revision`, `CampaignSummary`, `CampaignDetail`, `CampaignSeed`, `CampaignEvent`, `CampaignRepository`, `SessionRepository`, `EventRepository`.
- Produces: `CreateCampaign::execute(CreateCampaignCommand) -> Result<CampaignDetail, CampaignError>`.

- [x] **Step 1: Write repository-independent workflow tests**

Test an in-memory port implementation with this behavior:

```rust
let detail = create.execute(CreateCampaignCommand {
    title: "Серебряная долина".into(),
    premise: "Древняя дорога проснулась".into(),
    seed: CampaignSeed::Collaborative { answers: answers() },
}).await.unwrap();

assert_eq!(detail.ruleset_id, "srd-5.2.1");
assert_eq!(detail.revision, Revision(1));
assert_eq!(list.execute().await.unwrap()[0].id, detail.id);
```

- [x] **Step 2: Run and verify missing-type failures**

Run: `cargo test -p app-application --test campaign_workflow`
Expected: FAIL because the campaign use cases and ports are absent.

- [x] **Step 3: Define IDs and revision as newtypes**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CampaignId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Revision(pub u64);
```

- [x] **Step 4: Define repository contracts with atomic append semantics**

```rust
#[async_trait]
pub trait EventRepository: Send + Sync {
    async fn append(
        &self,
        campaign_id: CampaignId,
        expected: Revision,
        events: Vec<CampaignEvent>,
    ) -> Result<Revision, RepositoryError>;
    async fn since(
        &self,
        campaign_id: CampaignId,
        after: Revision,
    ) -> Result<Vec<CampaignEvent>, RepositoryError>;
}
```

- [x] **Step 5: Implement creation as one application transaction boundary**

Creation must write the campaign, initial session, and `campaign_created` event or write none. Express the required unit-of-work operation as one inward repository method rather than coordinating three fallible writes in React or HTTP.

- [x] **Step 6: Run workflow and contract tests**

Run: `cargo test -p app-application --test campaign_workflow && cargo test -p app-application --test contracts`
Expected: PASS.

- [x] **Step 7: Commit**

```bash
git add crates/app-application/src/models/identity.rs crates/app-application/src/models/session.rs crates/app-application/src/campaigns/mod.rs crates/app-application/src/campaigns/create.rs crates/app-application/src/campaigns/list.rs crates/app-application/src/models/mod.rs crates/app-application/src/lib.rs crates/app-application/src/ports/repositories.rs crates/app-application/tests/campaign_workflow.rs
git commit -m "feat(application): add campaign lifecycle contracts"
```

### Task 3: Implement SQLite campaign unit of work and event revisions

**Files:**
- Create: `crates/adapter-sqlite/src/campaigns.rs`
- Create: `crates/adapter-sqlite/src/events.rs`
- Create: `crates/adapter-sqlite/src/sessions.rs`
- Modify: `crates/adapter-sqlite/src/lib.rs`
- Delete: `crates/adapter-sqlite/src/legacy.rs`
- Test: `crates/adapter-sqlite/tests/campaign_contracts.rs`

**Interfaces:**
- Consumes the ports from Task 2.
- Produces `SqliteStore` implementations with monotonic revision and rollback on conflicts.

- [x] **Step 1: Write contract tests for create/list/get and append conflict**

Include:

```rust
let rev1 = EventRepository::append(&store, id, Revision(0), vec![event("created")]).await?;
assert_eq!(rev1, Revision(1));
let stale = EventRepository::append(&store, id, Revision(0), vec![event("stale")]).await;
assert!(matches!(stale, Err(RepositoryError::Conflict { .. })));
assert_eq!(EventRepository::since(&store, id, Revision(0)).await?.len(), 1);
```

- [x] **Step 2: Run and verify trait-implementation failures**

Run: `cargo test -p adapter-sqlite --test campaign_contracts`
Expected: FAIL because `SqliteStore` lacks the new implementations.

- [x] **Step 3: Implement campaign and session mapping in focused files**

Keep UUID parsing and JSON decoding failures mapped to operation codes; do not log row payloads.

- [x] **Step 4: Implement event append inside `BEGIN IMMEDIATE` transaction**

Read current revision, compare to `expected`, insert ordered event rows, update the session/campaign revision, and commit. Roll back on every mismatch or SQL error.

- [x] **Step 5: Delete the legacy adapter surface and update imports**

Run `rg -n "adapter_sqlite::legacy|mod legacy|LegacyDb" crates` and remove all remaining callers instead of re-exporting them.

- [x] **Step 6: Run adapter tests**

Run: `cargo test -p adapter-sqlite`
Expected: PASS.

- [x] **Step 7: Commit**

```bash
git add crates/adapter-sqlite/src/campaigns.rs crates/adapter-sqlite/src/events.rs crates/adapter-sqlite/src/sessions.rs crates/adapter-sqlite/src/lib.rs crates/adapter-sqlite/src/legacy.rs crates/adapter-sqlite/tests/campaign_contracts.rs
git commit -m "feat(sqlite): persist versioned campaigns and events"
```

### Task 4: Add thin campaign HTTP endpoints and schemas

**Files:**
- Create: `crates/adapter-http/src/routes/campaigns.rs`
- Modify: `crates/adapter-http/src/routes/mod.rs`
- Modify: `crates/adapter-http/src/services.rs`
- Modify: `crates/adapter-http/src/lib.rs`
- Modify: `crates/app-server/src/http_services.rs`
- Test: `crates/adapter-http/tests/campaign_wire_contract.rs`

**Interfaces:**
- Produces `GET /campaigns`, `POST /campaigns`, `GET /campaigns/{id}`.
- Wire responses use `schema_version`, `ruleset_id`, snake_case fields, UUID strings, and RFC 3339 timestamps.

- [x] **Step 1: Write exact wire tests**

Assert:

```json
{
  "schema_version": 1,
  "ruleset_id": "srd-5.2.1",
  "title": "Серебряная долина",
  "premise": "Древняя дорога проснулась",
  "seed": { "kind": "collaborative", "answers": { "genre": ["classic_fantasy"] } }
}
```

and `409` for duplicate/stale commands, `422` for invalid seed values, and safe error bodies without internal messages.

- [x] **Step 2: Run and verify missing route failure**

Run: `cargo test -p adapter-http --test campaign_wire_contract`
Expected: FAIL with route not found.

- [x] **Step 3: Add `CampaignHttpService` methods using application DTOs**

Handlers deserialize/validate, call exactly one service method, and serialize the result. They do not import SQLx or construct repositories.

- [x] **Step 4: Wire the service in `app-server` compatibility composition**

Keep `app_server::router` and existing test support compiling until later cleanup; do not create a second composition root.

- [x] **Step 5: Run HTTP and server tests**

Run: `cargo test -p adapter-http && cargo test -p app-server`
Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add crates/adapter-http/src/routes/campaigns.rs crates/adapter-http/src/routes/mod.rs crates/adapter-http/src/services.rs crates/adapter-http/src/lib.rs crates/adapter-http/tests/campaign_wire_contract.rs crates/app-server/src/http_services.rs
git commit -m "feat(http): expose campaign lifecycle endpoints"
```

### Task 5: Replace lazy session IDs with a campaign route projection

**Files:**
- Create: `src/api/contracts/campaign.ts`
- Create: `src/api/campaigns.ts`
- Create: `src/features/campaigns/model/types.ts`
- Create: `src/features/campaigns/model/route.ts`
- Create: `src/features/campaigns/useCampaigns.ts`
- Create: `src/state/campaigns.ts`
- Modify: `src/state/session.ts`
- Modify: `src/state/useStore.ts`
- Test: `src/api/__tests__/campaigns.test.ts`
- Test: `src/features/campaigns/model/__tests__/route.test.ts`
- Test: `src/state/__tests__/session.test.ts`

**Interfaces:**
- Produces `AppRoute` from the master plan.
- Produces `fetchCampaigns`, `createCampaign`, `fetchCampaign` with Valibot-validated responses.
- Removes automatic UUID minting from `ensureSession`; active IDs come from backend campaign creation/selection.

- [x] **Step 1: Write API schema rejection tests**

Assert invalid `schema_version`, `ruleset_id`, UUID, or missing `last_session_id` shape rejects before store mutation.

- [x] **Step 2: Write pure route tests**

```typescript
expect(resolveAppRoute({ hydrated: true, onboardingComplete: true, active: null }))
  .toEqual({ kind: 'campaign-home' });
expect(resolveAppRoute({ hydrated: true, onboardingComplete: true, active: campaignWithoutHero }))
  .toEqual({ kind: 'character-create', campaignId: campaignWithoutHero.id });
```

- [x] **Step 3: Run focused Vitest and verify failures**

Run: `bun run test -- src/api/__tests__/campaigns.test.ts src/features/campaigns/model/__tests__/route.test.ts src/state/__tests__/session.test.ts`
Expected: FAIL because modules do not exist and `ensureSession` still mints IDs.

- [x] **Step 4: Implement validated transport and pure routing**

API modules may not import Zustand. The route resolver accepts a plain input object and returns a discriminated union.

- [x] **Step 5: Change session state to explicit activation**

Replace `ensureSession()` with:

```typescript
activate: (campaignId: string, sessionId: string) => void;
clear: () => void;
```

Existing agent/save callers must fail closed with a localized `campaign_required` UI state until routed to the game table; they must not mint new IDs.

- [x] **Step 6: Run focused tests, typecheck, and architecture guard**

Run: `bun run test -- src/api/__tests__/campaigns.test.ts src/features/campaigns/model/__tests__/route.test.ts src/state/__tests__/session.test.ts && bun run typecheck && bun run architecture:check`
Expected: PASS.

- [x] **Step 7: Commit**

```bash
git add src/api/contracts/campaign.ts src/api/campaigns.ts src/features/campaigns/model/types.ts src/features/campaigns/model/route.ts src/features/campaigns/model/__tests__/route.test.ts src/features/campaigns/useCampaigns.ts src/state/campaigns.ts src/state/session.ts src/state/useStore.ts src/api/__tests__/campaigns.test.ts src/state/__tests__/session.test.ts
git commit -m "feat(frontend): add explicit campaign routing"
```

### Task 6: Build the approved campaign home and creation shell

**Files:**
- Create: `src/features/campaigns/CampaignHome.tsx`
- Create: `src/features/campaigns/CampaignHome.module.css`
- Create: `src/features/campaigns/CampaignCreate.tsx`
- Create: `src/features/campaigns/CampaignCreate.module.css`
- Create: `src/features/campaigns/components/CampaignHeroCard.tsx`
- Create: `src/features/campaigns/components/CampaignPathCard.tsx`
- Modify: `src/app/App.tsx`
- Modify: `src/app/AppShell.tsx`
- Modify: `src/i18n.ts`
- Create: `src/locales/en/campaigns.json`
- Create: `src/locales/ru/campaigns.json`
- Test: `src/app/__tests__/productionRoot.test.tsx`
- Test: `src/features/campaigns/__tests__/CampaignHome.test.tsx`
- Test: `e2e/campaign-home.spec.ts`

**Interfaces:**
- Consumes `AppRoute` and `useCampaigns` from Task 5.
- Produces Option A home and Option A default creation path without document ingestion yet.

- [x] **Step 1: Extend production-root tests before changing App**

Assert the real `App` renders Russian labels by default:

```typescript
expect(await screen.findByRole('heading', { name: 'Ваши хроники' })).toBeVisible();
expect(screen.getByRole('button', { name: 'Новая кампания' })).toBeVisible();
expect(screen.queryByText(/создать комнату|join room/i)).not.toBeInTheDocument();
```

- [x] **Step 2: Write home behavior tests**

Cover empty library, primary last-campaign hero, continue callback, new campaign path, keyboard focus order, and no modal semantics.

- [x] **Step 3: Run tests and verify current-root failure**

Run: `bun run test -- src/app/__tests__/productionRoot.test.tsx src/features/campaigns/__tests__/CampaignHome.test.tsx`
Expected: FAIL because the production root still opens the VTT/onboarding flow.

- [x] **Step 4: Implement prop-driven home components with existing tokens/icons**

Do not add colors, spacing scales, fonts, icons, or animation constants. Reuse theme variables and `Icons`. Keep all visible strings in RU/EN locale files.

- [x] **Step 5: Implement the four-step creation shell**

Steps are source, campaign tuning, character handoff, and preparation handoff. The source screen defaults to `collaborative`; `document` and `bundled` display honest next-stage placeholders only where the current phase routes to future feature screens, not disabled global navigation.

- [x] **Step 6: Add Playwright campaign-home coverage**

Mock `/campaigns`; verify continue, empty/new flow, Russian default, English switch, no inherited actor identity, and that the screen occupies the root without a VTT behind it.

- [x] **Step 7: Run frontend checkpoint**

Run: `bun run lint && bun run typecheck && bun run build && bun run test && bun run e2e -- e2e/campaign-home.spec.ts`
Expected: PASS.

- [x] **Step 8: Commit**

```bash
git add src/features/campaigns/CampaignHome.tsx src/features/campaigns/CampaignHome.module.css src/features/campaigns/CampaignCreate.tsx src/features/campaigns/CampaignCreate.module.css src/features/campaigns/components/CampaignHeroCard.tsx src/features/campaigns/components/CampaignPathCard.tsx src/features/campaigns/__tests__/CampaignHome.test.tsx src/app/App.tsx src/app/AppShell.tsx src/app/__tests__/productionRoot.test.tsx src/i18n.ts src/locales/en/campaigns.json src/locales/ru/campaigns.json e2e/campaign-home.spec.ts
git commit -m "feat(ui): add campaign home and creation flow"
```

### Task 7: Replace character creation data with an attributed SRD 5.2.1 catalog

**Files:**
- Create: `crates/app-domain/src/compendium/data/srd_5_2_1.yaml`
- Create: `crates/app-domain/tests/srd_521_catalog.rs`
- Modify: `crates/app-domain/src/compendium/types.rs`
- Modify: `crates/app-domain/src/compendium/loader.rs`
- Modify: `crates/app-domain/src/compendium/mod.rs`
- Delete: `crates/app-domain/src/compendium/data/equipment.yaml`
- Delete: `crates/app-domain/src/compendium/data/feats.yaml`
- Delete: `crates/app-domain/src/compendium/data/races_classes_backgrounds.yaml`
- Delete: `crates/app-domain/src/compendium/data/spells_level_0_1.yaml`
- Delete: `crates/app-domain/src/compendium/data/spells_level_2_a_k.yaml`
- Delete: `crates/app-domain/src/compendium/data/spells_level_2_l_z.yaml`
- Delete: `crates/app-domain/src/compendium/data/weapon_properties.yaml`
- Modify: `src/state/charCreationSchema.ts`
- Modify: `src/state/charCreation.ts`
- Modify: `src/hooks/useCharacterDraft.ts`
- Modify: `src/components/CharacterWizard.tsx`
- Modify: `src/components/CharacterSheet.tsx`
- Modify: `src/components/__tests__/CharacterWizard.test.tsx`
- Modify: `src/state/__tests__/charCreation.test.ts`
- Modify: `src/locales/en/character.json`
- Modify: `src/locales/ru/character.json`

**Interfaces:**
- Produces `CharacterCatalog`, `CharacterSpecies`, `CharacterBackground`, `CharacterClass`, and `RulesetProvenance` owned by `app-domain`.
- Every catalog entry carries the fixed `ruleset_id = "srd-5.2.1"`; product-visible names come from locale keys, not copied prose in React.

- [ ] **Step 1: Add catalog identity and unsupported-entry tests**

Build the expected identifier sets from the checked-in SRD source manifest. Assert every species, background, class, feat, spell, weapon property, and mastery used by character creation is present in SRD 5.2.1, is unique, and has source attribution. Assert identifiers found only in the legacy catalog are absent.

- [ ] **Step 2: Run domain tests and verify the legacy catalog fails**

Run: `cargo test -p app-domain --test srd_521_catalog`
Expected: FAIL because the consolidated SRD 5.2.1 catalog and provenance fields do not exist.

- [ ] **Step 3: Import only official SRD 5.2.1 data**

Create one normalized YAML catalog from the official SRD 5.2.1 source. Store identifiers, numeric mechanics, locale keys, page/section provenance, and license metadata; do not copy non-SRD adventure or setting content. Remove the legacy mixed-edition character YAML after all loader callers use the new catalog.

- [ ] **Step 4: Update the character draft schema**

Replace legacy race/background/class assumptions with catalog-backed IDs. Reject a draft whose `rulesetId` is missing or differs from `srd-5.2.1`. Keep only UI state in Zustand; compute HP, proficiency, saves, skills, equipment choices, spell choices, and mastery eligibility from domain/API projections.

- [ ] **Step 5: Update the wizard and sheet tests before rendering changes**

Cover Russian-default visible labels, English parity, keyboard flow, catalog validation, deterministic derived values, and absence of legacy option labels. Do not introduce new visual tokens or icons.

- [ ] **Step 6: Run focused and frontend gates**

Run: `cargo test -p app-domain && bun run test -- src/components/__tests__/CharacterWizard.test.tsx src/state/__tests__/charCreation.test.ts && bun run lint && bun run typecheck && bun run build`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/app-domain/src/compendium/data/srd_5_2_1.yaml crates/app-domain/src/compendium/data/equipment.yaml crates/app-domain/src/compendium/data/feats.yaml crates/app-domain/src/compendium/data/races_classes_backgrounds.yaml crates/app-domain/src/compendium/data/spells_level_0_1.yaml crates/app-domain/src/compendium/data/spells_level_2_a_k.yaml crates/app-domain/src/compendium/data/spells_level_2_l_z.yaml crates/app-domain/src/compendium/data/weapon_properties.yaml crates/app-domain/src/compendium/types.rs crates/app-domain/src/compendium/loader.rs crates/app-domain/src/compendium/mod.rs crates/app-domain/tests/srd_521_catalog.rs src/state/charCreationSchema.ts src/state/charCreation.ts src/hooks/useCharacterDraft.ts src/components/CharacterWizard.tsx src/components/CharacterSheet.tsx src/components/__tests__/CharacterWizard.test.tsx src/state/__tests__/charCreation.test.ts src/locales/en/character.json src/locales/ru/character.json
git commit -m "feat(characters): adopt the SRD 5.2.1 catalog"
```

### Task 8: Make saves and restores revision-aware and atomic

**Files:**
- Create: `crates/app-application/src/models/snapshot.rs`
- Modify: `crates/app-application/src/campaigns/mod.rs`
- Create: `crates/app-application/src/campaigns/snapshots.rs`
- Create: `crates/app-application/tests/campaign_snapshots.rs`
- Modify: `crates/app-application/src/lib.rs`
- Modify: `crates/app-application/src/models/mod.rs`
- Modify: `crates/app-application/src/ports/repositories.rs`
- Modify: `crates/adapter-sqlite/src/saves.rs`
- Modify: `crates/adapter-sqlite/tests/repository_contracts.rs`
- Modify: `crates/adapter-http/src/routes/saves.rs`
- Create: `crates/adapter-http/tests/saves_routes.rs`
- Modify: `src/api/saves.ts`
- Modify: `src/features/saves/model/buildRestoredSession.ts`
- Modify: `src/features/saves/model/__tests__/buildRestoredSession.test.ts`
- Modify: `src/features/saves/useSaves.ts`

**Interfaces:**
- Produces `CampaignSnapshot { campaign_id, session_id, revision, schema_version, ruleset_id, projection }`.
- `CreateSnapshot` and `RestoreSnapshot` operate through one repository transaction and return a complete frontend projection.

- [ ] **Step 1: Write application invariants**

Test rejection of cross-campaign session IDs, future revisions, wrong schema/ruleset IDs, and partial projections. Test duplicate restore commands by causation ID and prove the second call is idempotent.

- [ ] **Step 2: Add repository rollback tests**

Inject a failure after the snapshot row is read but before the active-session pointer is updated. Assert campaign events, active revision, messages, combat, scene, journal, and NPC memory remain unchanged.

- [ ] **Step 3: Run and verify failures**

Run: `cargo test -p app-application --test campaign_snapshots && cargo test -p adapter-sqlite --test repository_contracts`
Expected: FAIL until snapshot contracts and the transactional unit of work exist.

- [ ] **Step 4: Implement snapshot creation and restore**

Snapshot only confirmed event revisions. Restore by validating the envelope, rebuilding the complete projection in memory, then committing the active session/revision change in one SQLite transaction. Never perform an HTTP follow-up fetch after server mutation.

- [ ] **Step 5: Return the full projection from HTTP**

Keep the existing save route names where compatible, but change the versioned response schema to include session, messages, character, combat, scene, journal, and NPC projections plus revision. Validate it in `src/api/saves.ts` before one atomic Zustand update.

- [ ] **Step 6: Run backend, frontend, and architecture checks**

Run: `cargo test -p app-application --test campaign_snapshots && cargo test -p adapter-sqlite --test repository_contracts && cargo test -p adapter-http --test saves_routes && bun run test -- src/features/saves/model/__tests__/buildRestoredSession.test.ts && bun run architecture:check`
Expected: PASS with no partial writes on any failure path.

- [ ] **Step 7: Commit**

```bash
git add crates/app-application/src/models/snapshot.rs crates/app-application/src/campaigns/mod.rs crates/app-application/src/campaigns/snapshots.rs crates/app-application/src/lib.rs crates/app-application/src/models/mod.rs crates/app-application/src/ports/repositories.rs crates/app-application/tests/campaign_snapshots.rs crates/adapter-sqlite/src/saves.rs crates/adapter-sqlite/tests/repository_contracts.rs crates/adapter-http/src/routes/saves.rs crates/adapter-http/tests/saves_routes.rs src/api/saves.ts src/features/saves/model/buildRestoredSession.ts src/features/saves/model/__tests__/buildRestoredSession.test.ts src/features/saves/useSaves.ts
git commit -m "feat(saves): restore campaign revisions atomically"
```

### Task 9: Phase checkpoint

**Files:**
- Modify only if required by failures: files already owned by Tasks 1-8.

**Interfaces:**
- Produces the stable foundation consumed by every later phase.

- [ ] **Step 1: Run the consolidated architecture and test gate**

Run: `bun run architecture:check && bun run gates`
Expected: PASS. Existing Biome warnings may be reported, but new warnings in changed files must be fixed.

- [ ] **Step 2: Run browser acceptance**

Run: `bun run e2e`
Expected: PASS, including campaign home and the unchanged deterministic combat authority tests.

- [ ] **Step 3: Run real Tauri root smoke with CDP**

Start: `bun run dev:all`

Then run: `bun run e2e:tauri`

Expected: real WebView shows Russian campaign home, backend readiness, no onboarding flash after hydration, and working existing window buttons.

- [ ] **Step 4: Record checkpoint commit if test-driven fixes were required**

Stage each corrected file explicitly, verify `git diff --cached --name-only`, then commit with `test(campaigns): close foundation acceptance gaps`.
