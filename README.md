# Dungeon Master AI

A multimodal, AI-powered D&D desktop application. Tauri v2 + React + Rust.

> **Status:** M1.5 - chat skeleton with multi-provider support. Three LLM
> backends configurable in Settings:
> - **Anthropic (cloud)** - paste an `sk-ant-...` key, default model
>   `claude-haiku-4-5-20251001`.
> - **OpenAI-compatible (cloud or local server)** - point at any endpoint
>   that speaks `/v1/chat/completions`: LM Studio, Ollama (`/v1/`),
>   llama.cpp server, vLLM, mistral.rs server, OpenRouter, Groq, DeepSeek,
>   Together, Fireworks, etc.
> - **Embedded local model** - reserved slot, lights up in M4.
>
> Empty PixiJS grid stub. No game engine yet (that's M2).

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
  app-domain/   shared domain types (will grow in M2)
  app-llm/      LLM provider abstraction (Anthropic, OpenAI-compat, Mock)
  app-server/   axum HTTP server: /health, /chat (SSE), /providers, /settings
src-tauri/      Tauri shell with sidecar lifecycle for app-server
src/
  api/          HTTP client (chat SSE, providers, settingsStore, errors)
  components/   ChatPanel, MessageBubble, SettingsForm, SettingsModal, VttCanvas
  hooks/        useChat
  state/        Zustand store with chat + settings slices, ProviderConfig union
  ui/           primitives: Modal, Field, Button
  locales/      en + ru (common, chat, settings, errors namespaces)
  styles/       theme.css design tokens, globals.css base
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
