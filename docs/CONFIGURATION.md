[← Getting started](GETTING_STARTED.md) · [Back to README](../README.md) · [Documentation index →](README.md)

# Configuration

Open **Settings** from the title bar or command palette. English tabs are Chat, Model on this computer, Image, Video and Behavior. Russian labels are localized equivalents; API identifiers remain unchanged.

## Model server

RouterAI, OpenRouter and local OpenAI-compatible servers use the same **Chat → Provider → Model server** option. They are not separate native provider types.

| Setting | Value |
| --- | --- |
| Base URL | The provider's API base, including its version path, without `/chat/completions` |
| API key | A key for that provider/account, entered only in Settings |
| Model | An exact model ID available in that provider's current catalog |

Verified API-base examples:

| Service | Base URL | Provider documentation |
| --- | --- | --- |
| RouterAI | `https://routerai.ru/api/v1` | [Official quickstart](https://routerai.ru/docs/guides/overview/quickstart) |
| OpenRouter | `https://openrouter.ai/api/v1` | [Official quickstart](https://openrouter.ai/docs/quickstart) |
| An existing LM Studio server | `http://localhost:1234/v1` | Use the address and model ID shown by your own server. |

The application passes the configured base to its OpenAI-compatible adapter. For the hosted examples, chat requests target `/api/v1/chat/completions`; entering that full completion URL as the base would duplicate the endpoint suffix.

Enter the base and key, request the model catalog using the explicit refresh/discovery control, and select an available model or enter its exact ID manually. A cached or curated model list is not proof of a live provider connection. Save the settings and verify an actual completed application turn.

Model names, availability, capabilities and prices change independently of this application. Use the provider's current catalog and account limits; an old QA model name or price estimate is not a recommendation or a guarantee of access. The DM requires compatible tool/structured-output behavior for game actions. Vision, reasoning options and long contexts depend on the selected model and endpoint.

Cloud turns send the configured campaign context and any enabled attachments to the selected provider. Review its privacy and billing terms. Guests use the organizer's shared DM and do not receive the organizer's API key.

## Model on this computer

The standard cloud bundle does not ship `mistralrs-server`, the Python media executable or model weights. This option requires separately staged matching runtime artifacts and installed models. See the [release guide](RELEASE.md) and [sidecar guide](../sidecar/README.md).

On **Model on this computer**, choose an installed model, use **Check status**, and start the selected model. The default local selection in source is Gemma 4 E2B; selection does not imply that its runtime or weights are installed. Advanced controls include the context limit and graphics-memory strategy. Start/restart the model after changing runtime configuration, then save Chat settings with that provider selected.

Distinguish an installed model, a running process, a ready runtime and a successfully completed DM turn. Unknown or failed status is not ready. Local GPU memory, driver support and simultaneous media generation need separate checks on the actual machine.

An independently managed local server can instead use **Model server** with its own API base and model ID. The user manages that server's lifetime; Tauri owns only the embedded child processes it starts.

## Optional media and presentation

- **Image:** enable generation and select a supported local or cloud backend. Local image models require the optional media runtime and matching weights; cloud media may require its own credential and fees.
- **Video:** choose the prerecorded library, live generation or a live/fallback mode. Live generation needs its separate backend and resources.
- **Behavior:** configure narration/UI language, DM instructions and cinematic moments. Cinematic presentation does not create a mechanical result or make an unavailable media backend ready.

Media setup is optional for campaign preparation, browsing and drafts. The table also uses bundled artwork; see [offline illustrations](visual/OFFLINE_ILLUSTRATIONS.md). Notification permissions and workspace layout have their own controls and guides.

## Startup credentials

An ordinary clean desktop launch automatically creates its backend profile at `${appDataDir}/backend/`, containing `data.db` and `dmai-vault.hold`. The native application creates the vault password in Windows Credential Manager, macOS Keychain or Linux Secret Service. That store must be available and unlocked. Failure stops startup; the desktop does not fall back to plaintext credentials or an in-memory secrets store.

A non-empty explicit `DMAI_VAULT_PASSPHRASE` takes priority and is not saved into the OS credential store. Use the existing vault's password for that operator override; it is separate from a provider API key or room invitation. A standalone production `dmai-server` still requires its operator to provide the credential through the process environment. Never put it in source, command history, screenshots or bug reports.

Existing legacy databases and backend vaults are retained in place, not moved or replaced. Preserve the backend vault and its matching OS credential together during upgrades. The frontend's `${appDataDir}/dmai-vault.hold` is a separate vault and is not overwritten by backend setup.

| Startup error | Action |
| --- | --- |
| `backend_credential_store_unavailable` | Make the OS credential store available/unlocked and retry. |
| `backend_vault_unlock_required` | An existing backend vault has no matching stored credential. Restore that credential or provide its original password through the explicit environment override; do not delete or replace the vault. |
| `backend_profile_ambiguous_set_database_url` | Both legacy and managed profiles exist. Select the intended existing database explicitly with `DATABASE_URL`. |
| `backend_vault_ambiguous_set_vault_path` | A database URI resolves to ambiguous existing vault locations. Select the intended vault explicitly with `DMAI_VAULT_PATH`. |

These are the implemented startup rules. Fresh packaged-GUI and platform credential-store acceptance must be recorded for the release; this guide does not claim those checks have already passed.

Provider keys stay in Stronghold-backed storage. UI preferences, workspace layout and campaign preparation records are separate from credentials. Campaign data stays in local SQLite; supported campaign exports are distinct from copying a live database or sharing a vault. See the [security policy](../SECURITY.md).

## Troubleshooting configuration

After a rejected Save, the form retains its draft and reports the failure. Cancel discards the form draft. A backend restart requires the saved provider configuration to be acknowledged before the first AI turn; see [provider startup](PROVIDER_STARTUP.md).

If a provider changes, becomes unavailable or rejects a recovery request, keep the original pending operation and follow the application's explicit recovery controls. Do not assume a successful catalog lookup, a listening port or a screenshot proves a completed game action.

## See Also

- [Getting started](GETTING_STARTED.md)
- [Provider startup and restart](PROVIDER_STARTUP.md)
- [Security and data handling](../SECURITY.md)
