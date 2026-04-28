# Dungeon Master AI

A multimodal, AI-powered D&D desktop application. Tauri v2 + React + Rust.

> **Status:** M1 milestone (chat skeleton). Working: install, configure Anthropic API key, stream chat responses. Empty PixiJS grid stub. No game engine yet.

## Stack

- Desktop shell: Tauri v2 (Rust)
- Frontend: React 19 + TypeScript + Vite + SWC + Zustand + react-i18next + PixiJS v8
- Backend: Rust workspace - axum HTTP server, genai for LLM provider abstraction
- Toolchain: Bun (package manager + script runner) + Biome (lint + format)
- Targets: Windows, macOS, Linux

## Repo layout

```
crates/
  app-domain/   shared domain types (will grow in M2)
  app-llm/      LLM provider abstraction (Anthropic via genai, MockProvider)
  app-server/   axum HTTP server with /health and /chat (SSE)
src-tauri/      Tauri shell with sidecar lifecycle for app-server
src/            React frontend (chat, vtt grid, settings)
e2e/            Playwright frontend smoke tests
```

## Dev setup

Prerequisites:

- Rust stable: `rustup default stable`
- [Bun](https://bun.com/) 1.3+
- Tauri prerequisites for your OS: https://tauri.app/start/prerequisites/
- (Optional) Anthropic API key for live chat

```bash
bun install
bun run tauri dev
```

In the app window: click Settings, paste your Anthropic API key, save. Type a message, hit Send.

## Tests and gates

```bash
# Rust unit + integration tests
cargo test --workspace

# Frontend unit tests
bun run test

# Frontend E2E (Playwright, frontend only - Tauri shell e2e comes in M4)
bun run e2e

# Biome (lint + format + organize-imports in one pass)
bun run check

# Type check
bun run typecheck

# Rust clippy
cargo clippy --workspace --all-targets -- -D warnings
```

## Build

```bash
bun run tauri build
```

## License

TBD.
