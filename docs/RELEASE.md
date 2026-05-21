# Release pipeline

Tag a `v*` ref and the `.github/workflows/release.yml` workflow takes over: it
matrix-builds the four supported targets, signs Windows binaries, notarizes the
macOS .app, generates `latest.json`, and publishes a GitHub Release.

## Required secrets

Set these in `Settings -> Secrets and variables -> Actions`:

| Secret | Source | Notes |
|---|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | `tauri signer generate` | Base64 of the `.key` file. See `tauri.conf.json -> plugins.updater.pubkey` for the matching public key. |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | passphrase you typed when generating the key | |
| `APPLE_API_KEY` | App Store Connect API key | `.p8` contents, base64-encoded |
| `APPLE_API_ISSUER` | App Store Connect API key | issuer UUID |
| `APPLE_API_KEY_ID` | App Store Connect API key | 10-character key ID |
| `APPLE_TEAM_ID` | Apple Developer | 10-character team ID |
| `WINDOWS_CERT_BASE64` | code-signing cert | base64 of the `.pfx` (EV cert preferred; self-signed works for early betas) |
| `WINDOWS_CERT_PASSWORD` | code-signing cert | `.pfx` password |

## First-time keypair generation (run locally, ONCE)

```pwsh
bun tauri signer generate -w "$env:USERPROFILE/.dmai/updater-key"
```

The command prints the public key. Paste it into `src-tauri/tauri.conf.json`
under `plugins.updater.pubkey`. The private key stays on disk and is uploaded
to the GitHub repo as the `TAURI_SIGNING_PRIVATE_KEY` secret (base64-encoded).

**Never commit the private key.**

## What the workflow does (cloud-only first-GA path)

1. Matrix-builds for `x86_64-pc-windows-msvc`, `aarch64-apple-darwin`,
   `x86_64-apple-darwin`, and `x86_64-unknown-linux-gnu`.
2. Runs `bun run tauri:build:cloud` (cloud-only flavor) so the bundle
   contains the Tauri shell + the dmai-server backend + the auto-updater
   plugin signature.
3. Optional: signs Windows binaries via `signtool` if `WINDOWS_CERT_BASE64`
   is set; otherwise skips with no error.
4. Optional: notarizes the macOS .app via `notarytool` if `APPLE_API_KEY`
   is set; otherwise skips.
5. The `publish` job runs `scripts/build_latest_json.ts` to produce the
   updater manifest, then `gh release create` uploads all artifacts plus
   `latest.json`.

## Local Mode: mistralrs sidecar (built from source) + Python SDXL sidecar (deferred)

Upstream `EricLBuehler/mistral.rs` ships source-only releases (no prebuilt
server binaries), so `mistralrs-server` is built from source:

- `scripts/build_mistralrs.{sh,ps1}` clone `EricLBuehler/mistral.rs` at the
  pinned `MISTRALRS_TAG` and `cargo build --release` the `mistralrs-server`
  crate, staging the binary under `src-tauri/binaries/`. CPU-only by default;
  pass `--cuda` (bash) or `-Cuda` (PowerShell) for a GPU build on a machine
  with the CUDA toolkit installed.
- The `prebuild-sidecars.yml` workflow (manual `workflow_dispatch`) runs the
  CPU-only build for all four targets and uploads the binaries as artifacts.
  A CUDA CI build is intentionally out of scope - GitHub-hosted runners have
  no GPU; GPU users run the script locally with `--cuda`.
- For local `cargo test` / `tauri dev` without a staged binary, `build.rs`
  lays down an empty placeholder so the externalBin check passes. A release
  `tauri build` with no real binary now emits a loud `cargo:warning` instead
  of silently shipping a non-functional Local Mode.

The Python SDXL image sidecar is still deferred: it pulls torch + diffusers
(~5 GB) and PyInstaller bundling would balloon CI time. It returns in Batch C
of M11.

## Open issue: Windows EV certificate provisioning

Spec section 10 #8 is unresolved: until an EV cert lands in the secret
`WINDOWS_CERT_BASE64`, the workflow uses whatever (self-signed) cert is
provided. Self-signed installers trigger SmartScreen "Unrecognized app"
warnings; an EV cert removes them.

## Open issue: macOS notarization re-signing of CPython _internal/

PyInstaller bundles include CPython dylibs under `_internal/` that
notarization wants codesigned. The current workflow does not re-sign each
dylib individually; on the first real release run we may need to add a
`codesign --force --deep --options runtime --timestamp ...` pass before
`notarytool submit`. Track in spec section 10 #3.
