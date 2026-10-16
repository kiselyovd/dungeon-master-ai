# Dungeon Master AI

## Overview

Dungeon Master AI is a cross-platform desktop assistant for solo Dungeons & Dragons 5e play. It combines an agentic Dungeon Master, a rules-aware combat engine, a virtual tabletop, campaign persistence, and local or cloud multimodal generation in a Tauri application.

## Core Features

- Run an agentic, multi-round Dungeon Master loop with narration and validated tool calls.
- Resolve server-authoritative D&D 5e combat, initiative, conditions, damage, healing, spells, and action economy.
- Present combat and exploration on a PixiJS virtual tabletop with interactive tokens and overlays.
- Create and persist characters, campaigns, sessions, messages, saves, scenes, journal entries, and NPC memories.
- Use either local model sidecars or OpenAI-compatible cloud endpoints for chat, image, and video generation.
- Provide complete English and Russian user interfaces.

## Tech Stack

- **Desktop shell:** Tauri 2 with Rust.
- **Frontend:** React 19, TypeScript, Vite, Zustand, react-i18next, PixiJS, Valibot, and CSS Modules plus shared CSS tokens.
- **Backend:** Rust workspace with Axum, Tokio, SQLx, SQLite, `genai`, `tracing`, `thiserror`, and `anyhow`.
- **Media sidecar:** Python 3, FastAPI, Uvicorn, PyTorch, and pluggable image/video backends.
- **Persistence and secrets:** SQLite for application state and Tauri Stronghold for credentials.
- **Tooling:** Bun, Biome, Vitest, Testing Library, Playwright, Cargo, pytest, and GitHub Actions.
- **External integrations:** OpenAI-compatible APIs, OpenRouter-compatible routing, Hugging Face model discovery/downloads, embedded `mistralrs`, and local media sidecars.

## Current System Boundaries

- `crates/app-domain` owns deterministic D&D rules and SRD-domain behavior.
- `crates/app-llm` owns model-provider abstractions and local LLM process integration.
- `crates/app-server` owns HTTP/SSE delivery, agent orchestration, persistence, provider lifecycle, and local runtime coordination.
- `src-tauri` is the desktop composition root and owns native capabilities plus child-process startup.
- `src` owns the React presentation layer, local UI state, API clients, localization, and VTT rendering.
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
- **Mode:** Strict target architecture; the existing codebase will require a planned, checkpointed refactor.

## Non-Functional Requirements

- **Privacy:** Keep local campaign content and model inputs on-device unless the user explicitly configures a cloud provider.
- **Security:** Store credentials in Stronghold, constrain Tauri capabilities, validate untrusted payloads, and avoid logging secrets or full private prompts.
- **Reliability:** Use explicit timeouts, cancellation, retry/backoff where appropriate, process health probes, and honest degraded states.
- **Observability:** Emit contextual structured logs from Rust boundaries and actionable errors from frontend and sidecar boundaries.
- **Performance:** Keep UI work responsive while model inference, downloads, persistence, and media generation run asynchronously.
- **Portability:** Preserve Windows, macOS, and Linux behavior; isolate OS-specific process and packaging logic.
- **Localization:** Keep all human-visible frontend strings in the English and Russian locale catalogs.
- **Verification:** Treat `scripts/gates.sh` as the consolidated local quality gate and run browser or Tauri smoke coverage for visible application changes.
