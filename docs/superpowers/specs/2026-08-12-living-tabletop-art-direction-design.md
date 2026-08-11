# Living Tabletop Art Direction

**Status:** Approved for implementation planning

**Date:** 2026-08-12

**Scope:** Product artwork, material textures, and restrained ambient motion for the existing Dungeon Master AI frontend

## Objective

Replace the current uneven arcane-tech artwork with a coherent, tactile classic-fantasy presentation. The application should feel like a living tabletop beside a hearth: inhabited, handmade, atmospheric, and calm enough for long play sessions.

This work changes the visual asset layer and its presentation. It does not redesign product flows, alter game rules, add production dependencies, or change API and persistence contracts.

## Approved Creative Direction

The selected direction is **Living Tabletop**.

- Characters are recognizable people with distinct class, ancestry, age, equipment, posture, and personality.
- Materials feel physical: worn leather, dark oak, iron, wool, parchment, soot, wax, stone, and aged bronze.
- Lighting comes from believable sources such as candles, fireplaces, dawn, moonlight, or a nearby window.
- The rendering style is painterly realism with visible artistic texture. It must avoid plastic 3D rendering and uncanny photorealistic faces.
- Warm amber remains the dominant atmospheric accent. Existing purple is reserved for actual magic and supernatural events instead of decorating every surface.
- Frames and ornaments become quieter and thinner so the subject remains primary.
- Existing UI design tokens, layout, readable contrast, and interaction conventions remain authoritative.

### Prohibited Visual Motifs

- Gears, circuit traces, magic circuit boards, or generic arcane technology.
- Generated text, watermarks, malformed heraldry, and pseudotext.
- Decorative nonsense runes. Legible magical marks may appear only where the scene calls for them and must not resemble interface text.
- Repeated faces, identical armour silhouettes, or a single body type reused across classes.
- Flat, obviously tiled surfaces and noisy detail behind readable UI content.

## Motion Direction

The selected motion intensity is **A: calm ambient life**.

Motion creates presence rather than spectacle:

- subtle candle and firelight variation;
- sparse drifting dust, smoke, or mist only on the splash, onboarding, VTT-empty, and scene-transition surfaces;
- gentle depth or parallax on large atmospheric screens only;
- soft scene, portrait, and card entrances;
- restrained hover and focus transitions using the existing interaction language.

The main chat, controls, and reading surfaces must remain visually stable. The design excludes constant camera movement, screen shake, repeated flashes, aggressive hit effects, and decorative particles across the whole application.

All non-essential motion must stop or collapse to an immediate state under `prefers-reduced-motion: reduce`. Motion uses the existing CSS and browser capabilities; no animation dependency is added.

## Replacement Scope and Sequence

Implementation is split into three independently reviewable waves. A wave is accepted before the next one replaces production assets.

### Wave 1: Product Identity

Update the most visible assets first:

- splash artwork and its poster/fallback presentation;
- onboarding hero artwork;
- empty VTT artwork;
- four active hero/class portraits and their class-card presentation;
- corresponding combat tokens.

This wave establishes the art bible: palette behavior, brush texture, lighting, character diversity, framing, crop rules, and token readability.

The legacy paladin portrait and separate `class-*.png` files have no reachable consumer in the four-class product model. They are not regenerated; they are removed only after an import scan and production build confirm that they remain unused.

### Wave 2: Inhabited World

Apply the approved art bible to:

- NPC fallback portraits;
- save-game thumbnails;
- scene and event illustrations;
- transition imagery and existing transition-video surfaces.

NPCs must read as different individuals at small sizes. Save thumbnails must communicate situation and mood without relying on embedded labels.

### Wave 3: Physical Materials

Replace or refine the material set:

- parchment;
- dark oak;
- leather;
- dungeon stone;
- bronze and iron surfaces;
- restrained magical textile or velvet used only by explicitly magical UI.

Materials must work as backgrounds rather than hero images. They need low-frequency variation, quiet seams, and enough tonal restraint to preserve text contrast. Existing untracked material candidates are reference inputs, not automatically approved production replacements.

Platform icons and unrelated native packaging artwork remain out of scope unless a later visual review identifies a concrete mismatch.

## Asset Production Contract

Each asset family receives a shared prompt profile before individual generation. The profile defines the rendering medium, lighting logic, camera distance, palette behavior, negative constraints, and crop-safe region. Individual prompts change subject matter without drifting from the family profile.

Generated candidates are written beside the current asset set under distinct candidate names. Existing files are not overwritten during exploration. After visual comparison and approval, the chosen candidate is deliberately promoted to the existing import contract or the import is changed in one explicit patch.

The implementation preserves:

- current component import boundaries and public asset usage;
- expected aspect ratios and crop behavior;
- transparent backgrounds for combat tokens;
- PNG where alpha or lossless edges are required;
- WebP for opaque raster artwork where the repository already supports it;
- current offline behavior with all production artwork bundled locally.

No remote runtime image dependency is introduced. No secret, prompt containing private user data, or generated base64 payload is persisted or logged.

## Presentation Architecture

Motion and art remain presentation concerns:

- asset modules map semantic roles such as hero, NPC, save, or VTT state to bundled files;
- reusable CSS classes apply calm entrance, ambient light, and optional depth effects;
- components continue receiving semantic asset references rather than generation details;
- `prefers-reduced-motion` is handled centrally in the relevant style layer;
- no gameplay, Zustand state, HTTP contract, or Rust boundary depends on animation state.

If a generated asset fails validation, the current production asset remains the fallback. Partial replacement of a visually coupled family is not shipped: for example, hero portraits and their matching tokens are reviewed as one set.

## Quality and Acceptance Criteria

An asset is accepted only when it passes all relevant checks:

1. **Semantic clarity:** class, NPC role, or scene reads correctly without a caption.
2. **Family coherence:** lighting, medium, detail density, and framing match related assets.
3. **Human quality:** faces, hands, equipment, heraldry, and silhouettes contain no extra or missing anatomy, fused objects, broken perspective, or malformed symbols.
4. **UI crop:** the subject survives every production crop and responsive layout where it appears.
5. **Readability:** overlays and adjacent text retain existing contrast targets.
6. **Token clarity:** transparent edges are clean and the silhouette remains readable at combat scale on both light and dark map areas.
7. **Motion restraint:** animation supports atmosphere, does not delay interaction, and becomes still under reduced motion.
8. **Performance:** each replacement records its encoded size against the previous asset; any increase requires a documented visual reason. Ambient layers cause no layout shift and keep controls responsive during the browser smoke.
9. **Safety:** no generated text, watermark, copied trademark, secret, or private content is present.

## Verification Strategy

Each wave receives focused visual and automated verification before promotion:

- inspect candidates at original resolution and at their actual UI size;
- compare related assets as a contact sheet, not only one at a time;
- render the real production root and assert Dungeon Master AI labels and default identities remain correct;
- run the relevant frontend tests, lint, TypeScript checking, and production build;
- run a browser smoke against the production application;
- capture splash, onboarding, character choice, VTT-empty, NPC memory, and saves screens at the existing Playwright viewport;
- repeat the visual smoke with reduced motion enabled;
- use the real Tauri WebView for final splash and transition verification when the wave touches desktop-only presentation.

Focused checks, full project gates, browser evidence, Tauri evidence, and CI remain separately reported. A green focused check does not imply the later gates have run.

## Delivery Boundaries

The implementation plan may refine file-level sequencing after inspecting every consumer, but it must preserve the three-wave approval boundary. Production dependencies require separate explicit user approval. Unrelated worktree changes remain untouched, and commits stage only the completed visual checkpoint.

The completed result should feel richer because the world appears inhabited and materially believable, not because every pixel moves.
