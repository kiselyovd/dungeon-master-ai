[Map skins](MAP_SKINS.md) · [Offline illustrations](OFFLINE_ILLUSTRATIONS.md)

# Confirmed gameplay motion

Short presentation cues use the existing 120/180/280 ms scale and semantic
colours. HP loss/gain, spent action resources, ammunition/slots, movement and the
arrival of the owned actor's turn come from observed canonical combat projections.
A new round can identify that turn even when automated NPC turns return to the
same hero. A positive damage result with matching final HP also covers a hit
absorbed by temporary HP; immunity and mismatched results do not create a hit cue.

The baseline includes campaign/session/encounter and restored timeline scope.
Initial hydration, ownership changes, replay, stale revisions and unobserved
revision gaps establish or retain the baseline without celebration. Each cue is
identified by scope, canonical revision, actor and kind. The presentation retains
only the current short batch, clears its timer on replacement and never persists
animation state. Background/unfocused updates and reduced-motion changes are
consumed quietly and are not played later. Current status remains visible.

Drag remains an interaction preview. Canonical coordinates and committed move
transitions come from the server; a pending or denied drop cannot create a success
cue. Selection uses the existing outline, without blocking commands. Health
feedback and resource rings remain code-rendered overlays on the existing art.

## Verification

Model and hook checks cover concurrent actor identities, same-name/rebound
actors, stale/replayed/denied updates, restored scope, turn/round changes,
temporary-HP absorption, immunity, resource replenishment/consumption, reduced
motion, focus return and bounded expiry. Component checks cover drag, own/other
actors and shared room presentation.

Production-browser journeys in normal and reduced motion exercise the actual
root, delayed movement acknowledgement, confirmed healing, stale replies, target
selection, background panning, save/restore and reload. A mutation audit records
cue IDs rather than relying on a timing-sensitive screenshot assertion. The
normal full view and reduced static view were inspected.

Visual review also reproduced a 134px map-panel scroll after using combat controls.
The action bar referenced an undefined `--space-12`, invalidating its offsets.
The final fix composes existing spacing tokens. A browser assertion verifies zero
panel scrolling after casting. Temporary diagnostic output and an unsuccessful
dialog-portal experiment were removed.

The full frontend suite passed 1,304 tests in 209 files; focused checks, production
build and fast gates passed. The final spacing change also passed normal/reduced
browser checks. Grouped full Rust/native verification follows at checkpoint9;
the native checkpoint8 receipt predates these motion changes. Semantic model
quality and physical-LAN acceptance remain separate.
