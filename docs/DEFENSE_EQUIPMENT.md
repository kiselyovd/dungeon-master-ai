# Equipment defense foundation

The native fighter wizard supplied Chain Mail but computed only10+DEX, showing
AC12. Official SRD5.2.1 Equipment/Armor (printed page92) gives Chain Mail AC16:
https://media.dndbeyond.com/compendium-images/srd/5.2/SRD_CC_v5.2.1.pdf

The catalog now contains all13standard armor/shield entries with corrected prices,
weights, Dexterity caps, Strength requirements and Stealth disadvantages. Domain
DefenseChoice version1 records worn armor_id or null and shield use. Derivation
checks ownership/training, counts a shield once, ignores Dexterity for heavy armor,
caps positive medium-armor Dexterity while retaining negatives, and supports Monk
and Barbarian unarmored formulas. Low Strength reports the ten-foot speed penalty;
Stealth disadvantage is returned for downstream checks.

Canonical solo/party sheets derive AC and speed from declared choices. Combat uses
the same normalized data and respects a carried-but-unused shield. A recognized
trained legacy armor kit gets a deterministic starting recommendation; armorless or
unmodeled legacy values remain compatible. No versioned declaration is inferred
from untrained carried armor. Read projection upgrades only when the document fits
its existing256KiB metadata budget. A legacy document at that boundary remains
unchanged/readable and uses the same combat compatibility path; an explicit edit
must free space before a normalized write. Final writes validate normalized size
and roll back revision/data if it exceeds the budget.

Pure/application/storage and HTTP checks cover heavy/medium/negative Dexterity,
shield stacking, unarmored formulas, ownership, explicit carrying, repeated speed
projection, cached-AC override, exact retry, HTTP422 for unavailable equipment,
metadata-boundary reading and final-write rollback. Independent review found and
confirmed the budget repair. Architecture and changed-Rust Clippy pass.

This backend checkpoint does not finish armor gameplay. Wizard preview/explicit
wearing controls, frontend-server parity, native armor acceptance and downstream
Stealth checks remain pending. Wearing untrained armor and its RAW penalties,
including spellcasting restrictions, are not supported by this declared-loadout
subset yet; no complete-SRD claim. The overall solo/co-op goal remains active.


Frontend checkpoint: the equipment step now exposes an explicit worn body armor
and shield choice using existing controls. The live sheet and commit resolve the
same package/gold inventory and use the canonical catalog for AC and Strength
speed penalties. Unavailable selections block saving instead of silently switching
loadouts. PC projections and draft persistence retain versioned defense data.
The real partialize/storage roundtrip now also retains required draft rulesetId.
Nullable Rust armor fields are handled as absent caps/requirements.

Verification: 194 focused wizard/storage tests pass, including heavy/medium armor,
negative Dexterity, class defenses, package resolution, Rust-wire null and actual
storage roundtrip. Production build passes. A production-root Playwright case
confirms AC18 with Chain Mail/shield, AC12 when carried but unworn, and reload
persistence in an existing characterless campaign. Independent read-only review
found two defects (nullable wire and partialization); both were fixed and rechecked.
Full frontend regression passes: 170 files / 1112 tests. Fast consolidated gates
(architecture, Rust formatting, Biome, TypeScript, dash check) pass.
This is browser fixture evidence; native creation/combat/reopen, recreation of an
existing hero and downstream Stealth/untrained armor behavior remain pending.


Native acceptance, 2026-10-08 (working baseline with inherited changes):
- Rebuilt backend and Tauri with the isolated soloqa identity/profile. Created a
  second bundled campaign through the real UI, then Fighter/Human/Soldier,
  standard array, class package and hero name Boris (Russian label).
- The actual catalog-backed wizard displayed Chain Mail AC16, Strength17 and
  speed30. Confirmed character PUT stored defense version1, armor_id chain-mail,
  shield false, baseSpeedFt30 and AC16. No frontend store injection created it.
- A real backend combat start supplied one NPC only; server bound the sole human
  character. Real End Turn UI advanced round1 to round2/revision4. The NPC rolled
  13+2=15 and missed against AC16, leaving HP11. This demonstrates the armor's
  actual attack threshold, beyond matching cached numbers.
- Closed native/backend completely, verified old listeners absent, relaunched
  the same profile at a new backend port. Character UUID, defense, frontend AC16,
  combat AC16, round2 and revision4 survived. Read-only SQLite independently
  confirmed one character commit, four events and zero snapshots/restores.
- Screenshots inspected; independent reviewer confirmed DB/attack/reopen and
  cleanup. All owned processes/listeners stopped.

Evidence: .artifacts/solo-native/armor-creation-combat.json, armor-turn.json/png,
armor-reopen.json/png, armor-durable.json, armor-cleanup.json and armor build logs.
This native case covers Chain Mail without a shield and one human combat turn.
Shield/other armor native scenarios, existing hero recreation, full session and
packaged release remain outside this acceptance. Stealth and untrained armor
mechanics remain open; broader solo/co-op goal remains active.


Dice prerequisite, 2026-10-08: reproduced the DM roll_dice advantage flag being
ignored (one die instead of two). Domain roll_d20_detailed now retains both dice,
selects max/min for advantage/disadvantage and cancels opposing flags before
consuming RNG. The typed command and advertised schema include disadvantage.
Active mode flags reject expressions other than d20/1d20; damage rolls remain
additive. Missing optional fields retain legacy receipt serialization.

Verification: reproduced RED, two executor cases GREEN, seven domain dice tests,
one typed decoder case, actual typed-tool integration and all107server unit tests
pass. Independent code review found no actionable issues. This fixes generic DM
roll modes; it does not yet derive skill modifiers/armor penalties from the hero.
Canonical checks must bind the exact current solo/active-party character, read
abilities/proficiencies/declared defense from server data, and preserve the existing
completed tool receipt on retry. No check result may be invented by the model.
