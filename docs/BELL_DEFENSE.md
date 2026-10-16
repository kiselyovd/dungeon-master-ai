# The archive defense

After finding Tikhon in the flooded archive, the player may choose to hold the
passage. The peaceful rescue remains available. This optional encounter uses
two authored water echoes, rather than claiming SRD monster statistics.

The saved hero enters a 12 by 8 upper landing. A partition has one doorway;
the marked dry exit permits retreat. The server validates bounds, walls,
occupied destinations, movement costs and line of sight. NPCs find a walkable
route to the hero. An obstructed spell fails before spending a slot or action.

Dispersing both echoes or surviving three rounds opens the rescue route.
Retreat returns to the quay. Defeat is nonlethal: Mila and Agata recover the
hero to town at 1 HP after one hour, retaining spent resources. None of these
outcomes grants the rescue reward or repairs the ward. The player must still
escort Tikhon and resolve the river disturbance.

Encounter creation, its adventure link and campaign revision share one SQLite
transaction. Resolution commits combat, story, hero HP, elapsed time and the
command receipt atomically. Replaying a command cannot create a second fight;
restoring an active snapshot retains its map and link with a fresh revision.
The generic end-combat endpoint cannot bypass the required exit.

## Verification

Focused domain, application and SQLite regressions cover geometry, round limits,
outcomes, duplicate commands, transaction rollback and snapshot restoration.
The real-server browser scenario uses the production root and real SQLite:

```powershell
$env:DMAI_ADVENTURE_BACKEND_URL='http://127.0.0.1:31423'
$env:DMAI_E2E_PREVIEW='1'
$env:DMAI_E2E_PORT='1475'
bun run e2e e2e/bell-defense.spec.ts
```

It exercises reload during the fight, blocked casting without slot loss,
automatic NPC movement, two successful spells, rescue, the sealed-crossing
ending, one reward and reload of the result. Tauri transport is shimmed; the
campaign/combat routes and persistence are real. Native AI-led play, the real
model and LAN/VPN co-op require separate acceptance.
