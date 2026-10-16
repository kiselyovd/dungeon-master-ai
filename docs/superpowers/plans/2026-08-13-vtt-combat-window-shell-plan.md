# VTT, SRD 5.2.1 Combat, and Window Shell Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a non-cropping full-size VTT, expanded SRD 5.2.1 tactical rules, authoritative combat commands, stable scrolling, and window controls that work over every modal.

**Architecture:** Map view transforms remain local presentation state while terrain, tokens, action economy, and encounter state remain server projections. A root-level `WindowChrome` owns native window interactions independently from application overlays.

**Tech Stack:** Rust domain/application crates, Axum, React, TypeScript, CSS Modules/shared tokens, Tauri 2, Vitest, Playwright, CDP.

**Spec:** `docs/superpowers/specs/2026-08-13-campaign-platform-srd52-design.md`

## Global Constraints

- The server is the only combat authority; no optimistic HP, position, turn, round, condition, or resource mutation.
- Reuse current tokens, icons, typography, radii, shadows, and animation timings.
- The shell does not scroll; only chat timelines, modal bodies, and explicit lists may scroll.
- Window controls must work in the real Tauri WebView while every major modal is open.
- Do not add a production dependency.

---

### Task 1: Add pure contain-fit and view reconciliation

**Files:**
- Create: `src/features/combat/model/mapViewport.ts`
- Test: `src/features/combat/model/mapViewport.test.ts`
- Modify: `src/components/VttCanvas.tsx`
- Modify: `src/styles/vtt.css`

**Interfaces:**
- Produces `fitMapToViewport(image, viewport, padding) -> MapTransform`.
- Produces `reconcileMapTransform(previous, change) -> MapTransform`.

- [ ] **Step 1: Write failing geometry tests**

```typescript
expect(fitMapToViewport({ width: 1600, height: 900 }, { width: 800, height: 800 }, 16))
  .toEqual({ scale: 0.48, x: 16, y: 184 });
expect(fitMapToViewport({ width: 900, height: 1600 }, { width: 800, height: 800 }, 16))
  .toEqual({ scale: 0.48, x: 184, y: 16 });
```

Use floating-point close assertions where exact binary values differ. Cover zero dimensions, extreme aspect ratios, viewport resize, map replacement, and user-modified transforms.

- [ ] **Step 2: Run and verify the missing-module failure**

Run: `bun run test -- src/features/combat/model/mapViewport.test.ts`
Expected: FAIL because `mapViewport.ts` does not exist.

- [ ] **Step 3: Implement contain scale and centering**

```typescript
const usableWidth = Math.max(0, viewport.width - padding * 2);
const usableHeight = Math.max(0, viewport.height - padding * 2);
const scale = Math.min(usableWidth / image.width, usableHeight / image.height);
return {
  scale,
  x: (viewport.width - image.width * scale) / 2,
  y: (viewport.height - image.height * scale) / 2,
};
```

Return the identity transform for invalid dimensions and expose `userModified: false` in reconciliation state rather than inferring it from scale.

- [ ] **Step 4: Integrate intrinsic image loading and ResizeObserver**

`VttCanvas` records `naturalWidth/naturalHeight`, observes its own viewport, fits on first valid pair, and refits on map replacement. Resize preserves a user-modified view; the explicit Fit button resets it.

- [ ] **Step 5: Run focused tests and component suite**

Run: `bun run test -- src/features/combat/model/mapViewport.test.ts src/components/__tests__/VttCanvas.test.tsx`
Expected: PASS. If the component test file does not exist, create it in this task and assert image load plus Fit button behavior.

- [ ] **Step 6: Commit**

```bash
git add src/features/combat/model/mapViewport.ts src/features/combat/model/mapViewport.test.ts src/components/VttCanvas.tsx src/components/__tests__/VttCanvas.test.tsx src/styles/vtt.css
git commit -m "fix(vtt): fit complete maps to the viewport"
```

### Task 2: Expand SRD 5.2.1 tactical values and resolution

**Files:**
- Create: `crates/app-domain/src/combat/terrain.rs`
- Create: `crates/app-domain/src/combat/cover.rs`
- Create: `crates/app-domain/src/combat/concentration.rs`
- Create: `crates/app-domain/src/combat/mastery.rs`
- Create: `crates/app-domain/src/combat/objectives.rs`
- Modify: `crates/app-domain/src/combat/mod.rs`
- Modify: `crates/app-domain/src/combat/types.rs`
- Modify: `crates/app-domain/src/combat/resolver.rs`
- Modify: `crates/app-domain/src/combat/conditions.rs`
- Test: `crates/app-domain/tests/tactical_srd52_tests.rs`

**Interfaces:**
- Produces `TerrainRegion`, `Cover`, `Concentration`, `WeaponMastery`, `EncounterObjective`, `OpportunityWindow`, and typed result events.
- Consumes only deterministic RNG ports already present in `app-domain`.

- [ ] **Step 1: Write failing rules tests**

Add exact examples for half cover AC/save bonus, three-quarters cover, difficult terrain movement cost, prone melee/ranged interaction, concentration save DC, opportunity attacks, and at least the SRD 5.2.1 weapon masteries used by the bundled starter equipment.

```rust
assert_eq!(Cover::Half.ac_bonus(), 2);
assert_eq!(Cover::ThreeQuarters.dex_save_bonus(), 5);
assert_eq!(concentration_dc(22), 11);
assert_eq!(movement_cost(10, Terrain::Difficult), 20);
```

- [ ] **Step 2: Run and verify missing types**

Run: `cargo test -p app-domain --test tactical_srd52_tests`
Expected: FAIL because the modules are absent.

- [ ] **Step 3: Implement pure tactical values before resolver branches**

Use enums/newtypes with exhaustive matches. Mechanical tables carry `srd-5.2.1` source comments and tests; no human-visible localized strings live in the domain.

- [ ] **Step 4: Extend resolution with typed preconditions and result events**

Movement validates bounds, terrain cost, remaining movement, occupied space, and opportunity windows. Attacks derive cover and condition modifiers. Concentration changes emit explicit start/check/end events. Objectives emit progress/completion events.

- [ ] **Step 5: Run full domain tests**

Run: `cargo test -p app-domain`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/app-domain/src/combat crates/app-domain/tests/tactical_srd52_tests.rs
git commit -m "feat(combat): add SRD 5.2.1 tactical rules"
```

### Task 3: Extend authoritative combat commands and map specification

**Files:**
- Create: `crates/app-application/src/models/map.rs`
- Create: `crates/app-application/src/combat/map.rs`
- Modify: `crates/app-application/src/combat/commands.rs`
- Modify: `crates/app-application/src/combat/resolve.rs`
- Modify: `crates/app-application/src/combat/projection.rs`
- Modify: `crates/app-application/src/models/combat.rs`
- Modify: `crates/app-application/src/agent/tool_decoder.rs`
- Modify: `crates/app-application/src/agent/tools.rs`
- Test: `crates/app-application/tests/combat_authority.rs`

**Interfaces:**
- Produces `MapSpec`, `TerrainRegionSpec`, `ObstacleSpec`, `SpawnZone`, `EncounterObjectiveSpec`.
- Extends `CombatCommand` with `Interact`, `Ready`, `Dodge`, `Disengage`, `UseItem`, and `ResolveReaction`.
- Every command uses `CommandEnvelope<CombatCommand>` and returns a new `Revision` or a typed rejection.

- [ ] **Step 1: Write stale, duplicate, and tactical command tests**

Assert duplicate command ID returns the original acknowledgement, stale expected revision returns `stale_revision`, rejected movement emits no events, and successful commands monotonically increment revision.

- [ ] **Step 2: Write map-spec validation tests**

Reject negative/out-of-bounds polygons, zero grid scale, spawn zones outside traversable bounds, duplicate token IDs, and objectives referring to missing regions.

- [ ] **Step 3: Run and verify failures**

Run: `cargo test -p app-application --test combat_authority`
Expected: FAIL.

- [ ] **Step 4: Implement idempotent command handling**

Check command ID before resolution; validate expected revision; resolve domain events; atomically append events and update projection; persist the acknowledgement keyed by command ID.

- [ ] **Step 5: Implement map preparation validation and bundled fallback selection**

The application receives a generated or bundled image reference plus geometry, validates it, and chooses a bundled map asset ID when the media result is absent/degraded. Do not embed image bytes in domain events.

- [ ] **Step 6: Run application tests**

Run: `cargo test -p app-application`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/app-application/src/models/map.rs crates/app-application/src/models/combat.rs crates/app-application/src/models/mod.rs crates/app-application/src/combat crates/app-application/src/agent crates/app-application/tests/combat_authority.rs
git commit -m "feat(application): extend authoritative tactical combat"
```

### Task 4: Map combat HTTP and frontend projections

**Files:**
- Modify: `crates/adapter-http/src/routes/combat.rs`
- Modify: `crates/adapter-http/src/services.rs`
- Modify: `crates/app-server/src/http_services.rs`
- Modify: `src/features/combat/model/types.ts`
- Modify: `src/features/combat/model/commands.ts`
- Modify: `src/features/combat/model/combatProjection.ts`
- Modify: `src/features/combat/useCombatCommands.ts`
- Modify: `src/state/combat.ts`
- Modify: `src/components/ActionBar.tsx`
- Modify: `src/components/VttCanvas.tsx`
- Test: `crates/adapter-http/tests/combat_wire_contract.rs`
- Test: `src/features/combat/model/commands.test.ts`
- Test: `src/features/combat/__tests__/useCombatCommands.test.ts`

**Interfaces:**
- Produces exact typed combat command wire DTOs and authoritative snapshot responses.
- Frontend state accepts `replaceProjection(snapshot)` and pending command metadata only.

- [ ] **Step 1: Add exact backend wire tests**

Assert request includes `command_id`, `expected_revision`, `actor_id`, and tagged payload; response includes `accepted`, `revision`, optional rejection code, and projection. Assert no base64 map bytes.

- [ ] **Step 2: Add frontend rejection/reconciliation tests**

Cover pending move, rejection without local movement, duplicate revision ignored, newer projection replaces all mechanical fields, and pending entry clears only on matching acknowledgement or snapshot reconciliation.

- [ ] **Step 3: Run tests and verify failures**

Run: `cargo test -p adapter-http --test combat_wire_contract && bun run test -- src/features/combat/model/commands.test.ts src/features/combat/__tests__/useCombatCommands.test.ts`
Expected: FAIL until DTOs and reducers are extended.

- [ ] **Step 4: Implement thin HTTP command mapping**

The route calls one combat service method; it does not inspect SQL rows or reproduce tactical rules.

- [ ] **Step 5: Remove remaining optimistic mechanics from React**

Search `src/` for direct HP, turn, round, action economy, and token-position mutations. Replace callers with commands or local visual transforms. Delete obsolete mutation methods once zero callers remain.

- [ ] **Step 6: Make ActionBar projection-driven**

Render legal resources and suggested actions from projection capabilities. Free-form input sends an intent through the same agent/application command path.

- [ ] **Step 7: Run frontend/backend checkpoint**

Run: `cargo test -p adapter-http && bun run test && bun run typecheck && bun run architecture:check`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/adapter-http/src/routes/combat.rs crates/adapter-http/src/services.rs crates/adapter-http/tests/combat_wire_contract.rs crates/app-server/src/http_services.rs src/features/combat src/state/combat.ts src/components/ActionBar.tsx src/components/VttCanvas.tsx
git commit -m "feat(combat): reconcile tactical commands from the server"
```

### Task 5: Extract reliable root WindowChrome

**Files:**
- Create: `src/app/window/WindowChrome.tsx`
- Create: `src/app/window/WindowChrome.module.css`
- Create: `src/app/window/WindowControls.tsx`
- Create: `src/app/window/ResizeBorders.tsx`
- Create: `src/app/window/useModalOpen.ts`
- Modify: `src/app/App.tsx`
- Modify: `src/app/bootstrap.tsx`
- Modify: `src-tauri/capabilities/default.json`
- Test: `src/app/window/__tests__/WindowControls.test.tsx`
- Test: `src/app/__tests__/productionRoot.test.tsx`

**Interfaces:**
- Produces one root-mounted native-aware chrome.
- Consumes existing Tauri window APIs only.

- [ ] **Step 1: Write window action and modal interaction tests**

Mock `getCurrentWindow`; assert minimize, toggle maximize, close, `isMaximized`, resize listener cleanup, and pointer-down propagation stop. Render an open modal and prove the controls remain present and clickable.

- [ ] **Step 2: Run and verify the missing component failure**

Run: `bun run test -- src/app/window/__tests__/WindowControls.test.tsx src/app/__tests__/productionRoot.test.tsx`
Expected: FAIL.

- [ ] **Step 3: Implement the proven text2bim pattern using project tokens**

Mount `ResizeBorders`, modal-only drag strip, and fixed controls once in `bootstrap.tsx` or the top composition root. Buttons use `data-tauri-drag-region={false}`, `pointer-events: auto`, and `onPointerDown={stopPropagation}`.

- [ ] **Step 4: Remove inline titlebar native actions from App**

Keep product navigation in the application titlebar, but native controls come only from `WindowChrome`. Ensure no drag region overlays interactive product buttons.

- [ ] **Step 5: Verify capability parity**

Assert `allow-minimize`, `allow-toggle-maximize`, `allow-close`, `allow-start-dragging`, `allow-start-resize-dragging`, and `allow-is-maximized` exist. Add only missing Tauri capability entries.

- [ ] **Step 6: Run frontend tests and build**

Run: `bun run test -- src/app/window/__tests__/WindowControls.test.tsx src/app/__tests__/productionRoot.test.tsx && bun run lint && bun run typecheck && bun run build`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/app/window src/app/App.tsx src/app/bootstrap.tsx src/app/__tests__/productionRoot.test.tsx src-tauri/capabilities/default.json
git commit -m "fix(shell): keep native window controls above overlays"
```

### Task 6: Enforce shell scrolling and fixed modal regions

**Files:**
- Modify: `src/app/AppShell.tsx`
- Modify: `src/styles/shell.css`
- Modify: `src/ui/Modal.module.css`
- Modify: `src/components/ChatPanel.module.css`
- Modify: `src/components/SettingsModal.tsx`
- Modify: `src/components/SavesScreen.tsx`
- Modify: `src/styles/saves.css`
- Test: `src/app/__tests__/layoutInvariants.test.tsx`
- Test: `e2e/shell-layout.spec.ts`

**Interfaces:**
- Produces stable `100dvh` root and named scroll containers.

- [ ] **Step 1: Write DOM/CSS invariant tests**

Assert shell root has `data-testid="app-shell"`, chat timeline `data-scroll-region="chat"`, modal body `data-scroll-region="modal-body"`, and no scroll attribute on the root, titlebar, statusbar, or composer.

- [ ] **Step 2: Write Playwright viewport tests**

At 960x640 and 1280x800, inject a long chat and long settings content; assert `document.documentElement.scrollHeight === document.documentElement.clientHeight`, chat scrolls, modal title/footer remain within viewport, and VTT bounds do not move when chat scrolls.

- [ ] **Step 3: Run and verify current overflow failure**

Run: `bun run test -- src/app/__tests__/layoutInvariants.test.tsx && bun run e2e -- e2e/shell-layout.spec.ts`
Expected: at least the long-content shell assertion fails before CSS changes.

- [ ] **Step 4: Implement complete min-height/overflow chain**

Set `html`, `body`, root, app shell, main split, VTT pane, and chat pane to the necessary fixed/min-zero layout. Remove global viewport scrolling and localize overflow to explicit bodies.

- [ ] **Step 5: Pin modal and saves title/footer regions**

Use flex/grid rows `auto minmax(0,1fr) auto`; body owns overflow. Do not use `position: fixed` inside the dialog for footer pinning.

- [ ] **Step 6: Run layout tests and full frontend checks**

Run: `bun run test -- src/app/__tests__/layoutInvariants.test.tsx && bun run e2e -- e2e/shell-layout.spec.ts && bun run lint && bun run typecheck && bun run build`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/app/AppShell.tsx src/styles/shell.css src/ui/Modal.module.css src/components/ChatPanel.module.css src/components/SettingsModal.tsx src/components/SavesScreen.tsx src/styles/saves.css src/app/__tests__/layoutInvariants.test.tsx e2e/shell-layout.spec.ts
git commit -m "fix(layout): isolate chat and modal scrolling"
```

### Task 7: Real Tauri combat and window acceptance

**Files:**
- Modify: `e2e/combat-authority.spec.ts`
- Modify: `scripts/tauri-cdp-campaign.ts`
- Modify: `scripts/lib/tauri-campaign.ts`
- Test: `scripts/lib/tauri-campaign.test.ts`

**Interfaces:**
- Proves real native controls and tactical authority through CDP.

- [ ] **Step 1: Extend deterministic combat browser coverage**

Cover contain-fit, difficult terrain rejection, pending movement, cover display, concentration break, opportunity reaction, objective completion, stale revision, and chat-only scrolling.

- [ ] **Step 2: Extend the Tauri CDP campaign script**

Open onboarding/settings/saves/document modals and invoke minimize/maximize/restore checks with safe recovery. Run a fixture encounter and assert position/HP/turn do not change before backend events.

- [ ] **Step 3: Run script unit tests**

Run: `bun run test -- scripts/lib/tauri-campaign.test.ts`
Expected: PASS.

- [ ] **Step 4: Run full gates and browser tests**

Run: `bun run architecture:check && bun run gates && bun run e2e`
Expected: PASS.

- [ ] **Step 5: Run real Tauri CDP acceptance**

Start: `bun run dev:all`

Run: `bun run e2e:tauri`

Expected: window controls work over every tested modal, root never scrolls, map fits, and tactical commands reconcile only after authoritative revisions.

- [ ] **Step 6: Commit**

```bash
git add e2e/combat-authority.spec.ts scripts/tauri-cdp-campaign.ts scripts/lib/tauri-campaign.ts scripts/lib/tauri-campaign.test.ts
git commit -m "test(tauri): prove VTT combat and window shell"
```
