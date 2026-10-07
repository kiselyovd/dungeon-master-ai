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


## Native-controlled room transport boundary

The application-owned RoomListenerControl port now has a backend composition
implementation. RoomTcpListener opens one room-scoped player listener inside the
existing backend child; it does not spawn another process or start a listener at
boot. Repeating the same room/bind spec returns the existing address; a different
active spec is rejected. A listener accepts only its selected room UUID, including
when the requester has otherwise valid credentials for another active campaign.
Local administration routes remain absent. The privileged desktop listener rejects
non-loopback IPv4/IPv6 addresses before serving or publishing readiness.

The host use case closes/revokes its room before stopping transport. Stop signals
every accepted socket's I/O and then graceful server shutdown with a two-second
bound. Cancelable I/O covers incomplete request bodies and stalled reads/writes;
abort of the accept loop alone cannot own Axum's separately spawned connections.
A raw-TCP regression receives 100-continue for a partial POST, closes/stops the room
and confirms the old socket reaches EOF/reset in bounded time. Scope/auth/admin,
repeated start, wrong-room stop and refusal of new connections are also verified.

Protected owner commands, Tauri invocation, native credential/outbox storage,
host/join UI and physical-device LAN acceptance remain to be connected and tested.
The port implementation and loopback tests do not prove those user flows.


## Local native owner commands

When Tauri supplies its existing runtime-control token, bootstrap composes a
separate native-owner router on the loopback administrative listener. POST
`/native/rooms/host` and DELETE `/native/rooms/{room_id}` require that private token;
missing/wrong/duplicate Authorization is rejected. The public player router never
contains these routes, even if a caller knows the native token. The strict host
body contains request/campaign/session IDs, expected revision and bind address;
provider, member identity and arbitrary extra fields are excluded.

The real service persists the canonical request through the existing secrets port,
creates the room and starts its scoped listener. Failed binding compensates the
partial room before returning failure. Creation can commit before credential read
fails; native-owner reconciliation uses the immutable creation receipt to close
that exact room without another secret read. Failed reconciliation retains the
request and campaign lane until a subsequent start/stop can complete cleanup.
Already closed creation receipts cannot reopen a listener.

Stop stages durable room close, transport shutdown and recovery-record deletion.
Each completed stage survives a retry within the process, so an injected stop or
secret-delete failure can be recovered without authenticating a closed room again.
Successful repeated stop is idempotent. Real HTTP tests also confirm that local
solo chat after failed hosting stays out of the next room's shared snapshot.

Tauri registers host_room and stop_host_room. Both serialize the complete operation
through one async lifecycle coordinator, including response/cache commit. Renderer
responses contain public room/member/address and invitation, but never capability
or native owner token. The host capability is retained in Rust NativeRoomState;
existing room secrets remain backed by the configured backend secrets adapter.
Delayed response ordering and capability omission have native unit coverage.

This does not yet prove IPC execution in a real WebView, guest joining, encrypted
client/outbox recovery or physical-device session acceptance. Restart recovery of
the native selection and stable public binding also remains pending. No UI has
been changed in this slice. Only existing workspace libraries were added as test
(dev) dependencies in app-bootstrap; production dependencies are unchanged.


## Native guest join recovery

The protected loopback owner API also exposes POST `/native/room-client/join`
and `/native/room-client/resume`; neither route belongs to the public room router.
The native client persists a canonical SocketAddr/room/join request and a fresh
resume proof through the configured secrets adapter before sending the remote
join. A lost result or failed final local save can retry that same proof without
creating a duplicate member. Reusing the join ID with changed inputs is rejected.
A stored connection resumes through authenticated room refresh; revoked membership
fails rather than silently opening a new membership.

Remote requests disable redirects, use bounded timeouts, limit decoded responses
to 64 KiB and validate the returned room/member role and capability. The local
owner token never goes to the remote host. Tauri registers `join_room` and
`resume_room`, retains capabilities in native state and returns only public room,
member/join IDs and address. Renderer serialization has secret-omission coverage.
Real HTTP join/idempotency and fault-injection tests pass; a real Stronghold file
can be reopened and recover the same guest credential, with revocation enforced.

This checkpoint does not prove a WebView invocation or a playable guest screen.
Native action brokering, encrypted outbox, state/event subscription, room selection
recovery, stable host restart and physical-device LAN acceptance remain pending.
No production dependencies were added.


## Native room operation broker

`room_operation` IPC accepts a room UUID and a closed operation enum, including
state/update reads, private party/proposal reads, character submission/approval,
adventure, rest and combat. Native state supplies address, member and capability;
renderer input cannot supply an arbitrary URL, HTTP method, header or identity.
The protected loopback `/native/room-client/execute` route dispatches these finite
operations. Host wildcard addresses map to loopback. Selection generation guards
reject late responses after native lifecycle changes. Known remote and local
owner rejection statuses remain available to the presentation adapter.

Mutations persist exact semantic input before remote delivery in the existing
secrets adapter. A single pending mutation per room/member prevents overwriting
an uncertain result. `pending_action` exposes only the semantic operation for
explicit retry. Retry keeps the same participant/nonce/input and permits refreshed
native credentials or a rebound address. Reads remain available during recovery.
Confirmed 4xx rejection (except 408/429) releases the pending slot before body
read, including plain-text/oversized error bodies. Confirmed valid success also
releases it; unavailable host, 5xx, timeout, rate limit or malformed success retain
it. No redirect following, bounded timeout and 16 MiB response limit apply.

Real HTTP regressions cover unavailable host, service recreation, exact retry,
server duplicate receipt, guest own sheet, host-only rejection and plain-text
413 cleanup. Native tests cover wildcard mapping, cached identity and strict
unknown-field rejection. This does not yet cover DM streaming, subscriptions,
production UI, real WebView calls or the complete two-device session.


## Native shared streams

The same protected native broker exposes `/native/room-client/stream` for the
closed `subscribe` and `dm_turn` operations. Subscribe uses the authenticated
public room SSE route and its Last-Event-ID cursor. DM input contains only turn
UUID, player message and explicit resume flag; provider/history/actor remain host
owned. Native keeps the capability outside renderer IPC, forwards bounded channel
chunks tagged with room/stream UUID, and rejects stale selection generations.
`cancel_room_stream` cancels the connection; bounded early-cancel records cover
abort racing ahead of IPC registration. Duplicate stream IDs cannot replace an
existing cancellation handle, and registration is removed on completion/error.

The backend normalizes split UTF-8 and SSE framing with a 16 MiB frame limit,
filters private reasoning events, preserves known event IDs and bounds idle and
DM total runtime. Consumer disconnect cancels upstream reads. DM input persists
before delivery; active-key coordination prevents concurrent duplicate dispatch.
A valid `agent_done` clears the pending mutation. Disconnect or an interrupted
stream retains it for explicit resume. Known running/interrupted 409 codes retain
pending; superseded/request-mismatch codes after restore release it. Error-code
reading is restricted to known values, 4096 bytes and three seconds; ambiguous
409 responses preserve recovery rather than discard it.

Three actual HTTP regressions cover shared narration/updates/revocation,
disconnect -> conflict -> original pending -> resume, and real snapshot restore
-> obsolete pending release -> successful new turn. Two parser tests cover byte
splitting, private reasoning and invalid frames. These are transport tests;
frontend consumption, actual Tauri WebView invocation and physical LAN gameplay
remain pending.


The React room controller receives narrow ports through app composition and keeps
its state ephemeral. It discards responses from prior room generations and guards
snapshot head/revision. Native pending requests, rather than a new frontend nonce,
are reused on recovery. RoomUpdate includes `timeline_reset_cursor` from its
consistent SQLite transaction: restore cancellation remains observable even when
its event no longer fits the recent history window. A new marker aborts the old
DM stream and invalidates its later callbacks; ordinary full snapshots do not.
The controller is not yet mounted in the production room screen.


## Native guest selection recovery

The protected owner API supports recovery without renderer-held credentials or
invitation. Separate confirmed and pending selectors reference encrypted
canonical join records. Pending is recorded before dispatch and cannot be
replaced by another join while its remote result is uncertain. Known access
refusal releases pending only; a previously confirmed selection is preserved.

Explicit postpone writes a deduplicated archive entry before deleting pending.
It retains the canonical private proof and membership for later exact resume.
The UI asks for confirmation and lists public room/join IDs, address and display
name; capabilities, invitations and proofs never enter these IPC responses.
Archive capacity is 64; deletion/management is still outstanding.

Current focused HTTP/Stronghold tests prove reopen, revocation, rejected alternate
join, post-commit local save failure and unreachable pending selection. Production
preview smoke uses native stubs; it does not prove actual Tauri IPC or LAN play.


## Durable native host binding

The private saved binding references the exact original host request and effective
listener address. Retry after service recreation keeps the room and credentials
and attempts the same port. A busy saved port preserves room activity and records.
A different request is rejected until the existing selection is reconciled.
Closed-room replay clears both records before another host attempt is allowed.
Without an in-process host grant, stop returns recovery_required when a durable
selection exists; it does not report a closure that never occurred.

Service-recreation regressions and static checks cover this backend foundation.
Native saved-host recover/abandon, actual process restart and WebView proof remain
outstanding; the current UI cannot yet select a saved host after restart.


## Explicit saved-host recovery

POST /native/rooms/recover is restricted to the private owner router. It reads the
saved original request and invokes the existing immutable host-start path; no
renderer IDs, capability or new creation nonce are accepted. recover_host_room
stores the validated active host grant in native memory and returns its public
room/member/address/invitation view. The host-card recovery button uses a strict
schema and explicit retry. An absent selection returns null.

A locally isolated debug Tauri app was closed and relaunched with its existing
SQLite/Stronghold state. Real WebView IPC and the actual recovery button preserved
room/member/port/own hero; the renderer response contained no capability. This
establishes single-host native restart, not physical LAN or full co-op gameplay.
Explicit abandon when the saved port remains unavailable is still outstanding.


## Shared narration publication

Migration 0015 publishes messages from room_agent_messages checkpoints created by
the authenticated room-agent repository. The same transaction validates the room
lease and commits message, checkpoint, public visibility and event. User text and
final assistant narration are shared; tool context is private. Ordinary local/solo
message inserts never become shared merely because a durable room is active.
Snapshot restoration retains explicitly saved shared-message visibility and emits
its timeline reset. Existing pre-0015 visibility is preserved; migration does not
claim to identify or erase previously misclassified historical chat.

Regression evidence includes the original failing unscoped-chat reproduction,
private tool context, failed event publication rolling back message/checkpoint,
confirmed publication, snapshot restore, cursor backlog and database reopening.
The adapter-sqlite build script watches migration additions to invalidate cached
embedded migration code during incremental builds.
