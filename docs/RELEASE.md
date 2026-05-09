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

## Local Mode (mistralrs sidecar + Python SDXL sidecar) - deferred

Upstream `EricLBuehler/mistral.rs` ships source-only releases as of
v0.8.0 (no prebuilt server binaries). The Python SDXL sidecar pulls
torch + diffusers (~5 GB) and bundling via PyInstaller would balloon CI
time on every push. Both come back in a future point release once a
self-built binary plan or a Docker-based bundling approach lands.

Current behavior:
- `scripts/download_mistralrs.{sh,ps1}` tolerate the upstream 404 and
  leave the placeholder file `build.rs` creates so `tauri build`
  resolves the externalBin entry. The bundled placeholder is empty;
  flipping the app into Local Mode at runtime is a no-op until a real
  binary lands here.
- The `prebuild-sidecars.yml` workflow's automatic push trigger is
  commented out for the same reason.

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
