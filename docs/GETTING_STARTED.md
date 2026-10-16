[Back to README](../README.md) · [Configuration →](CONFIGURATION.md)

# Getting started

Dungeon Master AI is an experimental desktop application for solo D&D 5e and host-owned cooperative sessions. A prepared adventure, a custom opening scene, character creation, saves and recovery share the same authoritative backend. Some rules and real-device acceptance remain incomplete; see the release notes for the exact build you install.

## Install a release

1. Download the matching Windows x64, macOS Apple Silicon or Linux x64 artifact from [GitHub Releases](https://github.com/kiselyovd/dungeon-master-ai/releases). Read that release's signing and platform notes before opening it.
2. Use the standard cloud flavor for a model served by RouterAI, OpenRouter or another OpenAI-compatible endpoint. It includes the desktop shell and game backend. Embedded model/media executables and model weights are separate optional artifacts.
3. Keep your operating system's credential store available and unlocked. An ordinary clean desktop launch creates its backend profile and stores the vault password there automatically; no user-supplied environment password is required. See [startup credentials](CONFIGURATION.md#startup-credentials) for existing profiles and operator overrides.
4. Start the application. Choose English or Russian in the header. When upgrading, preserve the existing database, backend vault and matching OS credential together, as well as the separate frontend vault. Use the application's supported export/backup controls before changing a profile.

Building from source is covered in [Contributing](../CONTRIBUTING.md). Installing the application does not require Bun, Cargo or Python.

## Start a solo adventure

1. On first launch, choose **Start adventure**. Existing profiles open their campaign library; choose **New campaign** to start another story.
2. Choose the prepared **Bell Beneath the Water** adventure, or provide a title and opening description for a custom campaign. The custom route starts from that opening; it does not generate a complete authored quest in advance.
3. Choose **Choose hero**, select a prepared class and name, or open **Create your own hero** for the full wizard. Confirm with **Enter first scene**. A cancelled custom wizard retains its campaign-scoped draft.
4. Open **Settings**, then configure a **Model server** on the **Chat** tab. Use the [provider setup](CONFIGURATION.md#model-server) for RouterAI, OpenRouter or an existing local server. You can prepare a campaign before configuring a model; an attempted AI turn opens setup and retains your unsent draft when configuration is missing.
5. Send a short action in the composer. A completed visible reply and a confirmed game result establish that turn's outcome. A running model process or a successful model-list lookup alone does not establish gameplay readiness.

The table offers Split, Table and Chat views. Open Adventure for Scene, Hero, Journal and NPC context. Combat buttons send requests to the server; wait for the confirmed result before assuming movement, damage or a spent action occurred.

## Save and continue

Use **Saves** or quick save with **Ctrl+S** on Windows/Linux and **Meta+S** on macOS. Continue a campaign from the library after restarting. Loading a snapshot restores the associated campaign projection; the UI validates the response before replacing the current state.

If a turn or recovery action is interrupted, use its existing recovery/retry control. An uncertain reply can already have committed on the server. Retrying the preserved request keeps its identity; submitting the same action again as a new turn can create another action. See [campaign state](CAMPAIGN_STATE.md) and [AI turn recovery](AI_TURN_RECOVERY.md).

Local save commands are unavailable inside an active room, including on the host. Full campaign backups and player dossiers have separate permissions and purposes; see [portable campaigns](PORTABLE_CAMPAIGNS.md).

## Join or host a game

The organizer owns campaign storage and runs the shared AI Dungeon Master. A guest can choose **Join game** without setting up a local AI provider or solo hero.

**Organizer:** create or continue a campaign with a saved hero, open **Co-op session**, select the campaign and choose **Create a room**. In the Room panel, create an invitation using the numeric LAN/VPN address and port that the other computer can reach. `localhost`, `127.0.0.1`, `0.0.0.0` and `[::]` are not addresses to send to another computer. Share invitations privately and refresh them after use or expiry.

**Guest:** paste the invitation, choose **Check link**, review its destination, enter your player name and explicitly join. Prepare your own hero and wait for organizer approval where requested. If a join response is lost, use the saved connection/recovery option instead of creating another participant. Leave an existing room deliberately before changing destinations.

Use the same LAN or an existing VPN and allow the selected room listener through the applicable firewall. Public matchmaking and automatic Internet relay are not provided. The private desktop backend and provider credentials must not be exposed to guests. The [room invitation guide](ROOM_INVITATIONS.md) explains manual entry and native link opening; [co-op acceptance](COOP_ACCEPTANCE.md) distinguishes local two-client evidence from physical-device LAN/VPN verification.

## If setup does not complete

| Symptom | Next action |
| --- | --- |
| Startup does not reach the campaign screen | Check the release's platform requirements and that the OS credential store is available/unlocked. Preserve existing database/vault files and credentials; use the [startup error guidance](CONFIGURATION.md#startup-credentials) rather than resetting them. |
| Sending opens model settings | Complete Model server setup, or install/start the optional embedded runtime. Your draft remains available. |
| Models are listed but turns fail | Verify the selected model supports the application's tool/structured-output requests and is available to your account. Check the provider's quota and error, then retry the preserved operation when offered. |
| Embedded model controls report missing or unknown status | A cloud-only bundle does not include that runtime. Check the optional artifact, model installation and actual status before selecting it. |
| Guest cannot connect | Confirm the organizer's reachable IP/port, room state, invitation expiry and firewall/VPN path. Recover an uncertain existing join before starting a fresh one. |

## See Also

- [Configuration](CONFIGURATION.md)
- [Commands and shortcuts](COMMANDS.md)
- [Documentation index](README.md)
