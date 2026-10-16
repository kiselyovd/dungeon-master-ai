[Stable references](VISUAL_REFERENCES.md) · [Map skins](MAP_SKINS.md)

# Offline scenic illustrations

The eight bundled illustration IDs keep their existing selection order and
keywords: tavern, village, forest-road, ruins, cave, dungeon, temple and wizard-tower.
Their new 1280x720 WebP images total 2,064,188 bytes; each fits the existing 512 KiB
provider limit. Original pixels remain available under explicit legacy catalogue
IDs. High-resolution sources stay outside packaged imports. Exact prompts,
generation IDs, hashes, encoding and consumers are in `illustration-provenance.json`.

These are generic scenic paintings. They supply atmosphere without defining a
character identity, quest fact, collision geometry or tactical map. Illustration
fallback never supplies a portrait category. The provider emits its existing
bundled provenance and stable asset ID; the chat thumbnail and contained full
view consume the same bytes.

The full view uses the shared modal in a body portal. Escape and the localized
close button dismiss it, focus returns to the opener, and a changed image clears
the previous view. It does not depend on a working media inference runtime.

## Verification

Real resilient-provider tests exercise all eight categories after provider,
network, timeout, degraded-runtime and malformed-image failures. They check the
exact packaged bytes, dimensions, stable IDs and byte limit. Authentication and
cancellation remain errors instead of silently selecting unrelated artwork.

The production browser replays the actual exported fallback bytes through chat
image events. All eight thumbnails decode, full views contain their entire frame,
and Escape closes each view and restores focus. Category assertions prohibit
tactical-map promotion. All eight actual-consumer captures were inspected. A
focused regression test reproduced the old keyboard-close defect before the
shared-modal fix.

Fresh Tauri checks generate all eight illustrations and eight maps through the
actual backend with inference runtimes off, compare their bytes with packaged
files, and deliver them to production cards through a fixture chat transport.
Every full view decodes and closes with focus return. A portrait prompt containing
scenic keywords still returns a portrait. After cold restart, the actual SQLite
campaign retains revision 13, the Archive skin and the same hero hash/HP; uncaught
JavaScript exceptions are zero. Native screenshots wait for modal animation to
finish before visual inspection. Owned test processes are closed normally.

The grouped full gate passed 1,014 Rust tests and 1,292 frontend tests in 207
files. Fixture image delivery verifies presentation separately from semantic
model quality. Live-model quality, GPU and physical-LAN acceptance remain separate.
The final 100-master coverage audit belongs to the final product verification.
