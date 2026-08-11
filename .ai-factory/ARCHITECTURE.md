# Architecture: Strict Explicit Architecture

## Status

The migration target is implemented. This document defines the final enforceable boundaries for Dungeon Master AI. [The public architecture tour](../ARCHITECTURE.md) explains the same design at repository level; this document is normative for dependency decisions.

## Principles

- Dependencies point inward toward deterministic domain behavior and application-owned contracts.
- Application policy is expressed as use cases over narrow ports, never through framework globals.
- Adapters translate at their boundary and do not import one another.
- Composition happens in explicit roots: `app-bootstrap` for the backend, `src-tauri` for desktop process ownership, and `src/app` for React.
- Wire, persistence, secret, process, and visible-product contracts remain stable unless changed deliberately with migration coverage.
- The React client is a projection and command adapter, not a second game engine.

## Rust layers

### Domain

`app-domain` owns deterministic D&D rules, dice, combat invariants, and pure SRD concepts. Allowed workspace dependencies: none. It must not depend on Axum, SQLx, Tauri, Tokio, tracing, reqwest, genai, Stronghold, fastembed, serde_json, or an adapter crate.

### Application

`app-application` owns use cases, commands, events, protocol-neutral models, and ports for LLMs, repositories, media, secrets, events, and runtime control. Its only allowed workspace dependency is `app-domain`. It must not import HTTP DTOs, SQL rows/pools, concrete providers, Stronghold, Tauri handles, or adapter crates.

### Outbound and inbound adapters

- `adapter-http` maps Axum DTOs and SSE to one injected application service per handler.
- `adapter-sqlite` implements repositories and owns migrations and transactions.
- `adapter-llm` implements LLM and embedding ports.
- `adapter-media` implements model catalog/download, media, and runtime-control protocols.
- `adapter-secrets` implements encrypted and in-memory secret storage.

Adapters may depend on `app-application` and, where deterministic types are required, `app-domain`. They must not depend on sibling adapters. `adapter-http` must not import SQLx, Stronghold, provider constructors, or process launchers.

### Composition and compatibility

`app-bootstrap` constructs concrete adapters, assembles HTTP services, initializes paths/telemetry, and owns backend startup. `src-tauri` is the desktop adapter and only production owner of all child processes.

Two compatibility crates remain intentionally:

- `app-llm` re-exports stable application/provider contracts and test adapters for existing consumers.
- `app-server` preserves the stable package surface and test helpers while delegating routes, repositories, providers, and bootstrap behavior to explicit layers.

These facades may not regain policy, concrete composition, SQL, or transport logic.

## Frontend layers

- `src/main.tsx`: styles, i18n, bootstrap call only.
- `src/app`: production composition, store-port wiring, desktop shell, overlays, and root controllers.
- `src/features/<feature>`: feature-owned controllers and models; no imports from `app` or sibling feature internals.
- `src/api`: HTTP/SSE transport, runtime schemas, and wire conversion; no Zustand or component imports.
- `src/state`: Zustand slices, projection storage, persistence adapters, and one-shot atomic state updates; no component imports.
- `src/components`: transitional product views. Business rules belong in features/application, not components.
- `src/ui`: reusable presentation primitives only; no imports from app, features, state, or API.

The root `src/App.tsx` and legacy hook paths are compatibility re-exports for existing imports and tests. New code imports `src/app` and `src/features` directly. Compatibility paths can be deleted after caller count reaches zero.

## Authority and atomicity

- Combat state changes only through a newer authoritative server `revision`. Client commands set pending presentation state but do not commit mechanical outcomes.
- Stale or duplicate combat projections are ignored.
- Save restore validates the complete server response and applies session, messages, PC, combat, and scene in one frontend store transaction.
- Settings replacement constructs and validates the full provider/config snapshot before swapping it.
- Persistence operations spanning related entities use SQLite transactions.

## Runtime and packaging

Tauri starts `dmai-server` and hosts a per-launch authenticated loopback control service. The backend calls that service through the `RuntimeControl` port. Tauri starts/stops/observes `mistralrs-server` and `dmai-image-sidecar`, owns their handles, and shuts them down with the application. Tokens and vault credentials are never persisted or logged.

`tauri.conf.json` is the local bundle and declares all three binaries. `tauri.cloud.conf.json` overrides the bundle to `dmai-server` only. Normal cloud build/release matrices cover Windows x64, macOS arm64, and Linux x64. Manual model/media prebuild workflows use the same three targets, Python 3.12, and mistralrs tag `v0.8.3`.

## Mechanical guard

`scripts/check-architecture.ts` reads `scripts/architecture-boundaries.json`, verifies every workspace crate and production dependency through Cargo metadata, applies forbidden inner-package checks, and scans frontend production imports. No exception file is permitted.

Required rules:

- inner crates cannot depend on frameworks or adapters;
- adapters cannot import sibling adapters;
- API cannot import state/components;
- state cannot import components;
- UI cannot import app/features/state/API;
- features cannot import app or sibling features.

The guard runs in `scripts/gates.sh --fast`, full gates, hooks, and CI lint.

## Stable contracts

Preserve route paths/methods, JSON envelopes, SQLite data/migrations, Stronghold key names, provider/model IDs, SSE names and ordering, `dmai-server`, `backend_port`, `backend-ready`, `backend-exited`, `APP_SERVER_LISTENING port=...`, and localized visible behavior. Changes require explicit versioning or migration plus characterization tests.

## Security and observability

DEBUG may record safe IDs, event kinds, revisions, state transitions, byte counts, and durations. INFO records lifecycle milestones. WARN records recoverable degradation. ERROR records safe codes and failed boundaries. Prompts, chat content, secrets, tokens, base64 media, request bodies, and full external payloads are forbidden in logs.

## Verification

```bash
bun run architecture:check
bun run gates
bun run e2e
bun run e2e:tauri
bun run tauri:build:cloud
python -m ruff check sidecar
python -m pytest sidecar/tests -q
```

GPU/local-model acceptance is separate and opt-in. Cloud bundle success does not imply local-runtime success; mocked browser success does not imply real WebView success; local green does not imply remote CI or release publication.
