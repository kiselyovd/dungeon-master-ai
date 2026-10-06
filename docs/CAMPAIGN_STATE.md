# Campaign continuation and character recovery

## Player behavior

Continue opens the selected campaign's current server state: character, chat,
encounter, scene, journal and NPC records. It does not select the most recent
manual save or silently roll back an adventure. Use the Campaigns button to return
to the library and choose another chronicle.

The character wizard waits for server confirmation before entering play. A failed
confirmation preserves the draft, including biography, personality and selected
spells. Retrying the unchanged confirmation reuses its operation identity.
Character metadata has a 256 KiB encoded budget. The portrait URL or data URL
has a separate 8 MiB budget, so ordinary generated portraits do not consume the
biography budget. Oversized portraits remain in the draft with a localized
request to choose a smaller image; they are never silently discarded.
Game views and save shortcuts stay inactive until native persistence hydration
finishes. Failed or timed-out encrypted reads never authorize replacing existing
provider records with empty defaults. Debug builds optimize the existing vault
cryptography dependencies without changing derivation/security parameters.

Switching campaigns clears the former character, inventory, transient tool cards,
save selection, targeting and generated map image. Old asynchronous history,
narration and restoration results cannot overwrite the newly selected campaign.
An unfinished wizard draft is retained only when reopening its exact active
campaign/session; another campaign starts with its own empty draft.

## Durable state and checkpoints

Migration `0002_campaign_characters.sql` adds a campaign-scoped character document
and character-operation receipts. It preserves existing campaign and save data.
Existing character-bearing snapshots can seed this document; an explicitly
restored snapshot takes precedence over newer historical checkpoints.

Every distinct save allocates a new session revision and immutable snapshot in
one transaction. A retry with the same causation ID returns the original result;
using that identity for another campaign is rejected. The client reconciles the
returned revision before its next write. Server-side checkpoints without a supplied
sheet use the durable character when one exists.
Hotkey and manual saves share a per-campaign queue. A lost response retains the
original request until it is acknowledged; a newer changed intent first reconciles
that request and then captures a fresh checkpoint.

Explicit save restore updates current rows and the durable character together.
The saved snapshot revision and the current session revision are distinct: choosing
an old save must not make the renderer send an obsolete revision on its next write.

The current character changes on character confirmation and checkpoint capture or
restore. Automatic checkpointing of every exploration/combat action, durable map
geometry/media references, and a full authored adventure are subsequent work; this
foundation does not claim that every unsaved action survives termination.

## Legacy local characters

Before replacing an unidentified legacy sheet, the desktop settings adapter keeps
a separate copy in the `legacy_character_backups` key of `settings.json`. It copies
only character data, never provider credentials or store actions, and deduplicates
identical entries. A backup failure blocks the replacement.

Automatic migration targets only the exact previously persisted active campaign
pointer, when the server has no canonical character there. Failure leaves the old
local sheet intact and exposes retry. A local sheet with no established campaign
association is backed up rather than silently assigned to a different chronicle.
The backup key is a recovery record; this version has no backup-browser screen.

## HTTP contracts

- `GET /campaigns/{campaign_id}/state` reads a coherent current aggregate in one
  SQLite transaction, without changing its revision.
- `PUT /campaigns/{campaign_id}/character` requires schema/ruleset, session ID,
  stable character UUID, expected revision, causation ID and the complete document.
  It returns the same state envelope after an atomic commit.
- State envelopes contain `schema_version`, `ruleset_id`, campaign metadata,
  `session_id`, nullable `character_id`, and `projection`.
- Existing save-create responses retain `id` and additionally return `revision`.
  The client tolerates a legacy response without the added field.

Structural character validation does not authorize client-provided attack damage
or derived modifiers. Domain validation and participant ownership remain required
for the forthcoming cooperative transport. The current local API is not a public
multiplayer endpoint; host settings, secrets and runtime control must be excluded
from guest access when rooms are introduced.

## Verification

```powershell
bun run test -- --maxWorkers=4
bun run architecture:check
bun run build
cargo test -p adapter-sqlite
cargo test -p app-server --test campaign_state
bun run gates
```

Browser acceptance uses a fresh production preview:

```powershell
$env:DMAI_E2E_PREVIEW='1'
$env:DMAI_E2E_PORT='1441'
bun run e2e e2e/campaign-resume.spec.ts e2e/campaign-home.spec.ts e2e/full-campaign.spec.ts
```

The browser fixtures render the real production root with scripted transport.
Real SQLite/HTTP tests provide persistence and transaction evidence separately.
Tauri/WebView, model quality, packaging and CI require their own recorded runs.
Neither passing fixtures nor the presence of installed model weights proves a
complete real-model party or LAN/VPN co-op session.

### Recorded foundation acceptance (2026-10-06)

- `bun run gates`: full gate passed, including Rust workspace tests, Clippy,
  frontend type/style/architecture checks and 1012 Vitest tests in 142 files.
  The host's inherited `RUST_LOG=warn` was overridden with `RUST_LOG=info` for
  the INFO-level telemetry test. The isolated Tauri build used cloud-only
  sidecar flags; no GPU/media runtime was exercised.
- `bun run build`: production frontend build passed.
- Production-preview Playwright: 38 passed, one existing vision-flow skip.
- Isolated real Tauri profile: 8/8 native smoke checks; actual wizard confirmed
  Мира Вейл and biography, a hotkey created a revision-4 checkpoint, a changed
  HP sheet produced a distinct revision-6 snapshot, and loading the older save
  returned HP 11. Switching to Дарен's campaign and back preserved separate
  inventories. A cold native restart retained HP 11 and provider configuration.
  Evidence is in the untracked `.artifacts/native-foundation/` directory.
- Independent final review found four issues in late save callbacks, migration
  eligibility, portrait budgeting and lost-response retry. All four were fixed
  with regressions before the final gate.

Native acceptance used a synthetic loopback provider; it proves desktop,
HTTP/SQLite and vault behavior, not model narration. CI, packaging/release,
Bonsai inference, a complete adventure and LAN/VPN co-op were not run.
Starting equipment prices and level-one origin spells still need the subsequent
mechanics audit. Test apps, backend processes and preview servers were stopped.
