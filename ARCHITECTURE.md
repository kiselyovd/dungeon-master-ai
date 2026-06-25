# Architecture

A high-level tour of how Dungeon Master AI is put together. For setup and the development workflow, see [CONTRIBUTING.md](CONTRIBUTING.md); for the feature list and shipped status, see [README.md](README.md).

## The big picture

Dungeon Master AI is a Tauri v2 desktop app. The Tauri (Rust) shell renders a React frontend in a webview and, at startup, spawns a local `axum` HTTP backend (`dmai-server`). On demand it can also spawn two model sidecars: a `mistralrs` LLM process (local text/vision generation) and a Python image-generation sidecar (diffusers). The frontend never talks to model runtimes directly; it talks to `dmai-server`, which owns provider selection, the agentic DM loop, combat, persistence, and sidecar lifecycle.

```
+----------------------------------------------------------------+
|  Tauri shell (src-tauri/, Rust)                                |
|                                                                |
|  +--------------------------+      spawns       +-----------+  |
|  |  Webview: React frontend |  <-- HTTP/SSE -->  | dmai-server |
|  |  (src/)                  |                    | (app-server)|
|  +--------------------------+                    +-----+-----+  |
|                                                       |        |
|                            spawns + health-probes     |        |
|                       +-------------------------------+        |
|                       v                       v                |
|              +------------------+   +----------------------+   |
|              | mistralrs sidecar|   | Python image sidecar |   |
|              | (local LLM)      |   | (diffusers backends) |   |
|              +------------------+   +----------------------+   |
+----------------------------------------------------------------+
                       |  SQLite (sqlx)  +  Stronghold vault (secrets)
```

## Rust workspace

The Cargo workspace (`Cargo.toml`) has three library crates plus the Tauri binary crate.

### `crates/app-domain` - rules engine + SRD content

Pure domain logic with no I/O of its own.

- `combat/` - the D&D 5e combat engine: `initiative`, `action_economy`, `conditions`, `damage`, `healing`, `attack`, `ability_check`, `saving_throw`, the `turn_fsm` turn state machine, the top-level `resolver`, a `validator` (dispatch table for combat tool calls), and `result_events` for emitting structured outcomes.
- `dice.rs` / `rng.rs` - dice expression evaluation and a seedable RNG.
- `srd/` - the SRD content pipeline: a `loader` and `data` for SRD rules/spells/monsters/classes, an `embedder` (fastembed-backed; default `multilingual-e5-small` for RU+EN), and a cosine-similarity `retriever` (`SrdRetriever`) used for rules RAG.
- `compendium/` - the spell/monster compendium (`loader`, `types`, bundled `data`) consulted during spell resolution.
- `local_llm/` - the local model `manifest` (which models the app can download and run).

### `crates/app-llm` - LLM provider abstraction

The `LlmProvider` trait (`provider.rs`) plus its implementations, all built on the `genai` crate:

- `openai_compat.rs` - the single generic OpenAI-compatible provider (OpenRouter, LM Studio, Ollama, vLLM, Groq, DeepSeek, and so on).
- `mistralrs_provider.rs` - the local `mistralrs` provider, talking to the spawned sidecar.
- `mock.rs` - a `MockProvider` used throughout tests.
- Supporting modules: `genai_common` (shared mapping), `retry` (backoff), and `sidecar_launcher`.

The shared types (`ChatMessage`, `MessagePart` for text/image, `ChatRequest`, `ChatChunk`, `ToolCall`, `FinishReason`, `ReasoningSpec`, `LlmError`) live here so both the providers and the server agree on the wire shape. Vision support is a per-provider capability flag.

### `crates/app-server` - the backend (`dmai-server`)

An `axum` HTTP server. It owns all application state and exposes the API the frontend uses.

- `state.rs` - `AppState`. The active providers across all three modalities are consolidated into a single `registry: RwLock<Arc<ProviderRegistry>>` so a settings change can install a fully-built registry in one atomic swap without tearing down `AppState`. It also holds the SRD retriever, agent config, secrets repo, models dir, and local-mode config behind `RwLock`s, plus the `RuntimeRegistry` for sidecars.
- `providers/` - the `ProviderRegistry` (chat / image / video slots; chat is always present, image and video are optional), a `catalog` of curated providers and models, and a `discovery/` subsystem for dynamic model discovery.
- `agent/` - the agentic DM loop (see below): `orchestrator`, `context_builder`, `tools` (definitions + availability flags), and `tool_executor`.
- `routes/` - the HTTP surface: `agent` (`/agent/turn` SSE), `chat`, `combat`, `saves`, `journal`, `npc`, `messages`, `settings/` (settings v2), `providers`, `srd`, `hf` (HuggingFace), `image`, `video`, `local_llm`, `local_mode`, `character_assist`, and `health`.
- `image/` - the `ImageProvider` trait, a Replicate provider, a local-sidecar path, a result `cache`, and `retry`.
- `video/` - the `VideoProvider` trait and the video sidecar integration.
- `local_runtime/` - sidecar lifecycle: free-`port` discovery, `process_launcher`, `health` probing with backoff, a per-process `runtime` state machine, and a `RuntimeRegistry` aggregating the LLM and image runtimes.
- `models/` - the model `manifest`, `download` manager (resumable HTTP Range + sha256 verify), and `manager`.
- `hf/` - the HuggingFace `client`, tree-API `compat` walker, `manifest`, and `types` for browsing and downloading models.
- `secrets/` - the `SecretsRepo` trait, a `stronghold` implementation (`iota_stronghold`), and a one-shot `migrate` step that drains any pre-existing plaintext secrets into the vault.
- `db.rs` + `migrations/` - SQLite persistence via `sqlx` (campaigns, sessions, messages, snapshots, combat encounters and tokens, scenes, journal entries, NPC memory).

### `src-tauri` - the Tauri shell

The desktop binary: window/config (`tauri.conf.json`), `build.rs`, capabilities, icons, and the bundled sidecar binaries under `binaries/` (Tauri `externalBin`). It launches `dmai-server` and wires the Stronghold and updater plugins.

## Frontend (`src/`)

React 19 + TypeScript + Vite.

- `components/` - the UI: the chat panel and message bubbles, the PixiJS VTT (`VttCanvas`, `CombatOverlay`, `CombatToken`, `AoeTemplate`, `InitiativeTracker`, `ActionBar`), the character wizard and sheet, settings and onboarding flows, model-download and runtime-status UI, the journal and NPC memory views, and tool-call cards / inspector.
- `state/` - Zustand stores (chat, combat, conditions, pc, npc, journal, saves, session, settings, providers, local mode/LLM, tool log), with a split-storage persistence adapter (`persistStorage`) and a Stronghold-backed secrets store. Provider configs and API keys go to the encrypted vault; non-sensitive prefs go to `settings.json`.
- `api/` - typed clients for the backend (`agent`, `chat`, `combat` via `saves`/`srd`/etc.), an `sse` helper for streaming, valibot `schemas` for runtime validation, and the base `client`.
- `hooks/` - React hooks that drive flows: `useAgentTurn`, `useSession`, `useChat`, `useModelDownload`, `useLocalRuntimeStatus`, `useUpdater`, `useDiscoverProvider`, combat tool handlers, and more.
- `locales/en` + `locales/ru` - full English and Russian translations (react-i18next).
- `styles/` - design tokens and CSS Modules.

## Python image sidecar (`sidecar/`)

A FastAPI + diffusers process. `app.py` prints `LISTENING_ON_PORT=<n>` as its first stdout line; the Rust `LocalRuntime` parses that and probes `/healthz`. It exposes image and video generation endpoints over SSE. Backends include SDXL-Lightning (the recommended default), Z-Image-Turbo, and Nunchaku FLUX (optional). See `sidecar/README.md` for install details and the GPU/CUDA notes.

## Persistence and secrets

- Campaign and session data: a local SQLite database via `sqlx`, schema-managed by the `migrations/` directory.
- Secrets (provider API keys): an encrypted Stronghold vault, never plaintext. The frontend uses `tauri-plugin-stronghold`; the backend uses `iota_stronghold` and reopens the snapshot on startup, so a sidecar restart does not require the frontend to re-deliver keys.

## The provider registry

All three modalities live in one `ProviderRegistry` struct with `chat`, `image`, and `video` slots (each an `Arc<dyn _>`; chat is required, image and video are `Option`). The registry sits behind `RwLock<Arc<ProviderRegistry>>` in `AppState`. When the user saves settings, the server builds a brand-new registry and swaps the `Arc` atomically, so in-flight requests holding the old `Arc` finish cleanly while new requests pick up the new providers. This is how cloud/local provider switches happen with no restart.

## The agentic DM loop

A single player turn flows through `agent/orchestrator.rs`, which runs up to `max_rounds` rounds of: stream the LLM, collect any tool calls, execute them against the engine, feed the results back, repeat. The orchestrator emits `AgentEvent`s to a channel; the `/agent/turn` route converts those to SSE events for the frontend.

One turn, step by step:

1. The frontend posts the player message (and any attached images) to `POST /agent/turn` and opens an SSE stream (`useAgentTurn`).
2. `context_builder` assembles the prompt: the DM system prompt, recent chat history, the current scene, relevant NPC memory, and - when the message needs rules - SRD passages retrieved by `SrdRetriever` (RAG).
3. The orchestrator calls the active chat provider from the registry, streaming narration tokens (and reasoning, when enabled) back over SSE as they arrive.
4. If the model emits tool calls (for example `roll_dice`, `start_combat`, `apply_damage`, `set_scene`, `remember_npc` / `recall_npc`, `journal_append`, `quick_save`, `generate_map` / `generate_illustration`, `query_rules`), `tool_executor` runs each one. Combat tools go through the `app-domain` resolver; persistence tools hit SQLite; media tools call the image/video providers.
5. Tool results are appended to the conversation and the loop runs another round so the model can narrate the outcome. The set of exposed tools is filtered by `ToolAvailability` (image/video tools drop out when those modalities are disabled in settings; core tools like `roll_dice` are always present).
6. The loop ends when the model returns a final narration with no further tool calls, or when `max_rounds` is reached. The frontend renders streamed narration, inline tool-call cards, and any generated images/video.
