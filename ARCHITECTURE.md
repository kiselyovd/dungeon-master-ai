# Architecture

Dungeon Master AI uses Explicit Architecture: game rules and application contracts sit at the center, while HTTP, persistence, model providers, media runtimes, Tauri, and React are adapters around them. For setup and verification, see [CONTRIBUTING.md](CONTRIBUTING.md). The mechanically enforced dependency policy lives in [`.ai-factory/ARCHITECTURE.md`](.ai-factory/ARCHITECTURE.md).

## Runtime topology

```text
React WebView -- HTTP/SSE --> dmai-server
      ^                         |
      |                         | authenticated loopback control
      |                         v
Tauri shell --------------> runtime-control
      |                         |
      +-- owns dmai-server      +-- requests start/stop/status
      +-- owns mistralrs-server
      +-- owns dmai-image-sidecar

dmai-server --> SQLite adapter
dmai-server --> Stronghold adapter
```

Tauri is the only production process owner. It starts `dmai-server`, exposes an authenticated loopback runtime-control service, owns model/media child handles, and performs shutdown. The random control token is passed to the backend for the current launch only and is neither persisted nor logged. Cloud bundles declare only `dmai-server`; local bundles also declare `mistralrs-server` and `dmai-image-sidecar`.

## Rust dependency direction

```text
app-domain <- app-application <- adapter-http
                           ^  <- adapter-sqlite
                           ^  <- adapter-llm
                           ^  <- adapter-media
                           ^  <- adapter-secrets
                           ^  <- app-bootstrap

src-tauri -> app-application runtime contracts
```

- `app-domain`: deterministic D&D rules, dice, combat invariants, and pure SRD retrieval. It has no transport, persistence, provider, process, or raw JSON tool-decoding responsibility.
- `app-application`: use cases, typed commands/events, protocol-neutral models, and inward-owned ports. Agent turns, combat resolution, settings updates, and atomicity policy live here.
- `adapter-http`: Axum routes, wire DTO validation, error mapping, and SSE serialization. Handlers delegate to an injected service bundle.
- `adapter-sqlite`: SQLx pool, migrations, row mapping, transactions, and repository implementations.
- `adapter-llm`: concrete `genai`, OpenAI-compatible, local mistralrs, retry, mock, and embedding implementations.
- `adapter-media`: Hugging Face, downloads, image/video providers, local runtime protocol, and the Tauri runtime-control client.
- `adapter-secrets`: Stronghold, legacy-secret migration, and the in-memory test implementation.
- `app-bootstrap`: backend configuration, paths, telemetry, adapter construction, and Axum startup. It produces the stable `dmai-server` binary and `APP_SERVER_LISTENING port=...` readiness line.
- `src-tauri`: desktop delivery adapter and sole child-process owner.

`app-llm` remains a compatibility facade for stable provider imports and test doubles. `app-server` remains a compatibility facade for its stable public test helpers and backend package surface. Neither owns application policy or concrete composition.

## Frontend dependency direction

```text
main.tsx -> app -> features -> state/api -> validated wire contracts
                   |
                   +-> components and ui presentation

ui -> reusable primitives only
```

- `src/main.tsx` initializes global styles and localization, then mounts `src/app/bootstrap.tsx`.
- `src/app/` is the composition layer. It wires Zustand-backed ports and shell controllers to feature use cases.
- `src/features/` owns feature models and controllers. Features cannot import `app` or another feature's internals.
- `src/api/` owns HTTP/SSE transport, runtime validation, and wire mapping. It cannot import Zustand or React components.
- `src/state/` stores projections and local presentation state. It cannot import React components.
- `src/ui/` contains reusable primitives and cannot import app, features, state, or API modules.

Combat is server-authoritative. The frontend sends typed commands and replaces its combat projection only when a newer server revision arrives. It does not calculate or optimistically commit HP, conditions, initiative, movement legality, rounds, turns, or action economy.

Save restoration is atomic in the frontend: the restore response is validated and converted into a complete session projection before one Zustand commit. Secrets, chat streams, combat projections, and base64 media are excluded from normal persisted frontend state.

## Stable contracts

Refactors must preserve:

- registered HTTP paths, status behavior, JSON envelopes, and `/agent/turn` request mapping;
- SSE event names: `reasoning_text`, `image_generated`, `text_delta`, `tool_call_start`, `tool_call_result`, `video_generated`, and `agent_done`;
- SQLite migration history and existing data;
- Stronghold key names and non-disclosure behavior;
- Tauri command `backend_port`, events `backend-ready` and `backend-exited`, external binary names, and the backend readiness line;
- English/Russian visible product behavior while leaving machine contracts untranslated.

## Media runtime

The Python 3.12 sidecar exposes `/healthz`, `/backends`, `/generate`, `/unload`, and `/video/generate`, and prints `LISTENING_ON_PORT=<n>` on startup. Its media contract is versioned. Image and video pipeline operations are serialized for single-GPU safety. GPU tests are opt-in with `RUN_GPU_TESTS=1`; normal CI runs Ruff and offline pytest without downloading models.

## Security and logging

Provider credentials stay in Stronghold. Production startup requires an explicit vault passphrase source. Runtime-control bearer tokens are random per launch, loopback-only, replay-protected, and redacted.

Logs may include safe IDs, event kinds, revisions, lifecycle states, ports, counts, and durations. They must never contain prompts, chat content, request/response bodies, API keys, vault values or passphrases, runtime-control tokens, base64 media, or full third-party payloads.

## Enforcement and verification

`bun run architecture:check` combines `cargo metadata --no-deps` workspace allowlists with frontend import checks. The configuration has no legacy exceptions. `scripts/gates.sh` runs the architecture check as part of both fast and full gates.

```bash
bun run architecture:check
bun run gates
bun run e2e
bun run e2e:tauri
bun run tauri:build:cloud
python -m ruff check sidecar
python -m pytest sidecar/tests -q
```

Focused tests, full gates, mocked Playwright, real Tauri/WebView, local-model/GPU smoke, cloud bundle builds, remote CI, and release/deployment evidence are reported separately.
