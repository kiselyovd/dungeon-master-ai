[Stable references](VISUAL_REFERENCES.md) · [Hero identities](HERO_IDENTITIES.md) · [Opponent miniatures](OPPONENT_MINIATURES.md)

# Key images and save illustrations

Three v2 key paintings serve splash, onboarding and the peaceful empty table.
Four save paintings distinguish combat, exploration, dialog and NPC categories.
The seven WebP resources total 1,293,284 bytes. Source masters, exact prompts,
generation IDs, encoding settings and consumers are in `key-save-provenance.json`.
Legacy catalogue IDs and files remain available.

The paintings are generic atmosphere. They add no campaign event, actor,
discovery, quest item or tactical geometry. Save titles, summaries, timestamps,
categories and actions still come from actual snapshot metadata. Text and
controls are rendered by the application rather than baked into an image.

Onboarding retains its wide central composition. Campaign library cards use the
peaceful landscape, while the empty library uses the welcome book painting.
The latest campaign title uses an existing smaller heading token so a long
Russian name remains readable beside its continuation action. The save detail
uses a contained full composition; the small category thumbnail can crop its
quiet edges. A failed decorative resource leaves its container background and
the owning labels/actions intact. A changed resource can recover from failure.

Splash health polling and fade remain independent of image decoding. Loading
status uses the existing English/Russian common catalogues. Reduced motion uses
the existing duration/iteration overrides; browser checks inspect whether motion
actually remains running.

## Verification

Focused component checks cover missing-art recovery, healthy startup, localization,
onboarding, save actions and the campaign library. Production-root RU/EN browser
cases cover all seven resources at 960/1280/1920 widths, long saved titles and
summaries, real actions, empty/populated library, reduced motion and failed image
requests. Full-size and actual-consumer screenshots are reviewed separately.
An initial wide save preview clipped heads despite passing decode checks; the
contained-frame fix was reviewed and retested.

Native, full checkpoint gates, live-model, GPU and physical-device evidence are
separate. The active plan records each layer when it is completed.

## Checkpoint7 native evidence

A freshly built debug Tauri application and backend passed real WebView and
file-backed QA SQLite checks. Eight generic NPC records were explicitly seeded
offline in the stopped isolated profile; six Bell portraits were revealed through
canonical UI actions. Two authored Water Echo miniatures and four current-state
snapshot category images rendered. Normal close/cold restart retained the public
identities, miniature bindings and canonical character fingerprint, including
eight generic faces after their stored roles changed. JavaScript exceptions: 0;
LLM/image runtimes remained off. The debug image-sidecar placeholder was never
invoked. Full gates passed 1,011 Rust and 1,282 frontend tests in 204 files.
Live-model, GPU, physical-LAN, DPI and packaged-release acceptance remain separate.
