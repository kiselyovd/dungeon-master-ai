# Durable media references

Selected portraits and generated maps use immutable references. Bundled art keeps
its asset identity and version. Uploaded, generated and migrated images keep a
UUID, source category, provenance and SHA-256 descriptor. Preferences and character
documents contain metadata; image bytes are stored once in SQLite `media_content`.
The selected image never acquires another face because a collection changes.

## Ownership and delivery

Draft images belong to an opaque draft UUID. A character commit binds its portrait
to the campaign and actor. Guest uploads derive their owner from the authenticated
native room lease; the renderer does not choose another participant's ownership.
Public lookup exposes only portraits attached to current active actors and the
selected shared map. It never returns a private sheet or an unapproved upload.
Lookup revalidates the room lease, so departure, revocation and changed epochs are
denied. Local campaign lookup also checks the explicit campaign binding.

Media fetches verify byte count, MIME and hash. The browser additionally uses its
pixel decoder when available. Shared in-memory resources are reference counted;
scope changes, disconnection and the final unmount abort pending reads and revoke
object URLs. Room cache identity includes membership/epoch rather than every game
revision, avoiding a repeated image transfer on every HP or clock update.

PNG, JPEG and WebP are supported. SVG and animated formats are excluded. The native
container inspector checks bounded dimensions/chunks and PNG checksums; it does
not implement a complete pixel decoder. A valid header alone is not proof of a
usable image. Backend storage validity and actual browser/native rendering remain
separate verification layers.

## Limits and legacy data

Each image is limited to 8 MiB, 8192 pixels per side and 32 million pixels total.
The legacy portrait URL envelope remains 8 MiB, plus 256 KiB of other character
metadata. Upload JSON has a separate base64-aware body bound; there is no silent
truncation to fit the old character envelope.

Valid inline legacy raster data is registered before its URL is cleared. A failed
storage/character commit leaves the source intact. Unknown, oversized or malformed
sources remain recoverable; remote/file URLs are neither fetched nor treated as
trusted portrait locations. Existing bundled URLs can resolve to the exact known
collection identity. Migration never generates a replacement picture or a matching
miniature for a custom portrait.

Solo migration uses an explicit POST and waits for gameplay recovery/mutations to
settle. Read-only campaign/recovery GET requests do not repair or mutate storage.
Guest legacy proposals use an idempotent proposal identity for media registration.
The draft carries its campaign/actor or room context: cold activation preserves
the matching form, while opening a different context starts a separate form.

## Maps and snapshots

A generated map's reference is selected in the same transaction as its durable
tool receipt. The request must still own the current session timeline. A repeated
receipt does not select the map twice or advance the revision twice. Snapshot
capture/restore includes the selected reference; stale post-restore generation
cannot overwrite it. Immutable bytes remain available after a selection is removed.

An image remains appearance data. It does not define walls, exits, movement or
visibility. The existing tabletop keeps authoritative geometry ahead of a generic
generated background. Geometry-calibrated authored skins are a separate plan task.

## Local APIs

- `POST /media/assets`: bounded private upload, trusted local draft/campaign scope.
- `GET /media/assets/{id}?draft_id=...` or `?campaign_id=...`: scoped raster bytes.
- `POST /image/generate` with `asset_scope`: a generated durable reference; older
  callers without the field keep the compatible data URL response.
- `POST /campaigns/{id}/media/portrait/migrate`: appearance-only canonical migration.
- Room upload/lookup use the closed native `upload_portrait` / `media_asset`
  operations; native credentials are never image URLs or renderer persistence.

Generated SSE events carry an additive `media_ref`; legacy encoded events remain
decodable. New durable tool records store the reference without repeating encoded
image data. Portability exports referenced bytes separately, validates them on
import and assigns a new ownership identity. Campaign packaging/UI remains in the
later portability task.

## Verification evidence

Focused storage tests exercise a real SQLite reopen, exact hashes, source ownership,
immutable deduplication, distinct host/guest portraits, denied private lookups and
revocation, near-limit legacy records, portable byte round-trip, atomic map receipt
selection and snapshot/timeline rejection. Browser tests render the real production
root, select built-in art, upload a portrait and reload the same face without a
second upload or base64/object URLs in preferences.

Full gates and fresh native integration are required before the plan task is
marked complete. These checks do not establish local image-model/GPU quality or
real-model narration acceptance.
