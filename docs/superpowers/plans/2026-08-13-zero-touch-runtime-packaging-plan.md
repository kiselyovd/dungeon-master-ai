# Zero-Touch Runtime and Packaging Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make one development command and one installed application automatically stage, launch, monitor, recover, and stop every required process.

**Architecture:** A single checked artifact manifest drives dev bootstrap, build staging, CI, installer validation, and runtime diagnostics. Tauri remains the sole process supervisor; `dmai-server` is one compiled backend binary containing all Rust crates.

**Tech Stack:** Bun/TypeScript scripts, Rust/Tauri 2, Cargo, Python 3.12/uv, JSON manifest, Playwright CDP, platform bundlers.

**Spec:** `docs/superpowers/specs/2026-08-13-campaign-platform-srd52-design.md`

## Global Constraints

- Do not install system-wide tools silently.
- Do not download or change production dependencies without explicit approval.
- Never use empty sidecar placeholders.
- Artifact versions, filenames, target triples, hashes, and license metadata have one source of truth.
- Tauri alone owns child handles and shutdown.
- Runtime-control tokens are ephemeral and never logged.

---

### Task 1: Define and validate the unified artifact manifest

**Files:**
- Create: `runtime/artifacts.json`
- Create: `scripts/lib/artifact-manifest.ts`
- Create: `scripts/lib/artifact-manifest.test.ts`
- Modify: `scripts/check-architecture.ts`
- Modify: `package.json`

**Interfaces:**
- Produces `ArtifactManifest`, `RuntimeArtifact`, `loadArtifactManifest(root)`, `artifactFilename(role, target)`.
- Roles are exactly `backend`, `model_runtime`, and `media_runtime`.

- [ ] **Step 1: Write failing manifest validation tests**

Test valid Windows/Linux/macOS entries and reject duplicate roles, unsupported targets, non-hex SHA-256, missing license metadata, path traversal, absolute filenames, and a mismatch between logical role and Tauri external-bin basename.

```typescript
expect(() => parseArtifactManifest({ version: 1, artifacts: [{ role: 'backend', sha256: 'bad' }] }))
  .toThrow(/artifact_sha256_invalid/);
```

- [ ] **Step 2: Run and verify missing module failure**

Run: `bun run test -- scripts/lib/artifact-manifest.test.ts`
Expected: FAIL.

- [ ] **Step 3: Implement strict parsing with no new package**

Use existing runtime schema tooling if already accessible to scripts; otherwise implement explicit TypeScript guards. Resolve paths under `src-tauri/binaries` and verify they remain within that directory.

- [ ] **Step 4: Add `runtime:manifest:check` and architecture integration**

`bun run runtime:manifest:check` validates structure; architecture check also compares `tauri.conf.json`, cloud config, capabilities, and manifest roles.

- [ ] **Step 5: Run tests and checks**

Run: `bun run test -- scripts/lib/artifact-manifest.test.ts && bun run runtime:manifest:check && bun run architecture:check`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add runtime/artifacts.json scripts/lib/artifact-manifest.ts scripts/lib/artifact-manifest.test.ts scripts/check-architecture.ts package.json
git commit -m "build(runtime): define one sidecar artifact manifest"
```

### Task 2: Replace build.rs staging logic with one explicit staging script

**Files:**
- Create: `scripts/stage-runtime-artifacts.ts`
- Create: `scripts/lib/stage-runtime-artifacts.test.ts`
- Modify: `src-tauri/build.rs`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/tauri.cloud.conf.json`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `package.json`

**Interfaces:**
- Produces a parameterized `runtime:stage` command. The Windows x64 invocation is `bun run runtime:stage -- --profile dev --target x86_64-pc-windows-msvc`; CI supplies the target triple from its explicit matrix.
- `build.rs` verifies already-staged artifacts and builds/stages only `dmai-server` through the shared script contract; it does not create placeholders.

- [ ] **Step 1: Write staging tests against a temporary directory**

Cover cached valid artifact, hash mismatch, missing local artifact, cloud profile omitting model/media, wrong triple, and a zero-byte file.

- [ ] **Step 2: Run and verify failure**

Run: `bun run test -- scripts/lib/stage-runtime-artifacts.test.ts`
Expected: FAIL.

- [ ] **Step 3: Implement deterministic staging**

Copy to exact `externalBin` target names using temporary sibling files and atomic rename. Verify SHA-256 after copy. A source-build path may update a generated local lock record, but must not mutate the committed release hash without an explicit release command.

- [ ] **Step 4: Remove debug placeholder creation from build.rs**

Dev failure must state the missing role and command that can provision it. Cloud build must intentionally use the cloud config rather than fabricate local binaries.

- [ ] **Step 5: Synchronize configs and capabilities**

Local config declares all three roles; cloud config declares backend only. Shell spawn capability matches each local role. Add an automated assertion for this exact relationship.

- [ ] **Step 6: Run staging and Rust build checks**

Run: `bun run runtime:manifest:check && bun run test -- scripts/lib/stage-runtime-artifacts.test.ts && cargo check -p dungeon-master-ai`
Expected: PASS with real staged dev artifacts or an explicit, tested missing-artifact result in fixture mode.

- [ ] **Step 7: Commit**

```bash
git add scripts/stage-runtime-artifacts.ts scripts/lib/stage-runtime-artifacts.test.ts src-tauri/build.rs src-tauri/tauri.conf.json src-tauri/tauri.cloud.conf.json src-tauri/capabilities/default.json package.json
git commit -m "build(tauri): unify sidecar staging"
```

### Task 3: Make `dev:all` an idempotent bootstrap

**Files:**
- Rewrite: `scripts/dev-all.ts`
- Refactor: `scripts/setup-local.ts`
- Create: `scripts/lib/dev-bootstrap.ts`
- Create: `scripts/lib/dev-bootstrap.test.ts`
- Modify: `package.json`
- Modify: `CONTRIBUTING.md`

**Interfaces:**
- Produces `DevBootstrap.plan(profile, host) -> BootstrapAction[]` and `executeBootstrap(actions)`.
- `bun run dev:all` provisions project-local prerequisites, stages artifacts, starts Tauri with CDP, waits for readiness, and prints structured summary.

- [ ] **Step 1: Write plan tests for clean, cached, and broken machines**

Assert no-op for a valid cache, Python/uv remediation, model runtime build/download action, media sidecar build action, checksum repair, low-disk refusal, and CPU-only profile selection.

- [ ] **Step 2: Run and verify current-script mismatch**

Run: `bun run test -- scripts/lib/dev-bootstrap.test.ts`
Expected: FAIL.

- [ ] **Step 3: Extract pure discovery and planning**

Discovery returns tool versions, target triple, paths, hashes, disk bytes, GPU capability hints, and cache state. It does not mutate the system.

- [ ] **Step 4: Implement project-local remediation actions**

Allowed automatic actions are Cargo builds, `uv sync` into the project environment, approved pinned sidecar retrieval/build recipes, staging, and starting Tauri. Missing system-level Bun/Rust/uv produces one actionable error and exits before partial startup.

- [ ] **Step 5: Start Tauri with CDP and wait for health**

Use the existing CDP/profile helpers. Wait separately for backend, model runtime when enabled, and media runtime when enabled. Print role, lifecycle state, safe port, duration, and log path; never print control tokens.

- [ ] **Step 6: Run script tests and a dry run**

Run: `bun run test -- scripts/lib/dev-bootstrap.test.ts && bun scripts/dev-all.ts --dry-run`
Expected: PASS and a deterministic action list.

- [ ] **Step 7: Commit**

```bash
git add scripts/dev-all.ts scripts/setup-local.ts scripts/lib/dev-bootstrap.ts scripts/lib/dev-bootstrap.test.ts package.json CONTRIBUTING.md
git commit -m "feat(dev): bootstrap the full Tauri stack automatically"
```

### Task 4: Harden the Tauri process supervisor

**Files:**
- Create: `src-tauri/src/processes/supervisor.rs`
- Modify: `src-tauri/src/processes/mod.rs`
- Modify: `src-tauri/src/processes/backend.rs`
- Modify: `src-tauri/src/processes/control.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/processes/supervisor.rs` unit module
- Test: `src-tauri/tests/process_lifecycle.rs`

**Interfaces:**
- Produces `ProcessState::{Stopped, Starting, Healthy, Degraded, Failed, Stopping, Restarting}`.
- Produces generation-safe bounded restart and `shutdown_all()`.

- [ ] **Step 1: Write state-machine tests**

Cover ordered backend/runtime startup, health timeout, child exit, two retries with bounded backoff, exhausted retries, stale generation events, partial startup shutdown, and idempotent double shutdown.

- [ ] **Step 2: Run and verify missing supervisor**

Run: `cargo test -p dungeon-master-ai process_lifecycle`
Expected: FAIL.

- [ ] **Step 3: Implement pure transition decisions**

Separate observations/effects from Tauri handles so most tests need no process. Effects include spawn, probe, schedule retry, emit safe status, and kill.

- [ ] **Step 4: Integrate backend/model/media handles under one owner**

Remove duplicate lifecycle bookkeeping. Generation IDs prevent an old child exit or probe from replacing a newer state.

- [ ] **Step 5: Guarantee close and destroy cleanup**

On Tauri exit/close, stop runtime-control intake, cancel pending starts, kill media/model/backend in dependency-safe order, and wait a bounded interval before process exit.

- [ ] **Step 6: Run Tauri tests**

Run: `cargo test -p dungeon-master-ai`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/processes src-tauri/src/lib.rs src-tauri/tests/process_lifecycle.rs
git commit -m "feat(runtime): supervise all Tauri child processes"
```

### Task 5: Expose safe runtime diagnostics in the application

**Files:**
- Create: `src/api/runtimeDiagnostics.ts`
- Create: `src/features/runtime/RuntimeDiagnostics.tsx`
- Create: `src/features/runtime/RuntimeDiagnostics.module.css`
- Modify: `src/components/StatusBar.tsx`
- Modify: `src/components/SettingsModal.tsx`
- Modify: `src/locales/en/settings.json`
- Modify: `src/locales/ru/settings.json`
- Test: `src/features/runtime/__tests__/RuntimeDiagnostics.test.tsx`
- Test: `src/api/__tests__/runtimeDiagnostics.test.ts`

**Interfaces:**
- Produces safe role states and remediation codes; never returns executable paths containing user secrets or control credentials.

- [ ] **Step 1: Write diagnostics schema/privacy tests**

Reject unknown states and assert serialized DTO contains no token, prompt, key, command line, or environment fields.

- [ ] **Step 2: Write component tests**

Cover healthy, image-degraded, retrying, failed, and unavailable-by-profile. Verify gameplay remains accessible for media-only degradation.

- [ ] **Step 3: Run and verify failures**

Run: `bun run test -- src/api/__tests__/runtimeDiagnostics.test.ts src/features/runtime/__tests__/RuntimeDiagnostics.test.tsx`
Expected: FAIL.

- [ ] **Step 4: Implement diagnostics and localized remediation**

Use existing button/select/status primitives. Provide Retry only for a supported safe lifecycle operation and show fallback use explicitly.

- [ ] **Step 5: Run frontend checks**

Run: `bun run lint && bun run typecheck && bun run build && bun run test`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/api/runtimeDiagnostics.ts src/api/__tests__/runtimeDiagnostics.test.ts src/features/runtime src/components/StatusBar.tsx src/components/SettingsModal.tsx src/locales/en/settings.json src/locales/ru/settings.json
git commit -m "feat(ui): show safe runtime diagnostics"
```

### Task 6: Validate the real installer and packaged lifecycle

**Files:**
- Create: `scripts/verify-package.ts`
- Create: `scripts/lib/package-verifier.ts`
- Create: `scripts/lib/package-verifier.test.ts`
- Create: `scripts/packaged-cdp-smoke.ts`
- Create: `scripts/package-smoke/windows.ps1`
- Create: `scripts/package-smoke/linux.sh`
- Create: `scripts/package-smoke/macos.sh`
- Modify: `package.json`
- Modify: `.github/workflows/release.yml`
- Modify: `docs/RELEASE.md`
- Modify: `docs/RELEASE_CHECKLIST.md`

**Interfaces:**
- Produces `bun run package:verify -- --artifact src-tauri/target/release/bundle --target x86_64-pc-windows-msvc` for the Windows x64 release job; the other matrix jobs pass their concrete bundle directory and target triple.
- Produces `bun run e2e:packaged` for an installed/unpacked build.

- [ ] **Step 1: Write package manifest verification tests**

Test missing role, wrong hash, extra unexpected executable, wrong architecture metadata, cloud/local profile mismatch, and complete valid bundle.

- [ ] **Step 2: Run and verify missing verifier**

Run: `bun run test -- scripts/lib/package-verifier.test.ts`
Expected: FAIL.

- [ ] **Step 3: Implement archive/install layout verification**

Inspect without executing where possible; resolve exact packaged sidecar paths and compare hashes to the release manifest. Do not trust filenames alone.

- [ ] **Step 4: Implement packaged WebView and lifecycle smokes**

On Windows x64, install the MSI/NSIS artifact into an isolated directory, launch with WebView2 remote debugging, attach through CDP, assert campaign home and runtime states, close the window, and verify all recorded backend/model/media PIDs exit. On Linux x64, install or mount the package under Xvfb, enable the WebKit remote inspector, assert the same DOM contract through the inspector adapter, then verify child cleanup. On macOS arm64, install the app bundle into a temporary Applications directory, enable the WebKit remote inspector, assert the same DOM contract through the inspector adapter, then verify child cleanup. Each script fails with `webview_inspection_unavailable` when inspection cannot be established; process launch alone is not a passing UI smoke.

- [ ] **Step 5: Add CI matrix steps**

Each release target builds, verifies package contents, installs or mounts the package, runs its named platform script, and uploads redacted evidence. Keep cloud bundle and local-capable bundle results distinct.

- [ ] **Step 6: Run local verifier tests and available package smoke**

Run: `bun run test -- scripts/lib/package-verifier.test.ts && bun run package:verify -- --artifact src-tauri/target/release/bundle --target x86_64-pc-windows-msvc`
Expected: tests PASS; package verification PASS when the corresponding local artifact has been built, otherwise report `package_artifact_missing` without claiming packaged acceptance.

- [ ] **Step 7: Commit**

```bash
git add scripts/verify-package.ts scripts/lib/package-verifier.ts scripts/lib/package-verifier.test.ts scripts/packaged-cdp-smoke.ts scripts/package-smoke package.json .github/workflows/release.yml docs/RELEASE.md docs/RELEASE_CHECKLIST.md
git commit -m "test(packaging): verify installed runtime artifacts"
```

### Task 7: Phase checkpoint

**Files:**
- Modify only files from Tasks 1-6 when a checkpoint failure identifies a defect.

**Interfaces:**
- Produces zero-touch development and packaged evidence.

- [ ] **Step 1: Run consolidated gates**

Run: `bun run runtime:manifest:check && bun run architecture:check && bun run gates`
Expected: PASS.

- [ ] **Step 2: Prove fresh dev bootstrap behavior**

Run `bun run dev:all` from a clean project-local runtime cache after preserving/removing only the explicitly documented generated cache directory. Expected: bootstrap provisions/stages, launches Tauri with CDP, and reaches healthy or honestly degraded status without manual sidecar commands.

- [ ] **Step 3: Run real Tauri acceptance**

Run: `bun run e2e:tauri`
Expected: PASS.

- [ ] **Step 4: Build and verify one local installer profile**

Run the platform build command documented by `package.json`, then `bun run package:verify` and `bun run e2e:packaged`. Expected: manifest matches, first run opens, and no child process survives shutdown.

- [ ] **Step 5: Commit checkpoint-only fixes if present**

Stage each corrected file explicitly and commit with `test(runtime): close zero-touch acceptance gaps`.
