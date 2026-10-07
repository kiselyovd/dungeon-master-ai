# Hero vitality and recovery

The server stores vitality under combatResources.vitality on each canonical
character sheet. It contains durable character identity, current combat token
identity, death saves, conditions, death flag and optional natural-recovery clock.
It is gameplay state, not a client character-creation preference.

Combat writes vitality together with HP/resources in the existing transaction.
A new encounter creates its own token and restores the character state; a recovery
deadline is retargeted only after verifying its durable character owner and keeps
its original clock values. Party/solo snapshots preserve this state with the full
sheet. Malformed counters, mismatched identity, invalid deadlines and contradictory
health/death states are rejected.

Initial character commits, guest proposals/enrollment and missing-character save
fallback remove player-supplied vitality. Once canonical vitality exists, character
editing preserves HP and resources. Actual HP gain resets saves/recovery and removes
Unconscious while preserving other conditions. Wait or unchanged HP does not remove
conditions. Dead heroes require an explicit resurrection/story rule, not ordinary
rest or healing.

Current verification includes typed application tests, dead-host/stable-guest ended
combat, serialized recovery deadline, real SQLite reopening, snapshot restoration,
forged initial/fallback vitality, edit protection, unchanged-HP Wait and rollback
coverage from the existing group combat suite. Owned recovery intentions/API/UI,
damage at zero HP, automatic death turns and the native held-zero-HP continuation
are still outstanding. The observed gameplay dead end is not fixed by persistence
alone. The scheduler test uses trusted fixture state, not a production aid command.


## Owned recovery use case

RecoveryIntent accepts Aid(target character UUID), Wait(target UUID) or an empty
DeathSave action. It accepts no dice outcome or RNG seed. Verified active membership
selects the helper, approved party ownership selects the target, and expected revision
protects the current world. A conscious helper uses committed Wisdom/Medicine; poison
imposes disadvantage. Stable owners can passively wait safely. Dying owners can make
their required save. The receipt records actual rolls, identity, status and deadline.

Migration 0016 adds scoped recovery receipts and a public mechanical update in the
same transaction as HP/vitality, clock and campaign revision. Exact committed retry
reuses the result; a restore reset invalidates the prior receipt. Failed final writes
roll back all state. Normal non-combat time handles natural recovery and bounded
mandatory saves. Restore does not roll new outcomes; combat timing uses its own path.
Revoked/absent guest records cannot be aid targets and are not ticked as active heroes.

The owned use case is verified through actual repositories/services, including a
zero-HP continuation to rest without loading a save. It is not yet exposed through
HTTP/native/frontend controls. Active-combat death turns, damage-at-zero and native
held-defense continuation remain separate pending acceptance.


## Room controls and transport

The public player router exposes authenticated recovery intentions; it rejects
client roll fields and binds the helper to verified membership. Native Recovery
operations pass through the existing bounded broker and durable outbox. The room
snapshot carries public mechanical status and allowed intentions for each active
hero; no other private sheets are exposed. First-deadline creation for a stable
hero is confirmed in the same owned transaction.

Room controls show dying/stable/dead status and aid, passive waiting or the owner's
save. Waiting confirms advancement of shared game time. Stale confirmation closes
on revision/restore or lost availability; pending/disconnected controls are blocked.
EN/RU labels reuse the existing design system. Focused HTTP/broker and production
browser tests pass. Browser evidence uses native stubs and real Rust snapshots;
actual Tauri continuation after held defense at zero HP remains pending.


## Combat turn recovery checkpoint

Mandatory death saves now run at the start of a dying human hero's turn, once per
turn transition. Stable/dead heroes and zero-HP NPCs are skipped. Natural20 restores
one HP and returns the refreshed turn to its human owner; the engine chooses no
hero action. A hero downed during movement does not immediately roll mid-turn.
Encounter opening uses the same transition. Authored round4/all-down boundaries
stop before further rolls. Authored defeat rescues living downed survivors only;
a hero already dead remains dead and cannot block the whole group's transaction.

Independent review found missing natural-recovery scheduling after in-combat
stabilization. Combat sheet synchronization now creates the authoritative 1d4-hour
deadline once, at the committing transition's shared campaign clock (existing
six-second round granularity), preserving it through later combat and its ending.
Exact receipt replay does not reroll or rebase. Failed final receipt insertion
rolls back saves, sheets, deadline and clock together. Existing real-file reopening
and snapshot restoration now verify the scheduled timer without manual seeding.

Focused domain/application/storage checks pass; combat damage at zero, solo
recovery transport/UI and full native session/model/physical-LAN acceptance remain
open. This checkpoint changes mechanics/storage; no new UI or production dependency.


## Damage and mortality checkpoint

Positive damage at zero HP removes stability and adds one death-save failure;
critical damage adds two. Temporary HP does not make damage at zero into healing
or prevent the failure. Immunity/zero damage causes no change. At zero, damage at
least equal to maximum HP causes death. From positive HP, massive damage uses the
remainder after temporary/current HP. Ordinary downing adds Unconscious without
claiming a death event. Nearby hits on unconscious targets are critical.

Weapon/spell resolution accepts hostile damage against downed human targets while
preserving the existing restriction on human-on-human attacks. NPC automation still
targets conscious opponents, never chooses a human hero's action. NPC zero-HP damage
resolution marks the NPC dead. True death events now match the dead flag. Persisting
lost stability removes the old natural-recovery deadline; failed final receipts
roll back HP, counters, timer and death together.

Behavioral RED tests reproduced absent failure/stability/massive-death transitions.
Domain, application and SQLite suites pass, including critical/save-spell paths and
the saved deadline/death rollback regression. No new UI/runtime proof is claimed.
Solo recovery transport and complete native/model/physical-LAN acceptance remain open.

Magic Missile now keeps each dart as a separate damage instance while sharing the
damage die. It cannot manufacture massive damage by summing separate darts; each
hit at zero causes its own failure. Multi-hit spell output records a death once.


## Solo recovery backend checkpoint

A solo owner can request a mandatory death save or wait for their own stable
recovery in a safe location. The application accepts no supplied dice/seed;
self-aid/foreign targets, dead heroes and active combat are rejected. Status offers
only eligible actions. An active co-op room rejects both status and mutation before
receipt replay, keeping room authority intact.

GET/POST /campaigns/{id}/recovery uses the private campaign router and existing
campaign execution lane. Strict DTOs reject extra roll fields. Migration0017 stores
solo receipts separately from rest operations; independent review found that a
simple key prefix could collide with valid rest nonces, so separate tables remove
that intersection. Snapshot restore clears both operation types atomically. Identity,
revision, HP/vitality, clock and receipt remain in one transaction. Exact retries
preserve the original outcome even when the execution seed changes.

Storage/HTTP evidence includes stable zero HP -> own wait -> HP1 -> Long Rest,
one-save/6-second timing, replay, receipt-failure rollback, forged fields, foreign
identity/self-aid, active-room exclusion, restore invalidation and nonce isolation.
The fixture deliberately seeds canonical zero-HP vitality; it does not prove a
complete native solo battle/recovery session. Frontend controls, native acceptance
and broad model/physical-LAN/session requirements remain outstanding.
