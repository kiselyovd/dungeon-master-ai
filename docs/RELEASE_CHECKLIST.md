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

## Tag

- [ ] `git tag v0.5.0-m4`
- [ ] `git push <remote> m4-local-mode-and-packaging` (after explicit user authorization)
- [ ] `git push <remote> v0.5.0-m4`
- [ ] Open PR `m4-local-mode-and-packaging -> main`
