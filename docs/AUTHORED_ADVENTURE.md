# The Bell Beneath the Water

Choose **Prepared adventure** when creating a campaign. The package
`bell-beneath-water` initializes its authoritative state together with the
campaign. Confirm a hero with the existing wizard, then open **Adventure** from
the game table's title bar.

The panel shows the current authored location, discovered facts, available
intentions, journey and outcome in English or Russian. Essential investigation
and the safe upper archive route have no compulsory roll. Optional land-ledger
publication is independent from rescue and ward repair.

The domain graph connects Veresk to the quay, shrine and warehouse; the quay to
the archive; the archive to the belfry. Every dangerous location has a return
path. Scene-local intentions cannot be executed elsewhere. Ward restoration
requires the repaired clapper, a memorial roster and acknowledgment. Sealing the
sluice never sets the restored-ward fact. Finishing without rescue and a resolved
disturbance records an unfinished debt and leaves the adventure resumable.

Confirmed rescue grants ten gold pieces exactly once per authoritative timeline.
The server changes adventure state, canonical inventory, revision and operation
receipt in one SQLite transaction. Same-causation retry returns the original
receipt; a changed request with the same identity is rejected. Character/session
mismatch, unavailable intentions and stale revisions make no changes.

Campaign GET, save capture and explicit restore include the optional adventure
projection. Older snapshots without adventure data remain readable. Restoring a
save allocates a new current revision, preventing delayed pre-restore commands
from changing the restored timeline. The immutable saved revision remains intact.
Current-version HTTP restore also returns `current_state`, which the renderer
applies coherently; legacy seedless sessions return `current_revision` instead.

Pending adventure or restore confirmation blocks new chat turns and saves until
acknowledged. Closing the panel does not remove that protection. An ambiguous
restore failure retains its exact request across closing/reopening the saves
screen. Retry the same save or reopen the campaign to reconcile server state.

Private canon and scene-relevant NPC knowledge are assembled in
`app-application::campaigns::adventure_context` for the DM. Public projections
contain identifiers for discovered facts and available actions, never private
NPC agendas. A model receives engine-confirmed state rather than inventing the
quest's progress. This stage uses explicit UI intentions; adding the adventure
tool to recoverable AI turns is the next mechanics/AI stage.

## Verification

```powershell
cargo test -p app-domain --test bell_adventure
cargo test -p adapter-sqlite --test adventure_contracts
cargo test -p app-server --test adventure_routes --test saves_routes
bun run test src/features/adventure src/features/campaigns/__tests__/CampaignCreate.test.tsx
bun run build
bun run gates
```

Real-backend browser acceptance requires an isolated app-server and database:

```powershell
$env:DATABASE_URL='sqlite://.artifacts/bell-runtime.sqlite?mode=rwc'
$env:DMAI_BIND_ADDR='127.0.0.1:31419'
# Run the built backend in its own terminal, then:
$env:DMAI_ADVENTURE_BACKEND_URL='http://127.0.0.1:31419'
$env:DMAI_E2E_PREVIEW='1'
$env:DMAI_E2E_PORT='1443'
bun run e2e e2e/bell-adventure.spec.ts
```

Three scenarios passed against production preview and real HTTP/SQLite: peaceful
ending and renderer reload; unresolved retreat followed by sealed ending and
independent ledger publication; real save/restore and a fresh rescue without
duplicate reward. Full gates passed with 1021 frontend tests. Independent review
found and reproduced two recovery/race issues; both fixes have passing deferred
request and saves-screen-remount regressions. The full browser suite passed
41 scenarios, with one existing vision-flow skip.

An isolated real Tauri profile created the bundled campaign through its UI and
reached the wizard. Hero confirmation used the real character API with the
complete fixture hero created by the earlier native wizard test. Native UI
actions reached the peaceful ending at revision 25 with one ten-gold reward.
A full native shutdown/restart retained Дарен Вейл, the ending and provider
configuration. Evidence is in `.artifacts/bell-adventure/native-proof.json`,
`native-restart-proof.json` and their screenshots. No renderer-store injection
was used. Test applications, servers and previews were stopped afterward.

Real-model turns, defensive encounter, authored map geometry and two-device
LAN/VPN acceptance remain separate requirements. Installed Bonsai weights and
LM Studio runtimes were inventoried; no model was loaded or promoted.
