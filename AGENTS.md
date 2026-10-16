# AGENTS.md

> Maintain this map when major project boundaries, entry points, or verification commands change. Detailed product and architecture context belongs in the linked AI Factory artifacts.

## Project Overview

Dungeon Master AI is a Tauri desktop application for solo D&D 5e play. It combines a React virtual tabletop and chat UI with a Rust rules/server workspace and an optional Python media-generation sidecar.

## Tech Stack

- **Languages:** TypeScript, Rust, Python, CSS, shell/PowerShell.
- **Desktop and frontend:** Tauri 2, React 19, Vite, Zustand, PixiJS, react-i18next.
- **Backend:** Axum, Tokio, SQLx with SQLite, `genai`, `tracing`.
- **Media runtime:** FastAPI/Uvicorn with pluggable PyTorch image and video backends.
- **Tooling:** Bun, Biome, Vitest, Testing Library, Playwright, Cargo, pytest.

## Project Structure

```text
.
|-- src/                    # React presentation adapter
|   |-- app/                # Production composition, shell, bootstrap, store-port wiring
|   |-- features/           # Feature models and controllers
|   |-- api/                # Typed HTTP/SSE clients and runtime schemas
|   |-- components/         # Product views during incremental feature migration
|   |-- hooks/              # Compatibility hook exports and shared UI hooks
|   |-- state/              # Zustand projections, atomic updates, persistence adapters
|   |-- ui/                 # Reusable UI primitives
|   `-- styles/             # Shared tokens and application-wide styles
|-- crates/
|   |-- app-domain/         # Deterministic D&D 5e and pure SRD behavior
|   |-- app-application/    # Use cases, typed models, and inward-owned ports
|   |-- adapter-http/       # Axum transport and SSE mapping
|   |-- adapter-sqlite/     # SQLx repositories, transactions, migrations
|   |-- adapter-llm/        # Concrete LLM and embedding providers
|   |-- adapter-media/      # HF, downloads, image/video, runtime-control client
|   |-- adapter-secrets/    # Stronghold and in-memory secret stores
|   |-- app-bootstrap/      # Backend composition and dmai-server process
|   |-- app-llm/            # Compatibility provider facade
|   `-- app-server/         # Compatibility server and test facade
|-- src-tauri/              # Sole process owner, runtime control, capabilities, packaging
|-- sidecar/                # Independent FastAPI image/video generation process
|-- e2e/                    # Playwright browser smoke coverage
|-- scripts/                # Quality gates, setup, CDP/Tauri smoke, release helpers
|-- prompts/                # English and Russian Dungeon Master prompts
|-- public/                 # Static application assets and bundled fonts
|-- docs/                   # Release guidance, research, screenshots, archived planning
|-- .ai-factory/            # AI Factory description, rules, plans, and architecture context
|-- .agents/skills/         # Project-local AI Factory skills
`-- .codex/                 # Project Codex agents and MCP configuration
```

## Key Entry Points

| File | Purpose |
|---|---|
| `src/main.tsx` | Initializes localization/styles and invokes the production bootstrap. |
| `src/app/bootstrap.tsx` | Mounts the production React root. |
| `src/app/App.tsx` | Composes feature controllers, shell, and overlays. |
| `crates/app-bootstrap/src/main.rs` | Stable `dmai-server` binary entry point. |
| `crates/app-bootstrap/src/wiring.rs` | Constructs concrete adapters and HTTP services. |
| `crates/app-application/src/lib.rs` | Public use-case, model, and port surface. |
| `crates/app-domain/src/lib.rs` | Public surface of the deterministic game-domain crate. |
| `crates/adapter-http/src/lib.rs` | Axum router and transport adapter surface. |
| `src-tauri/src/main.rs` | Native executable entry point. |
| `src-tauri/src/lib.rs` | Tauri builder, plugins, capabilities, backend process lifecycle, and app events. |
| `sidecar/app.py` | FastAPI image/video sidecar application and process entry point. |
| `scripts/gates.sh` | Consolidated local and CI quality gate. |
| `package.json` | Bun frontend scripts and JavaScript dependency manifest. |
| `Cargo.toml` | Rust workspace membership and shared dependencies. |

## Documentation

| Document | Path | Description |
|---|---|---|
| README | `README.md` | Product overview, setup, development, testing, and build instructions. |
| Changelog | `CHANGELOG.md` | Versioned user-facing changes. |
| Release guide | `docs/RELEASE.md` | Release workflow and operational guidance. |
| Release checklist | `docs/RELEASE_CHECKLIST.md` | Detailed release verification checklist. |
| Sidecar guide | `sidecar/README.md` | Python media-sidecar setup and runtime notes. |

## AI Context Files

| File | Purpose |
|---|---|
| `AGENTS.md` | Repository map and high-priority agent rules. |
| `.ai-factory/config.yaml` | AI Factory language, path, workflow, and Git settings. |
| `.ai-factory/DESCRIPTION.md` | Product, stack, boundary, and non-functional context. |
| `.ai-factory/ARCHITECTURE.md` | Strict Explicit Architecture target, dependency rules, and migration sequence. |
| `.ai-factory/rules/base.md` | Codebase-derived naming, structure, error, logging, and test conventions. |

## Agent Rules

- Preserve inward dependency direction. Domain rules belong in `app-domain`; use cases and ports belong in `app-application`; HTTP, SQLite, LLM, media, and secrets remain isolated adapters; concrete backend wiring belongs in `app-bootstrap`; native process ownership belongs in `src-tauri`; heavy inference belongs in `sidecar`.
- Keep React as a presentation adapter. Combat mechanics are server-authoritative, features do not import sibling internals, API modules do not import Zustand/components, and app modules own composition only.
- Keep secrets in Stronghold-backed flows. Never place provider tokens in source, fixtures, logs, Zustand persistence, screenshots, or generated documentation.
- Use existing components, CSS variables, spacing, typography, radii, shadows, icons, and interaction conventions before adding new UI primitives or visual tokens.
- For frontend layout, styling, flow, or interaction work, create a compact design brief when no concrete visual source exists, plan before editing, run project checks, and verify the real result in a browser or Tauri WebView.
- Do not add production dependencies without explicit user approval.
- New production roots or adapted frontends require a test that renders the real production root, plus lint, production build, and deployed/browser smoke evidence with the adapted product labels and default identities.
- Keep human-visible strings in the English and Russian locale catalogs. Do not translate API keys, event names, error codes, storage keys, or other machine contracts.
- Treat `scripts/gates.sh` as the consolidated quality-gate source. Report focused tests, full gates, browser/Tauri checks, CI, and deployment as separate evidence.
- Run `bun run architecture:check` for boundary changes. The guard has no legacy exception mechanism; fix forbidden edges instead of allowlisting them.
- Run Python 3.12 Ruff and offline pytest for sidecar changes. GPU tests stay opt-in with `RUN_GPU_TESTS=1`.
- Use the `gh` CLI for GitHub operations; this project does not use GitHub MCP.
- Preserve unrelated worktree changes and avoid broad formatting or cleanup outside the active task.
- Decompose failure-sensitive shell operations into separate commands.
  - Incorrect: `git checkout main && git pull`
  - Correct: first `git checkout main`, then `git pull origin main`.

## CodeGraph

This repository has a `.codegraph/` index. Before using text search or reading files to locate or understand code, query CodeGraph first.

- Prefer the `codegraph_explore` MCP tool when it is available.
- The shell fallback is `codegraph explore "<symbols or question>"`.
- Name a file or symbol in the query when current line-numbered source is required.
- Use `rg` or direct file reads afterward only for details CodeGraph did not provide.
