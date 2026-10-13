# Stable visual references

`asset-manifest.json` inventories the existing frontend and bundled-media art plus
the four selected v2 pilot assets. Identity, version, category, dimensions, alpha,
byte count, hash and portrait/miniature pairing are explicit. Backend portrait
aliases bind to the identical frontend bytes. Distinct scene/map identities stay
distinct even if their bytes match. `pilot-provenance.json` records generator jobs,
references and the intended art constraints.

`src/assets/visualCatalog.ts` resolves an explicit identity and category. It never
selects a random face on a read. Legacy URL/class portraits remain compatible;
an unavailable assigned ID resolves to an unavailable image/initial rather than
silently assigning another character. New builtin portrait choices persist only
`portraitAssetId` in the character draft, sheet, campaign document and nonsecret
preferences. The offline collection stays available when generation is disabled.

The selected portrait is shown in the sheet, party strip and actor presentation.
The shared production table renders an explicitly paired whole miniature; its
image is pointer-free and anchored to the unchanged mechanical cell. An owned or
custom portrait without a reviewed pair uses the portrait fallback.
Generated/uploaded media now uses bounded owned content references (Task9).
Unresolved legacy sources remain recoverable; arbitrary external URLs are never
fetched automatically.

`character_portrait`/`portrait` requests select only portrait resources in the
bundled provider. Map and illustration selection remains separate. Resilient
generation rejects empty/oversized/unknown-signature responses and incorrectly
classified bundled references; a fallback must have the matching category and
compiled bytes. This is a bounded signature/container check, not a full codec
decoder or a visual classifier for generated subjects. Reviewed bundle files were
decoded using existing image tooling during manifest creation.

Run `bun run assets:check`. The consolidated gate invokes it. It verifies identity
uniqueness, immutable hashes, header dimensions/alpha, bounds, backend equality,
portrait pairing and generation provenance. WebP header interpretation follows
the [official container specification](https://developers.google.com/speed/webp/docs/riff_container).
Full visual quality, silhouette readability and calibrated geometry remain image
review/browser checks rather than claims inferred from a hash.

Evidence: eleven focused Rust bundle/resilience cases, frontend identity/portrait
selection checks, and a production-browser wizard scenario selecting the v2 face
with media disabled. The browser uses the real production root. No model call is
needed for a builtin selection, and the existing hero name is not changed.

Generated/uploaded images use the owned storage and authorization contract in
[Durable media references](MEDIA_REFERENCES.md). A reference carries no promise of
a generated matching miniature; only an explicit reviewed catalog pair provides it.

The material/frame production set and actual consumers are documented in
[Materials](MATERIALS.md). Typography and self-hosted font licenses are documented
in [Typography](TYPOGRAPHY.md).
The four-class portrait/miniature collection is documented in
[Hero identities](HERO_IDENTITIES.md).

## Fighter identity set, 2026-10-13

Four v2 portraits pair with four transparent whole miniatures through stable IDs
`hero.fighter.01.v2` through `hero.fighter.04.v2`. The pilot identity stays at its
existing gallery position. New identities are appended; previously saved faces
and generic bundled fallback choices retain their previous mappings. Explicit
backend portrait IDs address the same bytes as the frontend gallery.

Each choice is cosmetic: it preserves the draft name, class, abilities and
equipment. All four choices load in the actual production wizard, and the last
selection survives cold reload. The public room projection renders all four
matching miniatures at 30, 48 and 64 screen pixels on light/dark geometry without
changing actor coordinates, hit targets or rules. Original generator prompts,
references and master hashes are retained in `hero-provenance.json`; the pilot
refinement is in `pilot-provenance.json`.

Evidence: `e2e/hero-art.spec.ts`, the four fighter cases in
`e2e/room-production.spec.ts`, and the cosmetic-draft cases in
`src/components/wizard/__tests__/PortraitTab.test.tsx`. Captures and qualified
logs are under `.artifacts/visual-production/` and `.artifacts/product-uiux/`.
This is production-browser and focused-test evidence. The combined native art
acceptance is scheduled at the hero-set checkpoint; real-model and physical LAN
acceptance remain separate.
