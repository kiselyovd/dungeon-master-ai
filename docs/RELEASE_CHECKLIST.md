# M4 release checklist

## Pre-release

- [ ] All M4 commits pushed to `main` via PR review
- [ ] CI `build-bundle.yml` is green on all 4 matrix targets for the merge commit
- [ ] `tauri.conf.json -> plugins.updater.pubkey` matches the `TAURI_SIGNING_PRIVATE_KEY` secret
- [ ] `CHANGELOG.md` has a section for the upcoming tag (M4: Local Mode + Packaging)
- [ ] `docs/RELEASE.md` secret list is up to date

## Manual smoke (RTX 3080 + 64 GB RAM dev machine)

- [ ] Install dev build, start app
- [ ] Open Settings, swap to a known-good cloud provider, verify chat still works
- [ ] Open Local Mode modal
- [ ] Download Qwen3.5-2B (smallest reasonable text default) and watch progress to completion
- [ ] Click "Start runtimes" and wait for the LLM pill to flip to `ready`
- [ ] Send a chat message; response streams via `local-mistralrs`
- [ ] Download SDXL-Turbo
- [ ] Trigger an image generation tool-call (e.g. "Show me the tavern")
- [ ] Auto-swap: while LLM is mid-response, request an image; LLM unloads, image generates, LLM reloads
- [ ] Switch to Qwen3.5-9B; VRAM warning badge appears; download + swap still work
- [ ] Toggle Local Mode off; chat falls back to the previously-active cloud provider

## Manual notarization smoke (macOS)

- [ ] Pull the .dmg from the GitHub release on a clean macOS user account
- [ ] Open the .dmg; drag the app to Applications
- [ ] No "unidentified developer" Gatekeeper warning
- [ ] App launches; Local Mode modal renders without errors

## Manual signing smoke (Windows)

- [ ] Pull the .msi (or .exe) from the GitHub release on a fresh Windows user
- [ ] SmartScreen shows the publisher (or "More info -> Run anyway" path is acceptable for a self-signed beta)
- [ ] App launches and the Local Mode modal renders

## Manual updater smoke

- [ ] Bump `version` in `tauri.conf.json`, tag a new release
- [ ] Old build (one version behind) shows the `UpdateAvailableModal` after ~30 s
- [ ] "Update now" downloads + installs successfully

## M4.5 vision smoke

- [ ] Anthropic + `claude-haiku-4-5-20251001` (or current Claude vision model): paste a small PNG into the composer, type "What is in this image?", send. Response references image content.
- [ ] OpenRouter + `gpt-4o` (or current vision-capable model): same.
- [ ] Local mistralrs + Qwen3.5-VL-4B (when manifest entry lands - currently M5 carry-over): same.
- [ ] Drag-drop a JPEG onto the chat panel: thumbnail strip shows it; gold ring fades on dragleave.
- [ ] Ctrl+V paste a screenshot from clipboard: stages a thumbnail.
- [ ] Click + on the paperclip: file picker opens; pick a WebP; staging works.
- [ ] Try a 6 MB image: red "Image exceeds 5 MB" toast; nothing staged.
- [ ] Stage 4 images, try a 5th: "Up to 4 images per message" toast.
- [ ] Send a message with images, restart the app, reopen the campaign: chat history rehydrates with images intact (if a `session_id` is wired in - M5 may need to plumb it).
- [ ] Click an image bubble: lightbox opens at full size; Escape closes; backdrop click closes.

## Tag

- [ ] `git tag v0.5.0-m4`
- [ ] `git push <remote> m4-local-mode-and-packaging` (after explicit user authorization)
- [ ] `git push <remote> v0.5.0-m4`
- [ ] Open PR `m4-local-mode-and-packaging -> main`

## M4.5 tag

- [ ] `git tag v0.5.5-m4.5` (lightweight, local-only until M5 close)
- [ ] On the M5 push, include this tag

## M6: Character creation wizard

- [ ] Fresh install, walk through onboarding (2 steps: Welcome + Connect AI)
- [ ] Wizard mounts automatically once onboarding completes
- [ ] Walk: Class -> Race -> Background -> Abilities -> Skills -> Spells (caster path) -> Equipment -> Persona -> Portrait -> Review
- [ ] Surprise me on Class tab fills all 9 other tabs (visible progress)
- [ ] Per-field Generate sparkles on Persona fill name / ideals / bonds / flaws / backstory
- [ ] Live test chat on Review tab streams an NPC reply
- [ ] Begin Adventure transitions to game with the fully-customised PC visible in CharFab
- [ ] Right-click CharFab -> Create new character -> wizard re-opens in edit mode
- [ ] Settings -> Character section -> Re-create character also opens wizard in edit mode
- [ ] Close app mid-wizard, relaunch: draft resumed on the same tab
- [ ] EquipmentTab Package mode: inline wildcard chooser appears for 'any X weapon' options; pick resolves into pc.inventory at Begin Adventure
- [ ] EquipmentTab Package mode: background items + background.starting_gold land in pc.inventory (open CharacterSheet -> Inventory)
- [ ] EquipmentTab Gold mode: residual floor(goldRemaining) gold-pieces row appears in pc.inventory; catalog item icons promoted to canonical keys (sword/bow/shield)
- [ ] Review warning surfaces 'some equipment choices are unresolved' when a wildcard slot has no resolvedItemIds; user can still Begin Adventure (literal-name fallback row)
- [ ] PersonaTab: click sparkle on each of the 6 personality-flag slots. Each generates a short single-sentence flag from the LLM; slot enters custom-mode automatically.
- [ ] Sparkle disabled while another assist is in flight (visible during long generations).
- [ ] Sparkle with no background/race/alignment selected still works (pool is empty; LLM writes a fresh entry).

## M7-DM Provider Registry + Media Local Stack - manual smoke

### Setup
- [ ] Fresh install (no settings.json): launch -> onboarding renders -> Settings opens with v2 shape applied (DEFAULTS_V2)
- [ ] Upgrade install (M5/M6 settings.json on disk): launch -> Settings reflects migrated v2 shape; no "settings reset" toast
- [ ] Corrupt settings.json -> launch -> defaults applied + one-shot toast "Settings could not be read..."

### Chat tab (Provider tab id, label "Chat")
- [ ] Switch provider -> sub-form swaps (Anthropic API key field vs OpenAI-compat base URL+key vs Local mistralrs runtime controls)
- [ ] Save with each provider and confirm POST /settings/v2 returns 200 (Network tab)
- [ ] Toggle Save under license-restricted mode + Quality preset -> 400 with "preset blocked" message

### Image tab
- [ ] 5 preset radio cards visible; Balanced is default selection
- [ ] License-restricted off: all 5 presets selectable
- [ ] License-restricted on: Fast (SAI NC) and Quality (FLUX-dev NC) greyed out; Balanced / Quality-OSS / Cloud remain
- [ ] Disable image generation -> all preset radios disabled

### Video tab
- [ ] Default state: enable checkbox unchecked, mode picker disabled
- [ ] Enable -> 3 mode radios become interactive (prerecorded / live / race)
- [ ] License-restricted on -> entire Video tab is disabled (per spec §8.3)
- [ ] Scene transition with mode=prerecorded -> picks bundled mp4 by scene tag (no backend call)
- [ ] Scene transition with mode=live, LTX-Video model not downloaded -> falls back to bundled mp4 + warning toast

### Behavior tab
- [ ] Renamed from "Model" but existing system prompt + temperature + replicate api key inputs still work
- [ ] license_restricted_mode toggle flips Image/Video gates immediately
- [ ] agent_max_rounds number input clamps 1..32

### Provider Registry API surface
- [ ] GET /providers/catalog returns { chat:[3], image:[5], video:[1] } with curated_models per entry
- [ ] GET /providers/anthropic/caps?model=claude-opus-4-7 -> all true
- [ ] GET /providers/openai-compat/caps?model=o3-mini -> reasoning:true, vision_input:false
- [ ] GET /providers/local-mistralrs/caps?model=qwen3.5-4b -> all true (VL+thinking)
- [ ] GET /providers/unknown-provider/caps -> 404

### Dynamic Discovery
- [ ] POST /providers/discover { provider_id: "anthropic" } -> 3 curated Claude 4 models
- [ ] POST /providers/discover { provider_id: "openai-compat", base_url, api_key } -> /v1/models response normalised with capability inference
- [ ] POST /providers/discover { provider_id: "local-mistralrs", search_query: "qwen vl" } -> HF Hub results with vision-language caps inferred from tags
- [ ] POST /providers/discover { provider_id: "replicate", api_key } -> cursor-paginated model list
- [ ] POST /providers/discover { provider_id: "not-a-provider" } -> 404 UnsupportedProvider
- [ ] Replicate without api_key -> 502

### Agent loop
- [ ] Default agent run includes generate_image tool definition (visible in tool inspector)
- [ ] When AgentConfig.tool_availability.image=false, generate_image is omitted from the tool list
- [ ] Core tools (roll_dice / apply_damage / query_rules) are ALWAYS present regardless of modality flags

### Chat composer vision gate (H.5)
- [ ] settings.visionEnabled=true: paste image + drag-drop image both accepted (stages in ComposerAttachments)
- [ ] settings.visionEnabled=false: paste image -> stagingError shows "Multimodal input is disabled..."; drag-drop also rejected with same toast
- [ ] visionEnabled toggle in Settings does NOT affect text-only chat

### Deferred to M7.5-DM (NOT in scope for this manual smoke)
- Real SDXL-Lightning / Nunchaku FLUX / Z-Image-Turbo / LTX-Video model loaders (sidecar backends raise NotImplementedError on load() today; full GPU smoke needs RTX 3080 + ~20 GB model downloads)
- Frontend useDiscoverProvider hook + ModelSelector "Discovered" + "Search HF" sections + "Add Custom HF repo" modal
- Tool Inspector handled_by_provider pill rendering
- StatusBar Chat/Image/Video modality indicators
