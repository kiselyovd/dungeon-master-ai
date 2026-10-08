# Party combat persistence

Present accepted campaign party characters enter a new encounter together. Each uses
its durable character UUID and a distinct encounter-token UUID, derives its
mechanical profile from its own stored sheet, and receives a separate spawn tile.
Names do not determine identity or ownership. NPC automation yields at each
living player turn; the same rules support solo and groups of up to four.

The existing CombatCampaignScope remains anchored to the host character for wire
compatibility. It identifies campaign/session/revision; its character_id is not
authority to control all party actors. The room combat interface now derives
the acting character from the authenticated member roster and checks the lease
inside its commit transaction. Production LAN/native composition remains pending.

Every combat persistence transaction requires the encounter's exact recorded character set.
Missing, repeated and foreign bindings fail before a partial outcome can commit.
Host state stays in campaign_characters, guest state in party_characters. HP,
ammunition/quantity profiles and spell slots update together with the encounter,
campaign revision and combat receipt. Shared time advances once for the round,
regardless of party size. Failed guest writes roll back all related effects.

Raw sheet queries are used during combat-profile recovery to avoid recursive
campaign capture. Legacy missing profiles derive from the corresponding actor's
sheet. Snapshot restoration uses the saved host/guest sheets, preserving saved
resources, token IDs and timeline. Reading current guest sheets during restoration
would contaminate the saved state and is explicitly covered by a regression case.
The authored archive defeat recovery applies its existing nonlethal rescue outcome
to every bound hero instead of only the first actor.

Focused verification:

```powershell
cargo test -p adapter-sqlite --test party_combat --test combat_hero --test spell_intents --test weapon_intents
cargo test -p app-application --test party_combat --test hero_binding --test authored_defense
```

Coverage includes two/four same-name heroes, actual resolver turn handoffs and NPC
automation, group resources/time/replay, guest-write rollback, malformed identity
sets, real database close/reopen and legacy-profile snapshot recovery. These checks
verify backend mechanics and persistence. Authenticated room command coverage is
described in ROOM_AUTH.md. Synchronized clients, a native LAN UI and physical-device
co-op acceptance remain separate pending work.

## Present and retained heroes

New encounters use the active members of the current room. Retained party sheets
remain durable after a member is revoked or a room closes, but those heroes do
not enter the next solo encounter. The host remains required. A trusted party
created through the internal pre-room API retains its existing behavior until
the campaign first has room history; this is not an unauthenticated network path.

Existing encounters keep their recorded owner/token UUID bindings even after
room changes. Hydration/resource synchronization selects those recorded owners
from retained sheets; writes cannot add/drop/reassign them, including by removing
campaign scope. Save restore likewise selects recorded owners from the full
snapshot roster, requires the host, and keeps missing/duplicate identity checks.
This lets a new host-only battle coexist with retained guest sheets and survive
save/restore/reopen without rewriting historical encounters.

Group long rest applies only to active accepted members. A revoked downed hero
cannot block the remaining group and its retained HP/resources remain unchanged.
Replay returns the receipt without another rest or revision. Existing solo rests
and recovery clock behavior remain unchanged.

Evidence: two RED regressions (guest in new fight; revoked downed guest blocking
rest), then file-backed revoked/closed-room -> new host-only fight -> save/restore
-> full SQLite close/reopen, with preserved guest sheet and rejected forged owner
removal. An independent reviewer found the initial restore mismatch; separate
RED reproduction and repair are retained. Logs under .artifacts/browser-session:
roster-{red,restore-red,green,reopen,backend-final,clippy}.log.

The subsequent departure section closes active-battle host revocation/closure and
old-save reconciliation. Explicit guest self-leave, retired roster capacity and
rejoining retained heroes remain open. This checkpoint does not prove native/two-device LAN behavior
or full session completion. No UI, schema or production dependency was added.

Final local collection after7842315:440 SQLite/server tests, changed-crate Clippy
alltargets, architecture, formatting, Biome and TypeScript pass. Independent
follow-up review closed the restore finding. The verified attached worktree keeps
its inherited baseline; original32source hashes are preserved. No new frontend or
real model/native session was exercised by this backend-only checkpoint.

## Explicit removal during an active battle

Host revocation and room closure atomically mark absent bound heroes withdrawn
without changing their recorded character/token IDs or stored sheets. The host
continues in solo after closure. This is an administrative participation change;
no player action is generated and no hero becomes an NPC. A network disconnect
does not change membership and the same player can reconnect.

Withdrawn actors remain in recorded combat history, but do not occupy tiles,
receive NPC attacks/spells, roll death saves, refresh budgets or take turns.
They do not keep a defeated party alive or receive authored defeat healing.
The current departed turn skips under the existing round/boundary rules; allowed
NPC turns settle to the next present human. If the present human already owns
the turn, its spent budget and clock remain unchanged. Public room actors and
active tabletop/initiative omit absent heroes; retained private sheets stay saved.

Old save restore uses saved sheets, then reconciles present membership rather than
returning control to an absent hero. Restore receipts prevent a second transition
or NPC settlement on retry. A failure inserting the shared participant-change
event rolls back room epoch/membership, combat, resources, clock and revisions
together. Legacy encounters with a NULL projection still permit revoke/close
and remain unchanged.

Evidence under .artifacts/browser-session/departure-*: RED for the inaccessible
guest turn and legacy NULL handling; real SQLite/service revoke and close
continuation, old-save restore/retry, rollback/spent-budget regression; seven
production-browser room cases with scripted native transport, plus focused
contract/projection tests and a production build. The browser fixture uses the
host identity and a matching recovery summary; strict validation was preserved.

Explicit guest self-leave, rejoining a withdrawn participant and retained-roster
capacity remain next lifecycle work. Native two-device/model acceptance and full
playability are not proved by this checkpoint. No schema migration or dependency.

Final departure collection: full gates pass (843 workspace Rust tests,1115
frontend tests, all-target/all-feature Clippy, architecture, formatting, Biome and
TypeScript). Two departure cases also close/reopen a file-backed database after
old-save restore, preserving the present turn, withdrawn flag and guest sheet.
A later focused six-case death-turn run confirms withdrawn dying heroes neither
roll nor revive/refresh/consume RNG. Seven production-browser room cases and the
production build pass; screenshot inspected. Independent follow-up closes both
findings. No CI/release, physical LAN or live-model narration was verified here.

## Voluntary exit and return

A player can leave its own room through a nonce/epoch-bound request. Voluntary
exit differs from host revocation; normal private/gameplay authentication is
revoked in both cases. Only the original private join proof plus a new versioned
return intent reactivates a voluntarily departed member. Original JOIN replay
is inert after departure, and an earlier return cannot undo a later exit. A host
ban denies return. Member/hero/token IDs and resources stay unchanged.

Migration18 stores departure/return receipts separately. Native exit uses its
own durable outbox, preserving an unknown gameplay/DM action. A confirmed exit
archives the matching private handle; returning settles an uncertain exit first,
then persists an immutable return intention before the request. Shared selection
serialization prevents archival from deleting a different new connection. Private
known-handle indexing preserves an old departure even when another room was
selected before its ACK. Records precede index pointers, and absent unrelated
records are safely ignored during matching archival.

Remaining participants keep their update streams across room epoch changes.
Every poll revalidates the original private proof; an auth/read epoch race retries
without emitting stale state. Departed/banned peers fail fresh authentication.
Returning bound heroes use a free cell if their old cell is occupied, with no
action/budget/HP refresh. The guest Leave UI waits for confirmed server acceptance
and private local archival. Unmount/network loss does not send a departure.

Evidence under .artifacts/browser-session/self-leave-*: core replay/ban/version
checks, actual HTTP stream and barrier RED/GREEN, both selection race orderings,
Stronghold cold reopen after dropped503ACK with prior outbox unchanged, and native
UI proof/durable SQLite readback. Two actual Tauri QA windows on one Windows PC
showed2->1->2 actors and epochs1->2->3; same full guest sheet, UUID andHP12. Host
subscription stayed live. Screenshots were inspected with invitations hidden.
The first QA harness picked an old unsuffixed server; that attempt failed, actual
executables were corrected/hash-checked, and the UI proof was repeated successfully.
Both native apps/backends were then stopped. No live LLM answer, physical-device
LAN, CI or release is claimed here. Retained roster/handle capacity remains open.

A confirmed server exit is remembered per connection while leaving the screen.
If local campaign activation fails afterward, retry only repeats navigation;
it does not send a new departure nonce to an already inactive member. Independent
review found this path; a failing-then-passing root test verifies one remote exit
and two local navigation attempts. Follow-up review closes the finding.
Full local gates pass (853 Rust tests, 1117 frontend tests); the final production
build and seven room browser cases pass. Verification includes preserved inherited
worktree changes and is not evidence of a clean-HEAD release.
