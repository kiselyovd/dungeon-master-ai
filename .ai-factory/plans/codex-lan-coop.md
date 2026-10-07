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

Then add own-sheet/shared-party projection, host approval of submitted heroes,
atomic room-lease checks on gameplay mutations, and event/reconnect handling.
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
