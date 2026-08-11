# Implementation Plan: Strict Explicit Architecture Refactor

Branch: feature/refactor-project-strict-explicit-architecture
Created: 2026-08-11

## Original Request

"Refactor the project to the strict Explicit Architecture target in .ai-factory/ARCHITECTURE.md"

## Settings

- Testing: yes
- Logging: verbose
- Docs: yes

## Goal

Refactor Dungeon Master AI into the strict inward-dependent architecture defined in `.ai-factory/ARCHITECTURE.md`: deterministic domain rules at the center, application-owned use cases and ports, isolated outbound adapters, thin Axum and Tauri delivery adapters, a single explicit composition root, and React as a presentation adapter rather than a second game engine.

## Architecture

The migration is contract-first and incremental. Stable HTTP paths, SSE event names, JSON shapes, SQLite data, Stronghold secret behavior, desktop commands/events, external binary names, and visible product behavior remain compatible while implementation ownership moves behind temporary facades. Each checkpoint must be green before the next extraction. Temporary facades are removed only after all callers migrate and the architecture guard proves the final dependency direction.

Target Rust dependency direction:

```text
app-domain <- app-application <- adapter-http
                           ^  <- adapter-sqlite
                           ^  <- adapter-llm
                           ^  <- adapter-media
                           ^  <- adapter-secrets
                           ^  <- app-bootstrap

src-tauri -> desktop process adapter and composition
```

Target frontend dependency direction:

```text
app -> features -> state/api -> validated wire contracts
ui  -> reusable presentation primitives only
```

## Tech Stack

- Rust 2021 workspace, Axum 0.8, Tokio, SQLx/SQLite, tracing, Stronghold, reqwest, genai
- React 19, TypeScript 6, Zustand, Valibot, Vite, Vitest, Playwright
- Tauri 2 desktop shell and external binaries
- Python 3.12 FastAPI media sidecar with offline pytest coverage
- Bun scripts and `scripts/gates.sh` as the local quality-gate entry point

## Global Constraints

- Do not add production dependencies. Reuse workspace crates, the standard library, Bun, and current test tooling.
- Treat the current dirty worktree as user-owned input. Capture `git status --short` and the relevant diffs before implementation, never use `git add .`, and stage only files explicitly owned by the completed checkpoint.
- Exclude `.git`, `.codegraph`, `.agents`, `.codex`, `target`, and `.dmai-fixpack-backup-*` from architecture scans and bulk formatting. Preserve those paths unless separately authorized.
- Preserve every route registered by the current `app-server`, `/agent/turn` request and SSE schema, chat serialization, settings/provider IDs, SQLite migrations and data, Stronghold key names, `dmai-server` binary naming, `backend_port`, `backend-ready`, `backend-exited`, and `APP_SERVER_LISTENING port=...`.
- Keep local-only and cloud-only builds supported on Windows, macOS, and Linux. Report deterministic browser smoke, real Tauri/WebView smoke, local-model smoke, CI, and release builds as distinct evidence.
- Use failing characterization or contract tests before changing behavior. Do not perform a repository-wide move that leaves the workspace uncompilable between checkpoints.
- Verbose logs are controlled by existing `RUST_LOG`/tracing configuration or a script-specific verbose flag. DEBUG records identifiers, event kinds, revisions, durations, and state transitions; INFO records lifecycle milestones; WARN records recoverable degradation; ERROR records failures with safe context. Never log prompts, chat text, API keys, Stronghold values, base64 media, vault passphrases, or full external payloads.

## Commit Plan

- **Commit 1** (after tasks 1-3): `refactor: establish explicit architecture core boundaries`
- **Commit 2** (after tasks 4-6): `refactor: move application workflows behind ports`
- **Commit 3** (after tasks 7-9): `refactor: extract runtime adapters and composition`
- **Commit 4** (after tasks 10-13): `refactor: make frontend a thin presentation adapter`

At every checkpoint, stage an explicit file list, inspect `git diff --cached`, run the listed focused checks and `bun run gates`, then create the checkpoint commit. Do not stage pre-existing unrelated changes merely to make the tree look clean.

## Tasks

### Phase 1: Freeze Contracts and Establish the Inner Core

- [x] **Task 1: Add characterization coverage and an executable architecture baseline**

  **Files:**
  - Create `crates/app-server/tests/agent_wire_contract.rs`
  - Modify `crates/app-llm/tests/provider_types_tests.rs`
  - Create `src/api/__tests__/agentEvents.test.ts`
  - Create `src/app/__tests__/productionRoot.test.tsx`
  - Modify `src-tauri/src/lib.rs` tests
  - Create `scripts/check-architecture.ts`
  - Create `scripts/architecture-allowlist.json`
  - Modify `package.json` and `scripts/gates.sh`

  **Deliverable:** Freeze the current external behavior before moves and add a zero-dependency guard that rejects new forbidden Rust dependency edges and frontend imports while explicitly listing only current legacy exceptions.

  **Implementation:**
  1. First capture `git status --short`, the current branch, and focused baseline results without modifying or staging existing dirty files.
  2. Add exact golden assertions for every agent SSE event: `reasoning_text`, `image_generated`, `text_delta`, `tool_call_start`, `tool_call_result`, `video_generated`, and `agent_done`, including event ordering and JSON field names.
  3. Extend provider serialization round trips for every `ChatMessage`, `MessagePart`, `ChatChunk`, `Tool`, `ToolCall`, and `ToolResult` variant used by SQLite and the frontend.
  4. Characterize LF, CRLF, CR, fragmented SSE blocks, final unterminated blocks, malformed events, abort, error, and finalization behavior without changing the public `streamAgentTurn` contract.
  5. Mount the real production `App` root with only Tauri/backend boundaries mocked. Assert `DUNGEON MASTER AI`, `Untitled Campaign`, `The Adventure`, the composer, hydration behavior, and absence of inherited product or actor identities.
  6. Characterize Tauri backend readiness, termination, malformed stdout, and duplicate event handling around the existing `parse_listening_port` seam.
  7. Implement `bun run architecture:check`. Use `cargo metadata --format-version 1 --no-deps` for workspace edges and a bounded source scan for forbidden frontend imports. The allowlist contains exact source/target/reason entries, rejects unknown entries, and can only shrink during later tasks.
  8. Add the architecture check to `scripts/gates.sh --fast`, so local hooks and CI lint run it.

  **Tests:** Write the assertions first and confirm they fail for the missing coverage/guard, then make them pass. Run the new focused Rust and Vitest files, `bun run architecture:check`, `bun run gates:fast`, `bun run build`, and a mocked `bun run e2e` smoke.

  **Logging:** The architecture script prints a concise INFO summary by default and edge/import details under `ARCH_CHECK_VERBOSE=1`; violations use ERROR with file and rule but never source contents. Test diagnostics may show event kinds and sequence numbers, not payload text or media.

  **Acceptance:** Existing wire behavior is executable, the real production root is covered, the guard is cross-platform and ignores generated/backup trees, no production dependency is added, and pre-existing failures are reported separately from regressions.

- [x] **Task 2: Introduce `app-application` with inward-owned contracts and compatibility exports** (depends on 1)

  **Files:**
  - Modify root `Cargo.toml` and `Cargo.lock`
  - Create `crates/app-application/Cargo.toml`
  - Create `crates/app-application/src/lib.rs`
  - Create `crates/app-application/src/ports/{llm,repositories,secrets,media,runtime,events}.rs`
  - Create `crates/app-application/src/models/{agent,chat,combat,campaign,local_models}.rs`
  - Modify `crates/app-llm/src/{lib,provider,sidecar_launcher}.rs` and its tests
  - Modify affected manifests that import moved contracts

  **Deliverable:** A real application crate that owns the stable protocol-neutral models and capabilities required by use cases, with `app-llm` temporarily re-exporting moved public types so current consumers continue to compile.

  **Implementation:**
  1. Move `LlmProvider`, `ChatRequest`, `ChatMessage`, `MessagePart`, `ChatChunk`, `Tool`, `ToolCall`, `ToolResult`, `Capabilities`, `LlmError`, and `ChunkStream` into `app-application::ports::llm` and `models::chat` according to whether each item is a capability or value.
  2. Define narrow async ports: `MessageRepository`, `SaveRepository`, `CombatRepository`, `JournalRepository`, `NpcRepository`, `SceneRepository`, `SrdRepository`, `SecretsStore`, `ImageGenerator`, `VideoGenerator`, `RuntimeControl`, and `ApplicationEventSink`. Each method accepts application/domain IDs and values, never SQLx rows, Axum types, genai types, Tauri handles, or raw HTTP DTOs.
  3. Move `SidecarLauncher`, `SidecarHandle`, `SidecarError`, and `SpawnSpec` to the runtime port while keeping compatibility re-exports.
  4. Move `AgentTurnRequest` and `AgentEvent` into application models and add a versioned `CombatProjection { encounter_id, revision, snapshot, events }` contract for downstream reconciliation.
  5. Make `app-application` depend only on `app-domain` plus current workspace-neutral support crates. It must not depend on `app-llm`, Axum, SQLx, Tauri, genai, reqwest, Stronghold, or adapter crates.

  **Tests:** Move and extend contract tests so they target `app-application` directly, retain facade import tests for `app_llm::*`, run `cargo test -p app-application`, `cargo test -p app-llm`, and `bun run architecture:check`.

  **Logging:** Ports do not mandate concrete logging implementations. Application errors carry safe codes and operation context; implementations will emit tracing fields such as `operation`, `session_id`, `encounter_id`, `provider_id`, and `duration_ms` without content bodies.

  **Acceptance:** Current callers compile through facades, all inward interfaces are adapter-neutral, no adapter-to-adapter dependency is introduced, and the architecture allowlist shrinks for the moved contracts.

- [x] **Task 3: Move raw tool decoding and runtime metadata out of the domain** (depends on 2)

  **Files:**
  - Create `crates/app-application/src/agent/{mod,tool_decoder,commands}.rs`
  - Move tests from `crates/app-domain/tests/validator_tests.rs` to `crates/app-application/tests/tool_decoder_tests.rs`
  - Modify `crates/app-domain/src/combat/{mod,validator,resolver}.rs` and `crates/app-domain/src/lib.rs`
  - Move `crates/app-domain/src/local_llm/**` callers to `app-application::models::local_models`
  - Create `crates/adapter-llm/Cargo.toml` and `crates/adapter-llm/src/{lib,embeddings}.rs`
  - Move `crates/app-domain/src/srd/embedder.rs` integration into `adapter-llm`
  - Modify root and affected crate manifests

  **Deliverable:** Untrusted tool JSON and model/runtime metadata stop at the application edge; `app-domain` receives typed actions and loses runtime/framework dependencies.

  **Implementation:**
  1. Define an exhaustive `AgentToolCommand` enum with typed variants for every currently supported tool name. `decode_tool_call(name: &str, args: &serde_json::Value) -> Result<AgentToolCommand, ToolDecodeError>` belongs to `app-application`.
  2. Translate combat variants into existing `CombatAction` values and let domain invariants decide legality. Remove raw tool-name dispatch and raw `serde_json::Value` from domain APIs.
  3. Move `SystemEntry` and `UserEntry` model-manifest types to application models and update HF/local-model callers through compatibility re-exports only where needed.
  4. Move fastembed initialization, query/chunk embedding, model parsing, cache/dimension invalidation, and background readiness concerns into `adapter-llm::embeddings`. Keep `SrdChunk`, pure cosine retrieval, and deterministic SRD concepts in the domain.
  5. Remove direct `fastembed`, Tokio, tracing, and serde_json dependencies from `app-domain`. Keep serde/uuid/thiserror/rand and data parsing that is deterministic and domain-owned.

  **Tests:** Add golden decode success/failure cases for every tool, unknown fields/names, invalid IDs, numeric bounds, and malformed JSON. Run all `app-domain` tests, `app-application` decoder tests, embedding adapter tests with the heavy model boundary mocked, `cargo tree -p app-domain`, and the architecture guard.

  **Logging:** Decoder DEBUG logs contain only tool name, call ID, variant, and validation code. WARN records rejected tool calls with safe codes. Embedding logs record model ID, cache path category, dimensions, counts, duration, readiness, and degraded state without SRD text or user queries.

  **Acceptance:** Domain behavior stays deterministic, `app-domain` has no forbidden runtime dependencies, all old validator behavior is represented by application tests, and Commit 1 can be created from explicit task-owned files.

### Phase 2: Move Workflows Behind Ports

- [x] **Task 4: Move agent and combat orchestration into application use cases** (depends on 3)

  **Files:**
  - Create `crates/app-application/src/agent/{turn,context,tool_dispatch,tools}.rs`
  - Create `crates/app-application/src/combat/{commands,projection,resolve}.rs`
  - Refactor `crates/app-server/src/agent/{orchestrator,context_builder,tool_executor,tools}.rs` into compatibility facades
  - Modify related app-server agent/combat tests

  **Deliverable:** `AgentTurnService` and combat use cases coordinate only inward-owned ports. The 1,700-line tool executor is split into typed per-capability handlers and no application module accepts a `SqlitePool`, concrete provider, Axum type, or global `AppState`.

  **Implementation:**
  1. Define `AgentTurnService::execute(AgentTurnCommand) -> Stream<AgentEvent>` with explicit constructor dependencies for LLM, repositories, media, runtime coordination, and event publication.
  2. Split context building, tool dispatch, campaign tools, combat tools, media tools, and persistence sequencing into focused modules. Preserve message persistence timing and SSE ordering.
  3. Implement `ResolveCombatAction::execute(ResolveCombatCommand) -> ResolveCombatResult` using `CombatRepository`, domain resolution, atomic persistence, and a monotonically increasing `revision` returned with the authoritative snapshot/events.
  4. Make duplicate/stale command handling explicit through request IDs and combat revisions. Never claim success until domain resolution and required persistence succeed.
  5. Keep `app_server::agent` facades only while route and test callers migrate.

  **Tests:** Add port fakes and deterministic use-case tests for success, tool rejection, provider failure, persistence failure, cancellation, duplicate request, event ordering, media-before/after-tool-result, stale combat revision, and rollback/no-partial-write semantics. Run focused application tests plus existing agent/combat integration tests.

  **Logging:** Add tracing spans per use case. DEBUG includes request/call IDs, event kind, sequence, combat revision, port operation, and elapsed time. INFO records turn/combat lifecycle completion. WARN records recoverable tool/provider degradation. ERROR records terminal failure and failed port, with content redacted.

  **Acceptance:** Agent and combat workflows run in tests without Axum, SQLx, concrete LLM/media providers, or AppState; existing endpoint tests remain green through facades; combat output is authoritative and versioned.

- [x] **Task 5: Extract SQLx persistence into `adapter-sqlite`** (depends on 2, 4)

  **Files:**
  - Create `crates/adapter-sqlite/Cargo.toml`
  - Create `crates/adapter-sqlite/src/{lib,pool,messages,saves,combat,journal,npcs,scenes,srd}.rs`
  - Move `crates/app-server/migrations/**` to the adapter while preserving embedded migration identity and packaging
  - Refactor `crates/app-server/src/db.rs` and direct SQL in `agent/**` and `routes/{agent,combat}.rs`
  - Move/expand database contract tests under `crates/adapter-sqlite/tests/**`
  - Modify root and affected manifests

  **Deliverable:** SQLx implements application repository ports in one outbound adapter; no application use case or HTTP handler performs SQL or imports a pool.

  **Implementation:** Split existing persistence by repository responsibility, translate rows at the adapter boundary, preserve all migrations/tables/serialization, and implement multi-entity combat/save/message operations in explicit transactions. Keep a temporary `app_server::db` facade for unmigrated test imports, then shrink it as callers move.

  **Tests:** Run each repository contract suite against isolated temporary SQLite databases, including migration from the existing schema, chat round trips, save/restore, combat start/resolve/end, journal/NPC/scene/SRD operations, concurrency, transaction rollback, and legacy data compatibility. Run existing app-server DB/route tests unchanged where possible.

  **Logging:** SQL adapter spans log repository method, safe entity ID, row count, transaction boundary, migration version, and duration. ERROR includes database error classification but not SQL-bound text, prompts, secrets, or serialized snapshots.

  **Acceptance:** `rg`/architecture guard finds no SQLx in `app-domain`, `app-application`, `adapter-http`, or application facades; all schema/data compatibility tests pass.

- [x] **Task 6: Extract secrets and make settings/provider reconfiguration atomic** (depends on 2, 4, 5)

  **Files:**
  - Create `crates/adapter-secrets/Cargo.toml`
  - Create `crates/adapter-secrets/src/{lib,stronghold,migrate,in_memory}.rs`
  - Refactor `crates/app-server/src/secrets/**`
  - Create `crates/app-application/src/models/settings.rs`
  - Create `crates/app-application/src/settings/{mod,update}.rs`
  - Refactor `crates/app-server/src/providers/registry.rs` and `routes/settings/**`
  - Move/expand settings and secret tests

  **Deliverable:** Stronghold and legacy-secret migration implement `SecretsStore`, while `UpdateSettings` validates all requested changes, constructs required capabilities through ports/factories, and atomically swaps the provider/config snapshot only after all fallible work succeeds.

  **Implementation:** Preserve `huggingface_token`, `openai_compat_api_key`, `replicate_api_key`, `.secrets_migrated_v1`, legacy backup behavior, and non-disclosure in route responses. Replace route-owned provider construction and partial mutation with an application result that either commits the complete new snapshot or leaves the old snapshot intact.

  **Tests:** Cover Stronghold/in-memory contract parity, wrong/missing vault credentials, legacy migration idempotence, secret deletion, concurrent reads, settings validation, provider factory failure, secret write failure, and no-partial-update behavior. Retain current `/settings/v2` wire tests.

  **Logging:** INFO records vault open/migration status and settings snapshot revision. DEBUG records key category and provider/model IDs, never values. WARN records degraded optional capabilities. ERROR records safe error codes and rollback outcome.

  **Acceptance:** Routes cannot access Stronghold or concrete provider constructors directly, no API returns secret values, atomicity tests pass, and Commit 2 includes only tasks 4-6 files.

### Phase 3: Extract Adapters and Composition

- [ ] **Task 7: Complete LLM and media adapter extraction with explicit runtime contracts** (depends on 3, 4, 6)

  **Files:**
  - Move concrete `crates/app-llm/src/{genai_common,openai_compat,mistralrs_provider,retry,mock}.rs` into `crates/adapter-llm/src/**`
  - Create `crates/adapter-media/Cargo.toml`
  - Move/refactor `crates/app-server/src/{image,video,local_runtime,models,hf}/**` into `crates/adapter-media/src/**`
  - Refactor `crates/app-llm` and relevant `app-server` modules into temporary re-export facades
  - Modify sidecar contract code and `sidecar/tests/**`
  - Modify workspace manifests and `Cargo.lock`

  **Deliverable:** Concrete genai, OpenAI-compatible, mistralrs, embedding, HF, image, video, download, and local-runtime code lives only in outbound adapters implementing application ports.

  **Implementation:**
  1. Keep adapters independent of one another; provider selection, cross-modal policy, and GPU ownership coordination live in application/bootstrap services.
  2. Preserve LLM `/health`, media `/healthz`, image `/generate` and `/unload`, video `/video/generate`, timeouts, retry semantics, and `LISTENING_ON_PORT=<n>`.
  3. Version the Rust/Python media contract and make `init_image_b64` either implemented or explicitly unsupported in capabilities. Add cancellation/degraded outcomes and serialize GPU-exclusive image/video/model operations with a real async lock rather than an atomic label.
  4. Preserve the 180-attempt, 2-second local-model readiness budget and explicit child-process liveness checks until Tauri becomes the final owner in task 9.

  **Tests:** Move provider tests to `adapter-llm`; add adapter contract tests for retries, malformed streams, health/liveness, cancellation, dropped video streams, unsupported init image, concurrent GPU requests, auto-swap recovery, and secret redaction. Run offline `pytest sidecar/tests`, Rust adapter tests, and existing local runtime probes.

  **Logging:** Provider/media DEBUG logs contain model/provider/backend IDs, attempts, chunk/event kinds, GPU owner transitions, progress, and durations. INFO records start/ready/stop. WARN records retry/degraded/cancel. ERROR records terminal protocol/process failures. Never log model input/output text or encoded media.

  **Acceptance:** `app-llm` is only a compatibility facade, application crates do not import concrete adapters, media concurrency is deterministic, and Rust/Python capability claims agree.

- [ ] **Task 8: Extract a thin Axum inbound adapter** (depends on 4-7)

  **Files:**
  - Create `crates/adapter-http/Cargo.toml`
  - Move/refactor `crates/app-server/src/routes/**`, `error.rs`, and router construction into `crates/adapter-http/src/**`
  - Create explicit HTTP DTO and SSE mapping modules
  - Refactor `crates/app-server/src/state.rs` to a narrow compatibility service bundle
  - Move route integration tests to `crates/adapter-http/tests/**` while keeping `app_server::test_support::TestServer`

  **Deliverable:** Every handler validates a wire DTO, calls exactly one application use case/service, and maps typed results/errors to HTTP or SSE. No handler imports SQLx, Stronghold, provider implementations, process launchers, or global mutable registries.

  **Implementation:** Preserve all current route paths/methods/status codes and serialized envelopes. Move `agent_event_to_sse` into a pure tested mapper. Replace route-level persistence, provider construction, GPU swap, and direct combat SQL with injected application services. Expose a narrow immutable `HttpServices` bundle rather than the current broad `AppState` service locator.

  **Tests:** Run the golden wire suite and all existing route suites against fakes and real SQLite adapter composition. Add malformed DTO, body-size, cancellation, backpressure, error mapping, SSE disconnect, and authoritative combat revision cases.

  **Logging:** HTTP tracing records method, route template, request ID, status, duration, SSE event kind/count, and safe application error code. It excludes headers/tokens, request bodies, response text, and media payloads.

  **Acceptance:** The architecture guard finds no direct adapter imports between HTTP and SQL/LLM/media/secrets adapters, all existing frontend wire tests pass, and `AppState` is no longer used inside application logic.

- [ ] **Task 9: Create the composition root and make Tauri the single process owner** (depends on 7, 8)

  **Files:**
  - Create `crates/app-bootstrap/Cargo.toml` and `crates/app-bootstrap/src/{lib,config,paths,telemetry,wiring,main}.rs`
  - Refactor `crates/app-server/src/{main,lib,config,paths,telemetry,state}.rs` into a compatibility facade/binary wrapper
  - Create `src-tauri/src/commands/backend.rs` and `src-tauri/src/processes/{backend,model_runtime,media_runtime,control}.rs`
  - Modify `src-tauri/src/lib.rs`, `Cargo.toml`, `build.rs`, `capabilities/default.json`, `tauri.conf.json`, and `tauri.cloud.conf.json`
  - Modify `scripts/tauri-build-cloud.ts`, sidecar build scripts, and prebuild/release workflows

  **Deliverable:** `app-bootstrap` wires concrete adapters into application services and exposes the Axum router; Tauri is the only production owner of `dmai-server`, `mistralrs-server`, and `dmai-image-sidecar` processes.

  **Implementation:**
  1. Preserve the external binary name `dmai-server` and readiness stdout contract while moving construction out of app-server.
  2. Add a narrow authenticated loopback runtime-control channel: Tauri binds loopback with a per-launch random token passed to `dmai-server` through environment, and the application `RuntimeControl` adapter requests typed start/stop/status operations. Never persist or log the token.
  3. Tauri owns child handles, health/liveness observation, shutdown ordering, crash events, and cloud-only absence. Remove the unused duplicate launcher path after all runtime routes use the control port.
  4. Align externalBin names, capability allowlists, Python bundle layout, target triples, Python 3.12 workflow, and mistralrs version source. Cloud builds package only `dmai-server`; local builds prove all declared binaries.
  5. Keep database/vault path compatibility and require an explicit secure passphrase source in production rather than silently relying on a built-in value.

  **Tests:** Add bootstrap wiring tests, authenticated-control rejection/replay tests, Tauri child lifecycle state-machine tests, shutdown/crash/degraded tests, capability manifest tests, binary staging tests for local/cloud modes, and cross-platform path tests. Run `cargo test --workspace`, `bun run tauri:build:cloud`, deterministic `bun run e2e:tauri`, and separately report any full local-sidecar runtime smoke.

  **Logging:** Tauri/bootstrap logs record process role, PID, safe executable category, lifecycle state, probe attempt, port, duration, and exit classification. INFO readiness remains machine-parseable. Tokens, vault credentials, arguments containing secrets, and model prompts are redacted.

  **Acceptance:** There is one concrete production process owner, bootstrap is the only place that constructs adapters, cloud/local packaging matches manifests, no default production vault secret remains, compatibility APIs stay green, and Commit 3 contains only tasks 7-9 files.

### Phase 4: Thin the React Presentation Adapter

- [ ] **Task 10: Separate typed agent transport from event reduction** (depends on 4, 8)

  **Files:**
  - Create `src/api/contracts/agent.ts` and `src/api/sseStream.ts`
  - Refactor `src/api/agent.ts`
  - Create `src/features/agent-turn/model/{ports,boardSnapshot,reduceAgentEvent}.ts`
  - Create `src/features/agent-turn/useAgentTurn.ts`
  - Refactor `src/hooks/useAgentTurn.ts` into a temporary re-export and migrate callers
  - Expand agent API/hook tests

  **Deliverable:** The API adapter owns validated wire conversion and incremental SSE parsing; a pure event reducer operates through narrow injected presentation ports; the hook only coordinates one turn lifecycle.

  **Implementation:** Preserve history mapping, event ordering, retry text-only behavior, image `kind: "map" | "chat"`, media attachment timing, abort, and finalize semantics. The decoder must flush EOF tails and handle LF/CRLF/CR. Reducer ports cover chat, tool log, journal, NPC, session, combat projection, and media without exposing `StoreApi<AppState>`.

  **Tests:** Add exhaustive event fixtures, malformed payload rejection, reducer idempotence/order, journal/NPC/scene mapping, map/chat routing, handler exception isolation, abort, and finalization. Run focused Vitest, `bun run lint`, `bun run typecheck`, and `bun run build`.

  **Logging:** API DEBUG logs record request ID, event kind, sequence, byte counts, parser state, and duration. Reducer logs state transitions by entity ID and revision. WARN/ERROR use safe event/error codes, never message text or base64.

  **Acceptance:** `useAgentTurn` no longer acts as a service locator, transport is independent of Zustand/components, production-root and existing chat tests remain green.

- [ ] **Task 11: Replace client combat mechanics with authoritative server projections** (depends on 4, 8, 10)

  **Files:**
  - Create `src/features/combat/model/{types,combatProjection,commands,selectors}.ts`
  - Refactor `src/state/combat.ts`, `src/hooks/useCombatToolHandlers.ts`, and `src/state/conditions.ts`
  - Refactor `ActionBar.tsx`, `VttCanvas.tsx`, `InitiativeTracker.tsx`, `CombatOverlay.tsx`, and `App.tsx` combat wiring
  - Create `e2e/combat-authority.spec.ts`
  - Expand combat state/component tests

  **Deliverable:** Zustand stores only the latest versioned server projection plus local presentation/pending state. React sends typed attack/cast/move/end-turn commands and never commits HP, conditions, turn, round, movement, or action economy before a newer authoritative revision arrives.

  **Implementation:** Move shared `CombatToken`, `SnapshotCombat`, and `AoeShape` into feature model types. Add `replaceProjection` with stale/duplicate revision rejection. Remove invented enemy defaults/random IDs, local initiative selection, movement validation, condition mechanics, HP clamps, and local action consumption only after every caller uses application results. Keep pan/zoom/layers/measurement/AoE preview local.

  **Tests:** Cover stale/duplicate revisions, replacement, rejected commands, pending/reconcile, PC-turn gating, drag without optimistic movement, condition/damage updates only after server events, and map versus illustration routing. Playwright must prove the command is sent while local state remains unchanged until the mocked authoritative event.

  **Logging:** DEBUG records command ID, encounter ID, requested action kind, current/incoming revision, pending/reconciled/rejected transitions, and render-safe entity IDs. WARN records stale/conflicting events. No board snapshot, prompt prose, or image data is logged.

  **Acceptance:** TypeScript contains no competing combat-rule tables or mutation paths, server revisions are the only authority, controls retain their visible behavior, and deterministic browser combat acceptance passes.

- [ ] **Task 12: Make save restoration atomic and split the production shell by feature** (depends on 10, 11)

  **Files:**
  - Refactor `src/api/saves.ts`
  - Create `src/features/saves/model/buildRestoredSession.ts` and `src/features/saves/useSaves.ts`
  - Add one atomic restore action to the relevant Zustand composition
  - Create `src/app/App.tsx`, `src/app/AppShell.tsx`, `src/app/AppOverlays.tsx`, `src/app/useAppShellController.ts`, and `src/app/bootstrap.tsx`
  - Move feature-owned UI/controllers under `src/features/{chat,combat,saves,settings,character,onboarding,journal,npcs,local-mode}/`
  - Refactor `src/main.tsx`, `src/App.tsx`, `src/components/**`, and `src/state/useStore.ts` through temporary re-exports
  - Expand production-root, save, session, chat, and hydration tests

  **Deliverable:** Restore validation builds a complete `{session, messages, pc, combat, scene}` projection before one store commit; the production root composes feature controllers and prop-driven views without owning product workflows.

  **Implementation:** Validate save/message/restore DTOs before mutation. Prefer a backend restore response containing the full projection so no later fetch can fail after server mutation. Preserve tool-call history filtering, persisted-state whitelist, Stronghold-only credentials, transient chat/combat/map-image exclusion, hydration gating, CSS classes/tokens, test IDs, localized labels, keyboard/window behavior, and retry semantics. Use compatibility re-exports until imports are fully migrated, then remove them.

  **Tests:** Cover invalid schema/version, no partial writes, V2 combat/scene mapping, message limits/roles, localized errors, persistence whitelist byte semantics, onboarding hydration, shell shortcuts/modals, chat composer/retry, and the real production root in English and Russian. Run full Vitest, lint, typecheck, production build, Playwright smoke, and real Tauri smoke.

  **Logging:** Save/shell controllers log operation IDs, save/session IDs, validation stage, entity counts, hydration and modal lifecycle, and durations. WARN/ERROR use localized user messages but structured safe log codes. Never log save bodies, chat content, persisted secrets, or map images.

  **Acceptance:** `main.tsx` only initializes styles/i18n and mounts bootstrap, `App` is composition-focused, feature modules do not import sibling internals, save restore cannot partially mutate the frontend, and visible EN/RU behavior is preserved.

- [ ] **Task 13: Enforce final boundaries, remove facades, document the delivered architecture, and run acceptance** (depends on 1-12)

  **Files:**
  - Tighten `scripts/check-architecture.ts` and delete `scripts/architecture-allowlist.json` when empty
  - Remove obsolete `app-llm`, `app-server`, legacy hook/component/state facades only after zero callers remain; retain a facade only when required by the stable binary or public test contract and document that role
  - Modify `scripts/gates.sh`, `.github/workflows/{lint,test,quality,prebuild-python-sidecar,prebuild-sidecars,release}.yml`, and package scripts as required
  - Update `.ai-factory/ARCHITECTURE.md`, top-level `ARCHITECTURE.md`, `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `RELEASE.md`, `RELEASE_CHECKLIST.md`, and `AGENTS.md` through the mandatory `$aif-docs` checkpoint

  **Deliverable:** The architecture guard describes the final strict graph with no legacy exceptions, all compatibility paths have an explicit final disposition, documentation matches actual runtime/package behavior, and the product passes layered acceptance.

  **Implementation:**
  1. Enforce Rust allowlists for inner crates, no route-to-concrete-adapter imports, no adapter-to-adapter imports, and frontend rules: `ui` imports no app/features/state/api, `api` imports no Zustand/components, `state` imports no React components, features import no app or sibling internals, and app remains composition-only.
  2. Add offline Python pytest and Ruff checks to normal CI using Python 3.12. Keep GPU tests explicitly opt-in.
  3. Run `$aif-docs` after implementation is stable. Synchronize both architecture documents and correct process ownership, target matrix, sidecar packaging, limitations, verification commands, secret handling, and module map.
  4. Run focused tests first, then `bun run gates`, `bun run e2e`, deterministic `bun run e2e:tauri`, sidecar tests, and cloud Tauri builds. Use `gh` for GitHub workflow inspection if remote CI is requested; do not use GitHub MCP and do not push without explicit authorization.
  5. Report local gates, mocked Playwright, real Tauri/WebView, local-model/GPU smoke, cloud bundle matrix, remote CI, and deployment/release evidence separately. A skipped layer is not implied green by another layer.

  **Tests:** Required final commands are `bun run architecture:check`, `bun run gates`, `bun run e2e`, `bun run e2e:tauri`, offline `pytest sidecar/tests`, and `bun run tauri:build:cloud`. Run the local-model play flow only when staged models/runtime are available and label it separately.

  **Logging:** Final gate output identifies each layer, command, duration, result, and artifact path. CI logs follow the same safe redaction rules. Documentation includes the logging controls and forbidden data list.

  **Acceptance:** All strict dependency rules in `.ai-factory/ARCHITECTURE.md` are mechanically enforced, no empty layers or unexplained facades remain, required local gates are green or have separately documented pre-existing blockers, docs match code, browser and Tauri evidence are current, and Commit 4 can be made without staging unrelated dirty work.

## Dependency Summary

```text
1 -> 2 -> 3 -> 4 -> 5
                   |
                   + -> 6 -> 7 -> 8 -> 9
                         |         |
                         +---------+-> 10 -> 11 -> 12 -> 13
```

- Tasks 1-3 establish stable contracts and the inward core.
- Tasks 4-6 move behavior and state behind ports before concrete extraction.
- Tasks 7-9 isolate adapters, bootstrap, packaging, and process ownership.
- Tasks 10-12 consume the new server contracts and remove frontend rule duplication.
- Task 13 is the mandatory enforcement, documentation, and full acceptance checkpoint.
