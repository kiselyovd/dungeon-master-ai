# Dungeon Master AI

A multimodal, AI-powered D&D desktop application. Tauri v2 + React + Rust.

> **Status:** M2 - rules engine + combat overlay. Combat resolver (4-phase
> turn, initiative, action economy, conditions, damage, saving throws,
> ability checks), SQLite persistence via `sqlx`, VTT combat overlay
> (tokens, AoE templates, initiative tracker, action bar), and chat
> refinements (typing indicator, sticky-scroll within 100px of bottom,
> drop-cap on narrator paragraphs). M3 wires the LLM agent loop and
> tool-call surface (the validator dispatch table is already in place).
>
> Three LLM backends remain configurable in Settings:
> - **Anthropic (cloud)** - paste an `sk-ant-...` key, default model
>   `claude-haiku-4-5-20251001`.
> - **OpenAI-compatible (cloud or local server)** - any endpoint that
>   speaks `/v1/chat/completions`: LM Studio, Ollama (`/v1/`), llama.cpp
>   server, vLLM, mistral.rs server, OpenRouter, Groq, DeepSeek, Together,
>   Fireworks, etc.
> - **Embedded local model** - reserved slot, lights up in M4.

## Stack

- Desktop shell: Tauri v2 (Rust)
- Frontend: React 19 + TypeScript + Vite + SWC + Zustand + react-i18next
  + PixiJS v8 + valibot (runtime schemas)
- Backend: Rust workspace - axum HTTP server, genai for LLM provider
  abstraction with hot-swap support behind `RwLock<Arc<dyn LlmProvider>>`
- Toolchain: Bun (package manager + script runner) + Biome (lint + format)
- Targets: Windows, macOS, Linux

## Repo layout

```
crates/
  app-domain/   rules engine: dice, RNG, initiative, action economy, turn FSM,
                damage, healing, conditions, saving throws, attack rolls,
                CombatResolver (phases 1+2), ToolCallValidator dispatch table
  app-llm/      LLM provider abstraction (Anthropic, OpenAI-compat, Mock)
  app-server/   axum HTTP server: /health, /chat (SSE), /providers, /settings,
                /combat/{start,action,end} (SSE) + sqlx SQLite persistence
src-tauri/      Tauri shell with sidecar lifecycle for app-server
src/
  api/          HTTP clients (chat SSE, providers, combat, errors)
  components/   ChatPanel, MessageBubble, TypingIndicator, SettingsForm,
                SettingsModal, VttCanvas, CombatOverlay, CombatToken,
                AoeTemplate, InitiativeTracker, ActionBar
  hooks/        useChat, useCombat, useStickyScroll
  state/        Zustand store with chat + settings + combat slices
  ui/           primitives: Modal, Field, Button
  locales/      en + ru (common, chat, settings, errors, combat namespaces)
  styles/       theme.css design tokens, globals.css base, fonts.css,
                combat.css (initiative tracker / action bar / token-pulse)
e2e/            Playwright frontend smoke tests
```

## Dev setup

Prerequisites:

- Rust stable: `rustup default stable`
- [Bun](https://bun.com/) 1.3+
- Tauri prerequisites for your OS: https://tauri.app/start/prerequisites/
- An Anthropic key OR a running OpenAI-compatible endpoint (LM Studio /
  Ollama / OpenRouter token / etc.)

```bash
bun install
bun run tauri dev
```

In the app window: click Settings, pick a provider, fill in the fields, Save.
Type a message, hit Send. ESC aborts a streaming response.

### Example Settings configurations

| Provider | Base URL | Model |
| --- | --- | --- |
| LM Studio | `http://localhost:1234/v1` | whatever you loaded (e.g. `qwen3-1.7b`) |
| Ollama | `http://localhost:11434/v1` | `qwen3:1.7b`, `llama3.2`, etc. |
| llama.cpp server | `http://localhost:8080/v1` | name from your gguf |
| vLLM | `http://localhost:8000/v1` | model alias |
| OpenRouter | `https://openrouter.ai/api/v1` | `anthropic/claude-3.5-haiku`, `qwen/qwen-3-8b`, etc. |
| Groq | `https://api.groq.com/openai/v1` | `llama-3.3-70b-versatile`, etc. |
| DeepSeek | `https://api.deepseek.com/v1` | `deepseek-chat`, `deepseek-reasoner` |

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
