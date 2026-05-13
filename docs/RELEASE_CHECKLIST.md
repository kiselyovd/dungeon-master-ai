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
