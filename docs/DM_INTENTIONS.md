# Canonical intentions from the Dungeon Master

The DM has `combat_action` and `rest_action` tools for the saved solo hero.
These accept intentions only. The server derives campaign/session identity,
actor, revision and random seed; existing domain rules and SQLite transactions
own damage, HP, turn budgets, spell slots, ammunition and elapsed time.

`combat_action` selects `weapon`, `spell`, `move`, `end_turn` or `retreat`.
Weapon/spell targets are current token UUIDs. Spell ID and slot level must pass
preparation, supported-effect and stock validation. Movement and retreat obey
the authoritative map. NPC turns run automatically. Extra actor, damage or
resource fields are rejected, including on parameterless intentions.

`rest_action` selects `short` with `hit_dice`, `long`, or `wait` with `minutes`.
The same safety, cooldown, time and resource checks apply as for manual rest.
The model must wait for engine confirmation before narrating recovery.

The tool ID combines the durable agent operation and provider call ID. Retries
reuse the original receipt's derived revision and action; changed intentions
under the same ID fail. This also covers interruption after a combat/rest commit
but before the agent tool checkpoint. Public results refresh current state.

Ended encounters are absent from ordinary campaign state. Tool delivery includes
the latest saved terminal combat projection so the renderer can close the fight
and reconcile story/HP during narration. An active current encounter takes
priority over old receipts. An old movement receipt cannot resurrect an ended
fight or replace a newer encounter with an earlier one.

Context and the advertised catalog refresh after each tool round, including
ordinary collaborative campaigns. Starting a fight immediately makes canonical
combat intentions available in the next model round. Context includes current
map, actors, budgets, loadouts, supported spell choices and rest status, without
the private character biography or portrait. Interrupted-turn instructions
remain present after refresh.

## Evidence and remaining acceptance

Decoder regressions cover intent acceptance and forbidden fields. Real SQLite
tests cover fresh revision derivation, immutable replay, changed intention,
terminal delivery and old receipts across two encounters. Scripted provider
integration covers authored/collaborative fight starts, context refresh, move,
retreat, wait, public results and interrupted narration. Production-browser
scenarios cover story delivery, rest HP and terminal combat without reload.
Browser SSE is scripted; the provider integration separately uses real SQLite.

These checks do not prove native AI-led play, real model quality, Bonsai runtime
compatibility, all spell/class effects, or LAN/VPN co-op. Those remain active
requirements of the full playability plan.
