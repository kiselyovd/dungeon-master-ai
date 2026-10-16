# Offline Media and Full-Game Acceptance Design

**Status:** Approved in conversation on 2026-08-12

## Objective

Make Dungeon Master AI playable and visually complete when no Stable Diffusion or cloud image provider is connected, then prove the full product through deterministic tests and a real Tauri/WebView2 campaign driven over CDP.

The implementation must not represent bundled artwork as newly generated media. Runtime-generated and bundled images share the existing agent media delivery path, but retain explicit provenance through every layer.

## Scope

This delivery contains three independently verifiable workstreams:

1. Correct the real-Tauri CDP play check so a transient reasoning indicator cannot be accepted as the Dungeon Master's completed reply.
2. Add a bundled, deterministic image fallback for tactical maps and chat illustrations.
3. Execute a comprehensive acceptance campaign covering character creation, narrative events, media, map interaction, combat, campaign records, saves, and the locally runnable model matrix.

Video generation remains an independently reported capability. This work does not synthesize fake video when the video runtime is unavailable.

## Architecture

### Media provider composition

The application layer continues to own the `ImageProvider` port. Concrete runtime, cloud, and bundled implementations remain in `adapter-media`; selection and composition happen in backend wiring.

```text
generate_map / generate_illustration
                 |
                 v
       ResilientImageProvider
          |              |
   configured primary    BundledImageProvider
   SD or cloud            deterministic catalog
          |              |
          +-------+------+
                  v
       ImageBytes with provenance
                  |
                  v
       AgentEvent -> SSE -> React projection
                  |               |
             map -> VTT      chat -> timeline
```

`ResilientImageProvider` first invokes the configured primary provider. It falls back for provider absence, network failure, timeout, runtime unavailability, or an explicitly degraded media service. Authentication and invalid configuration errors remain visible and are not converted into a bundled success.

When no primary provider is configured, backend composition installs the bundled provider directly so media tools remain available.

### Media result contract

Image results gain transport-independent provenance owned by `app-application`:

```rust
pub enum ImageSource {
    Generated,
    Bundled { asset_id: String },
}

pub struct ImageBytes {
    pub data: Vec<u8>,
    pub mime_type: String,
    pub source: ImageSource,
}
```

The agent media event and SSE payload expose a stable wire value:

```json
{
  "mime_type": "image/webp",
  "image_b64": "...",
  "kind": "map",
  "source": "bundled",
  "asset_id": "map-forest-crossing"
}
```

`source` is `generated` or `bundled`. `asset_id` is present only for bundled media. Existing clients that omit or do not know these fields retain their current behavior. Base64 media stays out of model history, SQLite message content, logs, and persisted Zustand state.

### Bundled catalog

The catalog is compiled into `adapter-media`, so it works from the development binary and packaged `dmai-server` without an extra process or runtime path lookup.

It contains two categories:

- Chat illustrations: tavern, village, forest road, ruins, cave, dungeon, temple, and wizard tower.
- Tactical maps: tavern interior, forest crossing, ruined courtyard, cave chamber, dungeon hall, temple floor, village square, and tower chamber.

All assets share the approved calm, living-classic tabletop art direction. Tactical maps are top-down, grid-friendly, contain no characters or tokens, and contain no text labels.

Selection is deterministic. The provider normalizes the requested content and hashes `category + style_preset + normalized prompt`; modulo catalog length selects the asset. Identical requests therefore yield identical assets across runs. Map and illustration catalogs never cross-select.

The implementation uses existing generated project artwork where it satisfies the category contract and creates only the missing scenes. Assets are WebP at a bounded size appropriate for SSE transfer. The catalog includes asset IDs, category, MIME type, keywords, and bytes; no filenames or paths leak into the transport contract.

### Presentation

The existing `image_generated` event remains responsible for routing:

- `kind: map` updates the ephemeral VTT map projection.
- `kind: chat` attaches the image to the corresponding chat tool call.

Bundled media receives a quiet localized label, `From the built-in collection` / `Из встроенной коллекции`. Generated media is unchanged and receives no extra badge. The label uses existing typography, colors, spacing, radii, and motion tokens; no new design scale or production dependency is introduced.

## CDP Truthfulness

The real-Tauri play script must establish all of the following before reporting PASS:

1. The actual Tauri WebView is reached through its CDP target.
2. `dmai-server` is discovered through the Tauri `backend_port` command.
3. The settings update and `/agent/turn` request finish with successful HTTP statuses.
4. A non-empty assistant message is rendered after the submitted user action.
5. The stream has completed, demonstrated by the post-submission transition out of streaming state or an observed terminal agent event.

Reasoning text, a `Thinking` pill, token counters, pre-existing assistant bubbles, and tool-call progress are not final replies. The script snapshots the pre-submit assistant message count and accepts only a later completed assistant message. Timeout diagnostics include safe endpoint statuses and a bounded chat tail, never prompts, secrets, base64 media, or Stronghold values.

The reply-detection logic is extracted into a testable module. Regression fixtures cover reasoning-only state, a pre-existing assistant message, a streaming partial, a completed new reply, and an agent error.

## Acceptance Strategy

### Deterministic product checks

Automated tests must prove:

- Production root renders Dungeon Master AI labels and product defaults.
- Character creation produces a valid character projection.
- Scene, NPC, journal, dice, and quick-save events reach their projections exactly once.
- Primary image success returns `generated` provenance.
- Missing or transiently failed primary image generation returns deterministic `bundled` provenance.
- Authentication/configuration failures are not masked.
- Maps reach the VTT and illustrations remain in chat.
- Combat uses server-authoritative revisions for initiative, movement, attacks, damage, healing, spells, conditions, turn advance, and encounter end.
- Duplicate and stale combat revisions are ignored.
- Save and restore apply an atomic session projection without partial frontend writes.
- Bundled base64 media is absent from persisted frontend state and stored LLM history.

### Browser mock checks

Playwright runs a scripted campaign with controlled SSE and authoritative combat events. It verifies character creation, map mounting, token drag request and reconciliation, PC-turn gating, attack/cast/end-turn commands, journal and NPC views, save/restore, English and Russian labels, and the bundled-media provenance badge.

### Real Tauri/WebView2 campaign

The application is launched with WebView2 remote debugging enabled, and the actual window is driven over CDP. The acceptance campaign must:

1. Complete onboarding and create a named level-one character.
2. Configure and start a locally runnable chat model.
3. Begin a narrative session and receive a completed Dungeon Master reply.
4. Trigger a scene illustration and a tactical map.
5. Stop or disable the media runtime and prove that the bundled fallback still produces both kinds.
6. Open the VTT, move the player token, and reconcile the authoritative position.
7. Start combat with at least one enemy, establish initiative, complete a player action, advance a turn, apply damage or healing, and end the encounter.
8. Verify an NPC record and journal entry.
9. Create a quick save, mutate the session, restore the save, and verify the restored scene and game state.
10. Capture screenshots and safe structured evidence at key checkpoints.

Live model prose is evidence only for the model/agent connection. Deterministic tool fixtures or direct authoritative commands prove game mechanics when model behavior is non-deterministic.

### Model matrix

Model reporting is capability-specific rather than a single ambiguous PASS:

| Capability | Required evidence |
|---|---|
| Manifest | ID, metadata, dependency resolution, and download contract pass |
| Runtime startup | Sidecar reaches `ready` and remains live |
| Basic inference | First token and completed reply |
| Tool use | At least one correctly executed typed tool call |
| Media | Image or video runtime completes a real request |

Already installed and hardware-appropriate models receive runtime tests. The proven safetensors/ISQ route takes priority over known-broken GGUF variants. Large uninstalled models are not downloaded merely to fill a matrix: their estimated size, installed state, and exact untested capabilities are reported as `not runtime-tested`. Cloud models without user-supplied credentials are exercised through contract-safe mocks only.

SDXL receives one real runtime generation when the installed weights and sidecar are healthy. The bundled fallback is then tested with the media runtime disabled. Video is tested only if its model and runtime are already available; otherwise its contract and explicit unavailable state are reported separately.

## Error and Privacy Rules

- Fallback is observable through provenance and structured safe logs containing codes and asset IDs only.
- Prompt text, output base64, provider keys, Stronghold values, and local secrets never enter logs or screenshots.
- A malformed catalog or missing compiled asset fails the build or a focused test rather than degrading silently.
- Primary provider panics are not caught; ordinary typed operational failures follow the fallback policy.
- Cancellation never starts a fallback after the caller has cancelled the operation.
- No generated or bundled image is persisted in Zustand storage.

## Verification Gates

Each implementation checkpoint runs its focused tests. Final evidence is reported separately for:

1. `bun run architecture:check`.
2. Frontend lint, typecheck, Vitest, and production build.
3. Rust formatting, Clippy, and workspace tests through `bun run gates`.
4. Browser Playwright through `bun run e2e`.
5. Python 3.12 Ruff and offline pytest if sidecar code changes.
6. Real Tauri/WebView2 CDP smoke.
7. Real local-model campaign.
8. Real SDXL generation and media-disabled bundled fallback.

CI, packaging, live-model proof, and deployment remain distinct claims. No check is described as passing without fresh command output or an attached runtime artifact from this delivery.

## Non-Goals

- Downloading every large catalog model automatically.
- Treating mock inference as a real local-model run.
- Creating fake fallback video.
- Persisting generated media into campaign history.
- Moving game mechanics into React.
- Adding production dependencies, a new media process, or new visual tokens.

