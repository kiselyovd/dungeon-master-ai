[Previous: Workspace preferences](WORKSPACE.md) · [Back to README](../README.md)

# Portable campaign contract

The server implements two versioned formats. Native file-picking controls are
the next implementation step; this contract does not yet claim installer or
native file persistence acceptance.

| Format | Authority | Contents | Restorable |
| --- | --- | --- | --- |
| `full_campaign_backup` | Local solo/host storage owner | Canonical current state, private seed/adventure state, accepted party documents, confirmed history and required media | Into a new campaign |
| `player_dossier` | Authenticated current actor | Own sheet/biography, published history and discovered facts | No |

Backup/import HTTP routes are composed only behind the native owner's private
token, independently of the player router. A `role` field in request JSON cannot
authorize an export. The room dossier route derives the actor from the current
authenticated member and rechecks access before returning data. Revoked members
are denied. No other player's biography, campaign seed, room credential or
private system/tool message appears in a player dossier.

Exports use a consistent read transaction and never create a save or advance
the source revision. System/tool/intermediate private messages and infrastructure
metadata are excluded. Confirmed image attachments become typed media members;
they are not silently discarded. Inline portraits are copied into the bundle
without modifying the source. Remote legacy images require an owned/local
reference; export/import never fetches an external URL.

The UTF-8 JSON envelope carries schema/ruleset/source identity, a canonical
SHA-256 digest of payload and media declarations, and base64 opaque image
members. Media use fixed `media/<uuid>.<extension>` names. Paths, links and
executable members cannot be extracted or interpreted. Supported image formats
are PNG, JPEG and WebP. Full decoding is required in addition to checksums and
container inspection. Bundled asset IDs/versions are checked against the current
asset manifest.

Limits apply before writes: file64MiB, metadata8MiB, media64 entries/32MiB,
expanded64M pixels, existing per-image size/dimension limits, and decoded output
buffer96MiB. Decoder-internal allocation limits are additionally best-effort;
they are not a hard process-memory guarantee. Compression is not used.

Import validates the complete envelope/media and creates fresh campaign,
session, actor/member, encounter, scene, journal/NPC and media identifiers. Only
structured identity fields are remapped; prose is preserved. All rows are staged
inside one SQLite transaction. Any error rolls back the new campaign and leaves
existing campaigns unchanged. Combat scope/session revisions remain aligned;
HP, positions, geometry, spent actions/resources and authored time are retained.

An import operation UUID and digest identify a durable receipt. Retrying the same
operation after losing its response returns the same imported campaign and ID
mapping. Reusing that UUID with different data is a conflict.

Local verification includes ten real SQLite/HTTP integration cases: source
immutability, authored shrine/Leon actions and900s time, party/privacy and revoked
guest access, owner-token/body-role checks, images, combat scope/geometry/budgets,
wrong versions/checksums/identities/missing media, forced write rollback,
idempotence, image history and unstarted campaigns. Ten application contract
tests and three actual decoder tests pass. Native picker/write/cancel workflows
and final grouped packaging checks remain separate acceptance items.

## See also

- [Room invitations](ROOM_INVITATIONS.md)
- [Desktop lifecycle](DESKTOP_LIFECYCLE.md)
- [Media ownership and migration](visual/MEDIA_REFERENCES.md)
