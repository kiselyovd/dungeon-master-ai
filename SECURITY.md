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

- API keys and provider credentials are stored in an encrypted Stronghold vault (`tauri-plugin-stronghold` on the frontend and `iota_stronghold` on the backend), not in plaintext config. The vault lives under the OS-native app data directory.
- Campaign data (campaigns, sessions, messages, snapshots, combat, journal, NPC memory) is stored locally in a SQLite database via `sqlx`.
- The app talks to whatever LLM / image endpoint you configure. In cloud mode your prompts and any attached images are sent to that third-party provider; review that provider's privacy terms.

Never commit secrets to the repository. The release signing keys and CI secrets are documented in `docs/RELEASE.md` and must stay out of version control.
