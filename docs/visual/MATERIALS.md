# Materials and frame kits

The current production set contains twelve selected material masters and four
transparent frame masters. The approved pilot oak is reused; fifteen new masters
were generated with the built-in image tool, one asset per call. Their exact
prompts and job IDs are preserved in `material-provenance.json`, alongside the
original workspace paths and consumers. Selected versioned derivatives live in
`src/assets/living-tabletop/v2/`; existing assigned character art is preserved.

| Material | Production consumer |
|---|---|
| Leather | Application shell and chat margins |
| Stone | Character sheet surface |
| Oak | Title bar and campaign-library table |
| Parchment | Save-tome pages |
| Velvet | Shared buttons and action controls |
| Bronze | Primary buttons and application crest |
| Stone floor | Decorative table surface around the authoritative map |
| River masonry | Map-layer controls |
| Pale paper | Journal reading surface |
| Worn board | Save-tome spine |
| Cloth | Shared field margins |
| Oxidized metal | Status bar |

Materials extend the existing `materials.css` roles. Readable text surfaces use
strong overlays from existing background tokens; paintings and textures carry no
mechanical meaning. A paving joint is never a wall, distance marker or obstacle.
The authored map still owns collision and movement. Decorative rendering adds
no runtime logs or dependencies.

Panel, journal and campaign frames use CSS border-image on pointer-transparent
pseudo-elements, with 15% corner slices and the existing 12px spacing token.
Their transparent center is not filled by border-image. The portrait frame is
a separate circular overlay, scaled proportionally. Focus and hit targets remain
on the original controls. In system forced-colors mode, decorative frame layers
are hidden; normal borders and focus outlines continue to work.

Materials export at 768px with WebP quality 82-86, each within the existing
220KiB budget. Frames export at 512px with lossless WebP and preserved alpha.
The new derivatives total 2,254,748 bytes; oak remains its original 156,058-byte
pilot derivative. The full catalog has 68 entries, including legacy resources;
19 selected v2 masters count toward the planned final 100.

Review covers every original master, each selected material in a 2x2 CSS repeat,
all four frames on light/dark backgrounds and stretched panel shapes. Numerical
edge measurements are supplementary, not proof of mathematically periodic art.
Frame centers have alpha at most 1/255 after export, with no opaque colored
edge noise detected. Visual approval is based on the actual selected derivatives
and production consumers, including the 64px portrait rim.

The nonshipping specimen is
`e2e/fixtures/visual-pilot/materials.html`; it reuses production tokens, Button
and Field and is excluded from the normal Vite production entry. The production
browser checks cover character sheet, journal, campaign library, keyboard focus
and forced-colors fallback at 960x640 and 1280x800. Fresh packaged Tauri renders
the owned portrait and both new type families from local files; native evidence
is in `.artifacts/product-uiux/materials/native-proof.json`. The isolated native
profile uses a debug Python placeholder and synthetic provider configuration,
so this check does not establish media GPU inference or model-led gameplay.

Run `bun run assets:catalog` to regenerate imports from the reviewed manifest,
then `bun run assets:check` to validate identity, content hashes, dimensions,
alpha, provenance, size budgets and decorative token consumers. The generator
does not invent identities or reassign portraits.
