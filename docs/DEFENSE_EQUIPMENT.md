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
