[Release guide](RELEASE.md) · [Back to README](../README.md) · [Documentation index](README.md)

# Release checklist

Copy this checklist into the release issue or PR and record command output or artifact links. A green layer does not imply another layer is green.

## Source and contracts

- [ ] Release commit is reviewed and `CHANGELOG.md` contains the intended version.
- [ ] Package, Tauri, native/workspace versions and the lockfile agree with the release tag.
- [ ] `bun run architecture:check` passes with zero legacy exceptions.
- [ ] `bun run gates` passes.
- [ ] `python -m ruff check sidecar` passes under Python 3.12.
- [ ] `RUN_GPU_TESTS=0 python -m pytest sidecar/tests -q` passes.
- [ ] Stable HTTP/SSE, SQLite, Stronghold, Tauri command/event, and external binary contracts are unchanged or explicitly migrated.
- [ ] Logs and artifacts contain no prompts, chat bodies, provider keys, vault values, runtime-control tokens, or base64 media.

## User-facing acceptance

- [ ] `bun run build`, then `CI=1 DMAI_E2E_PREVIEW=1 bun run e2e`, pass against the production frontend build using a free task-owned port.
- [ ] `bun run e2e:tauri` passes against the real Tauri WebView and backend process.
- [ ] English and Russian production-root labels render correctly.
- [ ] Save/load restores session, messages, PC, combat, and scene atomically.
- [ ] Combat commands do not mutate client mechanics before a newer server revision.
- [ ] Settings persist without exposing provider credentials.
- [ ] Clean desktop startup obtains a persistent backend credential from the actual OS store and uses a writable profile; restart reuses it.
- [ ] Explicit env overrides, legacy profiles, missing/locked credentials, ambiguous paths and wrong passwords preserve existing vault/database bytes and fail explicitly.
- [ ] Mock credential tests and the opt-in `real_os_credential_store_round_trip` are recorded separately; neither alone establishes a full desktop first-run flow.
- [ ] Full backups and player dossiers enforce their distinct roles; import preview/cancel, failed writes and retry receipts preserve existing campaigns.
- [ ] Actual native open/save dialogs handle Cyrillic paths, corrupt/large files, denied writes and existing destinations.
- [ ] Detached hero/journal production roots follow own actor, room/campaign and access scope; guest views exclude another player's private biography.
- [ ] Detached windows cannot invoke game, filesystem, store or Stronghold commands or fetch the private backend directly.
- [ ] Main exit closes every owned view/process; auxiliary close preserves the game; cold window restore covers a second/missing monitor and actual 125/150/200-percent scale.
- [ ] Encoded art/fonts, decoded image footprint, cold startup and 20-actor rendering are measured separately from real GPU/model contention.

## Cloud bundle matrix

- [ ] `bun run tauri:build:cloud` passes locally for the host target.
- [ ] `build-bundle.yml` is green for Windows x64.
- [ ] `build-bundle.yml` is green for macOS arm64.
- [ ] `build-bundle.yml` is green for Linux x64.
- [ ] Cloud artifacts contain `dmai-server` and do not require local model/media binaries.

## Optional local runtime

- [ ] The matching mistralrs `v0.8.3` binary is staged with its target suffix.
- [ ] The matching Python 3.12 media binary is staged with its target suffix.
- [ ] Tauri starts, probes, observes, and stops all three child processes.
- [ ] `bun scripts/tauri-cdp-play.ts` completes with the staged model/runtime.
- [ ] Image/video concurrency is serialized and the UI reports degraded/cancelled outcomes honestly.
- [ ] GPU-specific tests pass with `RUN_GPU_TESTS=1` on the supported machine.

## Signing and publication

- [ ] Updater public key matches `TAURI_SIGNING_PRIVATE_KEY`.
- [ ] Windows signing is verified, or the unsigned/self-signed limitation is documented.
- [ ] macOS notarization and Gatekeeper launch are verified, or the limitation is documented.
- [ ] `latest.json` references the uploaded bundles and signatures.
- [ ] All three updater signatures verify against the application's unchanged public key; the macOS `.app.tar.gz` is present.
- [ ] `release-verification.json` and `SHA256SUMS` correspond to the published assets.
- [ ] The release stays a draft until all intended files have uploaded successfully.
- [ ] Tag push and `gh release create` have explicit authorization.
- [ ] The published GitHub Release and updater flow are verified after publication.

## Evidence scope

Record the source/build identifier and actual environment for each result. Do not check a box from a historical QA report or a fixture run that exercises a different boundary. Document outstanding physical two-device LAN/VPN, local model/GPU, full real-model and manual OS acceptance explicitly; record a user waiver rather than treating it as a pass.

## See Also

- [Release guide](RELEASE.md)
- [Co-op acceptance](COOP_ACCEPTANCE.md)
- [Real-model acceptance](REAL_MODEL_SESSION_ACCEPTANCE.md)
