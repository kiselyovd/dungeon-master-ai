# LAN/VPN co-op implementation

Continue the approved full playability goal. Initial assumption: up to four
players, one authoritative host, separate devices over LAN/VPN. Preserve solo
campaigns and the user's original dirty checkout. No new production dependency
without explicit approval. Inline execution with independent final review.

- [x] Durable party roster and character ownership by member/character UUID.
  Host character remains the existing authoritative solo sheet; guest sheets
  have separate storage. Same names are allowed. Private sheets are projected
  only for their authenticated owner; public actors contain mechanical summaries.
- [x] Party snapshot capture/restore, replay, rollback and real SQLite reopening.
  Restore must preserve ownership and reject snapshots for another roster.
- [ ] Persistent room membership and capabilities through existing secrets ports;
  authenticate every peer and inject verified identities into application commands.
  Internal enrollment commands are not wire-deserializable. Guest requests cannot
  read provider configuration, other private sheets, snapshots or administrative
  commands. No public network exposure before these guards exist.
- [ ] Bind every accepted party hero to a unique combat token; synchronize all HP,
  equipment/resources and clocks atomically. NPC automation stops for each player's
  turn; NPC targets and outcomes consider the whole party.
- [ ] Scope adventure/rest/AI intentions to the authenticated initiating player;
  publish confirmed shared state, narration and receipt-aware updates to all peers.
- [ ] Ordered event stream, reconnect/current snapshot, independent client scopes,
  stale response guards and host restart recovery.
- [ ] Native host/join/reconnect and party UI using existing controls, tokens and
  EN/RU catalogs. Before UI edits write a compact design brief; run production root,
  lint/build and browser/native checks.
- [ ] Two-client acceptance: separate heroes, ownership rejection, actual turn
  actions, shared peaceful/defensive outcomes, disconnect/reconnect, restore and
  host restart. Distinguish local client isolation from physical-device LAN proof.
- [ ] Finish remaining solo mechanics/narrative/real-model/native requirements and
  independent whole-scope review before declaring the complete goal achieved.

Current work: first two items implemented locally. Room capability core, SQLite
metadata/receipts and secrets-port persistence are implemented, including saved
party invitations, private-proof replay and revocation. Six focused integration
tests pass. A separate HTTP router now verifies member identity and host-only room
management, returns own-sheet/public-actor projections, and excludes desktop
administration routes. Actual loopback HTTP acceptance passes. Production
composition, gameplay identity injection, network/UI and multi-hero combat
acceptance remain pending; the third item therefore stays open. See docs/ROOM_AUTH.md.
The separate reqwest approval remains pending.

## HTTP implementation sequence

Compose a dedicated room router separately from the local desktop router. First
verify join, fresh member authentication and host-only invitation/revoke/close
with actual HTTP requests; do not add room creation to the public router. Pass
capabilities in Authorization and member identity in a dedicated header, never
URLs. Bound JSON bodies, reject unknown command fields, and disable caching of
capability responses. The router has no settings/provider/save/local-runtime
routes. Keep it unbound to public interfaces until scoped party/game commands
and shared-state projections are implemented and tested.

Own-sheet/shared-party projection and host approval of submitted heroes are now
implemented and verified through actual loopback HTTP. Next add atomic room-lease
checks on gameplay mutations and event/reconnect handling.
Only after these guards pass should bootstrap compose the LAN listener and native
host/join controls. This sequence implements the existing room/party plan; it
does not replace the pending complete gameplay and two-device requirements.

Room-core checkpoint: independent follow-up review closed all three findings
(saved-member invitations, preflight before secret writes, cross-instance join
coordination). Architecture, format, Biome, typecheck, em-dash, Clippy and 1042
frontend tests passed. The initial consolidated run failed the INFO-dependent
telemetry test because RUST_LOG was omitted; the complete workspace Rust suite
then passed with explicit RUST_LOG=info. All 32 original baseline hashes remain
unchanged. No HTTP/LAN/native acceptance or complete goal success is claimed.

HTTP foundation checkpoint: actual loopback requests passed entry, private-proof
replay, UUID/capability spoof rejection, host-only management, private biography
isolation in both directions, JSON/header/body guards, revocation and closure.
Seven room integration tests and the full consolidated gate passed with explicit
RUST_LOG=info (workspace Rust plus 1042 frontend tests). Independent read-only
review found no actionable issue in the router and own-party projection. The
production listener remains unmodified; native/LAN/gameplay acceptance is pending.

Character acceptance checkpoint: migration 0010, private durable proposals and
host-only summary/approval routes now implement the actual guest HTTP flow.
Acceptance rechecks room authority and target activity inside the same transaction
as party enrollment, revision, status and receipts. Forced final-receipt failure
rolled back every effect. Changed proposal content conflicts; superseded drafts
and revoked members cannot be accepted. Real SQLite/Stronghold reopening retained
the draft and allowed host acceptance afterward. Nine room tests, seven party
storage tests, ten repository contracts and the full consolidated gate passed
(workspace Rust, architecture/static/Clippy and 1042 frontend tests). An additional
focused changed-content assertion passed after the full Rust run. Independent
review found no actionable issue. Original 32 source hashes remain unchanged.
The next step removes the verified single-hero restriction in combat persistence;
production LAN/native/gameplay and whole-goal acceptance remain pending.

Party combat progress: accepted heroes now bind together with unique tokens and
tiles. Persistence checks the exact accepted identity set and writes all host/guest
HP/resources with one clock/revision/receipt transaction. NPC turns yield to each
player, including a four-player same-name scenario. Legacy profile restoration
uses all snapshot sheets, and authored defeat rescue applies to every hero.
Six focused SQLite group cases plus solo weapon/spell/hero regressions and two
application group cases pass. Independent review/full consolidated collection is
in progress. Room command ownership, shared client delivery and LAN/native
acceptance remain pending; no complete co-op session is claimed. See
docs/PARTY_COMBAT.md for the scope and stable host-anchor compatibility contract.

Party combat checkpoint collection: full consolidated gates passed (workspace
Rust, architecture/static/Clippy and 1042 frontend tests). Six SQLite group cases
and two application group cases passed, alongside unchanged solo regressions.
Independent read-only review found no actionable backend issue. The single-token
and single-hero authored recovery failures were reproduced RED before repair.
All 32 original source hashes remain unchanged. Local checkpoint only; the next
step is authenticated own-actor room commands with lease checks at commit, then
shared delivery/reconnect and native/LAN acceptance. Whole-goal work stays active.

Room combat progress: authenticated own-actor intentions now use a transaction
that rechecks lease/party/session/encounter and namespaces receipts by room/member.
Responses redact other player profiles and separate current state from original
receipt events. Two-peer actual HTTP tests cover wrong turns, actor injection,
nonce isolation and late/terminal replay. Guest healing consumes its slot once;
forced receipt failure rolls back all effects. Revoked leases and foreign scope
are rejected. Review reproduced a superseded-encounter read after restore; both
restore-none and restore-A-over-B are now rejected while restored A remains usable.
Twelve room cases pass and independent follow-up closed the finding. Full gate
collection is in progress. Next implement shared room state/events and scoped
adventure/rest/AI commands, then reconnect/native/LAN acceptance.

Authenticated combat checkpoint collection: all 12 room integration cases and
the full consolidated gate passed (workspace Rust, architecture/static/Clippy,
1042 frontend tests). Independent review's superseded-battle P2 was reproduced
RED and both restore regressions passed GREEN; follow-up review closed it.
Late-state rollback and terminal/historical replay regressions also passed.
All 32 original baseline hashes remain unchanged. No new production dependency,
public listener, native multiplayer UI or complete goal acceptance is claimed.

Shared-state delivery progress: typed owner-filtered snapshots and durable
transactional changes now support state, paged updates and authenticated SSE.
Real two-peer streams, revocation EOF, 300-change ordered catch-up, rollback/replay,
real SQLite/Stronghold reopening and timeline-reset/privacy restoration pass.
System/tool drafts and pre-room solo chat are excluded; snapshot visibility
metadata preserves the distinction. Review-found reconnect replay was reproduced
RED and fixed by one transactional initial read; follow-up reports no remaining
actionable correction. Four delivery and twelve authorization tests pass. Full
gate collection remains in progress. Native receiver/outbox, public LAN, scoped
adventure/rest/AI and physical-device acceptance remain pending; broad boxes stay
open until their full interfaces and acceptance are verified.

Delivery checkpoint collection: four shared-state/delivery cases and twelve room
authorization cases passed; the complete consolidated gate passed (workspace
Rust, architecture/static/Clippy and 1042 frontend tests). Independent review's
SSE reconnect replay P2 was reproduced RED and fixed with one transactional
initial snapshot/delta read; follow-up closed the finding. Real host/guest SSE,
revocation, 300-record pagination, restore privacy/reset, failed commit/replay and
real SQLite/Stronghold reopening are verified. All 32 original source hashes remain
unchanged. Native browser/Tauri integration, image delivery, public LAN, scoped
adventure/rest/AI and whole-goal session acceptance still remain pending.

World-command progress: authenticated adventure intentions now reuse the solo
transactional rules and apply rewards to the verified initiator; the party shares
facts/location/time. Own short rest consumes only own hit dice; host long rest
updates all heroes from one clock and commits one revision. Host-only waiting and
personal rest status are implemented. Owner/kind/nonce receipts are additive in
migration 0012 and publish only public rolls/metadata. Review-found legacy combat
and final-sheet budget issues were reproduced RED and fixed; the same validated
write now protects combat creation/sync. Sixteen room cases and four delivery cases
pass, including actual HTTP, reward/rest rollback, stale lease, cooldown and
near-budget documents. Follow-up review reports no remaining actionable issue.
Full gate collection is in progress. Scoped AI, native receiver/host/join UI,
public LAN and full solo/co-op acceptance stay pending.

World-command checkpoint collection: full consolidated gates passed (workspace
Rust, architecture/static/Clippy and 1042 frontend tests), with sixteen room and
four delivery cases. Solo adventure/defense/rest/snapshot regressions passed.
Independent review's legacy combat and sheet-budget P2s were reproduced RED,
fixed and closed. A linked combat-create budget regression also reproduced RED;
combat now shares the validated party write and rolls back safely. Original 32
source hashes remain unchanged. Scoped AI, native/public LAN, image delivery,
remaining solo mechanics/model quality and complete-session acceptance remain
active work; no whole-goal completion is claimed.


Scoped-agent progress: owner-filtered provider context, shared-only history, three
intention tools, durable exact-argument tool preparations and private checkpoints
are implemented in migration 0013. Real AgentTurnService/SQLite integration covers
terminal replay, committed intention recovery and snapshot-reset invalidation.
Independent review found terminal combat receipt recovery blocked by phase catalog;
the catalog now preserves all three guarded intentions in both phases. A pending
terminal retreat regression passes. RED confirmation and final consolidated gates
are being collected. Runner/gateway, durable turn serialization/cancellation and
native/public LAN remain pending; no broad milestone is marked complete.


Scoped-agent independent review: terminal combat recovery P2 is closed. The real
pending-retreat test reproduces RED without the catalog fix
(tool_not_available_in_authored_adventure) and GREEN with it. A separate committed
adventure intention replays without another effect. Restored snapshots invalidate
old private checkpoints/preparations. Review found no additional actionable issue
in this bounded ports scope; production runner/HTTP gateway remain explicitly
outside acceptance. Final gate collection continues.


Scoped-agent checkpoint collection: final consolidated gates passed against the
fixed catalog (workspace Rust, architecture/static/Clippy and 1042 frontend tests).
The real engine/SQLite recovery test passes, including terminal retreat, pending
adventure, final replay, reset and revocation. Repository/migration tests pass.
Independent review closed the terminal combat P2 after RED/GREEN confirmation and
reported no remaining actionable issue in scope. Original 32 source hashes remain
unchanged. Production room runner/gateway, cancellation/serialization, native and
public LAN, remaining solo mechanics/model quality and full session acceptance
remain active; broad milestones and the overall goal remain incomplete.


Role correction from the user: the AI is one shared Dungeon Master for narration,
NPC portrayal and scene/combat adjudication; all heroes are human-controlled.
Earlier wording "act for the initiating hero" described a misleading role and is
superseded. Initiator scoping constrains mechanical hero intentions, not the DM's
identity or world knowledge. The trusted DM context now contains all accepted
party biographies plus hidden canon; player endpoints remain owner-filtered.
The prompt forbids choosing a hero's decisions, dialogue or actions on its behalf.
Real-engine coverage checks DM role, both accepted biographies, exclusion of old
solo history and non-disclosure of another biography through guest RoomState.
For future changes, keep DM knowledge, player-visible information and mechanical
actor authority as three separate contracts; do not implement an autonomous hero.


DM correction review found a reasoning disclosure P1 in collaborative rooms.
A real engine fixture reproduced RED: a ThinkingDelta containing a private
biography became ReasoningText. An explicit reasoning_is_private context policy
now suppresses it independently of adventure type; room contexts always enable
this policy. The regression is GREEN for stream, application event sink and saved
transcript, while role/party-knowledge and previous recovery cases remain GREEN.
Final full gates/review closure are being collected. AGENTS.md now records the
human-hero/shared-DM contract to prevent the role error recurring.


Shared-DM runner acceptance remains pending. Compose one campaign-wide DM lane,
not a provider/agent per hero. Each human request supplies its own declared action;
actor authority comes from authentication, while trusted DM context covers the
whole party/world. Persist turn ownership and delivery, resume only the requesting
member's request, and keep context/reasoning outside player APIs. Verify two human
clients making decisions, one DM narrating/NPC responses, deterministic combat,
disconnect recovery and host restart. No autonomous player characters are part of
solo or co-op acceptance.


DM-role checkpoint: final full gates passed (workspace Rust, architecture/static,
Clippy and 1042 frontend tests). Both real-engine room regressions pass. Independent
review closed the collaborative reasoning P1 after RED/GREEN and found no further
actionable issue in this correction. All original 32 source hashes are unchanged.
This verifies the role/context/output boundaries; live model narration quality and
complete solo/co-op session acceptance remain unproven and active.


Shared-DM runner progress: one campaign lane plus migration 0014 durable scoped
ownership/events now execute the real agent engine and an intention-only room HTTP
SSE endpoint. Bootstrap supplies host provider/configuration and a separate player
router; native listener activation remains pending. Canonical retry/resume preserves
original input/history/model; exclusive-authority startup recovery also reconciles
room runs. Reset/epoch invalidation, owner/sequence/final-response guards, confirmed
delivery repair and private consumption-time validation are implemented.

Independent review reproduced a sticky-running completion P1 and revoked completed
replay P2. Round-limit/empty replies now interrupt; whitespace persistence/replay is
normalized, including historical checkpoints. Delivery validates lease/timeline at
consumption even under backpressure. Repeated parallel tests then exposed SQLite
LOCKED (code6) causing valid HTTP EOF. A distinct busy code and six bounded fresh-tx
read retries now handle this without unverified output. Controlled temporary and
persistent lock tests cover recovery/fail-closed behavior. Final gates and review
closure are being collected. Native listener/receiver/outbox/host/join UI, physical
LAN and whole-goal solo/co-op/model acceptance remain active and incomplete.


Shared-DM runner checkpoint collection: final clean consolidated gates passed
(workspace Rust, architecture/static/Clippy and 1042 frontend tests), with 13 runner
and 2 role/recovery cases. Independent review closed sticky completion/whitespace,
revoked completed replay and temporary SQLite read-lock findings after reproduced
RED/GREEN checks. Bounded validation also has a one-second overall deadline so
unlock-notify waits cannot stall a stream indefinitely. Five repeated concurrency
runs passed after retry handling; controlled temporary/persistent lock cases pass.
A Windows oversized-upload transport abort in the unrelated health test passed
focused recheck and the final full gate. Original 32 source hashes are preserved.
Native/public LAN activation, host/join UI and full solo/co-op acceptance remain
incomplete; the overall goal remains active.

Compact UI/native design brief for the next stage: reuse existing campaign cards,
buttons, overlays, status feedback, typography and CSS tokens. Host opens a room
from an existing committed campaign/hero and gets a one-use invitation. Guest joins
with address/invitation, proposes their own hero, and waits for host acceptance.
Accepted peers share the scene, public party summaries and one DM's narration;
private hero cards remain owner-only. Guests cannot select the host's provider or
edit server-authoritative mechanics. Room lifecycle is owned by the existing native
backend process; never expose the privileged local router on the public listener.
Use typed room transport/session projections and fresh authoritative snapshots,
not local solo fallbacks. Connection credentials and pending-turn recovery belong
in encrypted native storage, not persisted Zustand. Disconnect/reconnect feedback
must preserve the user's human intention and prevent duplicate effects. Verify the
real production root, lint/build, browser/native host/guest and two-client session
paths before claiming the UI or LAN milestone complete.


Native transport progress: application-owned listener lifecycle port and backend
implementation now support one scoped room socket in the existing backend child.
Other valid active room IDs and all desktop admin routes return404; identical start
is idempotent and wrong-room stop is rejected. Privileged main listener is checked
for IPv4/IPv6 loopback before serving/readiness. Independent review found accepted
partial-body connections survived parent serve-task abort; a raw TCP regression
reproduced RED. Per-connection cancelable I/O now wakes/fails read/write/flush on
stop or sender drop, and the partial-body test is GREEN. Review closed the P2 and
reported no further actionable issue. Six bootstrap tests pass; consolidated gates
are being collected. Protected owner/Tauri commands, credentials/outbox, client,
UI and physical LAN remain pending; native user-flow milestone stays incomplete.


Native transport checkpoint collection: complete consolidated gates passed
(workspace Rust, architecture/static/Clippy and 1042 frontend tests). Six bootstrap
cases include actual TCP auth, other-live-room/admin404 isolation, repeated start,
wrong-room stop, loopback-only privileged listener and accepted partial-body socket
shutdown. Independent review's P2 was reproduced RED and fixed via per-connection
cancelable I/O; follow-up closed it without further actionable findings. Native
owner/Tauri invocation, client credential/outbox, host/join UI and full physical
LAN/model/session acceptance remain active and incomplete. No public listener is
automatically enabled by the current production bootstrap.


Native owner progress: loopback-only protected host/stop API now drives the real
listener service and Tauri host_room/stop_host_room commands. Capabilities remain
in Rust; renderer view omits them. Native token never reaches the public router.
Real HTTP tests cover authorization, duplicate headers, strict body, actual public
listener access, stop/retry, occupied-port compensation and private solo messages.
Native unit tests cover sanitized view and ordered delayed lifecycle responses.
Failure-injected transport stop/secret deletion prove staged cleanup; failed grant
credential read after room commit is compensated by exact creation receipt. Pending
failed reconciliation retains its request and campaign lane. Independent review
findings are being closed; final full gates are running. Guest connection, native
credential/outbox recovery, stable binding across actual restart, WebView/UI and
complete physical LAN/session/model acceptance remain pending. Production deps
are unchanged; bootstrap added existing workspace libraries only to dev tests.


Native owner review closure: staged transport/secret-delete retry, failed binding
compensation, delayed native response ordering and grant-read-after-commit recovery
are covered by focused GREEN regressions. Compensation resolves exact immutable
creation receipt without another credential read. A mismatched immutable retry owns
no creation, closes nothing and releases pending recovery/lane; changed stopped
nonce409 followed by freshnonce200 is verified through real HTTP. Independent review
closed these findings and reports no further actionable issue in this slice.
The previous full gate passed; the final post-mismatch full run is currently live
(.artifacts/native-owner-verified-gates.log). Do not claim latest full completion or
commit until that running verification finishes. Guest/broker/outbox, real native
WebView, native selection/binding restart and UI/full-session acceptance remain
incomplete; the overall goal stays active.


Native owner checkpoint collection: final post-mismatch consolidated gates passed
(workspace Rust, architecture/static/Clippy and 1042 frontend tests). Real native
owner HTTP cases pass; delayed lifecycle and sanitized renderer native unit cases
pass; injected stop/delete and grant-read failure cases pass. Independent review
closed all identified slice findings, including immutable-mismatch recovery. Tauri
commands compile/register, but actual WebView invocation and user host/join flow
remain unverified. Original 32 source hashes remain unchanged. Guest, broker,
client/outbox persistence, native restart/binding selection and complete session/
model/physical LAN acceptance remain active; no whole-goal completion is claimed.


### Native guest connection checkpoint (verified locally)

- Added protected join/resume routes on the existing loopback native-owner API.
- Guest saves canonical join and resume proof before remote POST; retries recover
  committed membership even when final local credential save failed.
- Native Tauri join/resume commands retain the capability outside the renderer.
- Focused real HTTP flow and two recovery tests pass, including actual Stronghold
  file reopening and refusal after host revocation.
- Independent review found no confirmed defect. Consolidated architecture/format/
  Biome/typecheck/Clippy/workspace Rust checks passed, including native guest DTO.
  The default frontend fork runner stalled without test output and was explicitly
  stopped; the complete suite passed with CLI threads pool (155 files/1042 tests,
  91.75 seconds). No project runner configuration was changed. This is separate
  fallback evidence, not a claim that the original consolidated command finished.
- All 32 original dirty-file hashes are preserved. No new production dependencies.
  UI/action broker/outbox, subscription, WebView invocation, stable host restart
  and full two-device session acceptance remain open.


### Next slice: native room transport into the production UI

Use a closed typed operation enum for state/update reads, own party/proposal,
proposal submission/host approval and adventure/combat/rest actions. Never accept
an arbitrary renderer URL, HTTP method, header, member identity or credential.
Resolve the selected native host/guest connection in Rust; host wildcard bind
addresses must map to a loopback destination for the local host client. Preserve
remote status codes so stale revision, denial and unavailable host remain distinct.
Bound response bytes and cancellation; do not hold the lifecycle lock across an
indefinite event stream. Session generation guards must discard late responses
from a previously selected room. Keep capabilities outside renderer projections.

Before mutation, persist canonical action/nonce in the existing encrypted secrets
store; an unknown delivery result must reuse the same nonce and expose pending
recovery. Read operations must not create an outbox record. Add real two-client
transport tests for accepted/rejected ownership, retry and disconnected host before
connecting views. Keep the server's current snapshot/cursor authoritative; no local
solo command fallback when the room connection fails.

Then implement the previously approved compact UI brief using existing cards,
buttons, overlays and EN/RU catalogs: host/join entry, own hero proposal, host
approval and shared play screen. One shared AI DM handles narration/NPCs; each
human controls their own accepted hero. Verify the real production root, lint,
production build and browser/native smoke. Full party-session/restart and physical
LAN evidence remain separate acceptance gates.


### Native operation broker (local verification)

Closed native IPC operations now resolve cached room/member/capability in Rust;
renderer input cannot choose destination, headers or identity. Protected loopback
broker maps only known room state/party/proposal/rest/adventure/combat endpoints.
Host wildcard binding becomes a loopback destination for its local client. Native
selection generations reject late results. Remote and owner error status remains
available to distinguish pending mutation, denial and stale revision.

Canonical mutations are saved in the configured secrets store before remote send.
One unknown pending mutation per participant prevents replacement by another;
PendingAction returns only its operation for explicit retry. Definitive acceptance
or rejection releases the slot; network/5xx/timeout/rate-limit uncertainty retains
it. Retry uses the same semantic operation with current native credentials and
address. Reads remain available during recovery. No automatic solo fallback.

Workspace compilation passed offline. Real HTTP integration passed unavailable
host -> service recreation -> exact retry -> duplicate receipt, guest private
party and host-only rejection. Four native mapping/lifecycle/secrecy tests and all 19 room DM/native integration
cases passed. Independent review found a plain-text 413 outbox issue; cleanup now
occurs before reading definitive rejection bodies and the regression passed.
Strict read-variant DTO tests also caught and fixed unknown-field acceptance.
Architecture and workspace all-target/all-feature Clippy passed. The first
architecture invocation lacked Cargo in PATH; the explicit toolchain rerun passed.
Final focused regressions passed after the test assertions were strengthened.
Full consolidated gates passed for this slice, including workspace Rust and
155 frontend files/1042 tests. DM streaming/subscriptions, UI and actual
WebView/two-device acceptance remain open.


### Native stream bridge (in verification)

- Added protected Subscribe/DmTurn stream broker and native channel/cancel IPC.
  Credentials remain native; room/stream IDs and selection generations scope data.
- Normalize SSE/UTF-8 with bounded frames and remove private reasoning events.
  Closed consumer stops upstream; interrupted DM retains canonical pending input.
- Independent review identified pending loss on interrupted409, then permanent
  pending after restored timeline. Known bounded error-code parsing now preserves
  running/interrupted requests and releases superseded/mismatch; unknown409 stays
  recoverable. The three actual HTTP regressions pass, including real restore.
- Parser tests pass (2); all five native tests pass, including early abort,
  duplicate stream ID, cancellation-handle cleanup and bounded cancel records.
  Independent follow-up review closed both pending lifecycle findings.
  Full gates passed, including workspace Rust and 155 files/1042 frontend tests.
  The frontend stream adapter, production room screen and actual WebView/two-device
  session are still pending. No new production dependency.


### React room stream adapter (in verification)

- Added isolated native Channel consumer with room/stream scope checks, bounded
  frame buffering, abort/cancel cleanup and stable recovery errors. Private
  reasoning is excluded. EOF/legacy done/DM error cannot masquerade as completion.
- Added strict room/update/proposal/public actor schemas and cursor/scope checks.
  Combat/scene/private own-sheet contents remain inward projections for their
  feature adapters to decode; no Zustand/local campaign mutation occurs here.
- Independent review caught the initial wrong campaign timestamp schema. The
  hand-written fixture was replaced by opt-in export from the real Rust native
  HTTP/SSE regression (DMAI_ROOM_WIRE_FIXTURE); room-specific epoch-ms fields now
  match the actual serialized model. Real fixture passes Channel and decoder.
- Fourteen focused tests, TypeScript and architecture pass. Production build
  passed again after the timestamp correction. The initial full gates failed only
  Rust/JSON formatting; the exporter newline and formatting were corrected.
  Full gates then passed, including workspace Rust and 157 files/1054 frontend tests.
  Follow-up review closed the wire mismatch. Production UI/controller integration
  and browser/WebView/physical-device acceptance remain pending.


### Room controller and native action adapter (focused verification)

- Added closed typed native host/join/state/pending/action API adapters and
  receipt/public-view validation. Native remains the owner of identities and
  destinations. Socket/UUID canonical serialization does not reject valid joins.
- Added an ephemeral RoomController and React hook with application-owned port
  composition. Room transitions abort old streams and discard late private state;
  newer snapshots cannot be replaced by older responses. Mutations retry native
  canonical pending input; DM resume preserves its turn UUID and player message.
- Review found late pre-restore narration. RoomUpdate now carries an explicit
  timeline_reset_cursor from the same SQLite transaction as head/state. Controller
  invalidates/aborts DM and ignores late callback/finally/refresh across that
  boundary; a normal full snapshot preserves the current DM. The marker survives
  clipping the reset event out of the retained 128 records.
- Twenty focused frontend tests and the real restore/clipping Rust regression
  passed; regenerated actual Rust wire fixture, TypeScript, architecture and
  workspace compilation passed. Independent follow-up review closed the finding.
  No production dependencies or local solo-state mutations were added.
- Production room screen/root integration and browser/WebView verification remain
  pending; focused checks do not establish a playable UI or two-device session.


### Production room UI (in verification)

Compact brief: retain the existing desktop shell, shared Button/Field, typography,
colors and spacing tokens. Add a library/table co-op entry, host existing saved
campaign, guest address/room/invitation form, public human party, own hero proposal,
host approval and a single shared AI DM chat. Room state remains ephemeral and
isolated from solo. No new visual token/dependency. Combat/map compose existing
features in the next slice.

The real App now opens RoomExperience/RoomHome/RoomPlay with EN/RU catalogs.
Initial controls cover shared chat, own hero, proposals, adventure and rests.
Production-root tests and locally served production-build browser smoke pass
with a native stub and the real Rust wire fixture (Tara/Mira human heroes).
Native IPC/actual two-device gameplay are not established by these checks.

Independent review found four lifecycle issues: hero confirmation changed UUIDs,
definitive host failure reused closed nonce, solo return exposed old state, and
solo hotkeys remained enabled. Fixes retain native proposal IDs/body (with sorted
semantic comparison), expose whitelisted definitive native host errors and reset
only those attempts, wait for local campaign reactivation before exit, and disable
solo shortcuts in room mode. Co-op wizard can cancel to reach recovery controls.
Fifteen focused tests, build, architecture, native compilation and current preview
browser smoke pass. Follow-up review is pending.

UI changes are not committed yet. App.tsx has inherited dirty changes; its exact
pre-UI contents are saved as .artifacts/App-before-coop.tsx. Stage only the own diff
from that baseline into the index, alongside owned files; do not stage inherited
App changes. Original D checkout remains preserved. First-boot guest entry,
native selection/host restart, combat/map/initiative, richer own-sheet and physical
LAN/two-client session acceptance remain required.


### Room VTT/combat UI checkpoint (verified locally)

Reused VttCanvas, CombatOverlay/CombatToken, InitiativeTracker and existing
weapon/spell dialogs. Room presentation supplies map/tokens/own UUID/portrait,
excludes local solo map/image/AOE, and limits drag and actions to the owned active
hero. Server intentions carry encounter/revision/nonce rather than client damage.
Dialog scope includes restore marker. No shared-store projection is overwritten.

Independent review found stale drag on permission loss and modal unmount during
pending. Fixes cancel unconfirmed drag on permission/identity/position/revision
change, retain modal choice while pending, disable confirmation, and close it on
new combat/turn/timeline. Regression tests pass. The old combined run failed only
uninitialized token i18n; the test now loads the real catalog and the current
combined suite passes 11 files/72 tests. Production build, architecture and two
production-preview browser stub scenarios pass. The battle fixture comes from
the actual Rust room-agent/defense projection; source regression passes. Screenshot
review caught and fixed clipped initiative and untranslated Your turn.

UI lifecycle follow-up closed the four earlier findings. All UI changes remain
local; actual IPC/physical LAN, first-boot guest entry, durable native selection/
stable host restart, and complete solo/co-op session acceptance are still required.
The App-only patch .artifacts/room-app-owned.patch applies cleanly to the index
without the inherited App changes; stage that patch separately from owned files.


### First-boot guest entry checkpoint

Welcome now offers a localized join entry using existing onboarding controls.
Entering/leaving co-op does not complete local provider setup. Review found that
room hero submission through the shared wizard completed solo onboarding; an
explicit completeSoloOnboarding policy now defaults true for solo and is false
for room submissions. The real Begin regression confirms room submit/close with
unchanged providers and onboarding=false. Twenty-nine focused root/wizard tests,
TypeScript, production build and architecture pass. Three production-preview
browser stub scenarios pass, including clean guest entry/leave. Full browser
wizard creation and actual IPC remain pending; follow-up review closed the issue.
App/Onboarding inherited diffs are excluded via their firstboot-owned patches.
Native durable selection/recovery and full two-client acceptance remain next.


### Native guest selection recovery checkpoint (verified locally)

Confirmed and pending guest selectors live in the existing encrypted secrets
port. Pending selection is persisted before remote dispatch; explicit recovery
replays the same stored proof or refreshes the saved membership. Another join
cannot overwrite an uncertain result. Definitive refusal clears only pending;
the previous confirmed connection remains recoverable.

Explicitly postponing an unreachable pending join archives its public handle
before releasing selection, without deleting the canonical private proof. A
confirmation explains that the host may already have created the participant.
Saved choices expose room/join UUID, address and display name only; selecting one
uses the original resume operation. Archive writes are deduplicated and retry safe.
The archive currently holds 64 entries; explicit removal/management remains TODO.

Independent follow-up review closed the selection-overwrite and unreachable-host
findings. Three real HTTP/Stronghold regressions, 14 frontend tests, TypeScript,
production build, architecture and workspace all-target/all-feature Clippy pass.
Four production-preview browser scenarios pass using native stubs and Rust wire
fixtures. Actual WebView IPC, stable host restart, archive management and full
solo/co-op session acceptance remain open. No new production dependencies.


### Native host binding checkpoint (backend foundation)

Persist the effective nonzero listener address with the immutable original host
request through the existing secrets port. Recreating the host service with the
same request reuses room/member/capability and binds the saved address; another
request cannot overwrite the saved selection. Busy saved ports and transient
credential reads preserve the durable room for retry. Closed-creation recovery
removes binding then request with retryable error propagation. Stop without an
in-process grant and with a saved selection returns recovery_required, rather
than falsely claiming closure.

Independent review closed orphan-binding and false stop-success findings.
Twelve native HTTP/storage regressions and app-server all-target/all-feature
Clippy pass. The regressions recreate service/state, not a real backend process.
Next: protected recover/abandon of the saved host selection, Tauri IPC/UI wiring,
real process restart and two-client acceptance. Existing generic shared-message
publication during a detached durable room still needs a privacy audit.


### Saved-host recovery through native UI (verified locally)

The protected owner recover endpoint reads the canonical saved host request and
reuses its original room/port. Registered recover_host_room holds native lifecycle
coordination, validates active host membership and keeps capability in Rust.
The strict frontend decoder and existing host-card button support explicit retry
without creating a replacement room or requiring creation inputs.

Backend service-recovery regression and 16 API/root/controller tests pass.
TypeScript, build, workspace all-target/all-feature Clippy and architecture pass.
Five production-preview browser scenarios pass using native stubs. Independent
read-only review found no actionable issue in this increment.

Real debug Tauri WebView verification uses a separate coopqa identifier, database,
WebView profile and encrypted Stronghold vault. The real backend creates and
recovers a room through IPC/API; full application close terminated both native
app and child backend. After a new application/backend launch, exact room/member,
nonzero address and human hero are preserved. The actual recovery button mounts
the room UI; renderer response excludes capability. Sanitized proof files:
.artifacts/coop-native-profile/host-proof.json and restart-proof.json. Screenshot
restarted-room.png hides invitation. App processes were closed afterward.
This is a single-host localhost/debug-WebView check, not two-device LAN acceptance
or packaged production runtime proof. Busy-port abandon, guest archive management,
shared-publication privacy during detached rooms and the complete two-client/solo
session remain required. Vite test server is stopped after this checkpoint.


### Scoped message-publication privacy (verification checkpoint)

Reproduced that unscoped local chat became shared whenever durable room metadata
remained active, including a detached host after restart. Migration 0015 replaces
the broad messages insert trigger with publication from lease-checked
room_agent_messages checkpoints only. User intentions and final assistant text
publish atomically with the checkpoint; tool context and ordinary solo messages
do not. Existing snapshot restoration preserves explicit visibility indices.
Historical pre-migration visibility records are retained: their provenance cannot
be safely reconstructed; this change proves future writes, not retroactive erasure.

The first incremental rebuild missed the newly added migration (RED persisted).
A dependency-free adapter-sqlite build script watches migrations so new files
recompile the embedded sqlx migrator. Existing manual upgrade contracts now apply
0015 too. Room-publication fixtures use the real scoped repository entry point.
Two room-agent, 25 DM/native and six update/publication tests pass, including forced
publication failure rollback, snapshot reset, backlog/reopening and private chat.
Storage/upgrade and workspace Clippy/architecture checks pass. Independent read-only review found no actionable issue. Actual two-client/native/physical LAN session remains next.


### Two-native-client acceptance preparation / invitation fix

Separate host/guest native identifiers, WebView profiles, SQLite databases,
Stronghold vaults and backend processes were launched locally. Actual guest join
exposed an expired ten-minute invitation returned by saved-host recovery; enrollment
stopped before character acceptance, so no complete two-client proof is claimed.
Native processes were closed; .artifacts/two-native contains scoped launch and
acceptance scripts/profiles for continuation (excluded from Git).

Host start/recover now loads canonical invitation and member projection, renews
only expired/consumed codes, and preserves an already usable code across retries.
The existing host-card controls provide Refresh invitation through recover IPC.
Native retains stream generation for unchanged room/member/address/capability;
changing identity/destination/credential still invalidates prior responses.
Two backend expiration/consumption/recreation regressions, ten UI/root tests and
one native stream-generation regression pass. TypeScript, build and workspace
Clippy/architecture pass. Independent review found no actionable issue. Five current production-preview browser scenarios pass, including invitation refresh. The earlier run used the previous build and failed; its result is superseded by the current build run.
Next: rebuild the isolated host/guest executables with this fix, complete native
join/approval/ownership checks, shared DM and peaceful/defense/restart acceptance.
Physical-device LAN remains distinct and unverified.


### Actual two-native-client acceptance checkpoint

Two isolated debug Tauri/WebView/backend profiles now verify enrollment, private
hero isolation, host-only rejection, guest recovery after uncertain join, a shared
host fixture-DM stream, peaceful remembered_river completion, owner-only gold/replay,
real SSE parity, snapshot reset, VTT render in both WebViews, six human combat turns,
wrong-owner rejection, movement replay and held defense. Full native/backend close
and relaunch for host and guest retain room/member/port/roster. See
[local acceptance](../../docs/COOP_ACCEPTANCE.md) and .artifacts/two-native proof JSON.

This is local two-client debug acceptance, not physical LAN or real-model quality.
The initial join timeout after remote commit needs diagnosis. Held defense left
one guest at zero HP: stabilize/recovery/death saves remain a required audit. Full
wizard/pointer/dialog play, persisted model startup, busy-port abandon and archive
management remain open. Native apps, child backends, Vite and provider are stopped.
Independent read-only evidence review confirms bounded native behavior, with
assertion limits documented in COOP_ACCEPTANCE.md. P1: downed guest after held
has no supported out-of-combat recovery, and blocks group Long Rest. Next priority:
implement authoritative stabilization/healing/recovery with owned intentions and
verify continuing the same session without a snapshot workaround. The full
playability goal stays active.


Recovery continuation uses codex-hero-recovery.md. The pure SRD rule foundation is
implemented and domain-tested; durable vitality, owned assistance/time progression,
UI and actual held-zero-HP continuation remain required before closing the P1.


### Co-op held-zero-HP P1 closure (local native acceptance)

Rebuilt c6871be native host/guest apps complete the observed dead-end path through
actual aid/wait-confirmation/group-rest controls. A deliberately injured pre-fight
host reaches HP0 through actual held defense; the guest rolls Medicine14, passive
waiting restoresHP1, group rest restoresbothHP12. No restore follows that selected
battle. Independent SQLite review confirms revisions135-140 and event/timestamp
continuity; screenshots and persisted records agree. An earlier failed aid/death
branch remains documented. All test processes are stopped.

The specific co-op P1 is closed locally. Complete solo, active-combat death saves,
damage-at-zero, real-model quality, physical LAN and remaining UX/lifecycle work
stay open. See COOP_ACCEPTANCE.md and codex-hero-recovery.md for bounded evidence.

## Present versus retained heroes

- [x] Reproduce excluded/closed-room heroes entering a new encounter or blocking
  group rest. Preserve historical sheets/ownership and old encounter readability.
- [x] New battle membership follows the current active room, with host-only solo
  after closure and an explicit trusted legacy party fallback before any room.
- [x] Group long rest touches only currently active accepted members; archived
  hero HP/resources stay unchanged. Check replay and real SQLite reopening.
- [x] Run focused/full relevant checks, independent review and local checkpoint.

No UI/dependency/schema changes in this checkpoint. Removing someone during an
already active battle, retired-roster capacity and returning old members require
separate lifecycle handling; do not claim this fixes all disconnect/room closure.

Roster checkpoint: both original failures reproduced (roster-red.log); new fights
use active accepted room members, with host-only after revocation/closure. Retained
sheets and historical encounter ownership remain unchanged. Old bindings are
immutable for existing encounter writes, including a forged scope removal.
Independent review found a save/restore mismatch for the new host-only encounter;
RED reproduced in roster-restore-red.log. Restore now hydrates only recorded
bound owners, requiring the host and exact known identities. File-backed full
close/reopen and restore regressions run for both revoked and closed cases.
Final collection:440 SQLite/server tests, Clippy alltargets, architecture/format/
Biome/TypeScript fast gates pass. Independent follow-up confirms restore fix.
Original32baseline hashes verified; scoped local checkpoint excludes inherited
edits. Active-fight removal and
returning-member/capacity lifecycle remain open.

## Explicit departure during combat

- [x] Reproduce removal/room closure leaving the current human turn inaccessible.
- [x] Keep recorded hero/token identities, HP/resources and history, mark explicitly
  removed heroes withdrawn from participation; never hand them to NPC automation.
  Skip their turns, death saves, targeting and occupancy. Normal network loss
  keeps membership/hero participation so the same player can reconnect.
- [x] Reconcile membership and encounter atomically on revoke/close and old save
  restore; settle permitted NPC turns to the next present human under existing
  authored boundaries. Preserve receipts/revision/rollback and publish shared state.
- [x] Extend current VTT/initiative projection to omit withdrawn tokens with the
  existing styles/components. Add canonical contract/root and browser checks.
- [x] Verify restart/replay/restore plus independent review and local checkpoint.

UI brief: an explicitly removed player's token/initiative row disappears from the
active table; its saved sheet stays intact. No new controls/styles are needed.
Host closure continues the present host in solo. Rejoining an old retained hero
and roster capacity remain subsequent lifecycle work. No production dependency.

Departure checkpoint: current guest turn failure reproduced. Revoke/close now
withdraw absent humans while preserving recorded UUIDs and retained sheets; NPC
settlement reaches the present host. Old co-op save restore reconciles actual
membership and same-restore retry is inert. Publication failure rolls back room
epoch/membership, encounter, HP/resources, clock and revision; removing someone
else preserves the present actor's spent budget and time. Independent review
found nullable legacy encounter handling and an invalid browser fixture; both
reproduced and corrected without loosening contracts.22 room cases and focused
frontend/browser checks pass; full integrated gates pass (843 workspace Rust and1115 frontend tests).
Dedicated dying-withdrawn/RNG regression added afterward passes all6 death-turn
cases. Revoke/close old-save cases now also close/reopen a real file pool.
Independent follow-up finds no actionable issue; scoped local checkpoint follows.
Explicit guest self-leave, return of withdrawn heroes, roster capacity and native
real-model/two-device acceptance remain open. Network loss keeps membership.
