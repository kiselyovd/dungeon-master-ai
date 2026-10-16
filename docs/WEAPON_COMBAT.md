# Authoritative weapon combat

The renderer sends an intention: actor, target, owned weapon, attack mode and
optional two-handed use. It sends no attack bonus, damage dice or damage type.
The domain derives those values from the accepted hero's class, level, ability
scores and existing equipment catalog. Bound legacy attack payloads also use
the canonical profile rather than their supplied damage fields.

Profiles cover proficiency, Strength/Dexterity and finesse, melee reach,
short/long range, thrown weapons, ammunition, versatile and two-handed use,
shield restrictions and heavy-weapon disadvantage. Range, target, turn,
consciousness and stock are checked before spending an action or ammunition.
Advantage and disadvantage cancel; critical hits double dice, not flat damage.
Negative damage cannot add temporary HP. Versatile damage applies only to melee.
The rules reference is the official
[SRD5.2.1](https://media.dndbeyond.com/compendium-images/srd/5.2/SRD_CC_v5.2.1.pdf).

Combat loadouts and remaining weapon/ammunition stocks are part of the durable
projection. HP and `combatResources` commit with the campaign revision and
action receipt in one transaction. Identical retries spend nothing twice,
including a retry arriving after the last enemy falls. Snapshot creation reads
the durable sheet whenever one exists, including outside combat; changing a
hero uses the explicit character commit operation.

Older bound encounters and snapshots can omit loadouts. Reads and restoration
recover only missing profiles from the canonical saved sheet and default NPC
profile, preserving HP, budget, position, round and saved resource counts. A
bound actor without a recovered profile rejects raw client damage. A real
SQLite file close/reopen and old-snapshot restore regression covers this path.
Restore derives missing profiles from the snapshot sheet itself, so current
stocks cannot contaminate the restored timeline (saved20/current5 restores20).

After the hero ends a turn, the server moves NPCs toward a living hero, attacks
when in reach and returns control to the next living hero. This initial NPC
profile uses unarmed attacks, not an official monster stat block. Movement
avoids occupied living cells. A defeated side closes the encounter durably;
creation returns the committed projection even if opening NPC turns resolve it.

The attack dialog reuses existing controls and English/Russian catalogs. It
selects a living enemy and owned weapon, exposes thrown/versatile options when
applicable, shows range and ammunition, and closes after server confirmation.
Its scope follows encounter, actor and campaign transition generation.

## Verification and limits

Focused domain, application, SQLite, transport, hook and dialog regressions
passed. A production-root browser scenario used a real isolated backend and
SQLite to select a longsword, submit the intention, complete combat and reload
the durable result. A separate scenario checked automatic NPC turns and hero
identity among same-name NPCs. Independent review findings for opening combat
closure, temporary HP, final duplicate replay and thrown versatile attacks were
reproduced and fixed; the limited follow-up found no remaining actionable issue.

Final collection on 2026-10-07 passed the full integrated gate: architecture,
formatting, lint, TypeScript, Clippy, workspace Rust and1036 frontend tests.
The production build passed. Both production-root real-backend browser
scenarios passed again on the final backend build; the new modal screenshot
was visually inspected after its entrance animation. Independent follow-up
closed the legacy profile and restored-resource findings. The isolated backend
was stopped, and the original32 source baseline hashes remained unchanged.

This remains a bounded mechanics vertical. Walls, line of sight, opportunity
attacks, full class features, spells, rests, authored defensive outcomes and
AI narration of attacks remain in the active plan. Browser transport shims are
not native or real-model session acceptance. LAN/VPN party control is pending.
