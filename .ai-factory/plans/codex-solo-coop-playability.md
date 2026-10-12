# Implementation Plan: Reliable, Unified and Visually Rich Desktop Game

Branch: `codex/solo-coop-playability`
Created: 2026-10-12
Status: in progress; Tasks 1-5 verified; Task 6 deterministic native/SQLite proof verified, real-model follow-up awaits a weights path; checkpoint 1 committed (92142a6). Independent Task 7 pilot preparation may proceed under the checkpoint policy.
Planning baseline: `890ab4cab4f547d05a257c74108c9be5323635b4` plus the actual inherited dirty worktree.
Plan mode: `$aif-plan full`; subsequent execution uses AI Factory only.

## Original Request

Давай сделай через $aif-plan full подробный план всех этих улучшений в актуальной ветке: восстановление после ошибок, чистка текста и онбординга, единый стол для соло и co-op, анимации и десктопные фичи. И я бы прокачал UI, шрифт, визуал, побольше негенерируй текстур, блоков, задних фонов, более качественных картинок персонажей, карт, самих мини фигурок персонажей на карте и прочего

## Confirmed preferences and settings

- Testing: yes; meaningful behavioral/contract regressions, real production-root coverage, production-build browser and native checks.
- Logging: verbose, using existing structured development logging with safe IDs/codes/counts/durations. Production noise remains reducible through current logging controls.
- Docs: yes; mandatory documentation checkpoints routed through `$aif-docs` during implementation.
- Visual direction: user-confirmed rich fantasy tabletop world with physical materials, paintings and dimensional-looking miniatures.
- Artifacts: English, following `.ai-factory/config.yaml`; RU/EN product copy remains in locale catalogs.
- Git: remain in the existing managed worktree and current branch. The explicit user branch instruction overrides automatic new-branch creation.
- Execution: ordered local checkpoints through `$aif-implement`; this planning turn writes plan files only and does not implement or generate production assets.
- Dependencies: no new production dependency is approved by selecting this plan. See the bounded native dependency decision below.
- Publication: no push, merge, tag, release, installation over a user's profile or deployment is authorized.

## Completed foundation and parallel active work

The former branch-bound foundation plan is preserved byte-for-byte at `codex-solo-coop-playability-foundation-completed.md` in this directory. Its completed tasks and historical evidence remain unchanged.
Original foundation SHA256: `7446532d763d9566bca67971a309d02fbb2e9163d135ed31bc6009a5ad44c59d`.

This file is now the discoverable active full plan for the current branch. Do not restart delivered campaign persistence, ownership, authored adventure or canonical mechanics.

Existing `.ai-factory/plans/codex-ai-solo-mechanics.md`, `codex-lan-coop.md`, `codex-hero-recovery.md`, `codex-defense-equipment.md` and `codex-bell-adventure.md` remain authoritative for their unfinished mechanics/model/native acceptance. This plan consumes their contracts and contributes UI/recovery/native evidence; it does not silently mark them complete.

## Goal and measurable outcomes

Let a solo player or LAN/VPN group enter a prepared adventure quickly, understand confirmed actions, recover interrupted delivery without duplicate effects, and play through the same readable, attractive desktop table.

- Prepared adventure route performs real steps: Adventure -> Hero -> First scene; optional media setup is deferred.
- Guest entry does not require a local model or solo hero setup.
- Chat composer, scene/map and relevant action/recovery controls remain simultaneously reachable at 960x640 and 1280x800.
- Recovery is discovered before the next mutation and resumes the same durable identity.
- One design-token system provides consistent RU/EN typography, quiet rich materials, public portraits and full miniature silhouettes for every actor.
- The final production catalog contains 100 selected masters with real consumers, stable references and offline fallback; thumbnail variants and unused images do not count.
- Window/workspace restoration, command palette, valid invitation links, notifications, portable campaigns and read-only detached windows work through native adapters.
- Full real-model peaceful/defensive and physical LAN acceptance remains a separate mandatory playability verdict, never inferred from browser fixture success.

## Evidence from the 2026-10-12 exploration

- The main checkout is on the older `feature/refactor-project-strict-explicit-architecture`; this worktree is the latest implementation branch at 890ab4c.
- Production build passed. Lint completed without errors, with seven existing warnings and one configuration deprecation information item.
- Selected production-browser scenarios: 13/13 passed; room-production: 7 passed and 1 timed out waiting for the recovery button. This is not a full current gates/native/model acceptance.
- **Corrected diagnosis:** the room failure fixture reports canonical pending_action=null after an injected failed stream. The controller refreshes that canonical value and consequently hides retry. Actual native code persists pending before dispatch and retains it on stream failure. Correct fixture fidelity and native proof first; do not call the observed fixture failure an established native defect.
- Preset choice scrolls at 1280x800, uses 11px explanatory copy and repeats technical choices. Campaign creation contains two informational handoff screens.
- Campaign cards translate a technical revision into 'Глава'; they show a raw ruleset ID. Runtime UI explains backend port plumbing.
- Co-op uses a scrolling page with battle above party/chat; solo already has a persistent map/chat shell.
- Transparent miniatures are clipped into portrait circles; other actors can become initials. Authored geometry suppresses raster map backgrounds.
- Portrait fallback currently maps non-map styles, including character_portrait, to scene Illustration: this semantic mismatch requires a regression and correction.
- Current visual corpus: 33 frontend images (~6.30 MB), 16 bundled maps/illustrations (~1.89 MB), 34 local font files (~0.99 MB). Latin/Cyrillic display headings use different families.
- Latest real Qwen/Bonsai notes still do not establish complete model-led playability; two native clients on one machine do not establish separate-device LAN/VPN acceptance.

Local exploration evidence: `.artifacts/ux-audit-20261012/{report.json,game-report.json,*.png}`, `.artifacts/room-ui-production-browser.png`, `.artifacts/room-combat-production-browser.png`, and the failing room test trace. These files are ephemeral evidence, not checked-in product assets.

## Research Context

Source: `.ai-factory/RESEARCH.md` (Agreed intent and Proposed authority and content contract; Updated: 2026-10-06; SHA256: 3b479742b6ab90427a07c56c7b1837a828ffd84e9b246040c50b7952b4d9b288)
Snapshot scope: the verbatim content between the markers below, normalized to LF, trimmed trailing spaces and one final newline. Older 'Initial audit' gaps are historical and are not blindly treated as current requirements.

<!-- uiux:research-copy:start -->
## Agreed intent

- Finish a complete playable adventure with an AI Dungeon Master.
- Use one authoritative campaign system for solo and cooperative play.
- Cooperative players use separate devices, initially over LAN or VPN.
- Prepare world history, characters, secrets and consequences in advance rather than delegating their invention to a small runtime model.
- Evaluate the installed Ternary Bonsai 2 27B as a replacement candidate.
- Preserve existing worktree changes. Add no production dependencies without approval.

Working assumptions: 1 player in solo; 2-4 players in cooperative play; AI is the Dungeon Master; a player hosts campaign storage and inference. These assumptions remain adjustable. Initial host migration, public matchmaking, voice chat and public Internet relay are outside this first delivery.

## Proposed authority and content contract

Host-owned state: campaign, participant membership, character ownership, current scene, encounter, resources, discovered clues, NPC facts, journal, rewards, save revisions and adventure flags.

Clients send intentions. The server checks membership, character ownership, turn, resources, range, request identity and expected revision. Attack modifiers and equipment effects are derived from canonical sheets, not trusted client values. The AI proposes narrative and permitted tools; the engine accepts or rejects changes and supplies their actual outcomes.

One ordered execution lane per campaign prevents two simultaneous AI turns from interpreting the same obsolete state. Chat discussion may remain concurrent. In combat, only the current actor can commit its turn actions. Outside combat, participants may submit individual actions or explicitly agree a group action; the system does not require every player to answer before every narration.

An adventure package should separate:

1. immutable world canon and actual secrets;
2. player-visible facts;
3. each NPC's knowledge, beliefs, agenda and possible lies;
4. authored scene descriptions, connections, clues and encounters;
5. runtime facts and explicit consequence rules;
6. optional flavor the model may invent without changing canon.

The model receives the current scene, relevant canon, participant sheets, known clues and the engine's recent outcomes. Hidden information is supplied only when needed for DM reasoning, never copied into ordinary player events. The whole adventure is not injected into every turn.

The model may vary wording, gestures and nonessential background detail. It may not invent new quest-critical objects, rewrite secrets, grant resources, resurrect characters, reveal undiscovered clues as already known, or silently change scene. Unsupported actions receive an honest response with alternatives. No player action is committed merely because prose describes it.

Narrative proposals and their mechanical events need a recoverable operation record. Cancellation before commit changes no resources; interruption after commit resumes from the recorded outcome without rerolling or duplicating effects. The policy for partial multi-tool turns must be explicit in the implementation specification.
<!-- uiux:research-copy:end -->

## Roadmap linkage

Skipped: the configured `.ai-factory/ROADMAP.md` does not exist. The current request and active branch are the scope anchor. Do not create a roadmap or modify rules/research under this planning command.

## Architecture and non-negotiable boundaries

- Follow `.ai-factory/ARCHITECTURE.md` over stale overview descriptions in older context files.
- Deterministic rules stay in app-domain; use cases/read projections and ports in app-application; SQLite/HTTP/media/LLM/secrets remain isolated adapters. Backend composition stays in app-bootstrap.
- Tauri remains the sole owner of native windows, runtime processes, room IPC and OS/file integration.
- React is presentation and intention dispatch. New game-table views accept neutral props/ports; no feature imports a sibling feature's internals. App modules compose controllers/adapters and must not accumulate new workflows.
- Retain authenticated participant ownership, stable actor IDs, server revision ordering, receipt replay, partial-command recovery, publication checks and timeline reset guards.
- Never publish DM reasoning, semantic checker drafts, hidden knowledge or other players' biographies as chat, progress, notifications or export previews.
- Extend existing visual/spacing/type/motion tokens in place; retire superseded values after consumer verification. The explicit visual upgrade permits changing the current system, not creating a parallel system.
- Images do not define mechanical armor, collision, walls, doors, range or rewards. Canonical geometry remains authoritative; art is a calibrated skin.
- Keep machine contracts in English and visible labels in RU/EN catalogs. Preserve supported save/media contract IDs and use additive migrations.
- Preserve the actual dirty baseline and existing user art; no broad formatting, cleanup, database reset or automatic source-cache deletion.
- Logs contain safe IDs, phase, revisions, sizes and durations only; no prompts, text bodies, biographies, keys, invitation URIs, capability/resume proofs or base64.

## Compact design brief

### Visual language

Evolve the existing calm painted fantasy: worn oak, quiet stone, parchment, cloth, restrained metal, warm practical light and violet/gold accents. Use rich backgrounds and edge detail to frame the game; keep prose, forms and combat data on quiet surfaces. Avoid glowing borders around every block, texture behind every line of prose, tiny uppercase paragraphs and inconsistent mixed-script headings.

### Typography

Default production candidate: Manrope for controls and Literata for narration/headings, with existing JetBrains Mono only for developer diagnostics. Verify real licenses/subsets and compare on RU/EN screen specimens before integration. Primary control text >=14px; narration 16-18px with adjustable reading scale, line height about 1.6-1.75 and bounded line length. Hierarchy comes from roles and spacing, not additional decorative fonts.

### Table layout and entry

Prepared play: choose adventure, choose/create its hero, enter the first real scene. Guests: open/paste invitation, review room, choose name/hero, join. Model preparation is essential only for the inference owner; images/video and expert settings are optional.

The shared table keeps a compact header/party status, one scene or tactical map area, stable chat/composer and contextual actions. Hero/journal/NPC/goals use a reusable side panel; standalone dialogs remain available where focus or confirmation matters. Room setup/recovery administration is contextual and does not dominate the game.

### Motion

Use the current 120/180/280ms hierarchy. Animate public canonical changes and navigation continuity, not fake work progress or unconfirmed mechanics. Reduced-motion and background visibility affect both CSS and JS timing; no animation blocks an otherwise available input.

### Miniatures

2D raster artwork with a dimensional painted miniature appearance, not a new 3D renderer. One complete actor per transparent image, matched portrait identity, common three-quarter overhead camera and feet anchor, unclipped weapons, code-rendered ground shadow/rings/status. Test at full size and actual 30/48/64px presentation; logical cell hit targets remain unchanged.

### Map treatment

Upgrade general bundled maps as artwork. Only a calibrated skin based on current authored geometry may be advertised as a tactical map. The current Archive is 12x8 with wall x=6 excluding passage y=3 and exit (1,3); verify current source before generation. Other Bell locations get atmospheric paintings unless a separately implemented geometry exists.

## Production art inventory: 100 selected masters

| Category | Count | Required consumers |
|---|---:|---|
| Materials | 12 | Table, quiet panels, journal/save surfaces |
| Transparent scalable frame/edge kits | 4 | Portraits, panels, journal/page and campaign cards |
| Key and save illustrations | 7 | 3 key images and 4 save categories |
| Bell location paintings | 6 | Town, quay, shrine, warehouse, archive, belfry |
| Hero portraits | 16 | Four distinct identities for each of four currently selectable classes |
| Matched hero miniatures | 16 | Every portrait pair selectable and persisted in solo/co-op |
| NPC portraits | 14 | 8 generic archetypes plus all 6 named Bell NPCs |
| Opponent miniature archetypes | 8 | Water Echo and 7 catalog-backed public archetypes |
| Tactical/general map images | 9 | 8 bundled maps plus 1 exact Archive skin |
| Bundled scene illustrations | 8 | Existing provider fallback categories |
| **Total** | **100** | Each retained master has a real consumer |

Existing satisfactory assets may be retained only after the same quality/provenance/consumer review; high-quality variants are selected rather than generated just to raise a count. The pilot is included, not extra. Four old generic transition roles reuse selected location/fallback illustrations; retained raw originals are excluded from shipping imports unless explicitly counted. Custom player media is durable user content and is not part of this fixed built-in 100-master count. No new character class or combat mechanic is inferred from the art inventory.

### Asset production rules

- Use the built-in imagegen workflow for requested raster authoring and artistic edits; one call per distinct asset/variant. Inspect local references before editing; preserve true alpha for miniatures/frames. Do not silently switch to API-key CLI generation.
- Generate source-appropriate framing rather than baking UI text/buttons or relying on destructive portrait crops. Existing SVG icons and deterministic geometric overlays remain code-native.
- Save selected shipping images under project production asset paths and wire consumers before counting delivery. Keep approved source masters and generation/provenance records under a durable workspace directory outside Vite/Rust packaged imports; rejected candidates are not bundled.
- Use existing asset tooling for measured encoding/derivative export where available; any new production package needs explicit approval. Artistic changes/background removal use imagegen, not an unrequested custom image-editing fallback.
- Manifest records stable ID/version, category, portrait/miniature identity, dimensions, alpha, SHA256, bytes, derivatives, source/reference/generator identifiers, license/usage record, consumer and selection notes.
- Core presets remain the currently supported fighter/rogue/wizard/cleric. Diversity is visual; identity and public permissions remain canonical. Generic NPC assignment must not change with catalog length.
- Generate and integrate in the bounded waves in Tasks 21-31. Inspect every final asset plus actual-size/table screenshots; no 'more files' substitute for readable, consistent art.

### Initial budget targets

Measure and confirm these in Task 7 before bulk waves: materials <=220KiB, optimized portrait <=180KiB, miniature derivative <=350KiB, wide illustration <=450KiB, existing bundled map <=512KiB, calibrated Archive skin <=900KiB. Target total encoded shipping art/fonts <=32MiB and visible decoded art <=96MiB. These are proposed project budgets, not already measured results.

Use portrait/miniature derivatives appropriate for 96/256/512px surfaces and 1280/1920px scenes, where existing tooling supports them. Keep higher-resolution approved originals outside installers. A justified measured budget change is recorded in the visual manifest/design decision before bulk production; do not degrade all art blindly to hit an arbitrary number.

## Native dependency decision

| Capability | Preferred existing path / proposed addition | Prerequisite |
|---|---|---|
| Workspace/window state | Existing Tauri core + store first; window-state plugin only if justified | Measured restore/DPI semantics |
| Command palette | Existing React/ui Modal and injected command registry | No new production package |
| Single instance | tauri-plugin-single-instance if approved | Register before process setup; preserve QA profile identities |
| OS invitation protocol | tauri-plugin-deep-link if approved; paste-link parser works first | Single instance, bounded URI grammar, native validation |
| OS notifications | tauri-plugin-notification if approved | Opt-in/focus/dedup behavior |
| Native file pickers | tauri-plugin-dialog if approved | Portable bundle contract and narrow path capability |
| Portable bundle encoding | Existing dependencies first; compression library only if needed/approved | Versioned schema, bounds, rollback and private-data policy |
| Detached windows | Existing Tauri core and per-label capabilities | Exactly-once process shutdown refactor |

Before adding a package during implementation, prepare a concrete reviewable dependency/capability change with supported version, license, platform/toolchain compatibility and narrowly scoped permissions, then obtain the missing explicit approval. Do not install during planning, add broad filesystem capabilities, use permission workarounds or claim paste-link tests prove OS registration.

Not included: autonomous player heroes, a new 3D engine, cloud sync/relay, public matchmaking, voice chat, host migration, implicit tray hosting, unimplemented PDF/DOCX ingestion, model training or an unapproved model download. These do not block independently authorized UI/asset work.

## Commit plan

| Checkpoint | Tasks | Suggested local commit |
|---|---|---|
| 1 | 1-3 | fix(recovery): align durable pending contracts and fixtures |
| 2 | 4-6 | feat(recovery): expose proactive safe continuation |
| 3 | 7-11 | feat(visual): establish typography materials and semantic assets |
| 4 | 12-15 | feat(onboarding): shorten entry and progressive setup |
| 5 | 16-20 | feat(tabletop): unify solo and cooperative game surfaces |
| 6 | 21-24 | feat(art): ship matched hero portraits and miniatures |
| 7 | 25-28 | feat(art): ship NPC opponents and key illustrations |
| 8 | 29-31 | feat(art): ship authored scenes and calibrated maps |
| 9 | 32-35 | feat(desktop): add purposeful motion workspace and commands |
| 10 | 36-39 | feat(native): add scoped lifecycle links and notifications |
| 11 | 40-42 | feat(campaigns): add portability and detached read-only views |
| 12 | 43-44 | perf(ui): validate polished desktop delivery and offline assets |

Execute tasks in dependency order, normally ascending IDs. Do not begin bulk art before the pilot/catalog/renderer can consume it. Tests and asset production may proceed independently of unresolved model quality, but final full-playability acceptance must remain honest. Commit only checkpoint-owned changes after relevant checks; never git add . or include inherited hunks. Planning creates no commits.

## Tasks

### Baseline and recovery

- [x] **Task 1: Freeze the actual dirty-worktree baseline and reproducible UX scenarios**

  **Files / ownership:** Existing scripts/gates.sh, e2e/playwright.config.ts, e2e/{campaign-home,living-tabletop,settings,room-production,agent-failure}.spec.ts; new .artifacts/product-uiux/ evidence during implementation.

  **Deliverable:** Record HEAD plus owned/unrelated source hashes before implementation. Reproduce the 2026-10-12 production-build screens at 1280x800 and 960x640. Preserve the inherited 39 dirty tracked files and untracked tests; edits to overlapping files must be staged by owned hunks. Bind every subsequent check to the actual worktree/build rather than clean HEAD.

  **Verification / acceptance:** Run the focused baseline, lint and production build. Record existing warnings, the co-op retry fixture failure, measured navigation and asset payloads. Existing defects remain named; no skipped assertion or false full-suite PASS.

  **Logging:** DEBUG safe file/build hashes and test IDs; INFO baseline result and duration; WARN named inherited failures. No source bodies, campaign content, keys or screenshots containing invitations.

- [x] **Task 2: Make the interrupted co-op fixture match durable native pending behavior** (depends on 1)

  **Files / ownership:** e2e/room-production.spec.ts, src/features/rooms/model/RoomController.test.ts, src/api/roomStream.test.ts; existing crates/app-server/src/native_room_transport/stream.rs and transport integration tests.

  **Deliverable:** The current fixture always returns null for pending_action, so refresh removes the retry button after an injected failure. Persist the fixture pending request before stream dispatch, retain it on SSE error/EOF/cancel, clear on agent_done and retain clarification identity on agent_needs_input. Add distinct definitive-rejection and unavailable-pending responses. Do not add a speculative frontend fallback that contradicts canonical state.

  **Verification / acceptance:** The original guest retry scenario passes with the same request_id and resume=true. Add failure-after-commit, failure-before-commit, clarification and terminal-rejection cases. Native transport integration verifies matching persistence/clear semantics, independently of the fixture.

  **Logging:** DEBUG request ID, operation kind, lifecycle and clear reason; INFO recovered delivery; WARN interrupted transport; ERROR persistence failure. Raw invitation/resume proofs stay private.

- [x] **Task 3: Define a safe shared recovery projection over existing canonical ledgers** (depends on 2)

  **Files / ownership:** crates/app-application/src/agent/recovery.rs, agent/mod.rs and models/room_transport.rs; adapter-http routes/services; adapter-sqlite agent_recovery.rs, human_commands.rs, room_human_commands.rs and operation actor-scope guards with additive migration 0022_agent_actor_scope.sql; app-server transport/service delegation; src/api/{agentRecovery,agentPending,rooms}.ts and contracts; src/features/recovery/model/. Preserve existing pending endpoints unchanged; recovery-input is an additive owner-only continuation endpoint. Actual solo ledger file is human_commands.rs, not the earlier assumed solo_human_commands.rs.

  **Deliverable:** Specify a versioned read-only projection for scope/request identity, confirmed public effects, incomplete narration, clarification and allowed recovery action. Derive confirmed progress from durable receipts, not prose, tool cards or client memory. Map solo HTTP and native-room pending through separate adapters into one presentation model. Preserve existing error/event names; any additive wire field must default compatibly.

  **Verification / acceptance:** Contract tests cover before/after effect commit, partial compound commands, canonical refusal, missing/unknown pending, stale generation, wrong participant and restored timeline. Prove public projection omits command source spans, hidden canon, checker drafts, reasoning, credentials and private biographies.

  **Logging:** DEBUG scope IDs, revision, phase and recovery classification; INFO operation completion; WARN unknown/stale state; ERROR projection failure. Never log command text or private ledgers.

- [x] **Task 4: Probe and present recoverable requests proactively** (depends on 3)

  **Files / ownership:** src/features/agent-turn/useAgentTurn.ts; src/features/rooms/model/RoomController.ts; proposed src/features/recovery/{useRecovery,RecoveryPanel}.tsx; src/app/{App,RoomExperience}.tsx; src/locales/{en,ru}/{errors,chat,rooms}.json.

  **Deliverable:** Discover pending on campaign/room activation and cold resume, before the next send. Present neutral states: confirmed action, narration unfinished, clarification, reconnecting, unavailable runtime, definitive refusal. Offer Continue answer using the original identity. If canonical pending cannot be read, show unknown state and read retry before allowing another mutation. Keep hero vitality recovery distinct.

  **Verification / acceptance:** Render the real production root for solo/host/guest. Pending and errors remain reachable after restart, rejoin, cancellation, campaign switch and save restoration. Late probes cannot affect the next campaign. Already committed effects never execute twice. Never reveal unpublished narration to make progress look lively.

  **Logging:** DEBUG safe transitions and probe duration; INFO successful resume; WARN unreachable canonical state; ERROR terminal boundary failure. No continuous polling console spam.

- [x] **Task 5: Keep runtime/settings repair atomic and preserve player input** (depends on 3, 4)

  **Files / ownership:** src/components/{SettingsModal,SettingsForm,ChatPanel}.tsx; proposed src/features/settings/useSettings.ts; src/api/settings.ts; src/features/rooms/RoomPlay.tsx; src/state/settings.ts; relevant settings/recovery tests.

  **Deliverable:** Maintain settings drafts until the backend confirms the full replacement; failed Save must not change the active model/configuration or lose the draft. Reuse Stronghold secret flows. Offer contextual model start/reconnect while preserving the pending request. Keep unsent player text and reading position through panel changes; reconcile only already-public messages, never private narration drafts. A pending operation is pinned to its original model/provider identity: changing active settings from model A to B affects future turns only. Resume uses the original execution environment or remains explicitly blocked until it can be safely restored; never silently execute the old request under B. This delivery does not invent a cross-model operation migration.

  **Verification / acceptance:** Deferred/rejected settings ACK, backend restart, double Save, cancel, provider repair then resume, and campaign/room switch regressions. Secret values never enter ordinary workspace persistence. The recovery action retains request identity and does not reroll or duplicate resource spending. Test pending model A plus active model B, unavailable original runtime/configuration and a legacy pending record lacking reconstructable provider details. Preserve original receipts/request identity throughout.

  **Logging:** DEBUG config revision/generation and elapsed time; INFO confirmed configuration; WARN timed-out ACK; ERROR safe boundary code. Do not include URLs with secrets, keys, settings bodies or player drafts.

  **Implementation evidence (2026-10-12):** All preference/provider fields are staged, a deferred ACK commits once, rejected Save retains edits, Cancel discards preferences, and duplicate submits are guarded. Original execution origin/effective model are durably bound before inference for solo and rooms (migration 0023); original environment conflicts preserve recovery and legacy unknown origin is blocked. Hosts can repair settings within their room. Three real HTTP environment/recomposition regressions plus the room host-model regression pass; 40 focused settings tests, all 1153 frontend tests and nine production-browser scenarios pass. Full consolidated gate passed in `.artifacts/product-uiux/checkpoint-2-verified-gates.log`; the tested worktree includes preserved inherited changes. No native UI/model acceptance is claimed by this task.

- [ ] **Task 6: Prove recovery with real persistence and native restart** (depends on 2, 3, 4, 5)

  **Files / ownership:** Existing application/SQLite/native-room recovery integration tests; e2e/{agent-recovery,agent-clarification,agent-failure,room-production,provider-startup}.spec.ts; scripts/tauri-cdp-campaign.ts and existing scripts/lib recovery helpers.

  **Deliverable:** Drive interruption before an effect, after one effect in a compound command, after all effects before narration, during clarification, and after a save/timeline reset. Use isolated native profiles and a deterministic provider first. Read actual SQLite receipts and encrypted native pending recovery through supported boundaries. Reuse existing native harnesses and clean up only owned processes.

  **Verification / acceptance:** Exact HP, slots, clock, movement and reward counts are unchanged on resumed delivery. Host/guest permissions and current timeline survive restart. Record a separate real-model repetition with installed approved models; semantic failures remain open in codex-ai-solo-mechanics.md rather than being concealed by UI success.

  **Logging:** DEBUG canonical IDs/revisions and effect counts; INFO each accepted checkpoint; WARN model-quality failure; ERROR recovery mismatch. Evidence is sanitized and explicitly classified by test layer.

  **Implementation evidence (2026-10-12):** A fresh Tauri production frontend/backend and isolated file-backed QA profile completed a clean interruption/restart/resume cycle. Request `8da06776-faab-40ad-a954-e3136716cebc` retained two compound-action receipts, 600 seconds, HP 12 and inventory; only one assistant message was added on continuation and the unsent draft survived. Native smoke 8/8 and the actual-CSP browser regression pass. Native proof exposed/fixed Pixi static-CSP initialization and nullable internal tool-message decoding (27 focused API/root checks). Existing real SQLite/application/Stronghold room regressions cover effect boundaries, clarification, actor/timeline replacement and receipt replay. This checkbox remains open for the requested real-model repetition: no approved weights were found in checked paths; a path was requested. Independent visual production is authorized by the checkpoint policy above and does not assert semantic model acceptance. Final artifacts: `.artifacts/product-uiux/native/{interrupted-proof,resumed-proof}.json`.

### Visual foundation

- [ ] **Task 7: Define the evolved art direction and prove a small integrated pilot** (depends on 1)

  **Files / ownership:** docs/visual/LIVING_TABLETOP_ASSETS.md; proposed docs/visual/PRODUCT_UIUX.md; src/styles/{theme,fonts,materials}.css only after pilot selection; .artifacts/product-uiux/ visual specimens. Proposed nonshipping e2e/fixtures/visual-pilot specimen, using current authoritative map data.

  **Deliverable:** Specify one evolved Living Tabletop system: warm practical light, restrained violet/gold, painted fantasy, tactile quiet materials, readable controls and miniature silhouettes. Build an EN/RU specimen and one real screen composition using existing components. Generate a pilot texture, portrait/miniature pair and geometry-aligned Archive skin via imagegen; these count toward the final inventory. Review a bounded nonshipping visual specimen before full production. The specimen composites candidate whole miniatures and current map geometry at real sizes; it is not a second production root and does not claim the completed renderers. Formal production silhouette/map integration belongs to Tasks 18 and 29.

  **Verification / acceptance:** The nonshipping pilot is readable at 960x640, 1280x800 and actual miniature sizes; portrait and miniature identity match, material tiles repeat, map aligns to current geometry. No flattened bitmap UI, baked labels or second token/component system. Select using the user-confirmed brief; routine asset waves do not need repeated approval.

  **Logging:** INFO pilot asset IDs and selected versions; DEBUG dimensions, alpha, bytes and measured surface contrast; WARN rejected candidates. Tool prompts/reference IDs belong in the asset manifest, never production logs.

- [ ] **Task 8: Create stable visual references and fix semantic portrait fallback** (depends on 7)

  **Files / ownership:** src/assets/livingTabletop.ts; proposed src/assets/visualCatalog.ts and docs/visual/asset-manifest.json; src/lib/heroPortraits.ts; src/components/wizard/PortraitTab.tsx; crates/adapter-media/src/image/{bundled,resilient}.rs and media tests; application/media contracts only as required.

  **Deliverable:** Add stable versioned asset IDs, category, dimensions, alpha, hash, provenance, reference/generator metadata, portrait-miniature pairing and consumer binding. Persist IDs rather than base64. Correct character_portrait fallback so it returns portrait art or an honest unavailable result, never scenery. Preserve map/chat routing, generated/bundled provenance and backward-compatible legacy IDs.

  **Verification / acceptance:** Manifest checks enforce unique identities, valid references, bounded files and required alpha. Primary unavailable/malformed tests never accept a tavern/forest as a portrait. Catalog expansion cannot change an already assigned face. Legacy saves and unavailable custom images resolve deterministically without data loss.

  **Logging:** DEBUG selection category/asset ID and fallback reason; INFO chosen provenance; WARN unavailable reference; ERROR semantic mismatch. Prompts, private portrait uploads and image bytes are excluded from runtime logs.

- [ ] **Task 9: Persist generated and uploaded media through bounded owned references** (depends on 8)

  **Files / ownership:** Proposed app-application media asset models/ports/use cases; adapter-media durable content store and authorized HTTP/native lookup; adapter-sqlite reference metadata/migration as needed; src/api media reference schemas, src/state/{pc,campaignProjection}.ts, wizard/portrait and room public asset adapters; media persistence/native tests.

  **Deliverable:** Define source-aware references: bundled asset ID/version versus immutable content-hash media stored under application-owned local storage. Keep private/public ownership and campaign bindings explicit. Persist selected generated/uploaded portraits and maps without base64 in ordinary preferences or repeating bytes in every room revision. Resolve through authorized media access/cache; guests receive only permitted public actor art. Migrate valid legacy data URLs/trusted local sources non-destructively; unresolved/oversized legacy data stays recoverable with an honest fallback. Verify actual current envelope limits before choosing bounds. No arbitrary remote fetch on restore/migration.

  **Verification / acceptance:** Real storage + cold restart preserves custom portraits/media and stable hashes; authorized two-client lookup retains distinct public identities without private-sheet access. Missing/corrupt media, cross-campaign access, scope revocation and near-limit legacy records including 8MiB cases have explicit results without truncation or deletion. Export/import round-trips referenced bytes and metadata. Source fallback never claims a matching miniature was generated. URLs/object resources are invalidated safely after scope change.

  **Logging:** DEBUG reference/hash/ownership IDs, sizes and migration status; INFO durable asset registration/migration; WARN retained unresolved legacy media; ERROR unauthorized/corrupt reference. No image bytes, full local paths, private portrait source URLs or provider output in logs.

- [ ] **Task 10: Upgrade bilingual typography and the existing token hierarchy** (depends on 7)

  **Files / ownership:** public/fonts/ and retained license files; src/styles/{fonts,theme,globals}.css; src/ui shared primitives; typography/root/browser tests.

  **Deliverable:** Default proposal: self-hosted Manrope for UI and Literata for narrative/headings, retaining JetBrains Mono only for diagnostics. Verify the actual font files, weights, Cyrillic/Latin coverage and licenses before use. Replace existing role tokens centrally; do not layer new ad hoc families. Use primary controls at least 14px, narrative 16-18px with reading scale, sensible line lengths and tabular resource digits.

  **Verification / acceptance:** Compare old/new RU/EN specimens, mixed-script names, italic/bold, symbols and offline rendering. At minimum window size key actions remain visible without shrinking essential text. Font loading causes no material layout shift. Remove obsolete font files only after a consumer inventory proves they are unused and not user-owned.

  **Logging:** INFO selected families/version/source; DEBUG payload and loading metrics; WARN missing glyph or fallback. No new production font package or CDN dependency.

- [ ] **Task 11: Produce and integrate 12 materials and four scalable frame kits** (depends on 7, 8, 10)

  **Files / ownership:** src/assets/living-tabletop/v2/ proposed final assets; src/styles/{materials,living-tabletop,character,saves}.css; src/ui/{Button,Field,Modal}.module.css; visual manifest and asset/browser tests.

  **Deliverable:** Upgrade six current material roles and add quiet stone-floor, river masonry, pale-paper, worn-board, cloth and oxidized-metal variants. Create four transparent edge/frame kits for journal, portraits, panels and campaign cards. Apply them through existing tokens and scalable CSS composition; keep text surfaces calm and reserve rich texture for margins/table surfaces.

  **Verification / acceptance:** Inspect every selected master at full resolution, 2x2 tile repetition and actual UI sizes; no visible seams, halos, blurry edges or text baked into art. Test transparency/high-contrast fallbacks, resizing and keyboard focus. Every production kit has an actual consumer.

  **Logging:** INFO approved asset/version and consumer; DEBUG dimensions, repetition/alpha checks and byte budgets; WARN visual reject. Do not add runtime logs for decorative rendering.

### Entry and settings

- [ ] **Task 12: Rewrite visible copy and replace technical or misleading metadata** (depends on 10)

  **Files / ownership:** src/locales/{en,ru}/{common,campaigns,onboarding,settings,local_mode,chat,rooms,agent,combat,adventure}.json; CampaignHome/CampaignHeroCard, StatusBar, ToolCallCard and their consumer tests.

  **Deliverable:** Remove development/architecture explanations and repeated role/marketing assertions. Replace campaign revision-as-chapter with actual current location, last visit and save status. Keep local/cloud handling, download requirements and costs once at the decision point. Hide raw JSON, tool rounds, model IDs and pipeline jargon behind existing diagnostics; show confirmed public outcomes in play. Preserve machine keys/contracts.

  **Verification / acceptance:** All reachable RU/EN flows use concise accurate labels; no unsupported 'best quality' or always-offline promise. Locale parity and root tests assert meaningful user behavior rather than every exact sentence. Errors name an available action. Old misleading metadata no longer appears on campaign cards.

  **Logging:** INFO locale audit counts in verification only; WARN missing/stale key; no production logging for copy-only rendering.

- [ ] **Task 13: Shorten first launch and preserve direct guest entry** (depends on 4, 10, 12)

  **Files / ownership:** src/components/Onboarding.tsx and onboarding/{presets,steps}/*; src/state/onboarding.ts and persistence migration; src/features/campaigns/model/{route,types}.ts; src/app/App.tsx; onboarding routing/root/e2e tests.

  **Deliverable:** First launch offers Start adventure and Join game. Remove global hero creation before campaign ownership exists. Keep only essential model readiness for a host/solo player; image/video setup is optional later. Guest joins without local model setup, preserving the already implemented native path. Migrate completed/partial onboarding without discarding a legacy hero draft or guest recovery choice.

  **Verification / acceptance:** Fresh solo/host/guest and migrated profiles reach the correct route. Next/Back remain visible without scrolling past long preset lists at 1280x800. A partially configured user can resume, browse and create a draft without contradictory blocking overlays. Model unavailability has a concrete repair action.

  **Logging:** DEBUG route/onboarding version and completion transition; INFO completed setup; WARN retained legacy draft. No personal input or provider secrets in logs.

- [ ] **Task 14: Make campaign creation perform real steps and commit heroes in scope** (depends on 12, 13)

  **Files / ownership:** src/features/campaigns/{CampaignCreate,CampaignHome}.tsx; existing character wizard/review, campaign controllers and contracts; proposed campaign-scoped visual/preset picker; src/locales/{en,ru}/{campaigns,wizard,adventure}.json.

  **Deliverable:** For the bundled Bell path use Adventure -> Hero -> First scene. Remove empty Hero/World handoff pages. Offer good prepared hero drafts plus the existing full wizard; commit the chosen sheet only to this campaign before play. Keep collaborative title/premise route honest about current world preparation. Hide unimplemented PDF/DOCX import instead of presenting a working choice.

  **Verification / acceptance:** Bundled/collaborative routes, Back/cancel, rejected campaign/character commit and interrupted preparation retain drafts and scope. No unrelated previous hero leaks. Library previews show actual location/last visit. WorldReady is based on real initialization for the chosen source, not merely pc.heroClass.

  **Logging:** DEBUG creation phase, campaign ID and revision; INFO confirmed campaign/hero readiness; WARN retriable preparation; ERROR initialization failure. No private backstory or world secrets logged.

- [ ] **Task 15: Expose simple AI/media readiness with advanced settings on demand** (depends on 5, 10, 12, 13)

  **Files / ownership:** src/components/{SettingsForm,SettingsModal,LocalModeModal,StatusBar}.tsx; components/settings/*; provider/runtime/download hooks; EN/RU settings/local_mode locales and focused e2e.

  **Deliverable:** Show selected model, installed/running state and Start/Test/Repair actions first. Put GGUF/mmproj, context/reasoning budget, VRAM strategy, system prompt and agent limits in Advanced sections. Remove Hugging Face instructions from cloud-only forms. Derive readiness from runtime/confirmed provider state rather than config existence alone. Keep optional images/video and cost decisions separate from essential chat setup.

  **Verification / acceptance:** Changing source does not lose per-provider drafts; errors preserve data; model switches during active turns stay gated. Missing model, slow start, failed download, offline catalog and denied permission are actionable. Runtime start occurs only through existing native ownership; no unrequested large model download or provider promotion.

  **Logging:** DEBUG safe readiness/download state and elapsed time; INFO confirmed runtime transition; WARN slow/unavailable runtime; ERROR stable safe code. Never log API keys or full provider responses.

### Shared game table

- [ ] **Task 16: Define shared presentation ports with separate solo and room adapters** (depends on 3, 4, 8, 9)

  **Files / ownership:** Proposed src/features/game-table/model/{types,layout}.ts and feature tests; src/app/{App,RoomExperience}.tsx; proposed src/app/GameTableExperience.tsx; existing agent/combat/adventure/room API projections.

  **Deliverable:** Define a prop-driven table model with scene, public actors, own actor, chat/public outcomes, recovery, connection state and enabled actions. Solo and room controllers retain authority/transport responsibilities; app composition adapts both. Reusable game-table views must not import sibling feature internals or become another engine. Model exploration without tactical geometry explicitly.

  **Verification / acceptance:** Contract/root tests cover solo/host/guest, no own hero, unknown readiness, active/ended fight, restored timeline and withheld private data. Architecture check forbids sibling feature imports, API/store coupling and app-owned workflows. Same inputs produce equivalent visible table behavior without duplicating control logic.

  **Logging:** DEBUG safe adapter scope/revision; INFO scope switch; WARN incompatible projection; ERROR validation failure. Shared props contain only player-visible data.

- [ ] **Task 17: Replace the scrolling co-op page with one persistent table layout** (depends on 10, 11, 16)

  **Files / ownership:** Proposed src/features/game-table/{GameTable,TableHeader,PartyStrip,SidePanel}.tsx and CSS Modules; src/styles/shell.css; src/app composition; RoomPlay.tsx and existing ChatResizer; production-root and browser smoke tests.

  **Deliverable:** Keep scene/map, chat composer and current action/recovery controls visible together in solo and co-op. Use a compact party strip and contextual side panel for hero/journal/NPC/objectives. Move room credentials/recovery administration into contextual room controls. Keep narration and panel scrolling local; collapse secondary titlebar labels at minimum width. Preserve one reachable Settings/Help/diagnostics path.

  **Verification / acceptance:** Real production-root renders and production-build browser smoke at 960x640, 1280x800 and 1920x1080, RU/EN. No lateral overflow, clipped main action, disappearing composer or full-page scroll pushing chat below combat. Keyboard resizing, focus return and campaign/room switching remain valid.

  **Logging:** DEBUG layout preference/version and scope changes only; INFO layout reset; WARN invalid persisted sizes. No logs on every pointer movement or React render.

- [ ] **Task 18: Unify combat interaction and public visual identity for every actor** (depends on 8, 9, 16, 17)

  **Files / ownership:** src/components/{VttCanvas,CombatToken,CombatOverlay,InitiativeTracker,ActionBar}.tsx; src/features/combat/{RoomCombatPanel,WeaponAttackDialog,SpellCastDialog}.tsx and model/{types,combatProjection}.ts; additive application/public actor visual fields if required.

  **Deliverable:** Resolve all actors by stable IDs and public visual references. Separate portrait from full miniature. Use shared selection, ownership, range/target hints, action/bonus/reaction/movement/slots and end-turn affordances. Show why an action is unavailable near that action. Scope targeting to encounter/timeline. Keep mechanical cell hit targets independent of a tall sprite silhouette.

  **Verification / acceptance:** Equal names, multiple human heroes, enemy actors, selected custom portrait fallback, downed/dead/withdrawn actors and zoom all work. Unauthorized move/attack/cast remains rejected by server. Late revisions or save reset cannot apply an obsolete target. Visual change never changes collision, coordinates, initiative or damage.

  **Logging:** DEBUG actor/encounter IDs and selected intent; INFO canonical command result; WARN rejected/stale action. No client-authoritative mechanics or private actor sheet copied to other players.

- [ ] **Task 19: Make conversation and confirmed outcomes readable and recoverable** (depends on 4, 10, 12, 17)

  **Files / ownership:** src/components/{ChatPanel,MessageBubble,ToolCallCard,TypingIndicator}.tsx; src/features/rooms/RoomPlay.tsx; shared game-table chat presentation; agent-turn reducers and public room message mapping.

  **Deliverable:** Use narrative typography, stable paragraph widths, speaker identity and a compact confirmed-outcome strip. Place recovery/clarification beside the composer. Keep Enter/Shift+Enter, attachments, copy/journal actions and reading-position behavior. Collapse raw diagnostics by default. Support compatible public message rendering in co-op without publishing DM reasoning, checker drafts or hidden tool details.

  **Verification / acceptance:** Long RU narration, streaming/finished output, scroll-away, keyboard focus, failed send/retry and image/video delivery remain usable. Confirmed mechanics and pending narration are visibly distinct. A retry neither duplicates the user message nor silently drops effects. Existing publication gates remain intact.

  **Logging:** DEBUG stream identity, byte count and terminal classification; INFO final delivery; WARN interruption; ERROR safe failure code. No text/reasoning/base64 in logs.

- [ ] **Task 20: Put discovered facts, goals, journal, NPCs and hero context within reach** (depends on 12, 16, 17, 19)

  **Files / ownership:** src/features/adventure/AdventurePanel.tsx; src/components/{JournalViewer,NpcMemoryGrid,CharacterSheet,SavesScreen}.tsx; proposed game-table context panels and existing public projections; locales and root/e2e tests.

  **Deliverable:** Replace repeated modal navigation with reusable contextual panels while preserving accessible modal fallback. Show current discovered facts, explicit public goals and supported actions; retain free-text play. Bind NPC art by stable canonical/record identity rather than catalog-size-dependent hash. Use actual save timestamps and current scene summaries. Separate guest-owned sheet from public party status.

  **Verification / acceptance:** Players can read journal/hero and continue a turn without losing the composer or selection. Unknown secrets and unencountered NPCs stay hidden; available buttons do not promise unsupported mechanics. Changed catalog preserves known faces. Keyboard open/close and nested dialogs return focus correctly.

  **Logging:** DEBUG panel/record IDs and public revision; INFO save/restore confirmation; WARN missing asset/record. No full journal, NPC memory or biography in logs.

### Character art production

- [ ] **Task 21: Produce four fighter portrait-miniature identity pairs** (depends on 7, 8, 9, 10, 18, 20)

  **Files / ownership:** Proposed src/assets/living-tabletop/v2/heroes/fighter-*; visual manifest/catalog, gallery/wizard and initiative consumers.

  **Deliverable:** Generate four distinct fighter identities with matching portrait and genuinely transparent full-body miniature, using approved references for face, ancestry, clothes and equipment. Use fixed three-quarter overhead view, practical light, neutral pose, no baked base/ring/HP/text. Preserve four-class mechanics; these are visual choices. Save final selected outputs to workspace and wire every pair.

  **Verification / acceptance:** Inspect every asset at full size and actual 30/48/64px table scale on light/dark maps. Feet/pivot and weapon silhouette are clear and unclipped. Gallery choice, persisted restore and co-op public visual references retain the identity; custom portrait without miniature has an honest fallback.

  **Logging:** INFO selected stable ID/version and consumer; DEBUG hash, alpha and bytes; WARN rejected anatomy/identity/edge quality. Generation metadata stays in manifest.

- [ ] **Task 22: Produce four rogue portrait-miniature identity pairs** (depends on 7, 8, 9, 10, 18, 20)

  **Files / ownership:** Proposed src/assets/living-tabletop/v2/heroes/rogue-*; shared visual manifest/gallery consumers.

  **Deliverable:** Apply the established portrait-miniature production recipe to four rogues with readable distinct silhouettes and consistent equipment. Match each miniature to its own portrait reference; no class stereotype substitutes a player's selected identity. Reuse catalog/persistence and actual table rendering.

  **Verification / acceptance:** Same identity, alpha, silhouette, pivot, bounds, gallery, restore and two-client tests as Task 21. Every pair is selected in the actual production gallery and shown on the table at least once. No unused generated asset enters the bundle.

  **Logging:** INFO asset/version/pair IDs; DEBUG size/alpha/hash validation; WARN rejected candidates. No additional decorative runtime logs.

- [ ] **Task 23: Produce four wizard portrait-miniature identity pairs** (depends on 7, 8, 9, 10, 18, 20)

  **Files / ownership:** Proposed src/assets/living-tabletop/v2/heroes/wizard-*; shared manifest/gallery consumers.

  **Deliverable:** Produce four wizard identities with matched faces/clothing, restrained magical props and readable overhead silhouettes. Magical glow must not obscure cells or encode active spells/conditions. Keep large ornaments within safe sprite margins; effects remain code-rendered after canonical events.

  **Verification / acceptance:** Full/actual-size QA, true alpha, no halos, paired identity and persisted selection pass. Spells, targeting/AoE and current actor highlights remain readable above the art. Low/reduced-motion modes retain clear status.

  **Logging:** INFO final pair IDs and consumer binding; DEBUG metadata and payload; WARN overbright/unreadable or mismatched candidates.

- [ ] **Task 24: Produce four cleric portrait-miniature identity pairs** (depends on 7, 8, 9, 10, 18, 20)

  **Files / ownership:** Proposed src/assets/living-tabletop/v2/heroes/cleric-*; shared manifest/gallery consumers.

  **Deliverable:** Produce four cleric identities in the established material/light/camera language. Match portrait and miniature equipment; avoid presenting art as mechanically granting armor, weapons or abilities. Keep shadow, ownership, HP and conditions as separate code layers.

  **Verification / acceptance:** Same pair/alpha/pivot/gallery/restore QA plus wounded, dying, dead and revived-as-supported rendering tests. Fixed artwork is cosmetic appearance and may retain its painted equipment after loadout changes; canonical panels/intent checks always show the actual current gear and modifiers. Never claim the picture grants equipment, encode live status into art or imply resurrection. Existing hero drafts survive visual selection changes.

  **Logging:** INFO approved identity/version; DEBUG metadata checks; WARN anatomy/equipment inconsistency. No game facts inferred from an image.

### World art production

- [ ] **Task 25: Upgrade eight generic NPC portraits with stable assignments** (depends on 8, 9, 20)

  **Files / ownership:** Proposed src/assets/living-tabletop/v2/npcs/archetypes/*; src/components/NpcMemoryGrid.tsx; visual catalog and NPC presentation mapping.

  **Deliverable:** Produce the existing eight archetypes in the evolved painting style and bind them through explicit stable asset references. Migrate prior deterministic choices without reshuffling known faces when categories grow. Keep generic fallback separate from authored canonical IDs.

  **Verification / acceptance:** Catalog expansion, RU/EN names, absent role, duplicate display names, restore and missing asset tests keep the same known NPC portrait. Dossier thumbnails and full portraits pass real-screen readability and crop QA.

  **Logging:** INFO stable migration/asset IDs; DEBUG fallback classification; WARN unavailable portrait. Never log private NPC memories.

- [ ] **Task 26: Produce all six Bell NPC portraits without revealing hidden canon** (depends on 8, 9, 20, 25)

  **Files / ownership:** Proposed canonical NPC art for mila, agata, leon, varden, tikhon and savely; visual catalog; authored public NPC presentation mapping.

  **Deliverable:** Create six distinct named identities based only on approved public appearance information. Pair canonical NPC IDs to saved visual references. Show a portrait only when the public projection permits that person/appearance; asset captions and selection labels must not reveal secrets, intentions or undiscovered relationships.

  **Verification / acceptance:** Every named NPC has an actual discovered-dossier/dialog consumer. Spoiler/privacy tests distinguish private DM knowledge from public presentation. Known portraits remain stable after save/restore and across host/guest.

  **Logging:** INFO public NPC/asset IDs; DEBUG consumer visibility decision; WARN missing art. No hidden canon or generated biography in logs.

- [ ] **Task 27: Produce eight opponent miniature archetypes and integrate full silhouettes** (depends on 8, 9, 18)

  **Files / ownership:** Proposed src/assets/living-tabletop/v2/opponents/*; token presentation catalog; CombatToken/InitiativeTracker and public encounter adapters.

  **Deliverable:** Create Water Echo first, then seven approved reusable opponent archetypes matching encounters actually supported by the current catalog. Use true alpha, common overhead camera and consistent ground anchor. Assign via explicit public presentation metadata, never initials/name regex or gameplay rules inferred from art.

  **Verification / acceptance:** All eight have a reachable renderer/gallery consumer and bounded fallback. Multi-actor boards, overlapping sprites, selection/range/conditions, dead actors and low zoom remain readable. Enemy art does not alter collision or reveal hidden encounter content.

  **Logging:** INFO archetype/asset binding; DEBUG alpha/bounds/bytes; WARN fallback/reject. Do not log hidden encounter rosters.

- [ ] **Task 28: Upgrade three key images and four save illustrations** (depends on 7, 8, 9, 11, 17, 20)

  **Files / ownership:** Existing KEY_ART/SAVE_ART consumers; proposed v2 splash/onboarding/exploration and save artwork; campaign library and saves components.

  **Deliverable:** Select seven higher-quality compositions for splash, onboarding, peaceful exploration and the four existing save categories. Leave calm space around actual text, use crop-safe composition and code-rendered labels. Improve populated and empty campaign library states without oversized explanatory blocks.

  **Verification / acceptance:** Each image is visually checked at all supported aspect ratios, with long RU headings and real action buttons. Splash does not delay healthy startup; save cards describe actual snapshot state. No baked text or generated controls.

  **Logging:** INFO selected key/save asset IDs; DEBUG crop/payload/readiness metrics; WARN unreadable composition. No campaign content embedded in generic artwork.

- [ ] **Task 29: Produce six Bell location paintings and use them during exploration** (depends on 8, 9, 17, 20)

  **Files / ownership:** Proposed town/quay/shrine/warehouse/archive/belfry scene assets; visual catalog; shared scene viewport and authored adventure public mapping.

  **Deliverable:** Produce six scene paintings faithful to public location descriptions. Bind to current canonical location and render atmosphere when no tactical map exists. Use public reveal rules and transitions; never add new quest-critical objects or imply a new walkable battle geometry. Include scene-preview/continuation uses.

  **Verification / acceptance:** All six appear through actual scene transitions. Restore/rejoin selects the correct art. No invented doors, critical clues or secret spoilers. Text readability, contain/crop, missing asset and reduced-motion checks pass.

  **Logging:** INFO current public location/asset version; DEBUG load/crop metrics; WARN unavailable visual. No private world canon in runtime logs.

- [ ] **Task 30: Upgrade eight bundled maps and add the exact Archive tactical skin** (depends on 8, 9, 17, 18, 29)

  **Files / ownership:** crates/adapter-media/assets/bundled/maps/*; bundled registry; src/components/{AuthoredMap,VttCanvas}.tsx; proposed map visual reference/calibration contract and map/e2e tests.

  **Deliverable:** Retain eight stable bundled map identities, upgrading selected raster art, and produce one Archive skin from the current 12x8 geometry reference: wall x=6 except passage y=3, exit (1,3). Permit authoritative arenas to use a calibrated visual skin while geometry/coordinates remain the source of truth. General generated maps without geometry stay decorative scene art.

  **Verification / acceptance:** Overlay landmarks, walls/passages/exits, grids, movement budget, distance/range and token coordinates remain aligned at zoom/fit/DPR changes. No artistic door adds a legal route. Map layers, streaming routing, save/restore and source provenance pass. Preserve the current bundled-map byte limit unless an explicit contract migration is justified.

  **Logging:** DEBUG map/geometry hashes, calibration and layer state; INFO selected skin; WARN missing/misaligned art; ERROR contract mismatch. No prompts or image bytes logged.

- [ ] **Task 31: Upgrade eight bundled scene illustrations and verify their offline fallback** (depends on 8, 9, 19, 25, 26, 28, 29, 30)

  **Files / ownership:** crates/adapter-media/assets/bundled/illustrations/*; image bundled/resilient providers; ToolCallCard, portrait fallback and visual catalog tests.

  **Deliverable:** Upgrade the existing eight illustration categories and verify their actual fallback consumers. Complete 100-master closure belongs to Tasks 43-44 after every art wave, not this eight-image task. Keep generated-vs-bundled provenance quiet but available. Every unsupported/missing image runtime still permits attractive offline play, with category-correct art. Store high-resolution masters/provenance outside packaged import paths and selected shipping images inside the project.

  **Verification / acceptance:** Offline native/browser play renders all required categories, portraits, maps and missing-image fallback honestly. No regenerated scene image is accepted as a character portrait. These eight illustrations have valid stable references and real fallback consumers; global catalog count/coverage is verified only at Tasks 43-44. No thumbnails/rejected variants are counted as masters.

  **Logging:** INFO catalog coverage and provenance; DEBUG manifest hashes/bytes; WARN honest unavailable category; ERROR bad semantic binding.

### Motion and workspace

- [ ] **Task 32: Animate confirmed gameplay events and spatial relationships** (depends on 18, 19, 27, 30)

  **Files / ownership:** src/components/{CombatToken,CombatOverlay,InitiativeTracker,ActionBar}.tsx; shared game-table components; src/styles/{combat,vtt}.css; event adapter/presentation tests.

  **Deliverable:** Reuse the existing 120/180/280ms motion scale. Add restrained own-turn arrival, confirmed hit/heal/resource change and target-selection continuity; move sprites only after canonical confirmation. Keep status rings/shadows/effects code-rendered and keyed by canonical event/revision so retries or reconnects do not replay a celebration or damage effect.

  **Verification / acceptance:** Denied/stale/replayed commands produce no false success motion. Reduced motion uses static status changes and immediate final positions. Concurrent actors, long sessions and save timeline reset do not stack stale effects. Inputs remain responsive throughout.

  **Logging:** DEBUG event ID and one-shot effect classification in development only; WARN stale effect dropped. No per-frame logging.

- [ ] **Task 33: Replace blocking scene transitions and respect reduced/background motion** (depends on 17, 29, 32)

  **Files / ownership:** src/components/SceneTransitionOverlay.tsx; src/styles/{overlays,living-tabletop,globals}.css; useClosingAnimation and token transition handling; settings and browser tests. src/assets/livingTabletop.ts and selected visual registry transition-role mappings.

  **Deliverable:** Use in-table scene art/title transitions for ordinary movement instead of a 3.2-second full-screen interruption. Keep cinematic transitions optional for major events. Honor reduced-motion in both CSS and JS timers, including token movement. Replace blanket descendant transform removal with scoped decorative overrides. Pause ambient work in hidden/unfocused windows without changing canonical operations. The four legacy generic SCENE_ART transition roles reuse the selected six location paintings/eight bundled illustrations through explicit semantic mappings; do not keep an undocumented additional shipping art set. Unreferenced originals remain preserved outside bundled paths.

  **Verification / acceptance:** Scene navigation never hides composer/actions or steals focus. Reduced mode has no delayed blocking overlay; transformed interactive map content remains correct. Background/resume does not rerun effects or lose streams. No user must wait for animation completion to act.

  **Logging:** DEBUG transition/event ID and visibility state; INFO preference change; WARN stale timer cleaned. No recurring decorative logs.

- [ ] **Task 34: Persist and reset the shared workspace and native window geometry** (depends on 17, 33)

  **Files / ownership:** src/state/{settings,persistStorage}.ts; proposed src/features/game-table/model/workspace.ts; src-tauri/src/commands/desktop.rs and window lifecycle; existing store plugin; tauri.conf.json/capabilities.

  **Deliverable:** Persist versioned pane widths, selected context panel, reading scale, view mode and native size/position/maximized monitor state. Reuse the existing store and Tauri core where practical. Validate/clamp old preferences and provide Reset layout. Restore safely after monitor removal and DPI changes; normal main-window close must not overwrite its saved normal geometry with auxiliary geometry.

  **Verification / acceptance:** Cold/warm restart, missing monitor, 125/150/200% native scale, min/max resize, malformed/legacy prefs and reset all keep essential controls on-screen. No secrets/drafts/private room proofs enter workspace settings. Browser panel persistence and native geometry are verified separately.

  **Logging:** DEBUG preference version and safe geometry/clamp reason; INFO restore/reset; WARN off-screen recovery. No per-resize event logging.

- [ ] **Task 35: Add a command palette and one cross-platform shortcut registry** (depends on 16, 17, 34)

  **Files / ownership:** Proposed src/features/commands/{model,CommandPalette}.tsx; src/app/useAppShellController.ts and composition registrations; src/ui existing Modal; locales and shortcut/browser tests.

  **Deliverable:** Create an app-composed registry of localized commands, enabled conditions, platform Ctrl/Meta modifiers and handlers supplied by owning controllers. Include hero, journal, NPCs, saves, recovery, room controls, settings, focus chat/map, layout reset and fullscreen. Show shortcuts in menus/tooltips. Preserve existing Ctrl+S/F11 behavior; keep diagnostics advanced.

  **Verification / acceptance:** Typing, IME, textarea newlines, nested-modal Escape, repeated shortcut and disabled guest/other-turn commands work. Ctrl on Windows/Linux and Meta on macOS are tested. Palette cannot execute a forbidden save/combat operation or bypass the server. No production command-palette package needed.

  **Logging:** DEBUG command ID and enabled/refused result; INFO explicit invocation where operational; WARN unavailable command. Never log search text or player input.

### Desktop integration

- [ ] **Task 36: Separate auxiliary-window closure from application process shutdown** (depends on 34, 35)

  **Files / ownership:** src-tauri/src/lib.rs; src-tauri/src/processes/*; commands/rooms.rs and proposed commands/desktop.rs; per-window capabilities; native lifecycle tests.

  **Deliverable:** Refactor exactly-once application shutdown before creating detached windows. Today any CloseRequested/Destroyed stops all backend/model/media/control processes. Auxiliary close must dispose only that view. Define main-window close behavior for an active room/operation, confirm the concrete impact and preserve durable effects. Stop owned room listeners and processes on actual application exit.

  **Verification / acceptance:** Close/open multiple auxiliary windows, main close, crash/failed startup and repeated close events prove no orphan processes and no premature backend stop. Host/guest departure uses existing canonical operations. No implicit tray/background hosting is added in this plan.

  **Logging:** DEBUG window label and lifecycle reason; INFO true app shutdown; WARN pending canonical operation; ERROR failed cleanup. Never log room credentials.

- [ ] **Task 37: Enforce a single application instance without duplicate backend startup** (depends on 36)

  **Files / ownership:** src-tauri/src/lib.rs and setup; proposed native invocation broker; Cargo.toml/package dependencies only after explicit approval; packaged native launch harness.

  **Deliverable:** Reuse an approved single-instance mechanism registered before backend spawning. Second launch focuses the existing main window and forwards a validated invocation to it. Preserve profile/application identifier separation used by native two-client QA; do not merge unrelated host/guest profiles.

  **Verification / acceptance:** Packaged cold/second launch proves one backend for one application identity, no duplicate ports/model processes, correct focus and no lost pending operation. Isolated QA identities can still run concurrently. Plugin refusal/unavailability is an explicit implementation prerequisite, not silently bypassed.

  **Logging:** INFO first/second launch and focus delivery; DEBUG safe identity/event IDs; WARN rejected invocation; ERROR broker failure. No raw argv/URI logging.

- [ ] **Task 38: Make room invitations one validated pasteable or openable link** (depends on 2, 13, 36, 37)

  **Files / ownership:** src/api/rooms.ts; src-tauri/src/commands/rooms.rs; proposed invitation parser/native broker; RoomHome/RoomExperience; tauri.conf.json/capabilities; deep-link dependencies only after approval.

  **Deliverable:** Define a versioned invitation URI containing only the existing expiry-bound/single-use invitation and required join address/room fields, never capability/resume proof. Parse natively, redact/discard raw URI, and open a join preview with name/room context; do not connect on link activation alone. Support pasted link before OS protocol integration. Hide bind address/advanced fields by default and share/rotate invitations honestly.

  **Verification / acceptance:** Cold/warm launch, first-boot guest, an already active game, expired/rotated/consumed invitation, malformed URI/address, repeated activation and uncertain committed join are exercised. Wrong links cannot replace a campaign silently or duplicate a member. Native recovery remains available. OS registration is tested in packaged native application.

  **Logging:** DEBUG parse/result/event ID; INFO join preview and confirmed membership; WARN expired/rejected link. Never log/share in screenshots the raw invitation or URI.

- [ ] **Task 39: Notify only meaningful confirmed events when the app is inactive** (depends on 17, 32, 36)

  **Files / ownership:** Proposed src/features/desktop/notification model/port; src/app registrations; src-tauri native notification adapter/capabilities; existing room cursor/actor/download events; settings/locales.

  **Deliverable:** Add opt-in native notifications for own turn, model/download readiness and actionable connection failure. Derive from canonical event IDs/own actor and deduplicate across subscribe/reconnect/restart. Suppress while focused and respect OS permissions. Clicking focuses the relevant existing view, preserving current state. No private narration in notification bodies.

  **Verification / acceptance:** Permission refusal, focus changes, reconnect replay, duplicate revisions, host/guest actor identities and notification clicks pass. No notification for another player's turn, each token, private DM reasoning or routine unchanged status. Actual OS delivery requires packaged native proof.

  **Logging:** DEBUG event classification/dedup key; INFO sent/clicked event type; WARN denied permission; ERROR adapter failure. No private message text or invitation payload.

- [ ] **Task 40: Define and implement a portable authoritative campaign bundle** (depends on 3, 8, 9, 20, 30)

  **Files / ownership:** Proposed app-application campaign import/export use cases and inward-owned ports/models; adapter-sqlite transaction implementation; typed HTTP services/composition; portable bundle tests.

  **Deliverable:** Define two discriminated formats and role policies before serialization: full_campaign_backup is permitted only for the solo storage owner or host storage owner, includes canonical hidden adventure state and accepted party documents privately, and is never exposed through guest export; player_dossier is permitted for an authenticated player's own sheet/biography and public messages/discovered facts only, never another player's private biography, hidden canon or room credentials. A dossier is not a restorable full campaign. Full backups export an authoritative validated snapshot plus bounded required media, schema/ruleset/version and hashes, excluding vault/settings/native outbox/private reasoning/discarded drafts. Import only full_campaign_backup into a new campaign with explicit ID remapping and a whole transaction. Use opaque bounded asset names; reject absolute/traversal paths, links and executable members, and never fetch remote URLs during import. Validate entry counts, decoded dimensions and total expanded size before staging.

  **Verification / acceptance:** Round-trip hero/equipment/resources/location/confirmed history/art references without changing the source. Wrong ruleset/schema/hash, oversized/corrupt/missing media, partial write and duplicate IDs leave existing campaigns untouched. Guest export permissions are enforced; raw SQLite file copying is not the portable contract. Compression, if introduced, is a separate approved dependency. Add role/mode allow-deny and secret-filter cases, dossier-as-backup rejection, traversal/absolute/symlink/executable input, external-URL nonfetch and decompression/dimension/count limits with atomic rollback.

  **Logging:** DEBUG bundle version/IDs/hashes and safe counts; INFO export/import completion; WARN unsupported input; ERROR transaction rollback. No bundle contents or filesystem secrets in logs.

- [ ] **Task 41: Expose native campaign import/export with clear progress and cancellation** (depends on 9, 20, 40)

  **Files / ownership:** Proposed src/api/desktop.ts; campaign library/SavesScreen or shared campaign menu; src-tauri commands/native path handling and optional dialog plugin; locales and packaged native/browser fixtures.

  **Deliverable:** Add Export campaign, Import campaign and Show file/location actions using validated native file pickers and the new bundle use cases. Keep filesystem access in the native adapter with narrow capabilities. Show progress without fake percentages when total work is unknown. Cancellation preserves source and existing campaigns; show an import summary before committing a new campaign. The export menu exposes only modes authorized for the current role; a host private-backup preview explains that it contains campaign secrets and party documents. Import rejects player_dossier as a campaign backup without creating partial data. Never automatically fetch linked media or extract arbitrary archive paths.

  **Verification / acceptance:** Native open/save/cancel, read-only destination, large/corrupt bundle, path with spaces/Cyrillic, failed final write and user cancel pass. No broad frontend filesystem permission or automatic overwrite. Browser fixture tests verify UI only; actual file persistence is separately proved natively.

  **Logging:** DEBUG safe operation ID and stage, not full private path; INFO completion/cancellation; WARN invalid destination/input; ERROR IO rollback.

- [ ] **Task 42: Add read-only detached hero and journal windows** (depends on 9, 17, 20, 34, 36)

  **Files / ownership:** src-tauri window registry/capabilities; proposed app auxiliary-window composition; existing CharacterSheet/JournalViewer views and validated public/owned projections; native multiwindow tests.

  **Deliverable:** Offer Open in separate window for the owned hero and permitted journal. Use existing views with a read-only versioned projection subscription, not an independent mutable game store. Secondary windows follow campaign/room changes and access scope, retain geometry and return focus. Keep private guest/host information separated.

  **Verification / acceptance:** Auxiliary close leaves game and processes running; main exit closes all owned views. Scope switch/leave, save timeline reset, expired subscriptions and offline state show the correct read-only data or unavailable state. Second-monitor/DPI checks and per-window capability denial are verified. Render the actual auxiliary hero/journal production compositions in tests for owned/guest/revoked/unavailable scopes; include each entrypoint in lint/build and browser or WebView smoke. Native lifecycle checks alone do not prove those production roots render.

  **Logging:** DEBUG window/scope/revision and subscription cleanup; INFO open/close/focus; WARN revoked scope; ERROR invalid projection. No cross-player private sheet/biography leakage.

### Final verification

- [ ] **Task 43: Meet measured rendering, loading and packaging budgets** (depends on 6, 9, 11, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42)

  **Files / ownership:** Vite production imports/chunking; visual catalog/cache and VttCanvas asset lifecycle; scripts/native packaging manifests; asset/browser/native performance tests.

  **Deliverable:** Load only required scene/actor assets and appropriate derivatives; reuse textures and release replaced object URLs/GPU resources. Split heavy presentation features where measurement justifies it. Measure cold startup, scrolling, map pan/drag and 20 actors at DPR2 on the same reference hardware; verify GPU/model contention separately. Keep approved source masters outside packaged imports and run offline local/cloud bundle checks.

  **Verification / acceptance:** Target encoded art/font shipping payload <=32MiB; preserve current bundled map limit and document any justified budget change before bulk production. Target visible decoded art <=96MiB and no >10% regression against a recorded comparable baseline. No blank art, long main-thread stalls or private URLs after repeated transitions. Metrics are measured targets, not claimed results; quality is not achieved by blindly downsampling.

  **Logging:** DEBUG safe cache/asset counts, decode bytes and frame samples in QA; INFO budget result/build hash; WARN measured regression; ERROR missing packaged asset. No per-frame production logging.

- [ ] **Task 44: Deliver the documented local product and prove all acceptance layers** (depends on 6, 12, 13, 14, 15, 16, 17, 18, 19, 20, 43)

  **Files / ownership:** README.md, CHANGELOG.md, AGENTS.md if entrypoints change, docs/visual/*, existing recovery/co-op/native guides, docs/RELEASE_CHECKLIST.md; all relevant tests and this plan progress.

  **Deliverable:** Through aif-docs update actual workflows, shortcuts, recovery, visual production/provenance, offline art, room links, import/export and auxiliary windows. Run final gates against frozen actual source/build, then production-browser, actual Tauri and packaged smoke. Exercise complete peaceful and defensive sessions, interruption/restart, two native clients and separately two physical LAN/VPN devices with approved installed models. Fix owned regressions before completion and clean up owned test processes.

  **Verification / acceptance:** All implemented tasks have functional/visual evidence, 100 selected masters with real consumers, and no forgotten dependency/capability approval. Keep UI/transport, real persistence, native packaging, semantic model quality, physical LAN, CI and publication verdicts separate. If model/device prerequisites remain unavailable or fail, those acceptance items stay open; never claim the whole product playable from fixture success. No push, release or deployment is authorized by this plan.

  **Logging:** INFO named acceptance layer/result and revision; DEBUG safe checkpoint IDs/counts; WARN unavailable prerequisites and existing warnings; ERROR real regressions. Docs contain no private campaign content, keys or invitations.

## Cross-cutting acceptance matrix

| Dimension | Required coverage |
|---|---|
| Roles | Solo, host, guest; own/other human actor; no accepted hero |
| Locales | RU/EN, long Russian labels, mixed-script names, symbols and Cyrillic font subsets |
| Screens | 960x640, 1280x800, 1920x1080 at DPR2; native 125/150/200% scale; removed monitor |
| Motion | Normal/reduced; hidden/unfocused; reload/rejoin without event replay |
| Recovery | Before/after effect commit; compound partial; EOF/error/empty/malformed response; clarification; cancel; cold restart; timeline restore; delayed old response |
| Rules | Movement, range, ownership, canonical weapons/armor/spells/resources, dying/dead/withdrawn actors; rejected actions |
| Art | True alpha, light/dark background, 30/48/64px miniatures, crop/tiling, all 100 consumers, missing/corrupt reference, offline fonts/media |
| Native | One instance, warm/cold link, expiry/single-use/rotation, denied notification, import/export/cancel, auxiliary close/main exit, process cleanup |
| Persistence | Same-request replay, current-state continuation, asset IDs, legacy migration, transaction rollback; no base64/secrets in normal preferences |
| Model/transport | Deterministic provider first; separate installed-model quality, actual Tauri, packaged profile, two-device LAN/VPN |

## Verification commands and evidence layers

1. Iterate with focused Vitest/Rust tests and behavior/contract regressions. RED/GREEN is appropriate for recovery and authority changes; do not add tests merely duplicating reversible text/CSS.
2. At relevant checkpoints run `bun run lint`, `bun run typecheck`, `bun run architecture:check`, `bun run build` and `bun run gates` according to scripts/gates.sh. Keep full gates distinct from focused tests.
3. Serve the freshly built production bundle using e2e preview mode. Run applicable existing specs plus new shared-table/layout/link/portability/auxiliary-window cases. Use unique ports/output directories and sequential runners when they share artifacts.

```powershell
$env:DMAI_E2E_PREVIEW = '1'
$env:DMAI_E2E_PORT = '<owned-free-port>'
bun run e2e -- <selected-spec-paths>
```

4. Run `bun run e2e:tauri` and `bun run e2e:tauri:campaign` with isolated profiles. Existing native harnesses must drive actual controls/APIs, not inject Zustand state as proof.
5. Build applicable cloud/local Tauri packages and inspect clean-profile offline art/fonts, OS links, dialogs, notifications, second launch and process/window cleanup. Record each actually tested platform; macOS/Linux cannot be claimed from Windows browser tests.
6. If sidecar code changes, run Python 3.12 Ruff and offline pytest. Real GPU/media tests remain opt-in with `RUN_GPU_TESTS=1` and owned model/process cleanup.
7. Complete real-model peaceful/defensive sessions, durable restart/recovery and two physical LAN/VPN devices separately. Existing planning/narration failures require continued owning-model work; do not weaken validators or substitute a fixture provider to call them passed.
8. Keep CI/release/deployment outside the local plan's authorization. Record current branch plus actual build/source hashes and explicit unavailable prerequisites.

## Definition of done

- All 44 task boxes describe delivered behavior and are checked only after their stated evidence exists.
- The selected 100-master catalog is coherent, optimized, persisted with stable IDs and consumed by real product surfaces; generations alone are not completion.
- Real production-root tests cover both mode adapters and all auxiliary entrypoints with their default labels/actor identities; lint, production build and local production-browser smoke pass.
- Recovery cannot reroll/repeat effects, lose scope or invent readiness; current state and canonical public outcomes determine controls.
- One readable, keyboard-accessible table serves solo/co-op with short truthful entry and optional technical details.
- Native additions meet narrowly approved dependency/capability decisions and actual packaged/native checks; auxiliary closure cannot stop the game.
- Current documentation describes actual shortcuts, recovery, portability, visuals and runtime limits.
- No unrelated source change, private data leakage, automatic model download or unauthorized publication is included.
- UI/transport delivery and overall AI-led/physical-LAN playability are reported separately. A missing prerequisite leaves its acceptance open and prevents an unqualified whole-product completion claim.

## Primary references verified for this plan

- Local `.ai-factory/ARCHITECTURE.md`, `AGENTS.md`, current source, `.ai-factory/RESEARCH.md`, owning active plans and docs/visual/LIVING_TABLETOP_ASSETS.md.
- [Manrope source and Cyrillic font history](https://github.com/google/fonts/blob/main/ofl/manrope/FONTLOG.txt). Pin and inspect the actual distributed font/metadata/license during Task 10.
- [Literata source, Cyrillic coverage and OFL](https://github.com/googlefonts/literata). The project repository is archived; use a verified pinned distributed font rather than assuming active upstream maintenance.
- [Tauri deep linking and single-instance integration](https://v2.tauri.app/plugin/deep-linking/). Validate the actual supported release/toolchain before package additions.
- [Tauri window-state](https://v2.tauri.app/plugin/window-state/) and [native dialogs](https://v2.tauri.app/plugin/dialog/). Their existence is not approval to install them.

## Planning handoff

Planning artifacts only: this active full plan and the byte-for-byte preserved completed foundation plan. No application source, tests, dependencies, art assets, config, research or roadmap is modified in this turn.
Next authorized workflow is `$aif-implement` when requested. Start at Task 1 using this worktree and actual dirty baseline; do not switch back to the older main checkout or execute historical superpowers instructions in the preserved foundation record.

## Execution record

- 2026-10-12 Task 1: captured 45 inherited file snapshots/hashes (39 dirty source files plus plan/untracked tests), actual HEAD and status in `.artifacts/product-uiux/baseline.json`. Production build and architecture guard pass; lint has 7 existing warnings/1 information item. Current production-browser baseline: 20 pass, 1 known retry-fixture failure, bounded 30s timeout. Sixteen screen captures cover 1280x800 and 960x640; fresh-context library readiness measured 406ms/259ms. Art/font payloads: 33 frontend images 6,295,143 bytes, 16 bundled images 1,891,590 bytes, 34 font files 989,052 bytes. This is browser/fixture evidence, not native/model acceptance.
- 2026-10-12 Task 2: fixture outbox is persisted before DM streaming, retained across errors and reconciled on terminal/clarification frames. Controller regressions cover before/after committed state, definitive rejection and unavailable pending reads. Focused Vitest 19/19, production co-op browser 8/8, native transport integration 17/17 and native parser unit tests 4/4 pass; TypeScript passes. Browser uses IPC fixtures; native integration proves supported service/storage boundaries, not actual Tauri UI or real-model quality. Rust uses the installed 1.96.0 toolchain explicitly because cargo is absent from the default PATH.
- 2026-10-12 Task 3: versioned player progress and owner-only continuation input are implemented for solo and authenticated native rooms. Canonical read does not repair delivery; hidden fields are excluded. Additive migration 0022 captures actor before planning and rejects replacement-hero resume. Focused projection 7/7, SQLite human-command contracts 22/22, authenticated real HTTP recovery, renderer parsing/native scope tests and production browser 21/21 pass. Full consolidated gates passed, including all Rust workspace tests and 173 frontend files/1146 tests; production build passes. Actual Tauri UI/model/physical LAN acceptance remains in Task 6/44.
- 2026-10-12 verification environment: preserved 115.87GiB target cache relocated from this worktree's C target to D:/codex-builds/dmai-uiux-20261012/target with a junction restoring the original path after C filled. Final full gates use installed Rust 1.96.0, CARGO_PROFILE_TEST_DEBUG=0 (Windows PDB limit), TEMP/TMP on the owned D directory, RUST_LOG=info (telemetry smoke requires INFO), and DMAI_SKIP_SIDECAR_BUILD=1 (unit gates only; real native checks rebuild sidecar separately). Three equivalent Clippy boolean expressions were simplified; no lint exception or test assertion was removed. New migration count/actor columns are asserted. Evidence: .artifacts/product-uiux/checkpoint-1-verified-gates.log and checkpoint-1-browser.
- 2026-10-12 checkpoint 1 committed as 92142a6. Full gates: 976 Rust tests and 1146 frontend tests pass; production build and browser 21/21 pass. Explicit owned-only patches for inherited-dirty route/turn/workflow/room-test files validated against HEAD. All original unrelated changes remain unstaged. No push/publication or real-model/Tauri-UI completion claim.
- 2026-10-12 Task 4: proactive solo read plus original-input hydration, shared recovery notice, exact-ID continuation and unsent-draft retention are wired into the production root. Native-room controller reads public progress for its durable DM request; app composition provides the same notice through a ReactNode slot without sibling feature imports. Delayed scopes/mixed snapshots/unavailable reads are guarded; configured VITE_RECOVERY_TRACE=1 emits only safe scope/phase IDs. Actual-root regression passes, focused UI/controller tests pass, production build and architecture pass; production-browser proactive/room coverage 10/10 passes. Actual Tauri/model restart proof remains Task 6. One inherited room-failure test label now expects Continue answer; preserve its pre-existing uncommitted test body when staging checkpoint 2.
