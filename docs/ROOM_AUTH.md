# Room capability foundation

The application layer now provides persistent room creation, invitations, member
authentication, revocation and closure. This foundation is not yet connected to
the production listener, native host/join controls or multiplayer gameplay. A
dedicated HTTP router is verified against a real loopback listener in tests; it
does not establish a playable LAN session by itself.

A room belongs to one campaign/session and supports the existing four-member
party limit. Member identities are UUIDs, independent of display names. A host
can issue an invitation for a saved player, including a player in a full party;
joining replaces that player's room credential while preserving their character
ownership. Recreating a closed room preserves party identities.

Invitations expire after ten minutes and are consumed once. An invitation grants
entry, while a separate member capability authenticates later requests. Retrying
entry also requires the client's private resume proof; a public join UUID cannot
recover another player's credential. Revocation, closure and credential
replacement invalidate earlier authority leases. Administrative repository
mutations recheck the lease inside their transaction.

SQLite migration 0009 stores room metadata, credential references and operation
receipts. Capability values, invitation values and resume proofs use the existing
SecretsStore port. They are excluded from public room projections and campaign
snapshots. The actual SQLite/Stronghold reopen test verifies credential recovery.
Room secrets deliberately implement neither Debug nor Serialize; a future
transport must explicitly handle their delivery without logging them.

All service instances share a process-wide mutation coordinator. Production
composition must also retain the existing exclusive database/process authority;
this coordinator does not synchronize independent host processes. Room creation
is an internal local-owner entry point, not an unauthenticated network command.
Provider configuration remains outside room-host authority.

Focused verification:

```powershell
cargo test -p app-server --test room_capabilities
```

Coverage includes private-proof replay, two simultaneous service instances,
saved four-player recovery across new rooms, expiration, revocation, closure,
rejection before secret writes, public metadata privacy and real vault/database
reopening. Production HTTP composition, party combat, event ordering and
native/LAN acceptance remain separate pending work in the co-op plan.

## Dedicated HTTP surface

`adapter_http::rooms::router` composes room routes separately from the desktop
router. It is not merged into or publicly bound by production bootstrap yet.

| Method | Path | Authority |
| --- | --- | --- |
| POST | `/rooms/{room_id}/join` | One-use invitation or private-proof replay |
| GET | `/rooms/{room_id}` | Active member capability |
| GET | `/rooms/{room_id}/party` | Active member; own sheet and public actors |
| POST | `/rooms/{room_id}/invitation` | Host; optional saved target member |
| DELETE | `/rooms/{room_id}/members/{member_id}` | Host |
| DELETE | `/rooms/{room_id}` | Host |
| POST | `/rooms/{room_id}/character-proposals` | Player; server injects owner |
| GET | `/rooms/{room_id}/character-proposals` | Host; pending public summaries |
| GET | `/rooms/{room_id}/character-proposals/own` | Member; own latest private draft |
| POST | `/rooms/{room_id}/character-proposals/{proposal_id}/approve` | Host |
| GET | `/rooms/{room_id}/combat/{encounter_id}` | Accepted active member |
| POST | `/rooms/{room_id}/combat/{encounter_id}/actions` | Accepted member; own actor |
| GET | `/rooms/{room_id}/state` | Active member; owner-filtered snapshot |
| GET | `/rooms/{room_id}/updates?after={cursor}` | Active member; durable changes |
| GET | `/rooms/{room_id}/events` | Active member; authenticated SSE |
| POST | `/rooms/{room_id}/adventure/actions` | Accepted member; own initiator |
| GET | `/rooms/{room_id}/rest` | Accepted member; personal status |
| POST | `/rooms/{room_id}/rest` | Own short rest; host long rest/wait |

Member requests use `Authorization: Bearer ...` and `x-room-member-id`. Duplicate
credential/identity headers are rejected. Join JSON contains `join_id`,
`display_name`, `invitation` and `resume_proof`; clients must retain the private
proof before the first request to recover a lost response. JSON rejects unknown
fields, request bodies are limited to 8 KiB, and responses use `Cache-Control:
no-store`. No secrets travel in route/query parameters.

The party route returns null for a room member without an accepted hero. Accepted
members receive the existing PartyView allowlist: their own complete sheet,
stable roster identities and public mechanical actor summaries. Host authority
does not grant access to another player's full sheet. The route does not expose
the complete campaign snapshot. Provider, save, campaign-library and local-model
administration routes are absent from this router; room creation remains internal.

The real HTTP test covers identity spoofing, host/guest restrictions, private-proof
replay, both directions of biography privacy, unknown fields, oversized requests,
duplicate headers, revocation and closure. Shared game projection, gameplay
mutations, events and production/native wiring remain pending.

## Character submission and acceptance

Migration 0010 stores proposals and approval receipts separately from accepted
party characters. A proposal contains an immutable proposal UUID, character UUID
and complete owner-private sheet. Neither its Rust command nor the HTTP request
contains an authoritative member/campaign identity supplied by the client.
The service injects those identities from the authenticated room lease.

Submitting a fresh proposal supersedes that player's previous pending draft.
An unchanged proposal UUID replays its current status; changed content under the
same UUID conflicts. Each member has at most one pending draft and can submit at
most sixteen drafts per room. An accepted member cannot replace their hero through
this workflow. The host receives mechanical summaries for active pending players;
full drafts remain readable only by their owner. The submission route permits the
existing portrait/character document budget; other room JSON routes retain 8 KiB.

Approval takes an expected campaign revision and an immutable causation ID. Its
SQLite BEGIN IMMEDIATE transaction rechecks the host lease, campaign/session and
active target, then commits the party binding/sheet, campaign revision, proposal
status and both operation receipts together. Superseded drafts, revoked players,
active combat and stale revisions cannot be accepted. An exact approval retry
returns the recorded revision; changed arguments under the same causation ID
conflict. Clients should refresh the current party projection after a receipt.

Coverage includes a real HTTP submission/approval/replay path, a biography larger
than the ordinary room JSON limit, forced failure at the final approval receipt
with complete rollback, stale lease/revocation rejection, and real database/vault
reopening followed by acceptance of the recovered private draft.

## Authenticated combat intentions

Combat JSON contains `request_id`, `expected_revision` and an intention tagged
with `kind`: `end_turn`, `retreat`, `move`, `weapon` or `spell`. It accepts no acting
token, character/campaign/member identity, damage, attack modifier or RNG seed.
The existing HeroCombatIntent schema rejects unknown fields. The server derives
the actor from the authenticated member's accepted party binding.

The room repository verifies current room epoch, membership, party/session and
encounter scope inside the same BEGIN IMMEDIATE transaction that resolves and
commits the action. The mechanics use the same pure prepare_action function as
solo play. NPC turns, every party sheet, clock, revision and receipt commit as one
operation. Invalid ownership, wrong turn, revoked leases and foreign encounters
cannot produce an effect. Room and member UUIDs namespace client request IDs, so
two players can safely use the same nonce. Changed arguments conflict.

Responses separate the latest authoritative `projection` from immutable
`receipt_revision` and `receipt_events`. An exact retry retains its original rolls
without rolling the state back to an earlier turn or reopening a finished battle.
Renderers should apply current projection revisions and deliver receipt events
once per immutable request. Other heroes' loadout/casting profiles are removed
from both read and action responses; the caller retains its own profiles.

Genuinely finished encounters remain readable/replayable by their original
accepted participants after another hero joins. Superseded encounters closed by
snapshot restoration are rejected with a state conflict, including restore to no
combat or to a different encounter. Their stale active JSON is never returned as
a live battle. Newly accepted heroes cannot read or act in an old encounter they
did not participate in.

Focused coverage adds actual two-peer HTTP turns, actor injection, nonce isolation,
late/terminal replay, post-combat enrollment, a guest cleric's healing/slot receipt,
forced receipt failure rollback, direct stale-lease checks and both restoration
supersession scenarios. Production LAN/native composition and shared event delivery
remain pending.

## Shared state and reconnect

Migration 0011 adds a transactional room-change cursor. State invalidations cover
room/proposal/world changes and agent activity; combat receipt payloads copy only
public result events and encounter/revision identifiers. Credentials, signatures,
private profiles and raw tool/system messages are excluded from that journal.
Failed transactions publish no changes; receipt replay publishes no duplicate.

The update envelope contains `cursor`, `head`, `reset`, ordered `events` and a
current `state`. Delta pages contain at most 128 changes. The current snapshot may
already be at `head` while the delivery cursor advances through earlier pages;
clients must not roll state back while catching up on public events. Initial or
reset snapshots include recent changes, with a bounded message history of 100
text messages and a 16,384-character limit per message (`truncated` is explicit).
Image attachments and live thinking/tool drafts are not part of this text feed.

SSE uses `room_update` events with the delivery cursor as event ID and accepts
`Last-Event-ID` in a header. Its initial snapshot/cursor/delta are read atomically.
Reconnecting at the current cursor returns a fresh snapshot with empty events;
it does not repeat already delivered rolls or messages. Polls recheck the lease
every 250 ms and end the connection after revocation, closure or epoch change.
The client must authenticate again when reopening the stream.

The public state includes campaign/scene/adventure, clock, known journal/NPC data,
public actors, shared narration, current combat and agent activity. Only the owner
receives their full sheet/pending draft; the host sees other pending proposals as
mechanical summaries. Other heroes' combat profiles remain redacted.

Only user/final-assistant messages created while a room is active are marked for
sharing. Pre-room solo chat, system prompts, tool results and tool-call drafts are
excluded. Snapshot shared_message_indices preserve that visibility; restoration
suppresses automatic publication of historical messages. A timeline-reset cursor
discards obsolete changes and provides the restored state. Legacy snapshots
without visibility metadata retain conservative private-message behavior.

Verification includes real simultaneous host/guest SSE, revocation EOF, ordered
300-change catch-up, public/solo snapshot restoration, failed commit/replay and
actual SQLite/Stronghold reopening. This is backend delivery evidence; native
client integration, public LAN composition, image delivery and physical-device
acceptance remain pending.

## Adventure and rest

World JSON contains `request_id`, `expected_revision` and the existing typed
`action` or `intent`. It carries no acting character/member, campaign/session,
reward, healing amount or RNG seed. The repository rechecks the room lease and
accepted owner in the same transaction as the authored action/resources/clock.
Migration 0012 namespaces immutable receipts by room, member, operation and nonce;
changed arguments conflict. The response is a receipt, not a replacement private
campaign snapshot. Clients obtain current state through the shared-state feed.

Adventure actions reuse the solo authored rules. The party shares its location,
facts, encounter and clock; the hero who rescues Tikhon receives the single authored
reward in their own inventory. Another peer cannot redirect it to a supplied
character ID. Defense creation binds every accepted hero through the group path.

Short rest spends only the initiating hero's chosen hit dice and updates their
resources while advancing the common hour once. Other heroes' sheets remain
unchanged. The host controls long rest and explicit waiting as decisions about
shared time. Long rest resolves every accepted hero from the same starting clock,
then updates all sheets, advances time once and commits one revision/receipt.
An ineligible hero or failed write rolls back the whole operation. The personal
status endpoint derives cooldown/resources for the requesting owner.

All world changes reject an active encounter, including legacy rows whose
projection is absent. Final sheet structure/size is validated before writes;
resource additions that exceed the existing document budget fail without
truncating biographies or corrupting readable party state. Combat persistence
uses the same validated write helper.

Coverage includes actual guest rescue/reward/replay and short-rest HTTP, host-only
long rest, guest hit-die isolation, group recharge/time/cooldown, final-receipt
rollback, stale leases, legacy combat and exact-budget guest documents for rest
and combat creation. Scoped AI intentions, native/public LAN composition and
complete session acceptance remain pending.


## Scoped agent ports and recovery

RoomAgentPorts serves the shared Dungeon Master, not an AI-controlled hero.
The DM narrates, portrays NPCs and adjudicates the humans' declared intentions.
Every hero's decisions and dialogue belong to its human player. The DM sees the
accepted party sheets/biographies through a trusted transactional context port,
including hidden authored canon, but never treats that knowledge as automatically
public discovery. Player HTTP snapshots retain owner-only private sheets and public
party summaries; the DM context is not a player endpoint.

The command history uses only shared room messages; pre-room solo chat remains
excluded. Client history, board state, images and actor IDs cannot enter this path.
The tool catalog and dispatcher permit only adventure_action, combat_action and
rest_action, which delegate to the same authenticated deterministic room use cases.

Migration 0013 persists each room/member/turn/tool intention with exact arguments,
server-generated nonce and original revision/encounter. The private transcript
supports recovery through AgentTurnService. A pending tool whose effect committed
before its result was appended reuses its receipt. All three intentions remain in
both phase catalogs so a terminal combat receipt stays reachable after combat ends;
a fresh combat intention still requires a valid encounter. Saved timeline resets
invalidate prior tool preparations and transcripts before any replay.

Integration coverage uses real SQLite and the production agent engine with a
fixture provider: DM role/party knowledge, player snapshot privacy, forbidden tools, terminal replay without model
calls, committed adventure/terminal-retreat recovery, snapshot timeline reset and
revocation. These are engine ports, not the production room runner or HTTP agent
endpoint. Durable scheduling/serialization, cancellation, native wiring and actual
multi-device session acceptance remain pending.


Trusted DM context explicitly marks provider reasoning private independently of
adventure type. ThinkingDelta is not sent as ReasoningText to the requesting
player, event sink or saved transcript. A real collaborative-room engine test
injects a secret biography into a provider thinking chunk and verifies all three
output boundaries. Ordinary narrated text remains model output; narrative quality
and appropriate secret discovery still require model acceptance, not this wire gate.


## Shared Dungeon Master execution and delivery

The separately composed room router can expose POST `/rooms/{room_id}/dm/turn`.
Its strict body contains `request_id`, `player_message` (nonblank, at most 16,384
UTF-8 bytes) and optional `resume`. Campaign/session/actor/model/history/board and
media overrides are rejected. The host AppState supplies provider, configuration,
process owner and the same campaign lane used by local operations. The bootstrap
`build_room_router` helper composes this player API separately from privileged
local administration; native listener activation/public LAN remains pending.

Migration 0014 stores room/member/turn ownership, canonical original request/model,
room epoch, timeline cursor, status and ordered owner-only event delivery. A unique
pending-turn index covers the whole campaign. Interrupted turns require explicit
resume by their owner. Restart reconciliation runs only after the existing
OS-backed exclusive authority lock; a different process UUID cannot take a live
turn. Restore and epoch invalidation supersede old runs, including a claim whose
first transcript message never committed. Confirmed tool deliveries are repaired
from checkpoints without another effect. Completed replay preserves original
history/model and never calls the provider again.

AgentDone reaches the player only after a nonblank final Assistant and durable
completion. Tool-round limits, empty or whitespace-only replies become interrupted
and remain resumable. Historical blank terminal Assistant entries are ignored
for recovery. The owner stream checks authorization/timeline at consumption,
including buffered completed replay, and while idle. Disconnect, revoke, close or
restore cancel further unvalidated delivery; already sent bytes are not retractable.

SQLite BUSY/LOCKED read codes are distinguished from denied access. Checkpoint
validation retries at most six times in fresh transactions with 10-60ms delays and a one-second overall validation deadline,
without exposing an event before validation succeeds. Persistent failure closes
the stream; a later completed replay can recover without another model call.
Coverage includes real HTTP/SSE, two-player contention, disconnect/explicit resume,
bootstrap reconciliation, restore, revocation/backpressure, frozen input, completion
and sequence guards, repaired receipts and temporary/persistent lock contention.
These backend checks do not prove native host/join UI or a physical LAN session.
