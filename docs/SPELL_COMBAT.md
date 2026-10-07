# Initial authoritative spell combat

The bound hero's old `Cast` command consumed an action without an effect. It
now rejects before spending anything. The new `spell` intention carries caster,
prepared spell ID, target IDs and slot level. Its strict HTTP schema accepts no
client attack modifier, damage dice or healing amount.

The accepted sheet supplies class, abilities, level and selected spells.
Application mapping validates each selection against its class and spell level
in the existing catalog. The domain owns attack bonuses, save DCs, ranges,
effects and slot consumption. Initial combat effects cover Fire Bolt, Eldritch
Blast, Sacred Flame, Cure Wounds, Healing Word and Magic Missile. Current slot
initialization covers first-level slots; higher slots and Pact Magic recharge
remain explicit follow-up work.

Rules were checked against the official
[SRD5.2.1](https://media.dndbeyond.com/compendium-images/srd/5.2/SRD_CC_v5.2.1.pdf):
spellcasting page105, Cure Wounds121, Eldritch Blast127, Fire Bolt132,
Healing Word139, Magic Missile146, Sacred Flame159 and Unconscious191.
Validation checks preparation, target shape, range, consciousness, sight where
the spell requires it, action/bonus action and one slotted spell per turn before
changing resources. Cantrip scaling distinguishes damage dice from separate
Eldritch Blast beams. Magic Missile can distribute its simultaneous darts.
Healing caps at maximum HP; events report the actual increase and retain the
rolled dice separately. Unconscious targets fail Dexterity saves automatically,
and hits within5 feet are critical. Blinded casters can make ranged spell
attacks with disadvantage when the spell does not require sight.

Slots, HP, the combat projection, campaign revision and intention receipt
commit together. Retry replays the saved result. Canonical snapshots include
remaining slots; restoration retains both stock and the turn's slot-use flag.
Older bound projections recover missing casting profiles alongside loadouts.

The existing Cast button opens a dialog built with the current Modal, Field,
Select and Button controls. The server supplies prepared-spell metadata and
available slots. The dialog selects spell, target and slot, confirms the result,
and uses English/Russian catalogs. Cast availability includes a remaining bonus
action after a normal action. Unsupported prepared effects are shown explicitly
and cannot spend an action or slot.

## Evidence and remaining work

Twelve domain checks cover healing, slots, range, preparation, cantrips, separate
beams, capped healing, sight, critical hits and automatic save failure. A SQLite
scenario verifies atomic healing/resource state, exact retry and restoration.
Dialog, hook and action-bar tests passed. The production-root browser used a
real isolated backend and SQLite: Healing Word healed a2/20HP hero, spent one
slot while retaining the action, then reload and Sacred Flame retained the stock.
Its screenshot was visually inspected. Independent read-only core and edge-rule
reviews found no remaining actionable issue in the checked scope.

Final collection on 2026-10-07 passed the integrated full gate: architecture,
formatting, lint, TypeScript, Clippy, workspace Rust and1038 frontend tests.
The final production build passed. Three production-root scenarios against
the final real backend build passed: spells/reload, completed weapon combat,
and same-name hero identity/automatic NPC turns. The isolated backend was
stopped, and the original32 source baseline hashes were preserved. Evidence
describes the complete attached worktree, including inherited changes excluded
from this scoped local checkpoint. Native AI-led and real-model acceptance are
still separate pending requirements.

Remaining prepared spells, concentration, higher slots, rests and healing
outside combat, AI combat tools/narration, complete authored solo sessions and
LAN/VPN party play remain in the active implementation plan. This document
describes the initial six-spell vertical, not completion of those requirements.
