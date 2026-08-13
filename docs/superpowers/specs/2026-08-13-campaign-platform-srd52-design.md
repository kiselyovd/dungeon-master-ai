# Dungeon Master AI Campaign Platform and SRD 5.2.1 Design

**Status:** Approved
**Date:** 2026-08-13
**Target:** Strict Explicit Architecture defined in `.ai-factory/ARCHITECTURE.md`

## 1. Purpose

Turn Dungeon Master AI from a command-following chat wrapped around a VTT into a complete campaign platform:

- a campaign home appears before the game table;
- a player can resume a campaign, create one with the DM, import source documents, or choose an original bundled adventure;
- the DM maintains continuity, hidden plans, and a world that advances in response to play;
- the VTT displays a complete terrain map and supports authoritative tactical combat;
- the rules engine targets only the current 2024 rules in SRD 5.2.1;
- Tauri automatically owns and supervises every backend and inference process;
- multiplayer and distributed inference have stable inward-owned contracts before a concrete network transport is added.

There is no production user data to preserve. Legacy 2014-rule types, compatibility conversions, fixtures, and database structures may be replaced instead of maintained in parallel.

## 2. Product principles

1. **The DM runs a world, not a command executor.** Player input describes intent. The DM frames situations, applies consequences, advances threats, and offers possibilities without limiting free-form action.
2. **One authoritative state.** The host backend alone commits campaign and combat facts. React and future peers display confirmed projections and submit typed intentions.
3. **Secrets stay secret.** Hidden campaign knowledge, undiscovered document passages, provider secrets, and private prompts never appear in player projections or compute-worker jobs.
4. **Local-first and degradable.** A media failure must not stop play. Bundled maps and illustrations provide an offline fallback.
5. **Zero-touch operation.** Development has one command; distribution has one installer. Users never launch crates, Python, or model servers manually.
6. **Evidence is layered.** Unit tests, full gates, browser mock tests, real Tauri CDP tests, packaged smoke tests, and live-model playthroughs are reported separately.

## 3. Scope and delivery strategy

Implementation proceeds as vertical slices that remain runnable after each checkpoint:

1. replace the game domain and schemas with SRD 5.2.1;
2. introduce the campaign home and creation workflow;
3. add campaign documents and DM memory retrieval;
4. make the DM orchestrator continuity-aware and proactive;
5. complete map layout and authoritative tactical combat;
6. add transport-neutral multiplayer and distributed-compute contracts;
7. automate development, packaging, process supervision, and acceptance.

The following are deliberately deferred:

- a visible create-room or join-room UI;
- a production WebRTC, signaling, relay, or NAT traversal dependency;
- host migration after host disconnect;
- tensor or pipeline parallel execution across multiple computers;
- redistribution of copyrighted commercial adventures or settings;
- OCR for scanned documents unless separately approved;
- a second rules profile for the 2014 edition.

## 4. System ownership and dependency direction

The existing Explicit Architecture remains the boundary map:

- `app-domain` owns deterministic SRD 5.2.1 rules and pure game values;
- `app-application` owns campaign, DM, combat, session, multiplayer, and compute use cases plus their ports;
- adapter crates implement HTTP, SQLite, LLM/embedding, media, secrets, document extraction, and future network transport;
- `app-bootstrap` composes the backend into the stable `dmai-server` binary;
- `src-tauri` is the sole production process owner and native desktop adapter;
- React is a presentation adapter over typed projections and commands;
- the Python sidecar performs heavyweight image/video inference only.

The host contains the only authoritative application service bundle. Local HTTP/SSE and a future peer transport call the same application use cases. Neither transport may duplicate game behavior.

## 5. SRD 5.2.1 domain reset

### 5.1 Version policy

The product targets only SRD 5.2.1, licensed under CC BY 4.0. The project must include the attribution statement required by the SRD in product legal information and distribution documentation.

No `rules_version` branch is needed in gameplay code. Persisted aggregates still include a schema/contract version so incompatible development data is rejected clearly rather than decoded incorrectly.

### 5.2 Domain coverage

The deterministic domain must represent at least:

- D20 Tests and degrees/results required by use cases;
- 2024 character creation concepts, species, backgrounds, classes, feats, equipment, spells, and monsters included in SRD 5.2.1;
- Weapon Mastery;
- action, bonus action, reaction, movement, concentration, and resource economies;
- damage, healing, resistance, immunity, vulnerability, temporary HP, death, and stabilization;
- conditions and their mechanical effects;
- cover, difficult terrain, opportunity attacks, areas of effect, hazards, and objectives;
- initiative, turns, rounds, combat start/end, and encounter results.

Raw LLM JSON is decoded into typed application commands before it reaches the domain. Domain modules never import Axum, SQLx, Tauri, provider SDKs, or React concepts.

### 5.3 Data reset

Development databases that predate the new schema may be recreated. The implementation must remove or update old fixtures and tests rather than silently coercing 2014 values. Save and protocol fixtures become golden examples of the new version.

## 6. Campaign home and creation experience

### 6.1 Application routes

The production root has explicit presentation states:

1. bootstrap and hydration;
2. first-run technical onboarding when required;
3. campaign home;
4. campaign creation;
5. character creation when the campaign lacks an assigned hero;
6. world preparation;
7. active game table.

The campaign home is a full application screen, not a modal over the VTT. It uses the approved **Option A** layout:

- the last campaign is the primary hero card with **Continue**;
- secondary actions are **New campaign**, **Campaign library**, and **Adventures**;
- multiplayer actions remain absent until they work.

### 6.2 New campaign flow

The approved primary path is **Create together with the DM**. Two sibling paths remain available:

- import a world through PDF or DOCX;
- choose an original bundled adventure.

The collaborative questionnaire is intentionally compact:

- genre/direction;
- desired feelings and tone;
- geographic scale;
- challenge and content boundaries;
- expected campaign length;
- an optional seed sentence.

The DM produces a player-visible premise and a hidden initial campaign bible. Hidden twists are not returned in the creation response.

Character creation follows campaign creation and receives campaign context. World preparation then creates the opening scene, initial known NPCs, immediate hooks, a VTT map specification, and a first autosave before entering the table.

### 6.3 Original bundled adventures

Bundled adventures contain project-owned or permissively licensed original text, maps, illustrations, NPCs, encounter seeds, and attribution metadata. SRD material may be adapted under CC BY 4.0 with the required attribution. Commercial D&D adventures and protected settings are not copied into the product.

## 7. Campaign documents and memory

### 7.1 Documents are campaign sources

PDF and DOCX files are uploaded through a dedicated campaign library, never as chat attachments. The library shows extraction/index state, source metadata, page/section counts, and actionable errors.

Document bytes remain local to the host. A document ingestion use case:

1. validates file type, size, and bounds;
2. stores source metadata and a content checksum;
3. extracts text in an outward adapter;
4. normalizes and chunks it with page/section provenance;
5. labels access scope;
6. creates embeddings through an inward-owned embedding port;
7. atomically publishes a searchable document revision.

Encrypted, corrupt, scanned-without-text, excessively large, or unsupported files fail without partially publishing chunks. OCR is outside this design.

Selection of any new production parsing dependency requires separate explicit approval before manifest changes. The implementation plan must first inventory reusable workspace capabilities and standard-library options.

### 7.2 Three knowledge layers

Campaign knowledge is separated into:

1. **Source canon:** imported documents, bundled adventure content, and explicit author notes.
2. **Hidden campaign bible:** threats, NPC motives, relationships, secrets, clocks, future possibilities, and unresolved consequences.
3. **Discovered memory:** scenes, facts, NPCs, decisions, promises, damage to the world, quests, and journal content known to players.

Every retrievable record has provenance, revision, and one of `public`, `discovered`, or `dm_only` scope. Player and peer DTOs are built from safe projections; they do not serialize hidden records and then rely on UI filtering.

### 7.3 DM context orchestration

For each turn, the application layer:

1. classifies the active mode such as exploration, social, combat, or downtime;
2. loads the current authoritative scene and actor state;
3. retrieves relevant source canon, hidden plans, discovered history, and SRD rules using separate filters;
4. builds a bounded context with provenance;
5. asks the model for narration, typed tool calls, and structured memory proposals;
6. validates tool commands and memory proposals before committing events.

Documents are untrusted data. Their text cannot override system policy, grant tools, reveal secrets, or request external actions. Logs contain only safe identifiers, stages, counts, timings, and error codes.

### 7.4 Continuity ledger

The campaign maintains typed continuity records for:

- asserted facts and detected conflicts;
- NPC goals, attitudes, and commitments;
- unresolved hooks and promises;
- world clocks and threat progress;
- scene outcomes and chapter summaries;
- causation links to domain events and source material.

The LLM proposes changes; application validation owns scope, references, type constraints, and commit. Removing a document removes its chunks and index atomically but does not rewrite events that already occurred during play.

## 8. DM behavior

The system prompt and turn policy make the DM an active referee and storyteller:

- interpret player input as intent, not guaranteed outcome;
- call for checks only when uncertainty and consequences justify them;
- respect established canon and authoritative mechanics;
- advance clocks and NPC agendas when appropriate;
- frame a changed situation after each consequential turn;
- avoid exposing hidden reasoning or undiscovered source material;
- offer useful possibilities without constraining free-form input;
- create maps, illustrations, encounters, and journal entries through typed tools;
- end narration with a concise invitation for the next player choice.

The application limits tool loops, validates tool availability, protects against duplicate calls, and persists ordered events. A malformed provider response becomes a recoverable turn error rather than a partial campaign mutation.

## 9. VTT and tactical combat

### 9.1 Layout invariants

The application shell uses `100dvh` and a complete `min-height: 0` chain through grid/flex containers. The shell itself uses hidden overflow. Only these regions scroll:

- the chat timeline;
- the body of an intentionally long modal or settings panel;
- explicit library lists.

Title bar, status bar, chat header, and composer remain fixed within the application layout. Modal footers and titles remain pinned while their bodies scroll.

### 9.2 Map sizing

The VTT reads the intrinsic width and height of each map. Initial `fit-to-view` computes a contain scale against the actual VTT viewport, centers the image, preserves aspect ratio, and displays the entire map without cropping or stretching. Resize and map replacement recompute a fit only when the user has not deliberately changed the view, or when the user invokes **Fit map**.

Pan, zoom, selection, measurement, layers, and temporary AoE overlays are local presentation state. Token positions and mechanical terrain remain authoritative projections.

### 9.3 Scene-to-map preparation

World preparation produces a typed map specification with:

- image or bundled fallback asset;
- grid scale and alignment;
- traversable bounds;
- terrain regions and movement costs;
- obstacles, cover, doors, and hazards;
- lighting/visibility metadata where supported;
- spawn zones, tokens, and encounter objectives.

Generated geometry is validated by the application layer. If image inference is disabled or fails, a matching bundled map is selected and the session continues.

### 9.4 Combat commands and projections

The client submits typed commands such as move, attack, cast, interact, ready, dodge, disengage, use item, and end turn. Each command contains a unique command ID and expected combat revision.

The UI may display a pending intent but must not commit position, HP, conditions, resources, action economy, round, or current turn. A newer authoritative snapshot/event confirms or rejects the intent. Duplicate and stale commands are idempotently rejected or reconciled.

The action panel exposes current legal resources and useful suggestions without becoming the rules engine. Free-form combat intent remains available and is translated into the same typed application commands.

## 10. Multiplayer-ready session protocol

### 10.1 Authority model

The host owns campaign state, saves, LLM orchestration, secrets, and rule resolution. A peer owns no canonical state. When the host disappears, peers enter paused read-only mode until it returns. Host election and save divergence are out of scope.

### 10.2 Inward contracts

`app-application` owns versioned session protocol models and a transport-neutral session port. The protocol covers:

- handshake with protocol version, campaign ID, invite proof, and client capabilities;
- a player-safe join snapshot with assigned actor, current revision, and asset manifest;
- typed intentions with command ID, actor ID, and expected revision;
- acknowledgements, rejections, ordered events, and snapshot recovery after gaps;
- asset metadata and content hashes rather than embedded base64 payloads.

Authorization validates membership, actor ownership, turn ownership, command capability, expected revision, and domain rules. DM-only memory, private character information, prompts, credentials, and provider configuration never enter peer projections.

This phase provides an in-memory/loopback transport adapter and contract tests. WebRTC, signaling, relay, and room UI are later outward adapters.

## 11. Distributed inference workers

### 11.1 Roles

Multiplayer peers may opt into compute roles without gaining game authority:

- the host normally runs the LLM and always runs game/DM coordination;
- an image worker may load a complete supported image model;
- a video worker may load a complete supported video model;
- the host retains bundled, cloud, or lightweight fallback providers.

Workers advertise a signed capability document containing protocol version, modality, model digest, supported operations, VRAM, concurrency, limits, license flags, and current load. Users explicitly configure whether their machine may contribute, VRAM/temperature limits, battery behavior, and allowed modalities.

### 11.2 Private job construction

The host converts private scene context into a minimal visual production prompt. A worker receives only:

- job and attempt IDs;
- modality and versioned operation;
- sanitized prompt;
- seed, dimensions, duration, and other required generation parameters;
- hashes of explicitly permitted input assets;
- lease deadline and cancellation channel.

It never receives campaign documents, chat history, API keys, provider credentials, the hidden campaign bible, or unrelated NPC data.

### 11.3 Lease and result protocol

The scheduler selects a compatible worker by modality, capabilities, model digest, available VRAM, load, latency, and privacy constraints. Jobs use renewable leases and support progress, backpressure, cancellation, timeout, and bounded retry.

If a worker disconnects or its lease expires, that attempt is invalidated and the task may be reassigned or degraded to fallback. A late result cannot overwrite a newer attempt. The host validates result MIME type, dimensions/duration, maximum size, content hash, job attempt, and campaign revision before publishing an asset event.

Media is content-addressed. Protocol events carry hashes and safe metadata; authorized clients retrieve chunks and verify the final checksum.

### 11.4 Future sharding boundary

The capability schema reserves an `execution_topology` and optional stage/collective identifiers. The initial implementation accepts only `single_worker`, meaning the entire model is loaded on one device. Tensor/pipeline parallelism, distributed collectives, RDMA, activation transfer, and multi-node failure recovery are explicitly deferred.

## 12. Tauri shell and window chrome

Window management follows the proven pattern inspected in the `text2bim` client:

- a dedicated `WindowChrome` mounts once above application overlays;
- window buttons are explicitly excluded from drag regions;
- pointer-down propagation is stopped so modal outside-click handling cannot consume a native window action;
- maximize/restore state is synchronized through Tauri window events;
- modal-open drag space is provided separately without covering buttons;
- invisible resize borders invoke native resize dragging;
- Tauri capabilities explicitly permit every invoked operation.

The production-root test covers the real shell and window actions. Real Tauri CDP acceptance proves minimize, maximize/restore, close behavior, drag, resize, and operation while onboarding/settings/other modals are open.

Russian is the default first-run locale. The language control uses the established visual system and full localized labels. API event names, storage keys, identifiers, and other machine contracts remain unchanged English tokens.

## 13. Zero-touch development and distribution

### 13.1 One process owner

Tauri is the only production process supervisor. It directly owns:

- `dmai-server`;
- the local model runtime sidecar;
- the image/video sidecar.

The backend requests inference lifecycle operations through the authenticated loopback runtime-control port. The random token is launch-scoped, not persisted, and not logged.

### 13.2 Unified artifact manifest

Development, CI, build staging, the installer, and runtime validation consume one manifest containing:

- logical role and Tauri external-bin name;
- pinned version and platform/architecture;
- expected filename and SHA-256;
- source/build recipe;
- contract version and supported capabilities;
- required license/attribution metadata.

There is one staging implementation. Python sidecar packaging and Tauri `externalBin` use the same exact output layout. Empty debug placeholders are removed; a missing required sidecar is either built/downloaded automatically or reported as an explicit profile limitation.

### 13.3 Development command

`bun run dev:all` becomes an idempotent bootstrap:

1. validate Bun, Rust target, local tooling, disk space, and platform profile;
2. provision the pinned Python 3.12 environment through the existing project workflow when source-sidecar development is required;
3. build only changed Rust/backend artifacts using a stable cache;
4. obtain or build missing pinned model/media sidecars;
5. verify hashes and stage exact external-bin names;
6. start Tauri with CDP enabled;
7. wait for backend and requested runtime health probes;
8. print a single structured readiness summary and log locations.

Normal edits do not rebuild unrelated crates. Failure messages name the role, stable error code, attempted remediation, and next safe action. Secrets and prompts never enter bootstrap logs.

### 13.4 Installer and first run

The installed application requires no Python, Cargo, Bun, or manual terminal commands. A local-capable bundle includes executable sidecars. Large model weights are downloaded after an explicit user choice that shows size, license, storage location, and hardware suitability. A cloud-only bundle may omit inference sidecars but must declare that profile consistently.

First-run diagnostics verify artifact hashes, CPU/GPU architecture, VRAM profile, disk space, and writable data/model paths. The application selects a safe default runtime profile and degrades media independently from gameplay.

### 13.5 Supervision

The supervisor models explicit states such as stopped, starting, healthy, degraded, failed, stopping, and restarting. Startup follows dependencies. Health probes, bounded exponential restart, generation IDs, and graceful shutdown prevent stale tasks and orphan processes. Closing Tauri terminates all children even after partial startup.

## 14. Error handling and privacy

Errors are typed and mapped at adapter boundaries to stable codes. User messages are localized and actionable. Important failure behavior:

- document extraction failure publishes no searchable revision;
- malformed LLM/tool output commits no partial domain mutation;
- a rejected combat command leaves the projection unchanged;
- media or compute-worker failure falls back without ending the turn;
- backend loss shows a recoverable shell state rather than a blank WebView;
- stale revisions trigger reconciliation, not optimistic overwrites;
- save and restore apply complete projections atomically.

Logs may contain safe IDs, event kinds, revisions, lifecycle states, ports, counts, durations, sizes, and error codes. They must not contain documents, prompts, chat text, campaign secrets, API keys, Stronghold values, runtime-control tokens, base64 media, or full provider payloads.

## 15. Testing and acceptance

### 15.1 Focused tests

- SRD 5.2.1 domain tables and rule interactions;
- typed command decoding and exhaustive tool fixtures;
- repository contract tests and atomic campaign operations;
- document validation, extraction boundaries, provenance, ACL, deletion, and injection resistance;
- context-budget and memory-proposal reducers;
- VTT fit calculations and combat revision reconciliation;
- session protocol handshake, ACL, idempotency, gap recovery, and secret-free serialization;
- compute scheduling, leases, cancellation, late results, fallback, and capability negotiation;
- process supervisor state machines, stale generations, retries, and shutdown.

### 15.2 Project gates

Every checkpoint runs focused tests, followed by the relevant portions of:

```text
bun run architecture:check
bun run lint
bun run typecheck
bun run build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
python -m ruff check sidecar
python -m pytest sidecar/tests -q
bun run gates
```

No gate claim may imply browser, desktop, packaged, or live-model acceptance.

### 15.3 Browser and desktop acceptance

Deterministic browser tests cover campaign home, all creation paths, document states, safe projections, DM event ordering, VTT fit, authoritative combat, fixed shell layout, and localized visible labels.

Real Tauri is launched with CDP. A deterministic full session covers:

1. first run in Russian;
2. working window controls over every major modal;
3. campaign creation through questionnaire or a fixture document;
4. character creation;
5. world preparation and an opening map;
6. exploration and an NPC interaction;
7. transition to a tactical encounter;
8. movement, attack, spell/ability, damage, conditions, turn advancement, and combat end;
9. map versus illustration routing;
10. save, close, restart, and atomic restore;
11. media degradation and bundled fallback;
12. compute-worker completion and worker loss during a job.

Live local-model play is a separate, non-deterministic proof and cannot replace the fixture-driven Tauri session.

### 15.4 Packaged evidence

CI builds the actual bundle, verifies the artifact manifest and hashes, unpacks or installs in an isolated environment, launches the packaged application, runs a CDP smoke, and confirms that shutdown leaves no backend/model/media child processes. Platform bundle evidence is reported separately for each target.

## 16. Documentation and cleanup

After behavior is real, update:

- `README.md` and contributor setup;
- `ARCHITECTURE.md` and `.ai-factory/ARCHITECTURE.md` when boundaries change;
- `AGENTS.md` entry points and verification commands;
- release guide/checklist and sidecar documentation;
- changelog and SRD/license attribution;
- runtime diagnostics and troubleshooting.

Temporary compatibility re-exports are removed once no callers remain. Broad backup, agent, graph, artifact, and companion directories remain excluded from architecture scans, formatters, and Git staging.

## 17. Acceptance summary

The design is complete when a clean installation can create or resume a campaign without terminal work; the DM reliably uses protected campaign knowledge to run a proactive world; a complete map fits the VTT; SRD 5.2.1 combat remains server-authoritative; saves restore the whole projection; media failures degrade gracefully; P2P and compute contracts pass deterministic tests; and the full flow is demonstrated in the real Tauri WebView through CDP.
