# Release checklist

Copy this checklist into the release issue or PR and record command output or artifact links. A green layer does not imply another layer is green.

## Source and contracts

- [ ] Release commit is reviewed and `CHANGELOG.md` contains the intended version.
- [ ] `bun run architecture:check` passes with zero legacy exceptions.
- [ ] `bun run gates` passes.
- [ ] `python -m ruff check sidecar` passes under Python 3.12.
- [ ] `RUN_GPU_TESTS=0 python -m pytest sidecar/tests -q` passes.
- [ ] Stable HTTP/SSE, SQLite, Stronghold, Tauri command/event, and external binary contracts are unchanged or explicitly migrated.
- [ ] Logs and artifacts contain no prompts, chat bodies, provider keys, vault values, runtime-control tokens, or base64 media.

## User-facing acceptance

- [ ] `bun run e2e` passes against a production frontend build.
- [ ] `bun run e2e:tauri` passes against the real Tauri WebView and backend process.
- [ ] English and Russian production-root labels render correctly.
- [ ] Save/load restores session, messages, PC, combat, and scene atomically.
- [ ] Combat commands do not mutate client mechanics before a newer server revision.
- [ ] Settings persist without exposing provider credentials.

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
- [ ] Tag push and `gh release create` have explicit authorization.
- [ ] The published GitHub Release and updater flow are verified after publication.
