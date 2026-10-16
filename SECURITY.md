# Security Policy

## Supported versions

This is a pre-1.0 project. Security fixes are made against the latest release and the `main` branch only. Older milestone tags are not patched; please update to the latest release before reporting.

| Version | Supported |
| --- | --- |
| Latest release (`main`) | Yes |
| Older milestone tags | No |

## Reporting a vulnerability

Please report security issues privately through GitHub, not in a public issue.

1. Go to the repository on GitHub: https://github.com/kiselyovd/dungeon-master-ai
2. Open the **Security** tab and choose **Report a vulnerability** (GitHub private vulnerability reporting / Security Advisories).
3. Include a clear description, reproduction steps, affected version or commit, and the impact you observed.

Do not include secrets or personal API keys in your report.

## What to expect

- Acknowledgement of your report within a few days.
- An initial assessment and, where applicable, a coordinated fix and disclosure timeline.
- Credit in the advisory once a fix ships, if you would like it.

Because this is a hobby / portfolio project, response times are best-effort rather than contractual.

## Secrets and data handling

- API keys and provider credentials use encrypted Stronghold snapshots (`tauri-plugin-stronghold` on the frontend and `iota_stronghold` on the backend), not plaintext config. New desktop backend profiles live under the OS-native application data directory in `backend/`, separate from the frontend snapshot. Existing installation-relative backend profiles and explicit path overrides are preserved.
- Campaign data (campaigns, sessions, messages, snapshots, combat, journal, NPC memory) is stored locally in a SQLite database via `sqlx`.
- The app talks to whatever LLM / image endpoint you configure. In cloud mode your prompts and any attached images are sent to that third-party provider; review that provider's privacy terms.
- Tauri and `dmai-server` communicate with a random per-launch bearer token over a loopback-only runtime-control service. The token is replay-protected, never persisted, and must not appear in logs.
- The desktop launcher creates and reads back a random backend vault unlock credential in Windows Credential Manager, macOS Keychain or Linux Secret Service. It passes this value only to the backend child environment. Missing access to the OS store fails startup explicitly; there is no plaintext or temporary-password fallback.
- An explicit `DMAI_VAULT_PASSPHRASE` remains the supported override for legacy snapshots and standalone production backend runs. An existing snapshot without a matching OS credential requires its original passphrase. Startup does not replace, reset or delete that snapshot. See [Configuration](docs/CONFIGURATION.md) before changing profile paths.

Never commit secrets to the repository. Do not log prompts, chat bodies, provider keys, Stronghold values or passphrases, runtime-control tokens, base64 media, or full third-party payloads. The release signing keys and CI secrets are documented in `docs/RELEASE.md` and must stay out of version control.
