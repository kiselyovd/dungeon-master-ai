[Stable references](VISUAL_REFERENCES.md) · [Hero identities](HERO_IDENTITIES.md) · [NPC portraits](NPC_PORTRAITS.md)

# Opponent miniatures

Eight built-in cosmetic miniatures are available without an image model:
Water Echo, goblin, skeleton, zombie, wolf, bandit, orc and giant rat. Their
combined WebP payload is 648,452 bytes. Each keeps its whole silhouette and
transparent margins; source masters, exact prompts and generation IDs are
recorded in `opponent-provenance.json`.

## Public assignments

The application-owned combat projection supplies `appearances`, keyed by the
canonical combatant UUID. The authored Archive defense assigns `water_echo` to
its two existing opponents. Collaborative `start_combat` entries may provide an
explicit `appearance` from the eight-value allowlist. The typed tool boundary
preserves that field and rejects unknown values before committing the encounter.
Names, initials and class guesses never assign opponent art.

The seven reusable archetypes occur in the repository's existing opponent
retrieval corpus. The images add no SRD 5.2.1 monster statblocks. HP, AC,
loadouts, positions, movement, initiative and collision still come from the
engine. Only actors already present in the public encounter receive bindings;
there is no undiscovered-enemy gallery or hidden roster in the visual metadata.

Old projections deserialize with an empty assignment map. A bounded unknown ID
from a newer renderer catalogue keeps the neutral marker and emits a generic
missing-art warning; it cannot load an arbitrary URL or change to another face.
Human actors retain their own saved portrait/miniature assignment.

## Rendering and verification

`CombatToken` and `InitiativeTracker` consume the same stable asset IDs. Measured
alpha bounds align the visible feet to the existing token base at 30, 48 and 64
pixel cell sizes. Transparent margins remain in the file. Art does not receive
pointer input, and its visual bounds do not enlarge the gameplay cell.

Production-root RU/EN browser cases cover all eight resources, board and queue
selection, overlapping figures, conditions, a dead actor, an unavailable ID,
low zoom and cold reload. The ruler still reports 10 feet for two cells.
Light/dark review sheets use the actual production image frames at all three
sizes. A browser-discovered stationary-token click defect has a RED/GREEN
regression: the board no longer captures a token's primary selection click as
a pan gesture.

Focused frontend and Rust tool/SQLite tests, production build and static gates
are recorded in the active plan. Actual authenticated host/guest checks also pass through the canonical
Quay/Archive/find/defense transitions in 16 room setups. Grouped native and full
checkpoint verification remain pending after Task28.
Browser fixtures prove renderer behavior; they do not prove live-model, GPU,
physical-LAN or packaged runtime acceptance.

## Checkpoint7 native evidence

A freshly built debug Tauri application and backend passed real WebView and
file-backed QA SQLite checks. Eight generic NPC records were explicitly seeded
offline in the stopped isolated profile; six Bell portraits were revealed through
canonical UI actions. Two authored Water Echo miniatures and four current-state
snapshot category images rendered. Normal close/cold restart retained the public
identities, miniature bindings and canonical character fingerprint, including
eight generic faces after their stored roles changed. JavaScript exceptions: 0;
LLM/image runtimes remained off. The debug image-sidecar placeholder was never
invoked. Full gates passed 1,011 Rust and 1,282 frontend tests in 204 files.
Live-model, GPU, physical-LAN, DPI and packaged-release acceptance remain separate.
