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
