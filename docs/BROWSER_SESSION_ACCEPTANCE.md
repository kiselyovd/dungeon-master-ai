# Browser session acceptance

The production browser suite passes62 cases with one existing vision-flow skip
on the preserved actual worktree baseline after8ecc52b. This is local evidence,
not a clean-HEAD package, CI, release or deployment result.

The full-campaign fixture now creates a cleric through the real wizard, captures
the committed character UUID, binds its token and campaign/session revisions,
selects a prepared Cure Wounds spell, spends one of two slots, moves, ends the
human turn and attacks using the owned mace. NPC progression is supplied by a
scripted server response. Save captures the current state; restore/reload retain
the hero, NPC and journal. A delayed older combat projection cannot rewind the
visible position or HP. Other chat/movement/tabletop fixtures configure a provider
and bind the player character before sending intentions.

Negative proof: temporarily disabling both campaign-scope and combat revision
guards made this scenario fail with expected60px/actual270px token position.
The production source was restored byte-for-byte (SHA256
8b7bc5ef0acd50a1d1a90038a04d9cad831dd2bdd689673fa65b9c20fd9c3e80), rebuilt,
and the scenario passed. Independent review confirmed this proof and closure of
a timing race by waiting for the uniquely identified stale-round narration.

Four Bell adventure tests forward campaign/session/save requests to an isolated
real app-server and SQLite database. They cover peaceful completion and reload,
distinct sealed/unresolved endings, restoring before rescue (which rolls back the reward), then a new rescue
grants10gold, and defensive combat/map with a rescued sealed ending.
They exercise deterministic authored progression; no live LLM generates their prose.

Commands (PowerShell, from the attached worktree):

```powershell
$env:DMAI_E2E_PREVIEW='1'
$env:DMAI_E2E_PORT='15142'
$env:DMAI_ADVENTURE_BACKEND_URL='http://127.0.0.1:31425'
bunx playwright test --config=e2e/playwright.config.ts --timeout=20000
```

The backend uses a private DATABASE_URL under .artifacts/browser-session and
DMAI_BIND_ADDR127.0.0.1:31425. Artifacts: full-suite.log, mutation-red.log,
mutation-build.log and restored-build.log. The full-campaign scenario overrides
its timeout to30 seconds. The baseline original checkout remains unchanged.

This checkpoint does not close real-model Russian narrative quality, native
complete-session acceptance, physical two-device LAN/VPN or all D&D mechanics.
Bonsai remains an evaluated optional model with failed semantic acceptance.
