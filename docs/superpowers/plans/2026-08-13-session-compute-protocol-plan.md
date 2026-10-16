# Session and Distributed Compute Protocol Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make campaign state multiplayer-safe and distribute complete image/video models to opted-in peers through transport-neutral, privacy-preserving contracts.

**Architecture:** `app-application` owns session, ACL, safe projection, and compute job protocols. In-memory adapters prove behavior now; future WebRTC and signaling remain outward adapters. Game authority and job commit remain on the host.

**Tech Stack:** Rust, Tokio channels, serde, existing application ports, content-addressed media, property/contract tests; no new networking production dependency.

**Spec:** `docs/superpowers/specs/2026-08-13-campaign-platform-srd52-design.md`

## Global Constraints

- No room UI or nonfunctional multiplayer buttons.
- No WebRTC/signaling/relay dependency in this phase.
- Host disconnect pauses peers read-only; no election or host migration.
- Worker participation is opt-in and whole-model `single_worker` only.
- Workers receive sanitized visual job data, never chat, documents, campaign bible, provider keys, or runtime tokens.
- All protocol DTOs are versioned, bounded, and safe to deserialize from an untrusted peer.

---

### Task 1: Define session protocol and player-safe projections

**Files:**
- Create: `crates/app-application/src/models/protocol.rs`
- Create: `crates/app-application/src/session/mod.rs`
- Create: `crates/app-application/src/session/projection.rs`
- Create: `crates/app-application/src/session/acl.rs`
- Create: `crates/app-application/src/ports/session_transport.rs`
- Modify: `crates/app-application/src/models/mod.rs`
- Modify: `crates/app-application/src/ports/mod.rs`
- Modify: `crates/app-application/src/lib.rs`
- Test: `crates/app-application/tests/session_protocol.rs`

**Interfaces:**
- Produces `PROTOCOL_VERSION: u16 = 1`.
- Produces `Handshake`, `JoinSnapshot`, `PlayerProjection`, `SessionCommand`, `SessionEvent`, `SessionTransportPort`.

- [ ] **Step 1: Write serialization golden tests**

Assert exact tagged JSON for handshake, join snapshot, command envelope, acknowledgement, rejection, event batch, and snapshot request. Include maximum lengths for display name, capability list, event batch, and asset manifest.

- [ ] **Step 2: Write privacy projection tests**

Build an authoritative fixture containing DM-only continuity, hidden NPC motives, all characters, prompts, provider settings, and source metadata. Assert a player's serialized projection contains only discovered/public facts and authorized actor details; search serialized JSON for forbidden marker strings.

- [ ] **Step 3: Run and verify missing modules**

Run: `cargo test -p app-application --test session_protocol`
Expected: FAIL.

- [ ] **Step 4: Define bounded DTOs and explicit capabilities**

Use enums for command/event kinds. Reject protocol mismatch, duplicate capability, more than 64 assets, more than 256 events per batch, and strings above their documented bounds before application dispatch.

- [ ] **Step 5: Implement projection builder and ACL**

ACL inputs are membership, assigned actor IDs, active turn, command capability, and knowledge scopes. Filtering occurs while constructing DTOs; hidden values are never constructed inside `PlayerProjection`.

- [ ] **Step 6: Run protocol tests**

Run: `cargo test -p app-application --test session_protocol`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/app-application/src/models/protocol.rs crates/app-application/src/models/mod.rs crates/app-application/src/session crates/app-application/src/ports/session_transport.rs crates/app-application/src/ports/mod.rs crates/app-application/src/lib.rs crates/app-application/tests/session_protocol.rs
git commit -m "feat(protocol): define safe multiplayer projections"
```

### Task 2: Implement host session coordinator and in-memory transport

**Files:**
- Create: `crates/app-application/src/session/coordinator.rs`
- Create: `crates/app-application/src/session/idempotency.rs`
- Create: `crates/app-application/src/session/recovery.rs`
- Create: `crates/adapter-session-memory/Cargo.toml`
- Create: `crates/adapter-session-memory/src/lib.rs`
- Create: `crates/adapter-session-memory/src/session.rs`
- Modify: `Cargo.toml`
- Modify: `scripts/architecture-boundaries.json`
- Test: `crates/app-application/tests/session_coordinator.rs`
- Test: `crates/adapter-session-memory/tests/session_transport.rs`

**Interfaces:**
- Produces `SessionCoordinator::join`, `submit`, `events_since`, `snapshot`.
- Produces an in-memory `SessionTransportPort` adapter used only for deterministic tests and future local loopback diagnostics.

- [ ] **Step 1: Write join, command, and recovery tests**

Cover valid invite proof adapter result, wrong protocol, unauthorized actor, wrong turn, duplicate command ID returning the same result, stale revision, ordered event batch, gap detection, snapshot recovery, peer disconnect, and host disconnect transition to paused/read-only.

- [ ] **Step 2: Run and verify missing coordinator**

Run: `cargo test -p app-application --test session_coordinator && cargo test -p adapter-session-memory --test session_transport`
Expected: FAIL.

- [ ] **Step 3: Implement host-side command dispatch**

Validate protocol/session/membership/ACL/revision, then call the same campaign/combat use cases as local HTTP. Store bounded idempotency acknowledgements by session and command ID.

- [ ] **Step 4: Implement revision gap recovery**

Events are monotonically ordered. A request before the retained event window or a detected gap returns `snapshot_required`; the snapshot includes current revision and asset manifest.

- [ ] **Step 5: Implement in-memory channels with backpressure**

Use bounded Tokio channels. A slow peer is disconnected with `peer_backpressure` rather than growing an unbounded queue or blocking authoritative commits.

- [ ] **Step 6: Run coordinator tests**

Run: `cargo test -p app-application --test session_coordinator`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock crates/app-application/src/session crates/app-application/tests/session_coordinator.rs crates/adapter-session-memory scripts/architecture-boundaries.json
git commit -m "feat(session): coordinate authoritative peer commands"
```

### Task 3: Define compute worker capabilities and job leases

**Files:**
- Create: `crates/app-application/src/models/compute.rs`
- Create: `crates/app-application/src/compute/mod.rs`
- Create: `crates/app-application/src/compute/sanitize.rs`
- Create: `crates/app-application/src/compute/scheduler.rs`
- Create: `crates/app-application/src/ports/compute.rs`
- Modify: `crates/app-application/src/models/mod.rs`
- Modify: `crates/app-application/src/ports/mod.rs`
- Test: `crates/app-application/tests/compute_protocol.rs`

**Interfaces:**
- Produces `ComputeCapability`, `ComputeWorkerId`, `ComputeJob`, `JobAttempt`, `JobLease`, `JobProgress`, `JobResult`, `ExecutionTopology::SingleWorker`.
- Produces `ComputeWorkerPort` and `ComputeScheduler`.

- [ ] **Step 1: Write capability compatibility tests**

Cover image/video modality, model digest, contract version, supported dimensions/duration/init-image/cancel, VRAM, concurrency, license flags, and `single_worker`. Reject unknown topology and unsupported request features.

- [ ] **Step 2: Write sanitized job privacy tests**

Construct private scene context containing unique forbidden markers. Assert `build_visual_job` output contains the sanitized visual description and parameters but none of the chat, source, key, hidden motive, or provider markers.

- [ ] **Step 3: Run and verify missing compute modules**

Run: `cargo test -p app-application --test compute_protocol`
Expected: FAIL.

- [ ] **Step 4: Define bounded compute DTOs**

Limit prompt length, reference assets, progress frequency, output size metadata, and concurrency. `execution_topology` serializes explicitly as `single_worker`; deserialization rejects other values in protocol v1.

- [ ] **Step 5: Implement deterministic scheduler scoring**

Filter incompatible/offline/opted-out workers, then order by current jobs, sufficient VRAM margin, latency bucket, and stable worker ID. Do not expose precise hardware fingerprints to other peers.

- [ ] **Step 6: Implement lease state transitions**

Transitions are queued → leased → running → completed/cancelled/expired. Attempt IDs increase on reassignment; a result is committable only for the current attempt and unexpired lease.

- [ ] **Step 7: Run compute tests**

Run: `cargo test -p app-application --test compute_protocol`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/app-application/src/models/compute.rs crates/app-application/src/models/mod.rs crates/app-application/src/compute crates/app-application/src/ports/compute.rs crates/app-application/src/ports/mod.rs crates/app-application/tests/compute_protocol.rs
git commit -m "feat(compute): define private distributed media jobs"
```

### Task 4: Implement in-memory image/video workers and content-addressed results

**Files:**
- Create: `crates/adapter-session-memory/src/compute.rs`
- Create: `crates/adapter-session-memory/tests/compute_transport.rs`
- Create: `crates/app-application/src/compute/assets.rs`
- Modify: `crates/app-application/src/ports/media.rs`
- Modify: `crates/adapter-media/src/image/provider.rs`
- Modify: `crates/adapter-media/src/video/provider.rs`
- Test: `crates/app-application/tests/compute_scheduler.rs`
- Test: `crates/adapter-media/tests/distributed_contract.rs`

**Interfaces:**
- Produces content-addressed result metadata with SHA-256, MIME type, size, dimensions/duration, job ID, and attempt ID.
- Uses existing image/video provider ports behind a worker adapter.

- [ ] **Step 1: Write success, timeout, reassignment, and late-result tests**

Simulate an image worker completing, a video worker sending progress, a worker disappearing mid-job, fallback assignment, cancellation, and a late result from attempt 1 after attempt 2 completes. Assert only attempt 2 commits.

- [ ] **Step 2: Write asset validation tests**

Reject wrong hash, unsupported MIME, oversized bytes, invalid dimensions/duration, job mismatch, attempt mismatch, and stale campaign revision.

- [ ] **Step 3: Run and verify failures**

Run: `cargo test -p app-application --test compute_scheduler && cargo test -p adapter-session-memory --test compute_transport && cargo test -p adapter-media --test distributed_contract`
Expected: FAIL.

- [ ] **Step 4: Implement bounded in-memory worker adapter**

Use bounded channels for job, progress, result, and cancel. The adapter supplies only `ComputeJob`; it has no repository, secrets, chat, or campaign context dependency.

- [ ] **Step 5: Implement content-addressed validation and host commit**

Compute hash on received bytes, validate metadata and current attempt/revision, store through the existing media cache/asset port, and emit a normal image/video asset event.

- [ ] **Step 6: Run application and media tests**

Run: `cargo test -p app-application --test compute_scheduler && cargo test -p adapter-session-memory && cargo test -p adapter-media`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/app-application/src/compute/assets.rs crates/app-application/src/ports/media.rs crates/app-application/tests/compute_scheduler.rs crates/adapter-session-memory/src/compute.rs crates/adapter-session-memory/tests/compute_transport.rs crates/adapter-media/src/image/provider.rs crates/adapter-media/src/video/provider.rs crates/adapter-media/tests/distributed_contract.rs
git commit -m "feat(compute): schedule complete media models on workers"
```

### Task 5: Add host-only worker settings without room UI

**Files:**
- Create: `src/api/contracts/compute.ts`
- Create: `src/features/runtime/ComputePreferences.tsx`
- Create: `src/features/runtime/ComputePreferences.module.css`
- Modify: `src/components/SettingsForm.tsx`
- Modify: `src/state/settings.ts`
- Modify: `src/state/persistStorage.ts`
- Modify: `src/locales/en/settings.json`
- Modify: `src/locales/ru/settings.json`
- Test: `src/features/runtime/__tests__/ComputePreferences.test.tsx`
- Test: `src/state/__tests__/persistStorage.test.ts`

**Interfaces:**
- Produces local opt-in preferences only: modalities, VRAM limit, temperature limit, battery permission, concurrency, and model visibility.
- Does not expose create/join room controls.

- [ ] **Step 1: Write settings and persistence tests**

Default is opted out. Cover image-only, video-only, VRAM bound, temperature bound, battery false, concurrency 1, and persistence. Assert model path, hardware serial/fingerprint, job prompt, and peer IDs are not persisted in frontend storage.

- [ ] **Step 2: Run and verify failure**

Run: `bun run test -- src/features/runtime/__tests__/ComputePreferences.test.tsx src/state/__tests__/persistStorage.test.ts`
Expected: FAIL.

- [ ] **Step 3: Implement settings with existing controls**

Explain that jobs may reveal a minimized visual prompt and consume GPU resources. Require explicit consent before enabling each modality.

- [ ] **Step 4: Keep networking UI absent**

Add a test that campaign home, settings navigation, and production root contain no create-room, join-room, invite-code, or signaling controls.

- [ ] **Step 5: Run frontend checks**

Run: `bun run lint && bun run typecheck && bun run build && bun run test`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/api/contracts/compute.ts src/features/runtime/ComputePreferences.tsx src/features/runtime/ComputePreferences.module.css src/features/runtime/__tests__/ComputePreferences.test.tsx src/components/SettingsForm.tsx src/state/settings.ts src/state/persistStorage.ts src/state/__tests__/persistStorage.test.ts src/locales/en/settings.json src/locales/ru/settings.json
git commit -m "feat(settings): add opt-in compute worker preferences"
```

### Task 6: Protocol security and architecture checkpoint

**Files:**
- Create: `crates/app-application/tests/protocol_security.rs`
- Modify: `scripts/architecture-boundaries.json`
- Modify: `scripts/check-architecture.ts`
- Modify: `ARCHITECTURE.md`

**Interfaces:**
- Enforces that application protocols have no WebRTC/Tauri/Axum/SQLx dependency and DTO serialization contains no secret-bearing types.

- [ ] **Step 1: Add malicious-input protocol tests**

Test oversized batches/prompts/assets, unknown tags, malformed UUIDs, replayed command IDs, cross-campaign actor IDs, progress floods, hash mismatch, and unauthorized asset request.

- [ ] **Step 2: Add dependency/import guard rules**

Forbid `webrtc`, `libp2p`, `axum`, `tauri`, `sqlx`, provider SDKs, and adapter imports from application protocol/session/compute modules. Forbid React state imports from API contracts.

- [ ] **Step 3: Run security and architecture tests**

Run: `cargo test -p app-application --test protocol_security && bun run architecture:check && cargo deny check`
Expected: PASS.

- [ ] **Step 4: Update architecture documentation accurately**

Document in-memory transport, host authority, paused disconnect behavior, compute privacy, complete-model workers, and deferred WebRTC/sharding. Do not claim a working room/network feature.

- [ ] **Step 5: Commit**

```bash
git add crates/app-application/tests/protocol_security.rs scripts/architecture-boundaries.json scripts/check-architecture.ts ARCHITECTURE.md
git commit -m "test(protocol): enforce peer and compute boundaries"
```

### Task 7: Phase checkpoint

**Files:**
- Modify only Task 1-6 files when a failing checkpoint identifies a defect.

**Interfaces:**
- Produces deterministic multiplayer/compute readiness without shipping networking.

- [ ] **Step 1: Run all Rust and frontend gates**

Run: `bun run architecture:check && bun run gates`
Expected: PASS.

- [ ] **Step 2: Run deterministic worker interruption acceptance**

Run the focused test suites with logs enabled at safe metadata level; prove image success, video progress, worker expiration, reassignment, fallback, and late-result rejection.

- [ ] **Step 3: Run browser production-root tests**

Run: `bun run e2e`
Expected: no room UI, compute preferences default off, and existing campaign/game flows unchanged.

- [ ] **Step 4: Commit checkpoint-only fixes if present**

Stage corrected files individually and commit with `test(protocol): close distributed compute gaps`.
