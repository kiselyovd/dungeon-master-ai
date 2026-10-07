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
