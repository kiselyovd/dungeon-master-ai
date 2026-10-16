[Stable references](VISUAL_REFERENCES.md) · [Back to README](../../README.md) · [Hero identities](HERO_IDENTITIES.md)

# NPC portraits

Eight generic archetypes have reviewed 768x768 v2 portraits: innkeeper, guard,
merchant, rogue, mage, priestess, knight and peasant. Their combined WebP payload
is 773,516 bytes. Six Bell portraits add 525,690 bytes. Original sources, generation
prompts/job IDs and consumer records are retained in npc-provenance.json.

## Stable generic assignments

An NPC's campaign ID and canonical record ID select a device-local entry in
npc-appearance.json. An existing assigned ID stays authoritative even when its
role, display name or the catalogue changes. New assignments use the v2
collection; old IDs remain available. Initial classification supports English
and Russian roles and keeps the original eight-way absent-role fallback.

Two same-name NPCs remain separate during campaign/save restoration. A missing
assigned resource shows Portrait unavailable instead of silently changing faces.
Cards keep the whole face visible and open the same resource in an accessible
full portrait dialog. Escape closes the upper dialog and restores focus.

## Authored Bell identities

The application-owned AdventureProjection supplies public_npcs with only an
id and portrait_asset_id. The renderer consumes that list without deriving
future identities from private DM text. Initial known people are Mila and Agata.
Leon and Varden are added after confirmed visits/meetings; Tikhon's portrait
requires a confirmed find/rescue. Savely requires a corresponding roster/echo
discovery. Unrevealed people have no portrait control or caption.

Portrait appearance is cosmetic. It adds no biography, intention, affiliation,
quest item, equipment, ability or historical fact to canon. The art prompts use
public roles and omit secret/quest props.

## Verification

Task 25: 28 focused tests, full frontend: 201 files and 1,267 tests, architecture/assets/build
and six production-root RU/EN at 960/1280/1920 browser cases passed. These cases
open all eight resources, retain a legacy face, restore after role changes and
exercise same-name records. The restoration bug was reproduced RED before
changing name-keyed records to canonical IDs.

Task26: three public-projection Rust tests, actual SQLite snapshot restoration,
focused UI/nested-dialog tests, production build and two RU/EN public-reveal
browser scenarios pass. Host/guest projection and grouped native/checkpoint
verification are recorded in the active plan when complete. Browser fixtures
are renderer evidence; real-model, GPU, physical LAN and packaged acceptance
remain separate.

## See also

- [Stable references](VISUAL_REFERENCES.md)
- [Shared table](SHARED_TABLE.md)
- [Durable media](MEDIA_REFERENCES.md)

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
