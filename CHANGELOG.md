# Changelog

All notable changes to this project are tracked here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Tags before `v1.0.0` are pre-release milestones; the public surface and
on-disk schema may change between them. See `docs/RELEASE.md` for the
release pipeline and `docs/RELEASE_CHECKLIST.md` for the per-release
manual smoke list.

## [Unreleased] - M5 polish

### Added
- Persistent `SessionSlice` (`src/state/session.ts`) with lazily-minted
  `(activeCampaignId, activeSessionId)` UUID pair, persisted via the
  split-storage adapter (`settings.json`).
- `useSession` hook that ensures a session exists on app mount and
  rehydrates the chat from `GET /sessions/{id}/messages` so reopening a
  campaign restores chat history (incl. images sent in prior sessions).

### Changed
- `useAgentTurn` and `useChat` now read campaign and session IDs from
  the persistent `SessionSlice` instead of hardcoded zero-UUID
  placeholders.
- `streamChat` invocations pass the active `sessionId` so the backend
  persists user and assistant rows across launches.

## [v0.5.5-m4.5] - 2026-05-09 - Vision input + chat persistence

### Added
- Multimodal user messages: `MessagePart` enum (`Text` / `Image`) on
  both backend and frontend; `LlmProvider::supports_vision()` capability
  flag (Anthropic / OpenAICompat / mistralrs report `true`).
- Composer attachments: paperclip button, paste-from-clipboard, and
  drag-drop on the chat panel; PNG/JPEG/WebP, ≤ 5 MB, ≤ 4 images per
  message; thumbnail strip and `ImageLightboxModal` for full-size view.
- Backend chat persistence: `messages` table (migration
  `0003_m4_5_messages.sql`), `db::insert_message`,
  `db::list_messages_by_session`, and `GET /sessions/{id}/messages`
  endpoint returning `Vec<ChatMessage>` JSON.
- `/chat` and `/agent/turn` accept an optional `session_id` and persist
  user, assistant, and tool-result rows when present.
- HF Hub real tree-API walker (`/api/models/{repo}/tree/{rev}`) for the
  diffusers folder downloader; injectable `HfEndpoints` for testing.
- E2E vision-flow Playwright spec + RELEASE_CHECKLIST M4.5 section.

### Changed
- `ChatMessage::User { content: String }` -> `ChatMessage::User { parts:
  Vec<MessagePart> }`; `ChatMessage::user_text(impl Into<String>)`
  helper for the common text-only path.
- `HttpMessage` accepts both legacy `{role, content}` and new `{role,
  parts}` shapes via custom dual-shape `Deserialize` (size guards: 5 MB
  per image, 4 images per message - return HTTP 413 on overrun).

### Tests
- 206 cargo + 167 vitest (cargo per-crate to avoid Tauri-bin
  PDB-overflow LNK1140 on Windows).

## [v0.5.0-m4] - 2026-05-08 - Local Mode + cross-platform packaging

### Added
- Local LLM via `mistralrs-server` Tauri sidecar binary (pinned
  `MISTRALRS_VERSION=v0.8.0`), ProviderConfig variant `local-mistralrs`
  hot-swappable from Settings.
- Local image generation via Python sidecar (FastAPI + diffusers
  SDXL-Turbo), GPU coordination mutex (auto-swap LLM unload before
  image-gen).
- Model manifest (Qwen3.5 0.8B/2B/4B/9B + SDXL-Turbo), download
  manager with resumable HTTP Range + sha256 verify, per-model state
  machine with cancel.
- `LocalRuntime` state machine, free-port discovery, health-probe with
  exponential backoff retry, `RuntimeRegistry` aggregating two
  runtimes.
- Tauri updater plugin + `UpdateAvailableModal` + `useUpdater` hook
  (pubkey placeholder until first release).
- `SecretsRepo` trait + `InMemorySecretsRepo` (Stronghold real impl
  deferred to M5).
- Local Mode UI: `LocalModeSlice`, `useModelDownload` SSE hook,
  `useLocalRuntimeStatus` 5 s poll, `ModelDownloadCard`,
  `RuntimeStatusPill`, `LocalModeModal`.
- Cargo features: default `with-local-runtime`, `cloud-only` gates
  Local Mode routes off via cfg-attribute.
- CI: `build-bundle.yml` matrix (4 targets, no signing) + `release.yml`
  with Win EV cert + macOS rcodesign + notarytool + Linux AppImage.
- `RELEASE_CHECKLIST.md` for manual smoke after each release.

### Tests
- 179 cargo + 157 vitest.

## [v0.4.0-m3] - 2026-05-08 - LLM agent loop + RAG + journal + NPC

### Added
- `AgentOrchestrator` N-round tool-call loop with a 15-tool surface
  (combat: `roll_dice`, `apply_damage`, `start_combat`, `end_combat`,
  `add_token`, `update_token`, `remove_token`; narrative: `set_scene`,
  `cast_spell`, `remember_npc`, `recall_npc`, `journal_append`,
  `quick_save`, `generate_image`, `query_rules`).
- SRD content (20 spells / 15 monsters / 30 rules / 4 classes) +
  fastembed BGE-small-en (default switched to MultilingualE5Small for
  RU+EN coverage) + cosine retriever; RAG injection in
  `context_builder`.
- Journal: `journal_entries` table, `JournalViewer` overlay (full-screen
  modal, EN/RU, parchment polish deferred to M5).
- NPC memory: `npc_memory` table, `NpcMemoryGrid` (responsive 320x420
  dossier cards, name/role search, disposition chip filters).
- `ImageProvider` trait + `ReplicateProvider` (90 s deadline + 10 s
  per-request timeouts + status-checked polling) + cache-key
  (FNV-1a 64-bit hex of scene_id + sorted(npc_ids) + style_preset);
  `LocalSdxlSidecarProvider` stub for M4.
- `POST /agent/turn` SSE endpoint wrapping `AgentOrchestrator`.
- DM system prompts (EN+RU reference templates).
- Frontend `ToolCallCard` with settle animation (cycle digits at 100 ms
  while pending, snap to JSON + 600 ms gold flash on settle),
  `ToolLogSlice`, `streamAgentTurn` + `useAgentTurn` hook,
  `ToolInspectorDrawer` (480 px right slide-in, copy-as-cURL).
- Settings Model tab (system prompt + temperature 0..2 step 0.1 +
  Replicate API key).

### Tests
- 37 cargo + 140 vitest.

## [v0.3.0-m2] - 2026-04-30 - D&D 5e combat resolver + VTT overlay

### Added
- 4-phase turn (initiative + action economy + conditions + damage +
  healing + saves+checks + attack rolls) via `CombatResolver`.
- `ToolCallValidator` dispatch table for the seven M2 combat tools.
- sqlx 0.8 + uuid 1 + chrono 0.4 SQLite persistence: `campaigns`,
  `sessions`, `snapshots` (with v2 forward-compat `parent_save_id` /
  `branch_id` columns), `combat_encounters`, `combat_tokens`.
- Three combat HTTP endpoints (`/combat/start`, `/action`,
  `/combat/end`) streaming six SSE-event variants.
- PixiJS `CombatToken` HTML overlay (HP-by-percent bar / AC chip /
  status ring / active-turn pulse), `AoeTemplate` cone/sphere/line/cube
  SVGs from school-of-magic palette, `CombatOverlay` container (280 ms
  cross-fade entry), `InitiativeTracker` slide-in, `ActionBar`.
- Chat refinements: `TypingIndicator` three Cinzel diamonds 1.4 s loop,
  `useStickyScroll` 100 px threshold, drop-cap on finalised assistant
  bubbles.
- Global `prefers-reduced-motion` override.

### Tests
- 76 cargo + 116 vitest.

## [v0.2.0-m1.5] - 2026-04-29 - Multi-provider settings + UI hardening

### Added
- `OpenAICompatProvider` for BYO base_url + model + api_key (LM Studio,
  Ollama `/v1/`, llama.cpp server, vLLM, mistral.rs, OpenRouter, Groq,
  DeepSeek, Together, Fireworks).
- Hot-swap atomic via `RwLock<Arc<dyn LlmProvider>>` in `AppState`,
  driven by `POST /settings`.
- Self-hosted Inter / Cinzel / JetBrains Mono in `public/fonts/`.
- C1-C6 design tokens folded into `src/styles/theme.css`.

### Changed
- Toolchain: npm + eslint + prettier + `@vitejs/plugin-react-swc` ->
  `bun` + `biome` + `@vitejs/plugin-react` v6.
- `valibot` (replaces zod) for runtime schemas.
- Zustand `persist` middleware with split-storage adapter
  (`secrets.json` for provider configs / API keys; `settings.json` for
  non-sensitive prefs).
- All inline styles migrated to CSS Modules.
- `--color-accent-soft` -> `--color-accent-tint` (translucent rename),
  freeing the canonical name for the hover-gold semantic.

## [v0.1.0-m1] - 2026-04-28 - Tauri shell + chat skeleton

### Added
- Tauri v2 shell (Rust workspace: `app-domain`, `app-llm`, `app-server`
  + `src-tauri/`).
- axum HTTP server as embedded backend, Tauri sidecar launch.
- `LlmProvider` trait + `AnthropicProvider` (genai 0.6.0-beta.18 with
  native Anthropic prompt caching).
- Streaming chat skeleton via SSE (`/chat` endpoint).
- React 19 + TypeScript + Vite + Zustand + react-i18next frontend
  scaffolding.
