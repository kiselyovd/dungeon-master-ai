[Stable references](VISUAL_REFERENCES.md) · [Bell scenes](BELL_SCENES.md)

# Map artwork and calibrated skins

The eight existing bundled map IDs retain their provider order and keyword
selection. Updated 1280x720 WebP images total 2,408,582 bytes; each stays below the
existing 512 KiB contract. Previous pixels remain addressable through explicit
legacy catalogue entries. Source masters, prompts, generation IDs, encoding and
consumers are in `map-provenance.json`.

Uncalibrated map rasters render as contained scene artwork or a scene preview.
They do not become a tactical terrain layer merely because they depict walls,
doors or paths. During unmapped combat, tokens remain on the neutral engine grid.
Owned image references retain their scope and resolve after reload without
persisting inline bytes or blob URLs.

## Exact Archive skin

Public adventure/authored encounter metadata explicitly names `map.bell_archive.v2`.
The runtime binds it only when normalized geometry exactly matches the reference:
12x8 cells, blocked column x=6 except y=3, and exit (1,3). Array order does not
matter; missing/duplicate/changed cells, exits, dimensions or unknown IDs reject
the skin. The geometry itself remains supplied by the engine.

The existing reviewed 1536x1024 pilot is reused. Its complete frame maps to the
same SVG origin/size as the wall and exit overlays. `map-calibration.json` records
the source frame and landmarks; `mapSkins.ts` enforces them. Layers affect only
visibility. A failed image or mismatch leaves the supplied neutral vector arena
and its actual wall/exit overlays intact. No route or collision is derived from art.

## Verification

Focused UI checks cover exact/mismatched geometry, order independence, visibility
and image failure at 30/48/64px cells. Five provider tests validate category
separation, stable selection, all eight packaged outputs and the byte contract.
The exported fixture is compared with packaged source bytes before browser replay.
All eight render through production image events and their real preview consumer.
An owned-media scenario verifies reload and absence of stored base64/blob URLs.

Actual-backend production browser cases at DPR1/1.5/2 verify image/geometry frame,
wall/passage/exit positions through zoom/fit/layers, a two-cell10ft ruler, and actual
hero dragging with a server-confirmed 5ft movement cost. Reload retains the skin.
Tests measure geometry using SVG transforms separately from the visible stroke;
an initial half-pixel stroke-bound measurement failure was corrected without
changing geometry or relaxing its alignment requirement. Sources and all eight
actual-consumer captures were inspected.

Grouped checkpoint8 validation passed the full gate: 1,014 Rust tests and 1,292
frontend tests in 207 files. Fresh Tauri verifies the Archive skin with two Water
Echo miniatures and all eight real backend map fallbacks in contained full views
through a fixture chat transport. Cold restart retains the same revision, hero
hash/HP and explicit skin binding. Owned test processes are closed normally.
Live-model, GPU, physical-LAN and release acceptance remain separate.
