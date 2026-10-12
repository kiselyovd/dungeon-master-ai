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

The selected portrait is shown in the sheet and current token presentation.
Whole production miniatures and their mechanical picking anchors are Task18.
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
