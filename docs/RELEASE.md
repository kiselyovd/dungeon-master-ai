[Back to README](../README.md) · [Documentation index](README.md) · [Release checklist](RELEASE_CHECKLIST.md)

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
| `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY` | Optional macOS Developer ID signing |
| `APPLE_API_KEY`, `APPLE_API_ISSUER`, `APPLE_API_KEY_ID` | Complete optional macOS notarization credentials; the key is base64-encoded P8 material |
| `WINDOWS_CERT_BASE64`, `WINDOWS_CERT_PASSWORD` | Optional Windows Authenticode signing |

The configured updater public key must match the existing GitHub signing secret. Preserve that key across updates; replacing it without a migration breaks installed clients. For a new distribution only, generate a key once in an approved toolchain environment and keep the private material outside the repository:

```pwsh
bun tauri signer generate -w "$env:USERPROFILE/.dmai/updater-key"
```

Never commit private keys, vault passphrases, provider credentials, or runtime-control tokens. Updater signatures authenticate package bytes; they do not sign `latest.json` itself or replace platform code signing.

## Tagged release

A pushed `v*` tag triggers `.github/workflows/release.yml`. Before tagging, align `package.json`, the Tauri configuration, native/workspace Cargo manifests and `Cargo.lock`, add the exact version's changelog entry, and pass the protected PR checks on the intended source. For example, tag `v0.13.0` only when the application version is `0.13.0`; do not move an existing published tag.

1. Each matrix runner validates tag/package/Tauri/native versions, current-version notes and the presence of the updater signer.
2. A release-only cloud override enables `bundle.createUpdaterArtifacts`. Ordinary `build-bundle.yml` checks remain secret-free with updater artifacts disabled.
3. The runner builds for its explicit target. Optional Windows signing and macOS signing/notarization happen inside Tauri bundling, before the updater signatures are produced. Partial macOS signing configuration is an error.
4. Platform artifacts are uploaded from `target/<target>/release/bundle/`. The publish job stages stable asset names without changing signed bytes, builds `latest.json`, and extracts only this version's release notes.
5. `scripts/verify-release-signatures.ts` uses `minisign` to verify all three staged updater packages against the public key already bundled with the application. Missing, empty, mismatched or invalid signatures prevent publication.
6. The job uploads installers, updater archives, `.sig` files, `latest.json`, `release-verification.json` and `SHA256SUMS` into a draft release, then publishes it after successful upload.

The macOS updater consumes `.app.tar.gz`, not the installer `.dmg`. Windows uses the signed NSIS/MSI package, and Linux uses the signed AppImage. Each URL in `latest.json` must name an uploaded asset exactly. A valid signature does not establish that a GUI, provider or GPU flow was exercised.

Publishing is a state-changing external operation. Do not push a tag or create a release without explicit authorization.

## Backend startup and existing profiles

The desktop launcher owns backend startup configuration. New installations use a writable backend directory below the application's data directory, separate from the frontend Stronghold snapshot. The backend vault credential is retained in the platform's credential store; an explicit `DMAI_VAULT_PASSPHRASE` remains an operator override and is not automatically copied there.

Existing backend databases and vaults are not silently moved or replaced. An existing vault without its credential fails closed; keep its original passphrase and use the explicit configuration described in [Configuration](CONFIGURATION.md#startup-credentials). A standalone `dmai-server` still requires an operator-supplied credential for persistent release secrets. Do not delete a vault or credential to make an upgrade appear successful.

## Local runtime artifacts

Tauri is the only runtime process owner. A local bundle declares `dmai-server`, `mistralrs-server`, and `dmai-image-sidecar`; the cloud override declares only `dmai-server`.

- `.github/workflows/prebuild-sidecars.yml` builds CPU mistralrs `v0.8.3` for the three supported targets.
- `.github/workflows/prebuild-python-sidecar.yml` builds the Python 3.12 PyInstaller media sidecar for the same targets.
- `scripts/build_mistralrs.sh` and `.ps1` stage `mistralrs-server-<target>`.
- `sidecar/scripts/build.sh` and `.ps1` stage `dmai-image-sidecar-<target>`.

GPU builds remain local because hosted runners do not provide the required GPU toolchain. Before a local release build, verify that all target-suffixed binaries are real executables. Debug placeholders are not release artifacts; non-cloud release builds fail when required binaries are missing.

## Verification layers

Run and report these independently, using container-managed toolchains and the target-OS/native exception rules in [Contributing](../CONTRIBUTING.md):

```bash
bun run architecture:check
bun run gates
bun run build
CI=1 DMAI_E2E_PREVIEW=1 DMAI_E2E_PORT=15165 bun run e2e -- --workers=1
bun run e2e:tauri
bun run tauri:build:cloud
python -m ruff check sidecar
python -m pytest sidecar/tests -q
```

Use a free task-owned preview port. Plain `bun run e2e` uses the development server; the preview flag exercises the production build. Record the native credential-store round trip independently from mock credential tests, real WebView startup, full gameplay and provider acceptance.

The normal three-target bundle workflow runs profile/credential regression tests and the opt-in actual OS-store round trip. Linux uses an isolated D-Bus/Secret Service session; Windows and macOS use their runner's native credential store. These checks occur before the release tag and do not substitute for a clean-install GUI test.

Also run `bun scripts/tauri-cdp-play.ts` only when the real local runtime and models are staged. That live-model flow is non-deterministic and does not replace deterministic browser or Tauri smoke coverage.

## Current signing limitations

- Without a trusted Windows certificate, SmartScreen may show an unrecognized publisher warning.
- macOS PyInstaller dylibs may require an additional deep-sign pass before notarization on the first production local-runtime release.

## Wiki maintenance

The published [Wiki](https://github.com/kiselyovd/dungeon-master-ai/wiki) uses the versioned entry points in `docs/wiki/`. Keep detailed instructions in their canonical repository pages. Publish those Wiki files through its separate Git repository, preserve its existing history, and verify the rendered result after publication. The Wiki Git remote exists only after its first page has been created through GitHub.

## See Also

- [Release checklist](RELEASE_CHECKLIST.md)
- [Configuration](CONFIGURATION.md)
- [Contributing](../CONTRIBUTING.md)
