# Release pipeline

This guide describes the current cloud-first release path and the separate local-runtime artifacts. See [RELEASE_CHECKLIST.md](RELEASE_CHECKLIST.md) for the operator checklist and [../ARCHITECTURE.md](../ARCHITECTURE.md) for runtime ownership.

## Supported CI matrix

Normal cloud bundle and tagged release workflows build three targets:

- Windows x64: `x86_64-pc-windows-msvc`
- macOS Apple Silicon: `aarch64-apple-darwin`
- Linux x64: `x86_64-unknown-linux-gnu`

Intel macOS is not in the current matrix. The cloud flavor packages the Tauri shell and `dmai-server` only. Local model/media binaries are produced by manual prebuild workflows and are not implied by a green cloud bundle.

## Required release secrets

| Secret | Purpose |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | Tauri updater signature key |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Updater key passphrase |
| `APPLE_API_KEY`, `APPLE_API_ISSUER`, `APPLE_API_KEY_ID`, `APPLE_TEAM_ID` | Optional macOS notarization |
| `WINDOWS_CERT_BASE64`, `WINDOWS_CERT_PASSWORD` | Optional Windows Authenticode signing |

Generate the updater key once and keep the private material outside the repository:

```pwsh
bun tauri signer generate -w "$env:USERPROFILE/.dmai/updater-key"
```

Never commit private keys, vault passphrases, provider credentials, or runtime-control tokens.

## Tagged release

A pushed `v*` tag triggers `.github/workflows/release.yml`:

1. Each matrix runner executes `bun run tauri:build:cloud`.
2. Windows signing and macOS notarization run only when their secrets are present.
3. Bundles are uploaded as workflow artifacts.
4. The publish job builds `latest.json` and uses `gh release create` to publish the GitHub Release.

Publishing is a state-changing external operation. Do not push a tag or create a release without explicit authorization.

## Local runtime artifacts

Tauri is the only runtime process owner. A local bundle declares `dmai-server`, `mistralrs-server`, and `dmai-image-sidecar`; the cloud override declares only `dmai-server`.

- `.github/workflows/prebuild-sidecars.yml` builds CPU mistralrs `v0.8.3` for the three supported targets.
- `.github/workflows/prebuild-python-sidecar.yml` builds the Python 3.12 PyInstaller media sidecar for the same targets.
- `scripts/build_mistralrs.sh` and `.ps1` stage `mistralrs-server-<target>`.
- `sidecar/scripts/build.sh` and `.ps1` stage `dmai-image-sidecar-<target>`.

GPU builds remain local because hosted runners do not provide the required GPU toolchain. Before a local release build, verify that all target-suffixed binaries are real executables. Debug placeholders are not release artifacts; non-cloud release builds fail when required binaries are missing.

## Verification layers

Run and report these independently:

```bash
bun run architecture:check
bun run gates
bun run e2e
bun run e2e:tauri
bun run tauri:build:cloud
python -m ruff check sidecar
python -m pytest sidecar/tests -q
```

Also run `bun scripts/tauri-cdp-play.ts` only when the real local runtime and models are staged. That live-model flow is non-deterministic and does not replace deterministic browser or Tauri smoke coverage.

## Current signing limitations

- Without a trusted Windows certificate, SmartScreen may show an unrecognized publisher warning.
- macOS PyInstaller dylibs may require an additional deep-sign pass before notarization on the first production local-runtime release.
