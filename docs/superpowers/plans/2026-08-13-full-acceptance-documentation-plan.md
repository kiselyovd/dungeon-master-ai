# Full Campaign Acceptance and Documentation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the SRD 5.2.1 campaign platform with an original bundled adventure, deterministic full-session coverage, real Tauri CDP proof, opt-in live-model evidence, packaged lifecycle verification, and accurate documentation.

**Architecture:** Project-owned adventure content is loaded by a dedicated outbound adapter behind an application-owned catalog port. Acceptance is layered: pure and fixture tests first, browser second, real development Tauri third, packaged Tauri fourth, and live inference last. No layer may substitute for another.

**Tech Stack:** Rust, Axum, SQLx/SQLite, React 19, TypeScript, Tauri 2, Bun, Vitest, Playwright, Python 3.12, FastAPI, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-08-13-campaign-platform-srd52-design.md`

## Global Constraints

- Complete Phases 1-5 and keep their checkpoint commits green before starting this phase.
- Do not add production dependencies without explicit user approval.
- Use only project-owned, generated, public-domain, or permissively licensed content with recorded provenance. Never copy commercial D&D adventures, settings, maps, or art.
- Use fixture/mock/offline acceptance before starting local or cloud models.
- Keep document text, prompts, hidden lore, credentials, and media bytes out of logs, traces, screenshots, and test reports.
- Treat browser, development Tauri, packaged Tauri, CI, and live-model evidence as separate results.
- Preserve unrelated worktree changes and stage named paths only.

---

### Task 1: Ship an original bundled starter adventure behind a content adapter

**Files:**
- Create: `crates/app-application/src/models/adventure.rs`
- Create: `crates/app-application/src/ports/content.rs`
- Modify: `crates/app-application/src/models/mod.rs`
- Modify: `crates/app-application/src/ports/mod.rs`
- Create: `crates/app-application/tests/adventure_catalog.rs`
- Create: `crates/adapter-content/Cargo.toml`
- Create: `crates/adapter-content/src/lib.rs`
- Create: `crates/adapter-content/src/bundled.rs`
- Create: `crates/adapter-content/assets/moonlit-gate/adventure.json`
- Create: `crates/adapter-content/assets/moonlit-gate/ATTRIBUTION.md`
- Create: `crates/adapter-content/tests/bundled_catalog.rs`
- Create: `src/assets/adventures/moonlit-gate/cover.webp`
- Create: `src/assets/adventures/moonlit-gate/map-gatehouse.webp`
- Create: `src/assets/adventures/moonlit-gate/map-crypt.webp`
- Create: `src/assets/adventures/moonlit-gate/scene-gate.webp`
- Create: `src/assets/adventures/moonlit-gate/manifest.json`
- Modify: `Cargo.toml`
- Modify: `crates/app-bootstrap/Cargo.toml`
- Modify: `crates/app-bootstrap/src/wiring.rs`
- Modify: `scripts/architecture-boundaries.json`
- Modify: `src/features/campaigns/CampaignCreate.tsx`
- Create: `src/features/campaigns/AdventureLibrary.tsx`
- Create: `src/features/campaigns/AdventureLibrary.module.css`
- Create: `src/features/campaigns/__tests__/AdventureLibrary.test.tsx`
- Modify: `src/locales/en/campaigns.json`
- Modify: `src/locales/ru/campaigns.json`

**Interfaces:**
- Produces application-owned `BundledAdventureCatalog`, `AdventureManifest`, `AdventureSeed`, and `ContentAttribution` contracts.
- The adapter returns validated content and asset IDs; it cannot write campaign state or expose DM-only sections through player projections.

- [ ] **Step 1: Write catalog, privacy, and attribution contract tests**

Assert unique stable IDs, `ruleset_id = "srd-5.2.1"`, valid map dimensions/grid metadata, valid encounter seeds, locale parity, content hashes, complete attribution, and separate DM/player projections. Assert the adapter rejects absolute paths, parent traversal, undeclared assets, invalid hashes, and unsupported rulesets.

- [ ] **Step 2: Run the focused tests and verify the missing-adapter failure**

Run: `cargo test -p app-application --test adventure_catalog`
Expected: FAIL because the catalog port and models do not exist.

- [ ] **Step 3: Author the starter adventure data**

Create the original one-to-three-session adventure **Moonlit Gate / Врата лунного света** with a player premise, hidden truth, three NPC agendas, two clocks, exploration nodes, social scenes, two SRD 5.2.1 encounter seeds, treasure, consequences, and multiple endings. Keep mechanics as typed IDs/numbers and prose as RU/EN localized fields. Record authorship/license and every adapted SRD reference in `ATTRIBUTION.md`.

- [ ] **Step 4: Produce and verify the visual set**

Inventory existing living-tabletop assets first. Reuse any matching project-owned asset with its existing manifest entry. For missing cover, scene, or VTT maps, invoke the `imagegen` skill, request clean classic-fantasy compositions, and require maps to be top-down with no UI/text baked in. Inspect every output at original resolution, optimize to WebP without changing aspect ratio, and record generator/date/prompt summary/hash in `manifest.json`.

- [ ] **Step 5: Implement the adapter and composition**

Parse the embedded manifest once at bootstrap, validate it through application constructors, and expose the catalog port to the campaign-creation use case. `adapter-content` may depend on `app-application` and shared serde crates; `app-application` may not depend on it.

- [ ] **Step 6: Add the Adventures creation path**

Render the approved full-screen library with existing tokens and icons. Show title, premise, expected length, tone, and attribution. Selecting the adventure creates a draft through the same campaign pipeline and routes to character creation; DM-only content never enters the browser response.

- [ ] **Step 7: Run focused, visual, and boundary checks**

Run: `cargo test -p app-application --test adventure_catalog && cargo test -p adapter-content && bun run test -- src/features/campaigns/__tests__/AdventureLibrary.test.tsx src/assets/__tests__/livingTabletop.test.ts && bun run architecture:check && bun run lint && bun run typecheck && bun run build`
Expected: PASS, with every referenced asset present and every content file attributed.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock crates/app-application/src/models/adventure.rs crates/app-application/src/models/mod.rs crates/app-application/src/ports/content.rs crates/app-application/src/ports/mod.rs crates/app-application/tests/adventure_catalog.rs crates/adapter-content crates/app-bootstrap/Cargo.toml crates/app-bootstrap/src/wiring.rs scripts/architecture-boundaries.json src/assets/adventures src/features/campaigns src/locales/en/campaigns.json src/locales/ru/campaigns.json
git commit -m "feat(content): add an original starter adventure"
```

### Task 2: Build one deterministic full-campaign fixture harness

**Files:**
- Create: `e2e/fixtures/full-campaign.ts`
- Create: `e2e/fixtures/campaign-source.pdf`
- Create: `e2e/fixtures/campaign-source.docx`
- Modify: `e2e/fixtures/tauri-mock.ts`
- Rewrite: `e2e/full-campaign.spec.ts`
- Create: `e2e/campaign-creation-paths.spec.ts`
- Create: `e2e/media-degradation.spec.ts`
- Create: `scripts/lib/full-campaign-fixture.ts`
- Create: `scripts/lib/full-campaign-fixture.test.ts`

**Interfaces:**
- Produces one deterministic fixture transcript shared by browser mocks and the Tauri test backend.
- Every SSE event, event revision, combat projection, document chunk, media job, and save snapshot has a stable ID and explicit expected order.

- [ ] **Step 1: Write the transcript validator first**

Validate unique event IDs, contiguous revisions, legal SSE order, combat command/result pairing, no hidden source fields in player payloads, valid asset hashes, and a final snapshot equal to the replayed projection.

- [ ] **Step 2: Run and verify failure**

Run: `bun run test -- scripts/lib/full-campaign-fixture.test.ts`
Expected: FAIL because the unified fixture does not exist.

- [ ] **Step 3: Implement the fixture from typed contracts**

Cover campaign home, questionnaire creation, PDF creation, DOCX creation, bundled adventure selection, character creation, world preparation, first autosave, exploration, NPC social turn, ability check, map switch, movement, initiative, attack, condition, spell/save, end turn, combat end, illustration, image failure with bundled fallback, video progress/success, journal/NPC updates, manual save, restore, and resume.

- [ ] **Step 4: Rewrite browser acceptance around user-visible behavior**

Assert Russian default and English parity, keyboard/a11y roles, exact screen transitions, complete contain-fit map, chat-only scrolling, no optimistic combat mutation, stable reasoning counter, safe provenance, and absence of room UI. Do not assert internal component structure.

- [ ] **Step 5: Add failure and recovery cases**

Cover malformed document, rejected stale revision, duplicated event, worker lease expiry, late media result, runtime unavailable, failed save transaction, aborted turn, and application reload after a completed save.

- [ ] **Step 6: Run deterministic browser acceptance serially**

Run: `bun run test -- scripts/lib/full-campaign-fixture.test.ts && bun run e2e -- e2e/campaign-creation-paths.spec.ts e2e/full-campaign.spec.ts e2e/media-degradation.spec.ts`
Expected: PASS without any local model, GPU, network provider, or Tauri process.

- [ ] **Step 7: Commit**

```bash
git add e2e/fixtures/full-campaign.ts e2e/fixtures/campaign-source.pdf e2e/fixtures/campaign-source.docx e2e/fixtures/tauri-mock.ts e2e/full-campaign.spec.ts e2e/campaign-creation-paths.spec.ts e2e/media-degradation.spec.ts scripts/lib/full-campaign-fixture.ts scripts/lib/full-campaign-fixture.test.ts
git commit -m "test(e2e): cover the complete campaign fixture"
```

### Task 3: Drive the complete development Tauri session through CDP

**Files:**
- Modify: `scripts/lib/tauri-campaign.ts`
- Modify: `scripts/lib/tauri-campaign.test.ts`
- Rewrite: `scripts/tauri-cdp-campaign.ts`
- Modify: `scripts/tauri-cdp-e2e.ts`
- Modify: `package.json`
- Create: `docs/evidence/tauri-cdp/.gitkeep`

**Interfaces:**
- Produces `bun run e2e:tauri:campaign`, which attaches to the real WebView2 CDP endpoint and returns a nonzero exit code for any failed step.
- Evidence JSON contains timestamps, safe IDs, app/backend/runtime versions, assertions, durations, and screenshot paths; it excludes content and secrets.

- [ ] **Step 1: Extend the CDP driver tests before the live script**

Test target discovery, reconnect after navigation, timeout diagnostics, localized role selectors, window-state assertions, evidence redaction, screenshot naming, and process cleanup.

- [ ] **Step 2: Run and verify the new expectations fail**

Run: `bun run test -- scripts/lib/tauri-campaign.test.ts`
Expected: FAIL until the driver supports the full flow and redacted evidence.

- [ ] **Step 3: Implement the real WebView flow**

With `bun run dev:all` owning startup, attach through CDP and exercise: campaign home; all three source-path entry points; character creation; preparation; map fit; chat scrolling; social turn; check; combat movement/attack/spell/condition/turn; journal/NPC; image fallback; video state; save/restore; minimize; maximize/restore; close interception; and clean shutdown. Use the deterministic backend fixture for this task, not live models.

- [ ] **Step 4: Capture layout evidence at required viewports**

Record screenshots at 1280x720, 1600x900, and one portrait-like narrow window. Assert the document root and shell do not scroll, the chat timeline does, the map remains fully visible, footer/header do not overlap, and native controls remain clickable over every overlay.

- [ ] **Step 5: Run the development Tauri proof**

Start: `bun run dev:all`

Run: `bun run e2e:tauri:campaign`

Expected: PASS against the real Tauri WebView and real backend process lifecycle. Record the command result separately from browser Playwright.

- [ ] **Step 6: Commit**

```bash
git add scripts/lib/tauri-campaign.ts scripts/lib/tauri-campaign.test.ts scripts/tauri-cdp-campaign.ts scripts/tauri-cdp-e2e.ts package.json docs/evidence/tauri-cdp/.gitkeep
git commit -m "test(tauri): drive a complete campaign through CDP"
```

### Task 4: Add opt-in model inventory and live-session verification

**Files:**
- Create: `scripts/lib/model-matrix.ts`
- Create: `scripts/lib/model-matrix.test.ts`
- Create: `scripts/live-model-matrix.ts`
- Rewrite: `scripts/tauri-cdp-play.ts`
- Modify: `package.json`
- Modify: `docs/RELEASE_CHECKLIST.md`

**Interfaces:**
- Produces `bun run models:verify` for installed-model health/capability checks and `bun run e2e:tauri:play` for one opt-in live campaign.
- The matrix result distinguishes `passed`, `failed`, `not_installed`, `unsupported_hardware`, `missing_credentials`, and `skipped_by_policy`.

- [ ] **Step 1: Write model inventory and redaction tests**

Use fixture manifests for multiple LLM, image, and video models. Assert capability matching, bounded timeouts, failure classification, no auto-download, no secret/prompt logging, and deterministic JSON summaries.

- [ ] **Step 2: Run and verify failure**

Run: `bun run test -- scripts/lib/model-matrix.test.ts`
Expected: FAIL because the matrix runner is absent.

- [ ] **Step 3: Implement safe per-model smoke checks**

Discover only models already installed through runtime status APIs. For each LLM, run health plus a minimal structured-output/tool smoke. For each image/video model, run health plus one bounded low-cost generation and validate MIME, dimensions/duration, size, and hash. Do not download models or call paid providers automatically.

- [ ] **Step 4: Extend the live Tauri play script**

Use one explicitly selected healthy LLM and the selected image/video providers to run a short real campaign path through CDP. Verify DM initiative, a generated or fallback map, social interaction, combat, save/restore, illustration, and video. Treat live output semantically, not as an exact transcript.

- [ ] **Step 5: Run opt-in evidence only after fixture and Tauri tests are green**

Run: `bun run models:verify`

Then run: `bun run e2e:tauri:play`

Expected: installed compatible models pass; unavailable hardware/models are reported with the exact safe classification. A live failure does not invalidate deterministic gates, but it blocks claims that live inference is proven.

- [ ] **Step 6: Commit**

```bash
git add scripts/lib/model-matrix.ts scripts/lib/model-matrix.test.ts scripts/live-model-matrix.ts scripts/tauri-cdp-play.ts package.json docs/RELEASE_CHECKLIST.md
git commit -m "test(runtime): verify installed models and live play"
```

### Task 5: Gate packaged desktop lifecycle on every supported target

**Files:**
- Modify: `.github/workflows/build-bundle.yml`
- Modify: `.github/workflows/release.yml`
- Modify: `.github/workflows/test.yml`
- Modify: `scripts/verify-package.ts`
- Modify: `scripts/package-smoke/windows.ps1`
- Modify: `scripts/package-smoke/linux.sh`
- Modify: `scripts/package-smoke/macos.sh`
- Create: `scripts/lib/release-evidence.ts`
- Create: `scripts/lib/release-evidence.test.ts`
- Modify: `package.json`

**Interfaces:**
- Produces a machine-readable release evidence manifest per target containing artifact hashes, staged sidecar versions, cloud/local profile, install result, launch result, health result, and CDP smoke result.

- [ ] **Step 1: Test release-evidence validation**

Reject missing hashes, missing target, mismatched manifest versions, absent external binaries, zero-byte placeholders, unclassified skipped checks, and evidence containing secret-shaped fields.

- [ ] **Step 2: Run and verify failure**

Run: `bun run test -- scripts/lib/release-evidence.test.ts`
Expected: FAIL until the validator exists.

- [ ] **Step 3: Wire fixture and architecture gates into CI**

Run architecture, Rust, frontend, sidecar Ruff/offline pytest, and serial browser fixture jobs before bundle jobs. Keep GPU/live-provider checks opt-in and never mark them green by omission.

- [ ] **Step 4: Verify package contents and lifecycle**

For Windows x64, run `scripts/package-smoke/windows.ps1` and require the WebView2 CDP contract. For Linux x64, run `scripts/package-smoke/linux.sh` under Xvfb and require the WebKit inspector contract. For macOS arm64, run `scripts/package-smoke/macos.sh` and require the WebKit inspector contract. Every job verifies the artifact manifest, installs into an isolated location, observes backend readiness, asserts the campaign-home DOM contract, stops cleanly, verifies no child processes remain, and uploads redacted evidence. Cloud-only bundles explicitly report absent local runtimes as profile policy.

- [ ] **Step 5: Run local workflow-equivalent checks**

Run: `bun run architecture:check && bun run gates && python -m ruff check sidecar && python -m pytest sidecar/tests -q && bun run e2e`
Expected: PASS. Report actual remote CI status later through `gh`, not from local inference.

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/build-bundle.yml .github/workflows/release.yml .github/workflows/test.yml scripts/verify-package.ts scripts/package-smoke scripts/lib/release-evidence.ts scripts/lib/release-evidence.test.ts package.json
git commit -m "ci(release): require packaged lifecycle evidence"
```

### Task 6: Align documentation, architecture maps, and privacy guidance

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `AGENTS.md`
- Modify: `.ai-factory/DESCRIPTION.md`
- Modify: `.ai-factory/ARCHITECTURE.md`
- Modify: `.ai-factory/rules/base.md`
- Modify: `docs/RELEASE.md`
- Modify: `docs/RELEASE_CHECKLIST.md`
- Modify: `sidecar/README.md`
- Create: `docs/CAMPAIGNS.md`
- Create: `docs/DOCUMENTS_AND_PRIVACY.md`
- Create: `docs/DISTRIBUTED_COMPUTE.md`
- Create: `docs/TESTING_EVIDENCE.md`

**Interfaces:**
- Documents the implemented boundaries and exact commands; it does not describe aspirational networking or unverified platform support as shipped.

- [ ] **Step 1: Run a documentation drift audit**

Compare manifests, package scripts, route names, runtime profiles, process ownership, supported targets, environment variables, data locations, and test commands against the current code. Record every stale claim before editing.

- [ ] **Step 2: Update operator and player documentation**

Document campaign paths, dedicated document library, supported formats/limits, SRD 5.2.1 attribution, saves, media fallback, optional worker preferences, zero-touch dev, installer behavior, diagnostics, and privacy boundaries. State that network rooms and model sharding are future work until real transports exist.

- [ ] **Step 3: Update architecture enforcement maps**

Add `adapter-content`, campaign preparation, document extraction, session/compute protocols, and Tauri process supervision to the architecture map. Verify no compatibility facade or deleted legacy module remains documented.

- [ ] **Step 4: Validate docs and links**

Run: `bun run architecture:check && bun run gates:fast`

If the repository link checker has a local command, run it; otherwise confirm the `.github/workflows/link-check.yml` job in CI after publication.

- [ ] **Step 5: Commit**

```bash
git add README.md CHANGELOG.md AGENTS.md .ai-factory/DESCRIPTION.md .ai-factory/ARCHITECTURE.md .ai-factory/rules/base.md docs/RELEASE.md docs/RELEASE_CHECKLIST.md docs/CAMPAIGNS.md docs/DOCUMENTS_AND_PRIVACY.md docs/DISTRIBUTED_COMPUTE.md docs/TESTING_EVIDENCE.md sidecar/README.md
git commit -m "docs: describe the complete campaign platform"
```

### Task 7: Run the final completion gate and record an honest verdict

**Files:**
- Create: `docs/evidence/final-verification.md`
- Modify only when a failing check identifies a product defect: the exact file that owns that defect.

**Interfaces:**
- Produces one evidence ledger separating local focused tests, full gates, browser, development Tauri, packaged Tauri, live models, CI, and publication status.

- [ ] **Step 1: Capture repository state**

Record branch, commit SHA, `git status --short`, relevant tool versions, runtime profile, OS, and detected hardware without secrets or personal paths.

- [ ] **Step 2: Run all deterministic local gates**

Run:

```text
bun run architecture:check
bun run gates
python -m ruff check sidecar
python -m pytest sidecar/tests -q
bun run e2e
```

Expected: PASS. Fix defects at their owning layer and rerun the failed focused test before rerunning the consolidated gate.

- [ ] **Step 3: Run real desktop gates**

Run `bun run e2e:tauri:campaign` against the deterministic backend, then the packaged smoke for the current OS. Record both results independently.

- [ ] **Step 4: Run optional live evidence**

Run `bun run models:verify` and `bun run e2e:tauri:play` only when compatible installed models/hardware or explicitly configured providers are available. Record skips and failures honestly.

- [ ] **Step 5: Check CI through GitHub CLI after a push is explicitly authorized**

Use `gh run list`, `gh run view`, and release/artifact commands. Do not call GitHub MCP. Local green is not CI green, and CI green is not deployment or release proof.

- [ ] **Step 6: Write the evidence ledger and commit it**

Include command, start/end time, exit code, commit SHA, result class, and safe artifact/evidence path for every executed check. Stage `docs/evidence/final-verification.md` explicitly and commit with `test: record campaign platform verification`.

- [ ] **Step 7: Stop on any unproven required gate**

Do not mark the master plan complete while a required deterministic, real-Tauri, or packaged gate is missing or failing. Report the exact blocker and the narrow next action.
