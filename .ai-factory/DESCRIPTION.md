# Dungeon Master AI

## Overview

Dungeon Master AI is an experimental cross-platform desktop application for solo and host-owned cooperative Dungeons & Dragons 5e play. It combines an agentic Dungeon Master, server-authoritative mechanics, a virtual tabletop, durable campaign state and optional local or cloud media generation in a Tauri application. Implemented contracts and historical QA do not imply complete D&D rules or current real-model/physical-LAN acceptance.

## Core Features

- Run an agentic, multi-round Dungeon Master loop with narration and validated tool calls.
- Resolve server-authoritative D&D 5e combat, initiative, conditions, damage, healing, spells, and action economy.
- Present combat and exploration on a PixiJS virtual tabletop with interactive tokens and overlays.
- Create and persist characters, campaigns, sessions, messages, saves, scenes, journal entries, and NPC memories.
- Prepare a bundled adventure or custom opening, confirm a campaign-scoped hero, and continue only after the world is initialized.
- Host authenticated LAN/VPN rooms with separate human-owned heroes, shared public state, reconnect and durable request recovery.
- Preserve uncertain operation identities across retry and restart; support owner backups, player dossiers and scoped read-only desktop views.
- Use either local model sidecars or OpenAI-compatible cloud endpoints for chat, image, and video generation.
- Provide complete English and Russian user interfaces.

## Tech Stack

- **Desktop shell:** Tauri 2 with Rust.
- **Frontend:** React 19, TypeScript, Vite, Zustand, react-i18next, PixiJS, Valibot, and CSS Modules plus shared CSS tokens.
- **Backend:** Rust workspace with Axum, Tokio, SQLx, SQLite, `genai`, `tracing`, `thiserror`, and `anyhow`.
- **Media sidecar:** Python 3, FastAPI, Uvicorn, PyTorch, and pluggable image/video backends.
- **Persistence and secrets:** SQLite for application state, Stronghold for provider credentials, and the OS credential store for the desktop backend vault's unlock credential. Explicit operator environment credentials retain priority.
- **Tooling:** Bun, Biome, Vitest, Testing Library, Playwright, Cargo, pytest, and GitHub Actions.
- **External integrations:** OpenAI-compatible APIs such as RouterAI/OpenRouter and independently managed local servers, Hugging Face model discovery/downloads, optional embedded `mistralrs`, and local media sidecars. The standard cloud release packages the desktop shell and game backend; model/media runtimes and weights are separate.

## Current System Boundaries

- `crates/app-domain` owns deterministic D&D rules and SRD-domain behavior.
- `crates/app-application` owns use cases, protocol-neutral models and inward-owned ports; its only workspace dependency is `app-domain`.
- `crates/adapter-http` owns Axum HTTP/SSE translation; `adapter-sqlite` owns repositories, migrations and transactions.
- `crates/adapter-llm`, `adapter-media` and `adapter-secrets` implement provider, media/runtime-control and secret-storage ports without importing sibling adapters.
- `crates/app-bootstrap` composes concrete backend adapters, configuration and startup.
- `crates/app-llm` and `crates/app-server` are compatibility facades for stable consumers and tests. They do not own new application policy or composition.
- `src-tauri` owns native capabilities, private runtime control and every embedded child process. Heavy inference stays outside the frontend.
- `src/app` composes React controllers and store ports; `src/features` owns feature controllers/models; `src/api` validates transport; `src/state` stores projections; `src/components` and `src/ui` render presentation.
- `sidecar` is a separate loopback HTTP process for image and video generation; heavy model dependencies do not enter the Rust or frontend dependency graphs.

## Architecture Notes

- The product is shipped as one desktop application, even though it coordinates several local processes. These processes are deployment components, not independently owned microservices.
- Domain rules should remain deterministic and isolated from HTTP, Tauri, database, model-provider, and UI concerns.
- Frontend code should cross native, filesystem, network, model, and persistence boundaries through typed APIs rather than direct component-level access.
- Provider and sidecar implementations should remain replaceable behind stable interfaces and wire contracts.
- SQLite migrations, API schemas, frontend runtime schemas, and localized error mappings must evolve together when contracts change.

## Architecture

See `.ai-factory/ARCHITECTURE.md` for the detailed target architecture, dependency rules, and staged migration guidance.

- **Pattern:** Explicit Architecture (Technical Layer).
- **Mode:** Strict boundaries are implemented and enforced by `scripts/check-architecture.ts`; see `.ai-factory/ARCHITECTURE.md` for normative dependency rules.

## Non-Functional Requirements

- **Privacy:** Keep local campaign content and model inputs on-device unless the user explicitly configures a cloud provider.
- **Security:** Store provider credentials in Stronghold and the desktop backend unlock credential in the OS credential store; constrain Tauri capabilities, validate untrusted payloads, and avoid logging secrets or full private prompts.
- **Reliability:** Use explicit timeouts, cancellation, retry/backoff where appropriate, process health probes, and honest degraded states.
- **Observability:** Emit contextual structured logs from Rust boundaries and actionable errors from frontend and sidecar boundaries.
- **Performance:** Keep UI work responsive while model inference, downloads, persistence, and media generation run asynchronously.
- **Portability:** Preserve Windows, macOS, and Linux behavior; isolate OS-specific process and packaging logic.
- **Localization:** Keep all human-visible frontend strings in the English and Russian locale catalogs.
- **Verification:** Treat `scripts/gates.sh` as the consolidated local quality gate and run browser or Tauri smoke coverage for visible application changes.
