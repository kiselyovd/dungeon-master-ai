[← Configuration](CONFIGURATION.md) · [Back to README](../README.md)

# Documentation

Start with [Getting started](GETTING_STARTED.md) for installation and the first adventure, then [Configuration](CONFIGURATION.md) for model servers, optional local runtimes and credentials. Source development is covered in [Contributing](../CONTRIBUTING.md).

The topic guides below explain implemented contracts and their limits. Dated QA sections describe the revision and environment they name; they are not fresh proof for the current release. Fixture/browser checks, real backend checks, Tauri runs, physical LAN tests and real-model gameplay establish different things.

## Play and campaign state

| Guide | Use it for |
| --- | --- |
| [Campaign preparation](visual/CAMPAIGN_PREPARATION.md) | Adventure source, campaign-scoped hero confirmation, interrupted creation and world readiness |
| [Campaign state](CAMPAIGN_STATE.md) | Continue, snapshots, atomic restoration and campaign boundaries |
| [Bell Beneath the Water](AUTHORED_ADVENTURE.md) | Prepared adventure progression, discoveries, consequences and endings |
| [Commands](COMMANDS.md) | Keyboard shortcuts, palette, focus and role restrictions |
| [AI turn recovery](AI_TURN_RECOVERY.md) | Durable operation identity, uncertain delivery and safe recovery |
| [Human command ledger](HUMAN_COMMAND_LEDGER.md) | Human intentions, confirmed effects and receipts |
| [Portable campaigns](PORTABLE_CAMPAIGNS.md) | Owner backups, player dossiers and import validation |

## Combat, rest and party authority

| Guide | Use it for |
| --- | --- |
| [Hero combat binding](HERO_COMBAT_BINDING.md) | Stable character identity across sheets, combat and restoration |
| [Weapon combat](WEAPON_COMBAT.md) | Supported weapon choices and server-confirmed attacks |
| [Spell combat](SPELL_COMBAT.md) | Supported spell choices, targeting and resources |
| [Defense equipment](DEFENSE_EQUIPMENT.md) | Armor, shields, equipped choices and derived defense |
| [Rest resources](REST_RESOURCES.md) | Rest eligibility, time and supported resource recovery |
| [Hero recovery](HERO_RECOVERY.md) | Dying, stabilization, death saves and waiting |
| [Combat recovery](COMBAT_RECOVERY.md) | Encounter completion and recoverable outcomes |
| [Party state](PARTY_STATE.md) | Accepted member documents, ownership and restoration |
| [Party combat](PARTY_COMBAT.md) | Human turn ownership, party effects and group rest |

These guides describe the supported mechanics. They do not claim that every D&D 5e class feature, spell or rule is implemented.

## Cooperative play and desktop behavior

| Guide | Use it for |
| --- | --- |
| [Room invitations](ROOM_INVITATIONS.md) | Hosting, reachable addresses, paste/native links and deliberate joining |
| [Room authority](ROOM_AUTH.md) | Authentication, private/public projections, reconnect and request ordering |
| [Co-op acceptance](COOP_ACCEPTANCE.md) | Recorded two-client native evidence and remaining physical LAN/model checks |
| [Desktop lifecycle](DESKTOP_LIFECYCLE.md) | Main/auxiliary closure, process ownership and uncertain departure |
| [Detached windows](DETACHED_WINDOWS.md) | Read-only hero/journal views and access scope |
| [Workspace preferences](WORKSPACE.md) | Panes, reading size, view modes and native window restoration |
| [Notifications](NOTIFICATIONS.md) | Opt-in delivery, Windows permission and safe click activation |

## Models, media and presentation

| Guide | Use it for |
| --- | --- |
| [Provider startup](PROVIDER_STARTUP.md) | Restoring acknowledged settings before the first DM request |
| [Model readiness](visual/MODEL_READINESS.md) | Installed, running, unknown and ready states |
| [Media references](visual/MEDIA_REFERENCES.md) | Owned media, migration and cold restoration |
| [Hero identities](visual/HERO_IDENTITIES.md) | Stable portraits and miniatures |
| [NPC portraits](visual/NPC_PORTRAITS.md) | Persistent faces and public reveal |
| [Map skins](visual/MAP_SKINS.md) | Artwork bound to canonical tactical geometry |
| [Offline illustrations](visual/OFFLINE_ILLUSTRATIONS.md) | Bundled scenic fallback and full-view behavior |
| [Confirmed motion](visual/CONFIRMED_MOTION.md) | Cues driven by confirmed combat events |
| [Scene transitions](visual/SCENE_TRANSITIONS.md) | Cinematic moments, motion preference and focus |
| [Typography](visual/TYPOGRAPHY.md) | Bundled fonts, reading controls and glyph coverage |
| [Visual performance](visual/PERFORMANCE.md) | Encoded/decoded art budgets and measurement limits |
| [Python sidecar](../sidecar/README.md) | Optional image/video backends and packaging |

## Architecture, release and evidence

| Guide | Use it for |
| --- | --- |
| [Architecture](../ARCHITECTURE.md) | Application/domain boundaries, adapters and composition roots |
| [Release guide](RELEASE.md) | Cloud bundle matrix, optional local artifacts, signatures and publication |
| [Release checklist](RELEASE_CHECKLIST.md) | Independent source, browser, native, packaging and publication checks |
| [Security policy](../SECURITY.md) | Credentials, local data, logs and vulnerability reporting |
| [Browser session acceptance](BROWSER_SESSION_ACCEPTANCE.md) | Recorded browser/backend coverage and its limits |
| [Real-model session acceptance](REAL_MODEL_SESSION_ACCEPTANCE.md) | Dated provider/native gameplay observations and outstanding acceptance |
| [Solo recovery acceptance](SOLO_RECOVERY_ACCEPTANCE.md) | Recorded native recovery/rest continuation |
| [Bonsai evaluation](BONSAI_EVALUATION.md) | Historical candidate measurements and limitations |
| [Qwen 9B evaluation](QWEN9B_EVALUATION.md) | Historical candidate behavior and guarded continuation experiments |

The [changelog](../CHANGELOG.md) describes released user-facing changes. Research and QA reports retain their original scope; they should not be used as installation instructions or as a list of currently available provider models.

## See Also

- [Getting started](GETTING_STARTED.md)
- [Configuration](CONFIGURATION.md)
- [Contributing](../CONTRIBUTING.md)
