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
