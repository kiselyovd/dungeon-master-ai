# Dungeon Master AI

Dungeon Master AI is an experimental desktop application for solo D&D 5e and host-owned cooperative sessions. It combines an AI Dungeon Master, a virtual tabletop, persistent campaigns and server-confirmed game rules.

## Start here

| Topic | Canonical guide |
| --- | --- |
| Overview and installation | [README](https://github.com/kiselyovd/dungeon-master-ai/blob/main/README.md) |
| First adventure and joining a room | [Getting started](https://github.com/kiselyovd/dungeon-master-ai/blob/main/docs/GETTING_STARTED.md) |
| Providers, credentials and optional runtimes | [Configuration](https://github.com/kiselyovd/dungeon-master-ai/blob/main/docs/CONFIGURATION.md) |
| Downloads and version notes | [GitHub Releases](https://github.com/kiselyovd/dungeon-master-ai/releases) |
| All documentation | [Documentation index](https://github.com/kiselyovd/dungeon-master-ai/blob/main/docs/README.md) |
| Architecture and process ownership | [Architecture](https://github.com/kiselyovd/dungeon-master-ai/blob/main/ARCHITECTURE.md) |
| Development and checks | [Contributing](https://github.com/kiselyovd/dungeon-master-ai/blob/main/CONTRIBUTING.md) |
| Packaging and publication | [Release guide](https://github.com/kiselyovd/dungeon-master-ai/blob/main/docs/RELEASE.md) |

## Play and recover

- [Campaign state and saves](https://github.com/kiselyovd/dungeon-master-ai/blob/main/docs/CAMPAIGN_STATE.md)
- [Bell Beneath the Water adventure](https://github.com/kiselyovd/dungeon-master-ai/blob/main/docs/AUTHORED_ADVENTURE.md)
- [AI turn recovery](https://github.com/kiselyovd/dungeon-master-ai/blob/main/docs/AI_TURN_RECOVERY.md)
- [Room invitations](https://github.com/kiselyovd/dungeon-master-ai/blob/main/docs/ROOM_INVITATIONS.md)
- [Co-op acceptance and remaining checks](https://github.com/kiselyovd/dungeon-master-ai/blob/main/docs/COOP_ACCEPTANCE.md)
- [Portable campaigns](https://github.com/kiselyovd/dungeon-master-ai/blob/main/docs/PORTABLE_CAMPAIGNS.md)

## Release scope

The standard cloud flavor contains the desktop shell and game backend. Embedded model/media executables and model weights are separate. A cloud build does not establish real-model, GPU or physical two-device acceptance. Check the exact release notes before installing.

Provider keys remain local in Stronghold-backed flows. Keep existing application data and vault credentials when upgrading. Never include keys, vault files, live databases or private invitations in an issue.

The repository documents are the source of truth. Wiki navigation is maintained from the versioned files in [docs/wiki](https://github.com/kiselyovd/dungeon-master-ai/tree/main/docs/wiki).
