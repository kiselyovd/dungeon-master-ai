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
- [ ] Persist authoritative vitality/recovery deadlines with solo/party sheets;
  capture/restore/reopen, validate initial drafts and prevent renderer resource
  injection. Preserve death saves and conditions when combat ends or starts again.
- [ ] Add owned help/healing/recovery intentions, transaction/receipt/timeline/clock
  guards, and expose safe public recovery summaries. Use existing dice/resources;
  dead heroes must not become living through rest or natural recovery.
- [ ] Add existing-control EN/RU UI for helping a downed party member and waiting
  for confirmed recovery. Solo handles its authored rescue and death-save state.
- [ ] Reproduce actual held-zero-HP path and continue the same session through
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
