# Bilingual typography

The production theme uses **Manrope 4.504** for controls and resource values,
**Literata 3.103** for headings and narration, and the existing JetBrains Mono
for diagnostic code and technical identifiers. Both new families are self-hosted
under the SIL Open Font License 1.1. Their complete licenses are shipped in
`public/fonts/licenses/`; sources, sizes and SHA-256 hashes are in
`font-manifest.json`. No CDN is required at runtime.

The 12 variable WOFF2 subsets total **503,376 bytes**. Latin, Latin Extended,
Cyrillic and Cyrillic Extended are loaded only when their Unicode ranges are
used. Manrope exposes weights 400-800; Literata exposes 400-700, optical sizing
and real italic. The read-only font inspection verifies the complete Russian
and English alphabets, Ё/ё, numbers, № and common mixed-script names and
diacritics across the union of subsets. This is not a claim of coverage for
every Cyrillic language. Actual browser glyph use is checked with Chromium's
platform-font inspection, including mixed Latin/Cyrillic bold and italic text.

All roles live in the existing `theme.css` tokens. Essential controls use
`--text-sm` (14px), body text uses 16px, secondary metadata uses 12px. Narration
and the journal share Literata with 1.7 line height and a maximum of 65ch.
The chat's compact text-size control selects 16, 17 or 18px (17px default).
This device-local preference also affects the journal, persists across a cold
reload, and does not change model settings or campaign mechanics. Streaming
and finalized narration use the same family and size to avoid a font swap
when an answer finishes. Resource digits use Manrope's tabular figures.

Consumer migration replaces hard-coded Cinzel headings in the wizard and
settings, mixed Cyrillic fallback stacks, monospace resource values, save
metadata and public campaign chips. Technical model IDs, code blocks and
expanded diagnostic records retain the mono role. Existing colors, spacing,
focus outlines and primitives remain the source of truth.

Verification uses the real production root at 960×640 and 1280×800 in RU/EN,
with mixed-script names, italic/bold, symbols, no remote font access, persisted
reading size, reachable composer/send controls and layout-shift measurement.
Legacy-font removal is gated by the final consumer inventory; original files
are retained in Git history and the local audit archive.
