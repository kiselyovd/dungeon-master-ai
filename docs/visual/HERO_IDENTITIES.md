[Stable references](VISUAL_REFERENCES.md) · [Back to README](../../README.md) · [Durable media](MEDIA_REFERENCES.md)

# Hero portraits and paired miniatures

The built-in gallery has four reviewed v2 identities for each of the fighter,
rogue, wizard and cleric classes. Each portrait has one matching transparent
whole-body miniature. Existing legacy faces and assigned IDs remain available.

## Choosing an appearance

Open the character wizard's Portrait tab and choose a look. The collection works
with image generation disabled and requires no model or provider account. The
choice is stored as a stable ID in the scoped draft; completing the existing
character workflow carries it into the saved character and public party view.
The wizard reopened from settings exposes Cancel. Closing it retains the draft
appearance and leaves the saved character unchanged.

Selecting a look preserves the name, ancestry, class, abilities and equipment.
Painted equipment is cosmetic and can remain after a loadout change. Current
gear, armor class, resources and action permissions come from the character
sheet and server. Artwork does not grant equipment, spells or resurrection.

The sheet and party strip show the portrait. The map resolves the paired
miniature with a pointer-free image above the unchanged mechanical cell.
Uploaded/generated portraits without a reviewed pair use the portrait fallback.

## Selected set

| Class | Stable portrait IDs | Pairs | Frontend WebP bytes |
|---|---|---:|---:|
| Fighter | `hero.fighter.01.v2` through `hero.fighter.04.v2` | 4 | 514,016 |
| Rogue | `hero.rogue.01.v2` through `hero.rogue.04.v2` | 4 | 444,470 |
| Wizard | `hero.wizard.01.v2` through `hero.wizard.04.v2` | 4 | 447,290 |
| Cleric | `hero.cleric.01.v2` through `hero.cleric.04.v2` | 4 | 503,232 |

The 32 hero derivatives total 1,909,008 bytes before separate backend portrait
copies. Portraits are 512x512; miniatures are 512x768 RGBA WebP. Each portrait's
`pairedAssetId` resolves to `miniature.<class>.<number>.v2`. The images share the
established practical warm light and tactile painted fantasy materials.

New identities append to the existing gallery. New explicit backend portraits
are gallery-only resources; they do not change the generic fallback candidate
pool. The first pilot fighter retains its ID and original gallery position.
Its pedestal was removed by an identity-preserving generator edit. The third
cleric's mismatched shield and tinted-background candidates were rejected.

Original prompts, generator job IDs, portrait references, master hashes and
refinement history are in `hero-provenance.json` and `pilot-provenance.json`.
The built-in image generator authored every new master and refinement. Bundled
Sharp tooling performs only codec/size export; source masters and rejected
candidates remain in `.artifacts/visual-production/masters/` and are not bundled.

## Verification and limits

The production browser selects every pair and restores the last selection after
reload. Each public miniature is decoded at 30/48/64px cell scale on light/dark
maps, with unchanged world coordinates and click targets. Draft regressions
check noncosmetic fields. Cleric rendering tests preserve the selected identity
through wounded, unconscious, confirmed-dead and later living server projections.

Run the existing catalogue and focused behavior checks:

```bash
bun run assets:check
bun run test src/components/wizard/__tests__/PortraitTab.test.tsx src/components/actorVisuals.test.ts src/components/__tests__/CombatToken.test.tsx --maxWorkers=1
```

Use the freshly built production bundle for browser checks, as described in the
active plan. `e2e/hero-art.spec.ts` covers all four class galleries and cold
selection. `e2e/room-production.spec.ts` covers every pair at table scale. The
two-participant storage regression in `media_asset_storage.rs` checks public
IDs, same-name actor separation and private-sheet exclusion with real SQLite.

Browser fixtures prove renderer behavior. Server integration proves public
projection and storage contracts. Native cold-start, real-model quality,
physical LAN/VPN devices, packaging and publication remain distinct evidence
layers; a successful visual test alone does not establish them.

The actual Windows Tauri QA build also decoded and selected all16 portraits.
After normal close and cold relaunch it restored `hero.cleric.04.v2` from disk,
with the same campaign/character UUIDs, revision2, HP12, AC18 and saved-sheet
fingerprint. Models and image runtime remained off. These native checks found
and fixed the missing wizard Cancel control and out-of-order preference writes.
Preferences now serialize and coalesce pending snapshots independently of vault
writes. The delayed-write regression fails before the fix and passes afterward.
Native evidence: `.artifacts/product-uiux/hero-native/proof-{warm,cold}.json`.

## Troubleshooting

An unavailable assigned ID stays unavailable rather than silently changing the
face. An owned portrait requires its original authorized media scope. A portrait
without a reviewed miniature pair uses the portrait fallback. The catalogue
guard checks metadata, hashes, dimensions, alpha and backend/frontend equality;
visual identity and silhouette quality also require image and browser review.

## See also

- [Stable references](VISUAL_REFERENCES.md) explains identity and fallback semantics.
- [Durable media](MEDIA_REFERENCES.md) explains uploaded/generated ownership.
- [Shared table](SHARED_TABLE.md) explains map, party and canonical controls.
