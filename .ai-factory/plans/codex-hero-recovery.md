# Hero recovery after combat

Continue the approved full solo/co-op playability goal. Actual held defense left
one guest at zero HP with no supported continuation. Do not replace recovery with
snapshot rollback or silently grant hit points after victory.

Rules source: official SRD 5.2.1, Damage and Healing, pages 17-18:
https://media.dndbeyond.com/compendium-images/srd/5.2/SRD_CC_v5.2.1.pdf
Medicine DC10 stabilizes; stable creatures remain unconscious at zero HP and regain
one HP after 1d4 hours without healing. Death-save 1 gives two failures, 20 one HP,
three successes stabilize/reset counts, three failures kill; stable/dead heroes
must not roll. Healing removes unconsciousness and resets saves.

- [x] Correct existing pure death-save logic and add stabilization/natural recovery.
- [x] Persist authoritative vitality/recovery deadlines with solo/party sheets;
  capture/restore/reopen, validate initial drafts and prevent renderer resource
  injection. Preserve death saves and conditions when combat ends or starts again.
- [ ] Add owned help/healing/recovery intentions, transaction/receipt/timeline/clock
  guards, and expose safe public recovery summaries. Use existing dice/resources;
  dead heroes must not become living through rest or natural recovery.
- [ ] Add existing-control EN/RU UI for helping a downed party member and waiting
  for confirmed recovery. Solo handles its authored rescue and death-save state.
- [x] Reproduce actual held-zero-HP path and continue the same session through
  stabilization, elapsed recovery and normal rest without a snapshot workaround;
  verify two clients/reconnect and independent review before closing the P1.

Current scope: pure rules foundation first; the end-to-end P1 stays open until
persistence, authoritative commands, UI and the actual native scenario are verified.


## Pure rules checkpoint

Three real behavioral RED cases reproduced missing natural-1 double failure,
natural-20 one-HP revival and unwanted stable/dead rolls. The existing helper now
returns a typed result, applies those rules, marks third failure dead and resets
both counters on stabilization. Saturating counters avoid malformed-state overflow.
Medicine DC10 stabilizes a living dying target without granting HP. Natural recovery
has an actor-bound, serializable one-to-four-hour deadline and rejects invalid
clocks, wrong actors, early completion, unstable/dead targets and repeat completion.
Existing healing remains responsible for clearing unconsciousness and saves.
All 125 domain tests, all-target/all-feature domain Clippy and architecture pass.
Independent read-only review found no actionable issue in the pure rules. No storage/API/UI recovery path is claimed yet;
the actual held-zero-HP P1 stays open.

## Persistence integration requirements

Use server-owned vitality state per accepted character. Combat synchronization
must copy death saves, death and stability; starting another encounter must not
replace those with defaults. Preserve deadlines and identity through party/solo
snapshot restoration and SQLite reopening. Character creation must not accept
renderer-supplied death/recovery resources as authoritative state. Damage clears
stability/deadlines, and healing clears deadlines/saves. Exact retries read receipts
before rolling or advancing time; timeline restore supersedes old recovery requests.
Store aid, all affected character state, clock, revision, receipt and room update
in one transaction. Public summaries contain mechanical status only. Verify rollback
at the final receipt/event, wrong-owner/foreign-target rejection and the actual
native held-zero-HP continuation without loading a save.


## Authoritative vitality persistence checkpoint

Added a character-bound server vitality block to existing combatResources JSON.
Combat synchronization persists death saves, death, conditions and recovery deadline
with HP and resources. Binding a new encounter preserves the state and retargets
the actor-bound deadline after validating its durable character parent, without
rerolling its duration. Snapshots/SQLite reopening retain the block through the
existing trusted full-sheet path. Player initial commits, proposals/enrollment and
first-snapshot fallback strip incoming vitality. Existing edits preserve canonical
HP/resources once vitality is present. Positive HP gain clears saves/deadline and
unconsciousness; ordinary Wait/Short-0 preserves conditions. Dead heroes cannot be
healed through this helper. Invalid identities/state/deadlines fail closed.

Independent review reproduced two P2 gaps (snapshot fallback and unchanged-HP
Wait cleanup); both have RED-to-GREEN regressions and follow-up review closed them.
Eight application tests, the full 80-test SQLite suite and 48 room regressions pass.
Workspace Clippy and architecture pass. Full SQLite testing also caught the prior
0015 migration count assertion; it now verifies 15 plus presence of scoped message
publication and absence of its broad predecessor. No dependency/migration added.

The deadline test seeds trusted state through a fixture seam; a production owned
help/wait command does not exist yet. Damage-at-zero, automatic death turns and
historical contradictory vitality remain unverified. Owned assistance, clocks,
public summary/UI and native continuation without restore are the next scope;
the actual gameplay P1 remains open.


## Owned recovery command checkpoint

Application recovery policy owns Medicine proficiency/Wisdom, poisoned disadvantage,
mandatory out-of-combat death saves and natural recovery; SQLite owns transactions.
Accepted active party members can aid an active downed ally. Stable owners can choose
passive waiting in a safe location, and dying owners can request their prescribed
saving throw. No client roll/seed is accepted by RecoveryIntent. Typed receipts
label death saves by durable character UUID. Migration 0016 preserves exact requests,
results and restore cursor; committed retries reuse dice/time/results and restored
timelines reject obsolete receipts. Final receipt/event failure rolls back all effects.

Ordinary non-combat clock progression resolves unattended dying characters and due
stable recovery. Snapshot restoration deliberately restores time without resolving
new outcomes. Combat owns its projection/time: opening and closing use a clock path
that suppresses out-of-combat recovery. Final opening rescue state is synchronized
before the encounter row is inserted. Review found this opening-boundary gap and the
regression now verifies canonical rescued HP1, exact hour and no extra death events.
Inactive guest records are excluded from recovery ticking and cannot be aid targets.

Tests verify aid -> stable wait -> HP1 -> group Long Rest without snapshot rollback,
committed-roll replay, final-receipt rollback, foreign targets, restored-timeline
conflict, passive own save, revoked participant exclusion, poison/proficiency and
strict death-save input. Independent follow-up review found no actionable issue.
Current checks pass: 80 SQLite tests, 38 room/update regressions, three pure recovery tests, workspace Clippy and architecture. HTTP/native/frontend wiring
and real held-zero-HP continuation are next. Combat-turn death saves, damage-at-zero,
solo endpoint and broad gameplay/model/LAN requirements still remain open.


## Recovery transport and room UI checkpoint

Dedicated authenticated POST /rooms/{id}/recovery accepts only nonce, expected
revision and strict RecoveryIntent; client dice and unauthorized requests fail.
The closed native Recovery operation uses the existing private broker/outbox,
keeping member/capability outside renderer input. Actual HTTP/native-broker tests
verify aid, passive wait, HP1 and safe public controls. Room state includes active
hero mechanical recovery summaries and server-offered actions, without private
sheets. Public status reading does not derive private loadouts/spell profiles.
Stable heroes without a deadline can begin their owned wait atomically.

React uses existing room card/Button/Modal styles and EN/RU catalogs. Wait requires
explicit shared-time confirmation, closes on changed revision/action availability,
and remounts after timeline restore; temporary blocking preserves the choice and
prevents submission. API schemas validate intent/receipt/status and hero association.
Real Rust snapshots cover pre-aid/stable/recovered stages for the production-root
browser smoke. All 42 focused Rust cases, 22 frontend tests, six production-preview
browser scenarios, TypeScript/build, workspace Clippy and architecture pass.
Independent read-only review found no actionable issue in this increment.

Browser uses native stubs with real engine snapshots. The actual two-Tauri held
zero-HP continuation through these buttons is next; that P1 is not yet closed.
Solo recovery transport, active-combat death turns and full-game/model/LAN acceptance
remain outstanding. No new production dependencies or publication.


## Actual native co-op continuation

Both rebuilt isolated Tauri profiles run the real server/native/UI recovery path.
A deliberately injured pre-fight host (HP1/12) reaches HP0 through actual held
defense; guest UI aid rolls Medicine14, host waits/confirms in town to HP1, then
host group-rest UI restores both to HP12. No restore follows that selected battle
start; DB timestamps and reset-cursor checks confirm it. An earlier separate test
legitimately died after failed aid/saves and is retained as death-branch evidence.
See COOP_ACCEPTANCE.md and .artifacts/two-native recovery proof files/screenshots.
The concrete co-op continuation P1 is locally verified and independently reviewed: revisions135-140 link held, aid, movement, recovery and rest without any intervening restore. Both persisted heroes are HP12; the specific co-op P1 is closed.
Solo transport, active-combat death turns, damage-at-zero, real-model narration,
remaining session UX and physical LAN acceptance still require work. Test processes
are stopped; no release/publication or complete-goal claim.


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

Validation for the combat-turn checkpoint: full domain/application suites, eight
party-storage cases, nine authored-defense cases, eight action-receipt cases,
five bound-hero cases and server room/hero-route regressions pass. Clippy for all
changed Rust crates (all targets/features) and the architecture guard pass. Native
screenshots from the prior recovery checkpoint do not prove this new turn logic.


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
