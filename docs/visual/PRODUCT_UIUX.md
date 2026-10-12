# Living Tabletop evolution

The approved direction keeps the existing violet, gold, cream and tactile fantasy
language while making the actual game easier to read. Materials sit behind quiet
surfaces; controls and narration receive solid contrast. Paintings provide
character and place. Whole sculpted-looking miniatures occupy the map with a
clear base anchor. Artwork never determines movement, collision or rules.

## Selected pilot, 2026-10-12

The bounded nonshipping specimen reuses AppShell, Button, Icons and the existing
theme, spacing, font and elevation tokens. RU/EN copy lives in fixture catalogs.
It is served with `bunx playwright test --config=e2e/fixtures/visual-pilot/playwright.config.ts`.
The normal application build does not include this fixture. It demonstrates one
screen composition and art calibration, not completed gameplay or the future
production renderer. Production table/rendering work remains Tasks 16-18.

| Selected asset | Versioned workspace derivative | Dimensions | Bytes |
|---|---|---:|---:|
| Quiet aged oak | `src/assets/living-tabletop/v2/material-oak-v2.webp` | 768 x 768 | 156058 |
| Fighter identity 01 | `src/assets/living-tabletop/v2/hero-fighter-01-v2.webp` | 512 x 512 | 46958 |
| Matching whole miniature | `src/assets/living-tabletop/v2/token-fighter-01-v2.webp` | 512 x 512, alpha | 52832 |
| Archive background skin | `src/assets/living-tabletop/v2/map-archive-v2.webp` | 1536 x 1024 | 427426 |

Original generated masters remain in `.artifacts/product-uiux/pilot/originals/`
and the tool's generation folder. Existing portraits and their identities were
not overwritten. Texture, portrait, miniature and skin count toward the approved
100 selected masters. Shipping variants use existing bundled Sharp tooling for
codec/size export; no production dependency was added. Stable catalog IDs,
provenance and production consumer binding are established in Task 8.

The portrait and miniature preserve short black hair, the eyebrow scar, cobalt
cloak/shield and worn bronze-edged steel. The cutout includes sword, feet and
base, without a square background. The specimen compares 32, 48 and 64 pixel
versions; 48-64 pixels retain the useful shield/stance silhouette. The initial
base-center presentation anchor is `(0.52, 0.85)` in the square sprite. Task 18
must calibrate production picking/movement from the mechanical cell, not the
visual silhouette.

The Archive skin follows `bell_archive_map` in the current domain source: 12 x 8,
blocked column x=6 except y=3, exit `(1,3)`. A rasterized geometry reference
constrained generation. The specimen adds authoritative wall outlines and an
exit symbol above the painting. Floor seams and shallow water are decorative;
they do not create obstacles. The map skin is not a new mechanical map.

Four browser checks and visual review cover RU/EN at 960 x 640 and 1280 x 800.
The complete surface fills the viewport, composer and controls remain reachable,
and the miniature remains whole. Evidence and measurable alpha/contrast results
are in `.artifacts/product-uiux/pilot/{screens,quality-measurements.json}`.

## Production design decisions

- Keep ornaments at boundaries; prioritize solid readable narration and controls.
  Do not flatten interface text, counters or buttons into generated images.
- Use Literata for bilingual headings/narration and Manrope for controls and
  resource values. The production upgrade and local reading-size preference are
  documented in [Typography](TYPOGRAPHY.md); the initial pilot captures preserve
  the previous type system for comparison.
- A frame can enrich a hero, journal or campaign surface without adding another
  explanation paragraph. Decorative textures never imply an unavailable action.
- A shared table should carry the same map, story, hero and initiative structure
  in solo and co-op. Mode and permission differences belong in contextual
  controls, not a separate visual application.
- Changes of turn, confirmed results and recovery state receive purposeful motion.
  Never animate an unconfirmed outcome as a successful action. Respect reduced
  motion and background windows.
- Preserve old assigned faces. New variants are selected identities; availability
  fallback must respect portrait versus scene semantics. Custom media needs owned
  immutable references before it is treated as durable.
- Prototype captures exposed the old shell grid reserving an empty chat column;
  the fixture now spans the real shell grid and checks that its surface fills the
  viewport. No production shell token was changed to conceal the specimen error.

All final gameplay, performance, packaging, native DPI and real-model acceptance
remain separate gates in the active plan.
