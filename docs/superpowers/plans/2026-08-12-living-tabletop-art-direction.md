# Living Tabletop Art Direction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the reachable product artwork with one coherent classic-fantasy set and add calm ambient motion without changing gameplay, transport, persistence, or production dependencies.

**Architecture:** Introduce one semantic asset registry so React components consume roles rather than file paths; keep material roles in the existing CSS custom-property contract. Promote generated candidates in three independently reviewable waves, with asset-contract tests, focused component tests, and visual browser/Tauri checks at every wave. Motion remains CSS presentation state and collapses under `prefers-reduced-motion`.

**Tech Stack:** React 19, TypeScript, CSS, Vite asset imports, Vitest, Testing Library, Playwright, Tauri WebView2, built-in `image_gen`, `ffmpeg` for local offline conversion only.

## Global Constraints

- Follow `docs/superpowers/specs/2026-08-12-living-tabletop-art-direction-design.md` and `.ai-factory/ARCHITECTURE.md`.
- Add no production dependency.
- Keep all production artwork bundled locally; do not add runtime network image URLs.
- Preserve current product flows, API contracts, Zustand persistence, game rules, and English/Russian machine contracts.
- Use existing colors, spacing, typography, radii, shadows, and interaction timing tokens.
- Use warm amber for atmosphere; reserve existing purple for explicit magic.
- Exclude gears, circuit traces, pseudotext, decorative nonsense runes, watermarks, and malformed heraldry.
- Keep the chat, controls, and reading surfaces stable; ambient particles are permitted only on splash, onboarding, VTT-empty, and scene-transition surfaces.
- Every non-essential animation must stop under `prefers-reduced-motion: reduce`.
- Generate candidates under distinct filenames and inspect them before changing a production import.
- Keep transparent combat tokens as PNG; encode opaque artwork and materials as WebP.
- Do not stage unrelated worktree changes. In particular, preserve existing edits in `package.json`, `src/styles/globals.css`, and the pre-existing untracked material candidates unless a task names an exact overlapping hunk.
- Run shell operations separately when a prior command can fail; never hide a failed command behind a chained follow-up.

## File Structure

### New files

- `src/assets/livingTabletop.ts` - semantic imports for key art, heroes, tokens, NPCs, saves, and scenes.
- `src/assets/living-tabletop/*.webp` - opaque approved artwork.
- `src/assets/living-tabletop/token-*.png` - approved transparent token crops.
- `src/assets/__tests__/livingTabletop.test.ts` - registry completeness and binary file-budget contract.
- `src/styles/living-tabletop.css` - calm ambient presentation and reduced-motion shutdown.
- `src/styles/__tests__/livingTabletop.test.ts` - presentation-boundary assertions.
- `src/components/__tests__/SplashOverlay.test.tsx` - health wait, art render, and fade behavior.
- `src/components/__tests__/SceneTransitionOverlay.test.tsx` - tag selection, still-art lifecycle, skip, and cleanup.
- `e2e/living-tabletop.spec.ts` - production-root artwork and reduced-motion browser smoke.
- `docs/visual/LIVING_TABLETOP_ASSETS.md` - final prompts, encoded sizes, and visual acceptance record.

### Modified files

- `src/main.tsx` - import the dedicated living-tabletop stylesheet after existing global styles.
- `src/lib/heroPortraits.ts` - re-export the registry hero map without direct binary imports.
- `src/components/SplashOverlay.tsx` - render layered still artwork instead of the legacy looping MP4.
- `src/components/onboarding/steps/WelcomeStep.tsx` - show the approved onboarding hero artwork.
- `src/components/onboarding/steps/HeroStep.tsx` - keep semantic class-card rendering and expose stable art hooks.
- `src/components/CombatToken.tsx` - use the registry token map.
- `src/components/VttCanvas.tsx` - use the registry VTT-empty artwork.
- `src/components/NpcMemoryGrid.tsx` - use the registry NPC map.
- `src/components/SavesScreen.tsx` - use the registry save map.
- `src/components/SceneTransitionOverlay.tsx` - render timed scene stills instead of MP4 video.
- `src/styles/overlays.css` - retain overlay layout while removing video-only declarations.
- `src/styles/onboarding.css` - integrate the welcome artwork and calmer class-card framing.
- `src/styles/vtt.css` - add restrained empty-state depth.
- `src/styles/materials.css` - point existing material roles at the approved material set.
- Existing component tests named in each task - preserve behavior while asserting semantic art roles.

### Removed after zero-consumer proof

- Legacy reachable raster/video files replaced by the new registry.
- Unused `src/assets/class-*.png`, `src/assets/char-portrait-paladin.png`, `src/assets/npc-intro.mp4`, `src/assets/ornament-line.png`, and `src/assets/parchment-texture.png`.
- Do not remove the pre-existing untracked `src/assets/materials/` directory; leave it untouched unless the user separately authorizes cleanup of untracked source material.

---

### Task 1: Establish the Semantic Asset Registry

**Files:**
- Create: `src/assets/livingTabletop.ts`
- Create: `src/assets/__tests__/livingTabletop.test.ts`
- Modify: `src/lib/heroPortraits.ts`
- Modify: `src/components/CombatToken.tsx`
- Modify: `src/components/VttCanvas.tsx`
- Modify: `src/components/NpcMemoryGrid.tsx`
- Modify: `src/components/SavesScreen.tsx`
- Modify: `src/components/SplashOverlay.tsx`
- Modify: `src/components/SceneTransitionOverlay.tsx`

**Interfaces:**
- Consumes: current tracked image and MP4 imports; `HeroClassId`; `SaveTag`; the current `SceneTag` and `NpcArchetype` unions.
- Produces: `KEY_ART`, `HERO_ART`, `TOKEN_ART`, `NPC_ART`, `SAVE_ART`, and `SCENE_ART`, all as immutable string maps.

- [ ] **Step 1: Write a failing registry-shape test**

```ts
import { describe, expect, it } from 'vitest';
import {
  HERO_ART,
  KEY_ART,
  NPC_ART,
  SAVE_ART,
  SCENE_ART,
  TOKEN_ART,
} from '../livingTabletop';

describe('living tabletop asset registry', () => {
  it('exposes every reachable semantic role', () => {
    expect(Object.keys(HERO_ART)).toEqual(['fighter', 'wizard', 'rogue', 'cleric']);
    expect(Object.keys(TOKEN_ART)).toEqual(['fighter', 'wizard', 'rogue', 'cleric']);
    expect(Object.keys(NPC_ART)).toEqual([
      'innkeeper', 'guard', 'merchant', 'rogue', 'mage', 'priestess', 'knight', 'peasant',
    ]);
    expect(Object.keys(SAVE_ART)).toEqual(['combat', 'exploration', 'dialog', 'npc']);
    expect(Object.keys(SCENE_ART)).toEqual(['combat', 'dialog', 'exploration', 'dungeon']);
    expect(Object.keys(KEY_ART)).toEqual(['splash', 'onboarding', 'vttEmpty']);
  });
});
```

- [ ] **Step 2: Run the test and verify the seam is absent**

Run: `bunx vitest run src/assets/__tests__/livingTabletop.test.ts`

Expected: FAIL because `../livingTabletop` does not exist.

- [ ] **Step 3: Implement the registry with the current assets**

Create `src/assets/livingTabletop.ts` with the existing files first, so this task changes dependency direction without changing appearance:

```ts
import heroCleric from './char-portrait-cleric.png';
import heroFighter from './char-portrait-fighter.png';
import heroRogue from './char-portrait-rogue.png';
import heroWizard from './char-portrait-wizard.png';
import npcGuard from './npc-fallback-guard.png';
import npcInnkeeper from './npc-fallback-innkeeper.png';
import npcKnight from './npc-fallback-knight.png';
import npcMage from './npc-fallback-mage.png';
import npcMerchant from './npc-fallback-merchant.png';
import npcPeasant from './npc-fallback-peasant.png';
import npcPriestess from './npc-fallback-priestess.png';
import npcRogue from './npc-fallback-rogue.png';
import onboarding from './onboarding-hero.png';
import saveCombat from './save-thumb-combat.png';
import saveDialog from './save-thumb-dialog.png';
import saveExploration from './save-thumb-exploration.png';
import saveNpc from './save-thumb-npc.png';
import sceneCombat from './scene-transition-combat.mp4';
import sceneDialog from './scene-transition-dialog.mp4';
import sceneDungeon from './scene-transition-dungeon.mp4';
import sceneExploration from './scene-transition-exploration.mp4';
import splash from './splash.png';
import tokenCleric from './token-cleric.png';
import tokenFighter from './token-fighter.png';
import tokenRogue from './token-rogue.png';
import tokenWizard from './token-wizard.png';
import vttEmpty from './vtt-empty.png';

export const KEY_ART = { splash, onboarding, vttEmpty } as const;
export const HERO_ART = { fighter: heroFighter, wizard: heroWizard, rogue: heroRogue, cleric: heroCleric } as const;
export const TOKEN_ART = { fighter: tokenFighter, wizard: tokenWizard, rogue: tokenRogue, cleric: tokenCleric } as const;
export const NPC_ART = { innkeeper: npcInnkeeper, guard: npcGuard, merchant: npcMerchant, rogue: npcRogue, mage: npcMage, priestess: npcPriestess, knight: npcKnight, peasant: npcPeasant } as const;
export const SAVE_ART = { combat: saveCombat, exploration: saveExploration, dialog: saveDialog, npc: saveNpc } as const;
export const SCENE_ART = { combat: sceneCombat, dialog: sceneDialog, exploration: sceneExploration, dungeon: sceneDungeon } as const;
```

- [ ] **Step 4: Migrate consumers without changing their public props**

Replace direct raster imports with the semantic maps. Keep `splash.mp4` as the one temporary direct import until Task 3 removes video playback. Keep the existing typed assignments in each consumer:

```ts
export const HERO_PORTRAIT: Record<HeroClassId, string> = HERO_ART;
const CLASS_TOKEN: Partial<Record<string, string>> = TOKEN_ART;
const ARCHETYPE_PORTRAIT: Record<NpcArchetype, string> = NPC_ART;
const TAG_THUMB_SRC: Record<SaveTag, string> = SAVE_ART;
const TAG_VIDEO: Record<SceneTag, string> = SCENE_ART;
```

Use `KEY_ART.splash` and `KEY_ART.vttEmpty` in the two key-art consumers. Do not add `onboarding` to JSX yet.

- [ ] **Step 5: Run the focused registry and consumer tests**

Run:

```powershell
bunx vitest run src/assets/__tests__/livingTabletop.test.ts src/components/__tests__/CombatToken.test.tsx src/components/__tests__/VttCanvas.test.tsx src/components/__tests__/NpcMemoryGrid.test.tsx src/components/__tests__/SavesScreen.test.tsx
```

Expected: all selected tests PASS with unchanged UI behavior.

- [ ] **Step 6: Commit only the seam**

```powershell
git add src/assets/livingTabletop.ts src/assets/__tests__/livingTabletop.test.ts src/lib/heroPortraits.ts src/components/CombatToken.tsx src/components/VttCanvas.tsx src/components/NpcMemoryGrid.tsx src/components/SavesScreen.tsx src/components/SplashOverlay.tsx src/components/SceneTransitionOverlay.tsx
git commit -m "refactor(ui): centralize living tabletop artwork"
```

---

### Task 2: Generate and Validate Wave 1 Product Identity Art

**Files:**
- Create: `src/assets/living-tabletop/splash.webp`
- Create: `src/assets/living-tabletop/onboarding.webp`
- Create: `src/assets/living-tabletop/vtt-empty.webp`
- Create: `src/assets/living-tabletop/hero-fighter.webp`
- Create: `src/assets/living-tabletop/hero-wizard.webp`
- Create: `src/assets/living-tabletop/hero-rogue.webp`
- Create: `src/assets/living-tabletop/hero-cleric.webp`
- Create: `src/assets/living-tabletop/token-fighter.png`
- Create: `src/assets/living-tabletop/token-wizard.png`
- Create: `src/assets/living-tabletop/token-rogue.png`
- Create: `src/assets/living-tabletop/token-cleric.png`
- Modify: `src/assets/__tests__/livingTabletop.test.ts`
- Create: `docs/visual/LIVING_TABLETOP_ASSETS.md`

**Interfaces:**
- Consumes: the approved Living Tabletop style; the four `PRESETS` identities in `src/state/pc.ts`; built-in `image_gen`; local `ffmpeg` confirmed available for authoring.
- Produces: eleven reviewed Wave 1 files at fixed production dimensions, plus a recorded prompt/size ledger.

- [ ] **Step 1: Add a failing file-budget test for Wave 1**

Append this exact contract to `src/assets/__tests__/livingTabletop.test.ts`:

```ts
import { statSync } from 'node:fs';
import { resolve } from 'node:path';

const WAVE_1: ReadonlyArray<readonly [string, number]> = [
  ['splash.webp', 900_000],
  ['onboarding.webp', 900_000],
  ['vtt-empty.webp', 800_000],
  ['hero-fighter.webp', 600_000],
  ['hero-wizard.webp', 600_000],
  ['hero-rogue.webp', 600_000],
  ['hero-cleric.webp', 600_000],
  ['token-fighter.png', 1_000_000],
  ['token-wizard.png', 1_000_000],
  ['token-rogue.png', 1_000_000],
  ['token-cleric.png', 1_000_000],
];

it.each(WAVE_1)('%s exists within its encoded budget', (name, maxBytes) => {
  const file = resolve(__dirname, `../living-tabletop/${name}`);
  const bytes = statSync(file).size;
  expect(bytes).toBeGreaterThan(10_000);
  expect(bytes).toBeLessThanOrEqual(maxBytes);
});
```

- [ ] **Step 2: Run the test and verify all eleven files are missing**

Run: `bunx vitest run src/assets/__tests__/livingTabletop.test.ts`

Expected: FAIL with `ENOENT` for `splash.webp` first.

- [ ] **Step 3: Generate the three key-art candidates**

Call built-in `image_gen` separately for each prompt and save the returned PNGs outside the final filenames until inspection:

```text
Splash prompt: Wide classic high-fantasy tabletop illustration, a worn oak gaming table beside a stone hearth at night, open hand-drawn adventure map, four distinct adventurer miniatures, leather journal, dice, sealing wax and one steady candle, calm anticipation before a journey, painterly oil and gouache, visible brushwork, believable materials, deep shadow with warm amber firelight, restrained violet only as a tiny magical reflection, cinematic 16:9 composition, central crop-safe area, no people facing camera, no text, no letters, no logo, no watermark, no gears, no circuit patterns, no decorative runes.

Onboarding prompt: Wide classic-fantasy welcome illustration, four distinct adventurers gathered around a real wooden table as a human dungeon master opens an old map, human fighter with longsword and shield, high-elf wizard with quarterstaff and spellbook, halfling rogue with shortbow, hill-dwarf cleric with mace and shield, warm hearth and window moonlight, inviting companionship, painterly oil and gouache, visible brushwork, natural anatomy, worn cloth and leather, calm composition with clear negative space for interface copy, no text, no watermark, no gears, no circuit traces, no meaningless runes.

VTT-empty prompt: Top-down classic fantasy cartographer's desk, unmarked aged parchment map centered on dark oak, compass, charcoal, a few dice and wax seal near the edges, candlelight fading into quiet shadow, painterly handmade realism, generous empty center for interface labels, no readable writing, no labels, no watermark, no gears, no circuit motifs, no glowing runes, 4:3 composition.
```

- [ ] **Step 4: Generate four distinct hero portraits**

Use one call per prompt, square composition, head-and-torso crop, eyes readable at card size, and the shared suffix: `classic high-fantasy painterly oil and gouache portrait, visible brushwork, natural anatomy, worn practical materials, warm directional light, quiet dark background, no text, no watermark, no gears, no circuitry, no decorative runes, no plastic 3D rendering`.

```text
hero-fighter: seasoned human woman fighter and soldier, weathered face, practical mail and quilted gambeson, longsword and scarred wooden shield, calm protective posture.
hero-wizard: older high-elf man wizard and sage, intelligent lined face, travel-stained layered robes, ash quarterstaff and closed spellbook, one restrained violet spell-glow reflected in his eyes.
hero-rogue: young adult halfling woman rogue and former criminal, compact athletic build, practical dark wool and leather, shortbow across her shoulder and shortsword hilt visible, alert half-smile, no hood hiding the face.
hero-cleric: middle-aged hill-dwarf man cleric and acolyte, broad kind face, braided beard, worn scale armour, iron mace and battered shield with a simple non-text holy sun emblem, warm candlelight.
```

- [ ] **Step 5: Derive matching transparent combat tokens from the approved portraits**

For each approved portrait call `image_gen` in edit mode with that portrait as the referenced image:

```text
Preserve this exact character's face, ancestry, age, clothing, armour and equipment. Recompose as a top-down three-quarter tabletop combat token, complete readable silhouette inside a circular crop-safe area, neutral grounded stance, painterly edge detail matching the source portrait, isolated subject on transparent background, no ring, no text, no shadow outside the silhouette, no extra weapons, no watermark.
```

Reject and regenerate any token whose face, ancestry, dominant clothing color, or equipment no longer matches its portrait.

- [ ] **Step 6: Inspect before conversion**

Use `view_image` on all eleven candidates at original detail. Reject any extra digits, fused equipment, broken weapon geometry, fake writing, watermark, repeated face, unreadable token silhouette, or purple light unrelated to the wizard.

- [ ] **Step 7: Convert approved opaque candidates to exact production dimensions**

Run one explicit command per candidate, changing input and output names while keeping the target dimensions below:

```powershell
ffmpeg -i .artifacts/living-tabletop/candidates/splash.png -vf "scale=1376:768:force_original_aspect_ratio=increase,crop=1376:768" -frames:v 1 -c:v libwebp -quality 86 -compression_level 6 src/assets/living-tabletop/splash.webp
ffmpeg -i .artifacts/living-tabletop/candidates/onboarding.png -vf "scale=1312:816:force_original_aspect_ratio=increase,crop=1312:816" -frames:v 1 -c:v libwebp -quality 86 -compression_level 6 src/assets/living-tabletop/onboarding.webp
ffmpeg -i .artifacts/living-tabletop/candidates/vtt-empty.png -vf "scale=1200:896:force_original_aspect_ratio=increase,crop=1200:896" -frames:v 1 -c:v libwebp -quality 86 -compression_level 6 src/assets/living-tabletop/vtt-empty.webp
```

For each hero use `scale=1024:1024:force_original_aspect_ratio=increase,crop=1024:1024` and WebP quality 86. For each token preserve RGBA and write PNG at 1024 by 1024.

- [ ] **Step 8: Record prompts and size evidence**

Create `docs/visual/LIVING_TABLETOP_ASSETS.md` with a row per asset: semantic role, final path, exact prompt, source candidate path, pixel dimensions, old bytes, new bytes, QA result, and approval date. Do not record base64 data or secrets.

- [ ] **Step 9: Run the Wave 1 file contract**

Run: `bunx vitest run src/assets/__tests__/livingTabletop.test.ts`

Expected: PASS for all eleven file and registry assertions.

- [ ] **Step 10: Commit the approved Wave 1 binaries and ledger**

```powershell
git add src/assets/living-tabletop/splash.webp src/assets/living-tabletop/onboarding.webp src/assets/living-tabletop/vtt-empty.webp src/assets/living-tabletop/hero-fighter.webp src/assets/living-tabletop/hero-wizard.webp src/assets/living-tabletop/hero-rogue.webp src/assets/living-tabletop/hero-cleric.webp src/assets/living-tabletop/token-fighter.png src/assets/living-tabletop/token-wizard.png src/assets/living-tabletop/token-rogue.png src/assets/living-tabletop/token-cleric.png src/assets/__tests__/livingTabletop.test.ts docs/visual/LIVING_TABLETOP_ASSETS.md
git commit -m "feat(art): add living tabletop product identity"
```

---

### Task 3: Integrate Wave 1 and Calm Ambient Motion

**Files:**
- Modify: `src/assets/livingTabletop.ts`
- Modify: `src/main.tsx`
- Create: `src/styles/living-tabletop.css`
- Create: `src/styles/__tests__/livingTabletop.test.ts`
- Modify: `src/components/SplashOverlay.tsx`
- Create: `src/components/__tests__/SplashOverlay.test.tsx`
- Modify: `src/components/onboarding/steps/WelcomeStep.tsx`
- Modify: `src/components/onboarding/steps/HeroStep.tsx`
- Modify: `src/styles/onboarding.css`
- Modify: `src/styles/vtt.css`
- Modify: `src/components/onboarding/__tests__/HeroStep.test.tsx`
- Modify: `src/components/__tests__/VttCanvas.test.tsx`
- Modify: `src/components/__tests__/CombatToken.test.tsx`
- Modify: `e2e/onboarding.spec.ts`
- Modify: `e2e/smoke.spec.ts`

**Interfaces:**
- Consumes: Wave 1 paths from Task 2 and semantic maps from Task 1.
- Produces: `data-art-direction="living-tabletop"` hooks on the four large atmospheric surfaces and CSS animations limited to those hooks.

- [ ] **Step 1: Write failing component and CSS tests**

Add these assertions before changing production JSX:

```tsx
expect(screen.getByTestId('onboarding-hero-art')).toHaveAttribute('src', expect.stringContaining('onboarding'));
expect(container.querySelector('.dm-vtt-empty')).toHaveAttribute('data-art-direction', 'living-tabletop');
const pcToken = screen.getByTestId('combat-token-pc');
expect((pcToken.firstElementChild as HTMLElement).style.backgroundImage).toContain('token-fighter');
```

Create `src/styles/__tests__/livingTabletop.test.ts`:

```ts
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

describe('living tabletop motion boundary', () => {
  const css = readFileSync(resolve(__dirname, '../living-tabletop.css'), 'utf8');

  it('limits ambient animation to approved surfaces', () => {
    expect(css).toContain('[data-art-direction="living-tabletop"]');
    expect(css).not.toContain('.dm-chat-panel');
    expect(css).not.toContain('.dm-composer');
  });

  it('stops non-essential motion for reduced-motion users', () => {
    expect(css).toContain('@media (prefers-reduced-motion: reduce)');
    expect(css).toContain('animation: none !important');
    expect(css).toContain('transform: none !important');
  });
});
```

- [ ] **Step 2: Run tests and verify new art hooks are absent**

Run:

```powershell
bunx vitest run src/styles/__tests__/livingTabletop.test.ts src/components/onboarding/__tests__/HeroStep.test.tsx src/components/__tests__/VttCanvas.test.tsx src/components/__tests__/CombatToken.test.tsx
```

Expected: FAIL on the missing stylesheet or new selectors.

- [ ] **Step 3: Promote Wave 1 imports in the registry**

Replace only the Wave 1 imports in `livingTabletop.ts`:

```ts
import heroCleric from './living-tabletop/hero-cleric.webp';
import heroFighter from './living-tabletop/hero-fighter.webp';
import heroRogue from './living-tabletop/hero-rogue.webp';
import heroWizard from './living-tabletop/hero-wizard.webp';
import onboarding from './living-tabletop/onboarding.webp';
import splash from './living-tabletop/splash.webp';
import tokenCleric from './living-tabletop/token-cleric.png';
import tokenFighter from './living-tabletop/token-fighter.png';
import tokenRogue from './living-tabletop/token-rogue.png';
import tokenWizard from './living-tabletop/token-wizard.png';
import vttEmpty from './living-tabletop/vtt-empty.webp';
```

- [ ] **Step 4: Replace the splash video with a layered still**

Keep the health polling and fade lifecycle unchanged. Replace only the `<video>` block:

```tsx
<div className="dm-splash-art" data-art-direction="living-tabletop">
  <img src={KEY_ART.splash} alt="" className="dm-splash-image" />
  <span className="dm-ambient-light" aria-hidden="true" />
  <span className="dm-ambient-dust" aria-hidden="true" />
</div>
```

The outer `role="status"`, `aria-live`, and `aria-label="Loading"` remain intact.

- [ ] **Step 5: Add onboarding and VTT semantic hooks**

In `WelcomeStep.tsx` render the art before the tag:

```tsx
import { KEY_ART } from '../../../assets/livingTabletop';

<div className="dm-onboarding-hero" data-art-direction="living-tabletop">
  <img src={KEY_ART.onboarding} alt="" data-testid="onboarding-hero-art" />
  <span className="dm-ambient-light" aria-hidden="true" />
</div>
```

Add `data-art-direction="living-tabletop"` to `.dm-vtt-empty` and `.dm-hero-cards`. Preserve all visible strings and button names.

- [ ] **Step 6: Add the restrained motion stylesheet**

Import `./styles/living-tabletop.css` from `src/main.tsx` after `globals.css` and `combat.css`. Define three named effects only:

```css
@keyframes dm-ambient-breathe {
  0%, 100% { transform: scale(1); filter: brightness(0.98); }
  50% { transform: scale(1.012); filter: brightness(1.02); }
}

@keyframes dm-ambient-drift {
  from { transform: translate3d(-1%, 1%, 0); opacity: 0.16; }
  to { transform: translate3d(1%, -1%, 0); opacity: 0.28; }
}

@keyframes dm-art-enter {
  from { opacity: 0; transform: translateY(var(--space-2)); }
  to { opacity: 1; transform: translateY(0); }
}

[data-art-direction="living-tabletop"] .dm-ambient-light { animation: dm-ambient-breathe 9s ease-in-out infinite; }
[data-art-direction="living-tabletop"] .dm-ambient-dust { animation: dm-ambient-drift 14s ease-in-out infinite alternate; }

@media (prefers-reduced-motion: reduce) {
  [data-art-direction="living-tabletop"],
  [data-art-direction="living-tabletop"] * {
    animation: none !important;
    transform: none !important;
  }
}
```

Use `color-mix()` with existing color tokens for ambient overlays; do not add a new hex color.

- [ ] **Step 7: Implement splash lifecycle coverage**

Mock `backendUrl`, `fetch`, and fake timers. Assert the new still is rendered while waiting, `.is-fading` appears after a successful health response, and the overlay unmounts after `FADE_OUT_MS`.

- [ ] **Step 8: Run focused Wave 1 integration checks**

Run:

```powershell
bunx vitest run src/assets/__tests__/livingTabletop.test.ts src/styles/__tests__/livingTabletop.test.ts src/components/__tests__/SplashOverlay.test.tsx src/components/onboarding/__tests__/HeroStep.test.tsx src/components/__tests__/VttCanvas.test.tsx src/components/__tests__/CombatToken.test.tsx
bun run typecheck
bun run build
```

Expected: all commands PASS.

- [ ] **Step 9: Run browser smoke for the two reachable Wave 1 states**

Extend onboarding and smoke tests to assert `onboarding-hero-art` and `.dm-vtt-empty-art` are visible, then run:

```powershell
bun run e2e -- e2e/onboarding.spec.ts e2e/smoke.spec.ts
```

Capture screenshots of welcome, hero choice, and empty VTT into `.artifacts/living-tabletop/wave-1/` for review; do not commit transient screenshots.

- [ ] **Step 10: Commit the Wave 1 integration**

```powershell
git add src/assets/livingTabletop.ts src/main.tsx src/styles/living-tabletop.css src/styles/__tests__/livingTabletop.test.ts src/components/SplashOverlay.tsx src/components/__tests__/SplashOverlay.test.tsx src/components/onboarding/steps/WelcomeStep.tsx src/components/onboarding/steps/HeroStep.tsx src/styles/onboarding.css src/styles/vtt.css src/components/onboarding/__tests__/HeroStep.test.tsx src/components/__tests__/VttCanvas.test.tsx src/components/__tests__/CombatToken.test.tsx e2e/onboarding.spec.ts e2e/smoke.spec.ts
git commit -m "feat(ui): bring the living tabletop identity to life"
```

---

### Task 4: Generate and Validate Wave 2 Inhabited-World Art

**Files:**
- Create: `src/assets/living-tabletop/npc-*.webp` (8 files)
- Create: `src/assets/living-tabletop/save-*.webp` (4 files)
- Create: `src/assets/living-tabletop/scene-*.webp` (4 files)
- Modify: `src/assets/__tests__/livingTabletop.test.ts`
- Modify: `docs/visual/LIVING_TABLETOP_ASSETS.md`

**Interfaces:**
- Consumes: Wave 1 art bible and encoded-size ledger.
- Produces: sixteen reviewed Wave 2 opaque WebP files.

- [ ] **Step 1: Add failing file-budget rows**

Add eight `npc-*.webp` rows at 550,000 bytes each, four `save-*.webp` rows at 700,000 bytes each, and four `scene-*.webp` rows at 1,000,000 bytes each to the same parameterized test. Run it and expect `ENOENT`.

- [ ] **Step 2: Generate eight NPC portraits**

Use this shared suffix for every square portrait: `individual classic-fantasy NPC portrait, painterly oil and gouache, visible brushwork, believable face and hands, lived-in clothing, warm local light, simple dark environmental background, readable at 96 pixels, no text, no watermark, no gears, no circuit patterns, no decorative runes, no plastic 3D render`.

Generate these exact subjects separately:

```text
innkeeper: broad middle-aged human innkeeper woman, flour on rolled sleeves, keys at belt, welcoming but observant expression, tavern hearth.
guard: tired young human city guard man, plain kettle helmet and patched blue wool tabard, spear held safely upright, rainy gatehouse light.
merchant: elderly tiefling merchant woman with small swept horns, layered traveling coat, brass scales and cloth sample, shrewd friendly gaze, market awning.
rogue: lean half-elf information broker man, cropped hair, weathered dark coat, one concealed dagger hilt, watchful profile, alley lantern.
mage: older human hedge-mage woman, silver-streaked hair, ink-stained fingers, practical robe and wooden wand, restrained violet light from one glass charm.
priestess: young adult dwarf priestess woman, braided hair, cream wool vestments, simple iron sun pendant, compassionate steady gaze, shrine candles.
knight: dark-skinned human knight woman in scratched plate and red wool cloak, helmet under one arm, disciplined calm, castle dawn.
peasant: older halfling farmer man, sun-lined face, coarse linen and patched vest, small basket of herbs, open countryside light.
```

- [ ] **Step 3: Generate four save illustrations**

Each is 1280:832 crop-safe, with action concentrated away from the bottom metadata strip:

```text
combat: adventuring party defending a torchlit stone bridge from goblin raiders, readable silhouettes, no gore, warm fire against cool night.
exploration: four adventurers crossing a misty pine valley toward distant ruined towers at dawn, map and travel gear visible.
dialog: tense but peaceful negotiation at a wooden tavern table between adventurers and a guarded local noble, expressive faces and hands.
npc: quiet reunion with a familiar innkeeper beside a hearth, shared relief, small personal details from the innkeeper portrait.
```

Apply the same classic-fantasy painterly constraints and exclude embedded labels.

- [ ] **Step 4: Generate four scene-transition stills**

Each is a wide 16:9 environmental composition with no central UI text baked into the image:

```text
combat: abandoned shield and drawn sword on wet flagstones as torchlight approaches through smoke, imminent battle without gore.
dialog: two occupied chairs facing each other across a candlelit table, sealed letter between them, quiet tension.
exploration: old road leaving a village at sunrise, distant mountains, birds and moving cloud shadows, hopeful scale.
dungeon: descending stone stair into an ancient crypt, lantern glow, damp walls and hanging dust, no visible writing or runes.
```

- [ ] **Step 5: Inspect and encode Wave 2**

Use `view_image` at original detail. Encode each NPC with this command shape and its exact input/output basename:

```powershell
ffmpeg -i .artifacts/living-tabletop/candidates/npc-innkeeper.png -vf "scale=1024:1024:force_original_aspect_ratio=increase,crop=1024:1024" -frames:v 1 -c:v libwebp -quality 86 -compression_level 6 src/assets/living-tabletop/npc-innkeeper.webp
```

Repeat for `guard`, `merchant`, `rogue`, `mage`, `priestess`, `knight`, and `peasant`. Encode each save with `scale=1280:832:force_original_aspect_ratio=increase,crop=1280:832` and each scene with `scale=1920:1080:force_original_aspect_ratio=increase,crop=1920:1080`, WebP quality 86 and compression level 6. Inspect every encoded file again because crop can reveal defects.

- [ ] **Step 6: Update the ledger and run the file contract**

Record exact prompts, dimensions, previous bytes, new bytes, and QA outcomes. Run:

```powershell
bunx vitest run src/assets/__tests__/livingTabletop.test.ts
```

Expected: PASS for Wave 1 and Wave 2.

- [ ] **Step 7: Commit Wave 2 assets**

```powershell
git add src/assets/living-tabletop/npc-*.webp src/assets/living-tabletop/save-*.webp src/assets/living-tabletop/scene-*.webp src/assets/__tests__/livingTabletop.test.ts docs/visual/LIVING_TABLETOP_ASSETS.md
git commit -m "feat(art): populate the living tabletop world"
```

---

### Task 5: Integrate Wave 2 and Replace Video Transitions with Calm Stills

**Files:**
- Modify: `src/assets/livingTabletop.ts`
- Modify: `src/components/NpcMemoryGrid.tsx`
- Modify: `src/components/SavesScreen.tsx`
- Modify: `src/components/SceneTransitionOverlay.tsx`
- Create: `src/components/__tests__/SceneTransitionOverlay.test.tsx`
- Modify: `src/styles/overlays.css`
- Modify: `src/components/__tests__/NpcMemoryGrid.test.tsx`
- Modify: `src/components/__tests__/SavesScreen.test.tsx`
- Modify: `src/styles/living-tabletop.css`
- Create: `e2e/living-tabletop.spec.ts`

**Interfaces:**
- Consumes: Wave 2 files and registry maps.
- Produces: still-image scene transitions with `DISPLAY_MS = 3_200`, the existing `FADE_OUT_MS = 280`, click/Escape skip, and timer cleanup.

- [ ] **Step 1: Write failing Wave 2 component tests**

Assert NPC and save images resolve through their semantic filenames. For the transition overlay, use fake timers and assert:

```tsx
expect(screen.getByTestId('scene-transition-art')).toHaveAttribute('src', expect.stringContaining('scene-combat'));
act(() => vi.advanceTimersByTime(3_200));
expect(screen.getByRole('dialog')).toHaveClass('is-fading');
act(() => vi.advanceTimersByTime(280));
expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
```

Also cover Escape, the skip button, the 30-second scene-change debounce, disabled settings, and unmount timer cleanup.

- [ ] **Step 2: Run focused tests and verify legacy filenames still render**

Run:

```powershell
bunx vitest run src/components/__tests__/NpcMemoryGrid.test.tsx src/components/__tests__/SavesScreen.test.tsx src/components/__tests__/SceneTransitionOverlay.test.tsx
```

Expected: FAIL on missing still-art behavior.

- [ ] **Step 3: Promote Wave 2 registry imports**

Change the eight NPC, four save, and four scene imports to `./living-tabletop/*.webp`. Keep the registry keys unchanged.

- [ ] **Step 4: Convert the transition component to still art**

Rename `TAG_VIDEO` to `TAG_ART`, add `const DISPLAY_MS = 3_200`, and replace the video element with:

```tsx
<div className="dm-scene-transition-art" data-art-direction="living-tabletop">
  <img className="dm-scene-transition-image" src={TAG_ART[activeTag]} alt="" data-testid="scene-transition-art" />
  <span className="dm-ambient-light" aria-hidden="true" />
  <span className="dm-ambient-dust" aria-hidden="true" />
</div>
```

Start `fadeTimer` whenever `activeTag` becomes non-null. At 3,200ms set `fading`, then use the existing 280ms unmount timer. Clear both timers on skip, scene replacement, and unmount. Keep the dialog label, click-to-dismiss, Escape handling, skip label, and debounce unchanged.

- [ ] **Step 5: Update overlay CSS without introducing new UI tokens**

Rename video-specific selectors to `.dm-scene-transition-art` and `.dm-scene-transition-image`; use `object-fit: cover`, existing z-index variables, and `var(--t-slow)` for fade timing. Apply only `dm-ambient-breathe` and `dm-ambient-drift` from the dedicated stylesheet.

- [ ] **Step 6: Run Wave 2 focused checks and production build**

Run:

```powershell
bunx vitest run src/assets/__tests__/livingTabletop.test.ts src/components/__tests__/NpcMemoryGrid.test.tsx src/components/__tests__/SavesScreen.test.tsx src/components/__tests__/SceneTransitionOverlay.test.tsx src/styles/__tests__/livingTabletop.test.ts
bun run typecheck
bun run build
```

Expected: all commands PASS and no imported MP4 remains in these components.

- [ ] **Step 7: Browser-check NPC, saves, and a deterministic scene change**

Use the existing Tauri/browser fixtures to open the NPC and saves overlays. Add a deterministic mocked scene update to `e2e/living-tabletop.spec.ts`, assert the correct `scene-combat.webp` image appears, and verify reduced motion reports no running ambient animation.

- [ ] **Step 8: Commit Wave 2 integration**

```powershell
git add src/assets/livingTabletop.ts src/components/NpcMemoryGrid.tsx src/components/SavesScreen.tsx src/components/SceneTransitionOverlay.tsx src/components/__tests__/SceneTransitionOverlay.test.tsx src/styles/overlays.css src/components/__tests__/NpcMemoryGrid.test.tsx src/components/__tests__/SavesScreen.test.tsx src/styles/living-tabletop.css e2e/living-tabletop.spec.ts
git commit -m "feat(ui): animate an inhabited tabletop world"
```

---

### Task 6: Generate and Integrate Wave 3 Physical Materials

**Files:**
- Create: `src/assets/living-tabletop/material-leather.webp`
- Create: `src/assets/living-tabletop/material-stone.webp`
- Create: `src/assets/living-tabletop/material-oak.webp`
- Create: `src/assets/living-tabletop/material-parchment.webp`
- Create: `src/assets/living-tabletop/material-velvet.webp`
- Create: `src/assets/living-tabletop/material-bronze.webp`
- Modify: `src/assets/__tests__/livingTabletop.test.ts`
- Modify: `src/styles/materials.css`
- Create: `src/styles/__tests__/materials.test.ts`
- Modify: `docs/visual/LIVING_TABLETOP_ASSETS.md`

**Interfaces:**
- Consumes: existing material-role CSS variables and the pre-existing untracked textures only as visual baseline evidence.
- Produces: six seamless 1024 by 1024 WebP textures, each at or below 450,000 bytes, and unchanged CSS variable names.

- [ ] **Step 1: Add failing material contracts**

Add six 450,000-byte file-budget rows. Create `src/styles/__tests__/materials.test.ts`:

```ts
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

describe('living tabletop materials', () => {
  const css = readFileSync(resolve(__dirname, '../materials.css'), 'utf8');

  it.each(['leather', 'stone', 'oak', 'parchment', 'velvet', 'bronze'])(
    'maps the %s semantic material',
    (name) => expect(css).toContain(`--dm-texture-${name}`),
  );

  it('retains reduced-transparency fallback', () => {
    expect(css).toContain('@media (prefers-reduced-transparency: reduce)');
    expect(css).toContain('background-image: none');
  });
});
```

- [ ] **Step 2: Generate six seamless material candidates**

Call `image_gen` separately with these exact prompts:

```text
leather: seamless square macro texture of very dark worn vegetable-tanned leather, subtle pores, shallow creases and hand-rubbed edges, nearly black brown, low contrast, no stitches, no symbols, no objects, no vignette, tileable in both axes, realistic painterly material study.
stone: seamless square texture of old dungeon limestone blocks, irregular hand-cut joints, soot and slight damp variation, low relief, charcoal gray-brown, low contrast, no moss clumps, no objects, no symbols, orthographic tileable material study.
oak: seamless square texture of dark hand-planed oak boards, believable long grain, restrained knots, matte wax finish, warm brown, low contrast, no metal fittings, no objects, no symbols, orthographic tileable material study.
parchment: seamless square texture of aged handmade parchment, warm cream fibers, faint edge-free mottling and subtle handling marks, low contrast, no borders, no writing, no stains shaped like symbols, orthographic tileable material study.
velvet: seamless square texture of deep plum-black wool velvet reserved for magical controls, soft directional nap, extremely restrained violet undertone, low contrast, no embroidery, no symbols, no folds that create a focal object, orthographic tileable material study.
bronze: seamless square texture of aged brushed bronze, fine directional scratches, darkened patina in shallow variation, muted warm metal, low contrast, no rivets, no engraving, no symbols, orthographic tileable material study.
```

- [ ] **Step 3: Inspect 2 by 2 tiling behavior before promotion**

Build a 2 by 2 QA image for each candidate with `ffmpeg`, then inspect it with `view_image`:

```powershell
ffmpeg -i .artifacts/living-tabletop/candidates/material-leather.png -filter_complex "[0:v]split=4[a][b][c][d];[a][b]hstack[top];[c][d]hstack[bottom];[top][bottom]vstack" -frames:v 1 .artifacts/living-tabletop/candidates/material-leather-tile-check.png
```

Repeat for `stone`, `oak`, `parchment`, `velvet`, and `bronze`. Reject visible seams, isolated focal marks, high-contrast knots, readable shapes, or texture frequencies that compete with 12px UI text.

- [ ] **Step 4: Encode and promote materials**

Convert each approved candidate to 1024 by 1024 WebP quality 82. In `materials.css`, keep the existing `--dm-texture-*` variable names and change URLs to the six new `../assets/living-tabletop/material-*.webp` files. Preserve all unrelated user edits in the file.

- [ ] **Step 5: Run material, registry, and stylesheet tests**

Run:

```powershell
bunx vitest run src/assets/__tests__/livingTabletop.test.ts src/styles/__tests__/materials.test.ts src/styles/__tests__/livingTabletop.test.ts
bun run build
```

Expected: PASS; the built CSS resolves all six new material URLs.

- [ ] **Step 6: Browser-check tiled surfaces at 100% and 125% scale**

Inspect titlebar, chat, VTT background, action bar, parchment surface, and bronze controls. Confirm no seam is visible and text remains readable. Repeat with reduced transparency enabled.

- [ ] **Step 7: Update ledger and commit Wave 3**

```powershell
git add src/assets/living-tabletop/material-*.webp src/assets/__tests__/livingTabletop.test.ts src/styles/materials.css src/styles/__tests__/materials.test.ts docs/visual/LIVING_TABLETOP_ASSETS.md
git commit -m "feat(ui): apply tactile living tabletop materials"
```

---

### Task 7: Remove Proven-Dead Legacy Art and Lock Browser Acceptance

**Files:**
- Remove: replaced tracked files under `src/assets/` after zero-consumer proof
- Modify: `e2e/living-tabletop.spec.ts`
- Modify: `src/app/__tests__/productionRoot.test.tsx`
- Modify: `docs/visual/LIVING_TABLETOP_ASSETS.md`

**Interfaces:**
- Consumes: all three promoted registry waves.
- Produces: zero reachable legacy binary imports, deterministic browser coverage, and an auditable old-to-new asset ledger.

- [ ] **Step 1: Prove every legacy file is unreachable**

Run an explicit import scan before deletion:

```powershell
rg -n "char-portrait-|class-(fighter|wizard|rogue|cleric)|npc-fallback-|save-thumb-|scene-transition-.*\.mp4|splash\.(png|mp4)|token-(fighter|wizard|rogue|cleric)|vtt-empty\.png|npc-intro|ornament-line|parchment-texture" src e2e
```

Expected: no production import remains. References in the ledger are allowed only when clearly marked `legacy path`.

- [ ] **Step 2: Remove only the tracked legacy files**

Use these explicit tracked groups. Stop if any command names a path that the import scan still reports. Do not touch `src/assets/materials/` because it began as untracked user work.

```powershell
git rm -- src/assets/char-portrait-cleric.png src/assets/char-portrait-fighter.png src/assets/char-portrait-paladin.png src/assets/char-portrait-rogue.png src/assets/char-portrait-wizard.png
git rm -- src/assets/class-cleric.png src/assets/class-fighter.png src/assets/class-rogue.png src/assets/class-wizard.png
git rm -- src/assets/token-cleric.png src/assets/token-fighter.png src/assets/token-rogue.png src/assets/token-wizard.png
git rm -- src/assets/npc-fallback-guard.png src/assets/npc-fallback-innkeeper.png src/assets/npc-fallback-knight.png src/assets/npc-fallback-mage.png src/assets/npc-fallback-merchant.png src/assets/npc-fallback-peasant.png src/assets/npc-fallback-priestess.png src/assets/npc-fallback-rogue.png
git rm -- src/assets/save-thumb-combat.png src/assets/save-thumb-dialog.png src/assets/save-thumb-exploration.png src/assets/save-thumb-npc.png
git rm -- src/assets/scene-transition-combat.mp4 src/assets/scene-transition-dialog.mp4 src/assets/scene-transition-dungeon.mp4 src/assets/scene-transition-exploration.mp4
git rm -- src/assets/splash.mp4 src/assets/splash.png src/assets/onboarding-hero.png src/assets/vtt-empty.png
git rm -- src/assets/npc-intro.mp4 src/assets/ornament-line.png src/assets/parchment-texture.png
```

- [ ] **Step 3: Extend production-root assertions**

In `productionRoot.test.tsx`, keep `DUNGEON MASTER AI`, `Untitled Campaign`, `The Adventure`, and composer assertions. Add:

```tsx
expect(container.querySelector('.dm-vtt-empty-art')).toHaveAttribute(
  'src',
  expect.stringContaining('vtt-empty'),
);
expect(container.querySelector('[data-art-direction="living-tabletop"]')).toBeInTheDocument();
```

- [ ] **Step 4: Complete deterministic browser coverage**

`e2e/living-tabletop.spec.ts` must contain four independent tests:

1. onboarding welcome and four class portraits are visible;
2. empty VTT uses the Living Tabletop art and retains product labels;
3. NPC, saves, and a mocked combat scene use their correct semantic images;
4. `page.emulateMedia({ reducedMotion: 'reduce' })` yields no running animation on `[data-art-direction="living-tabletop"]` descendants.

Use stable roles/test ids; do not assert hashed Vite URLs in browser tests.

- [ ] **Step 5: Run the cleanup gate**

Run:

```powershell
bunx vitest run src/app/__tests__/productionRoot.test.tsx src/assets/__tests__/livingTabletop.test.ts src/styles/__tests__/livingTabletop.test.ts src/styles/__tests__/materials.test.ts src/components/__tests__/SplashOverlay.test.tsx src/components/__tests__/SceneTransitionOverlay.test.tsx src/components/__tests__/CombatToken.test.tsx src/components/__tests__/VttCanvas.test.tsx src/components/__tests__/NpcMemoryGrid.test.tsx src/components/__tests__/SavesScreen.test.tsx
bun run lint
bun run typecheck
bun run build
bun run e2e -- e2e/living-tabletop.spec.ts e2e/onboarding.spec.ts e2e/smoke.spec.ts
```

Expected: every command PASS and the build emits no missing-asset error.

- [ ] **Step 6: Record deletion evidence and commit**

Append the deleted legacy path and its replacement path to the ledger. Stage exact files and run:

```powershell
git add e2e/living-tabletop.spec.ts src/app/__tests__/productionRoot.test.tsx docs/visual/LIVING_TABLETOP_ASSETS.md
git commit -m "chore(art): retire legacy arcane-tech assets"
```

---

### Task 8: Full Gates, Real Browser/Tauri Evidence, and Handoff

**Files:**
- Modify only if verification reveals a task-scoped defect; otherwise no production file changes.
- Update: `docs/visual/LIVING_TABLETOP_ASSETS.md` with final evidence commands and dates.

**Interfaces:**
- Consumes: the completed three-wave implementation.
- Produces: separated focused, full-gate, browser, Tauri, and CI evidence.

- [ ] **Step 1: Run the complete local gate**

Run: `bun run gates`

Expected: architecture, formatting, Biome, TypeScript, Clippy, Rust tests, and Vitest all PASS. Report existing non-failing warnings separately.

- [ ] **Step 2: Run the full Chromium suite**

Run: `bun run e2e`

Expected: all Playwright tests PASS serially against the fixture backend.

- [ ] **Step 3: Capture final visual evidence**

At the existing Playwright viewport capture splash, onboarding welcome, hero choice, VTT-empty, NPC memory, saves, and each scene-transition tag. Repeat splash, onboarding, VTT-empty, and one scene transition with reduced motion. Store transient evidence under `.artifacts/living-tabletop/final/`.

- [ ] **Step 4: Run the real Tauri WebView smoke**

In terminal A run `bun run dev:all` and wait for the backend readiness line and WebView window. In terminal B run `bun run e2e:tauri`.

Expected: real WebView2 shows the new splash, production root, empty VTT, and persisted hydration behavior. Report this separately from fixture-browser evidence.

- [ ] **Step 5: Inspect the final encoded assets at original resolution**

Use `view_image` on every final PNG/WebP. Confirm the ledger has no missing row and no asset contains malformed anatomy, fused equipment, generated writing, watermark, or unapproved purple effects.

- [ ] **Step 6: Commit verification-ledger updates**

```powershell
git add docs/visual/LIVING_TABLETOP_ASSETS.md
git commit -m "docs(art): record living tabletop acceptance evidence"
```

- [ ] **Step 7: Report delivery truthfully**

Report focused tests, `bun run gates`, browser Playwright, real Tauri/WebView, CI, and deployment as separate lines. Do not claim CI, packaging, publication, or deployment unless each was actually run and observed.
