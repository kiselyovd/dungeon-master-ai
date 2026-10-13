[Stable references](VISUAL_REFERENCES.md) · [Key and save art](KEY_SAVE_ART.md)

# Current Bell location paintings

Six v2 paintings depict the public atmosphere of Veresk, the old quay, the shrine,
Varden's warehouse, the flooded archive and the lower belfry. Their combined WebP
payload is 1,234,548 bytes. Full source masters, prompts, job IDs, public description
references and encoding details are in `bell-scene-provenance.json`.

The application-owned public `AdventureProjection.scene_asset_id` names only the
current canonical location. Old projections can omit it. The renderer checks that
the asset belongs to that location and to the scene-art category, so a stale or
unavailable reference cannot substitute another location's image. The paintings
add no canonical people, document contents, quest objects, legal paths or collision.

`VttCanvas` shares this presentation between solo and co-op. Without a tactical
map, the painting fills the scene with a contained composition and a localized
caption. Tactical rulers and grids are not offered for the atmosphere. Archive's
existing authoritative arena remains the main view; its painting is available in
the same current-scene preview. The preview uses the existing modal in a body
portal so the scene's stacking context cannot clip it behind chat. Scene changes
close stale previews. Missing art keeps the current title and usable controls.

## Verification

Four application tests cover current-location bindings, legacy absence and public
reveal. Actual SQLite checkpoint restoration preserves the scene binding.
Authenticated room initial-state/reconnection tests compare the current public
reference. Nineteen focused UI checks include neutral missing/wrong-location art,
preview scope reset, non-tactical room presentation and portal placement.

Six production-root browser scenarios use an isolated actual backend, real HTTP
commands and saves at RU/EN x 960/1280/1920. Each traverses all six places through
canonical UI actions, keeps Archive's map, opens the full painting, and verifies
restoration after a real save and location change. A further actual-backend case
aborts the image request and verifies the fallback and close action. Source and
actual-consumer screenshots were inspected. An initial browser failure reproduced
the preview being clipped/intercepted by chat; the portal fix passed the same path.

Production build and static gates pass. Native/full checkpoint8 acceptance is
grouped after the map and offline illustration tasks. This does not claim GPU,
live-model, physical-LAN or release validation.
