# Living Tabletop Asset Ledger

Approved direction: calm classic high fantasy, painterly oil and gouache, warm practical light, believable worn materials, restrained violet magic, and no generated writing or technological ornament.

Approval date: 2026-08-12.

## Wave 1

| Role | Final path | Candidate | Pixels | Old bytes | New bytes | QA |
|---|---|---|---:|---:|---:|---|
| Splash | `src/assets/living-tabletop/splash.webp` | `.artifacts/living-tabletop/candidates/splash.png` | 1376 x 768 | 1,536,106 | 157,684 | Approved: four miniatures, crop-safe table, no text or technical motifs |
| Onboarding | `src/assets/living-tabletop/onboarding.webp` | `.artifacts/living-tabletop/candidates/onboarding.png` | 1312 x 816 | 1,770,393 | 151,318 | Approved: four classes plus DM, clear copy space, natural anatomy |
| Empty VTT | `src/assets/living-tabletop/vtt-empty.webp` | `.artifacts/living-tabletop/candidates/vtt-empty.png` | 1200 x 896 | 1,999,794 | 130,070 | Approved: empty center, no writing, clean edge props |
| Fighter portrait | `src/assets/living-tabletop/hero-fighter.webp` | `.artifacts/living-tabletop/candidates/hero-fighter.png` | 1024 x 1024 | 1,446,787 | 150,552 | Approved: readable face, sword and shield geometry |
| Wizard portrait | `src/assets/living-tabletop/hero-wizard.webp` | `.artifacts/living-tabletop/candidates/hero-wizard.png` | 1024 x 1024 | 1,227,378 | 128,014 | Approved: elf identity, staff and book, restrained violet |
| Rogue portrait | `src/assets/living-tabletop/hero-rogue.webp` | `.artifacts/living-tabletop/candidates/hero-rogue.png` | 1024 x 1024 | 1,677,300 | 107,306 | Approved: halfling identity, bow and shortsword, uncovered face |
| Cleric portrait | `src/assets/living-tabletop/hero-cleric.webp` | `.artifacts/living-tabletop/candidates/hero-cleric.png` | 1024 x 1024 | 1,595,071 | 176,850 | Approved after one rejected multi-character candidate: solo dwarf, mace and sun shield |
| Fighter token | `src/assets/living-tabletop/token-fighter.png` | `.artifacts/living-tabletop/candidates/token-fighter.png` | 1024 x 1024 | 1,267,575 | 771,706 | Approved: portrait identity preserved, alpha 0 to 255 |
| Wizard token | `src/assets/living-tabletop/token-wizard.png` | `.artifacts/living-tabletop/candidates/token-wizard.png` | 1024 x 1024 | 1,204,124 | 700,407 | Approved: portrait identity preserved, alpha 0 to 255 |
| Rogue token | `src/assets/living-tabletop/token-rogue.png` | `.artifacts/living-tabletop/candidates/token-rogue.png` | 1024 x 1024 | 1,176,880 | 690,716 | Approved: portrait identity preserved, alpha 0 to 255 |
| Cleric token | `src/assets/living-tabletop/token-cleric.png` | `.artifacts/living-tabletop/candidates/token-cleric.png` | 1024 x 1024 | 1,350,835 | 206,890 | Approved: portrait identity preserved, alpha 0 to 255, palette-optimized PNG |

## Prompts

### Splash

> Create a production-ready wide classic high-fantasy tabletop illustration for a desktop Dungeon Master application splash screen. A worn oak gaming table beside a stone hearth at night, an open hand-drawn adventure map with no readable writing, four distinct adventurer miniatures, leather journal, dice, sealing wax and one steady candle. Calm anticipation before a journey. Painterly oil and gouache, visible brushwork, believable worn wood, leather, parchment and metal, deep shadow with warm amber firelight. Restrained violet appears only as a tiny magical reflection. Cinematic 16:9 composition with a crop-safe central field and quiet visual hierarchy. No people facing camera, no text, no letters, no logo, no watermark, no gears, no circuit patterns, no decorative runes.

### Onboarding

> Create a production-ready wide classic-fantasy welcome illustration for a Dungeon Master desktop application's onboarding screen. Four distinct adventurers gather around a real worn wooden table as a human dungeon master opens an old map: a seasoned human woman fighter with longsword and scarred shield; an older high-elf man wizard with ash quarterstaff and closed spellbook; a young adult halfling woman rogue with shortbow; a middle-aged hill-dwarf man cleric with mace and battered shield. Warm hearth light and cool window moonlight, inviting companionship and anticipation. Painterly oil and gouache with visible brushwork, natural anatomy, worn cloth, leather and metal. Calm wide composition with generous clear negative space on the left for interface copy while the group occupies the center-right. No text, no letters, no watermark, no logo, no gears, no circuit traces, no meaningless runes, no plastic 3D rendering.

### Empty VTT

> Create a production-ready top-down classic fantasy cartographer's desk illustration for an empty virtual tabletop state. An unmarked aged parchment map centered on dark worn oak, with a brass compass, charcoal stick, a few bone and dark metal dice, a wax seal, and a small candle near the outer edges. Candlelight fades into quiet shadow. Painterly handmade realism, oil and gouache with visible brushwork and believable parchment and wood grain. Generous completely empty center for interface labels and map controls. 4:3 composition. No people, no miniatures, no readable writing, no labels, no letters, no watermark, no logo, no gears, no circuit motifs, no glowing runes, no decorative runes.

### Hero portraits

- Fighter: seasoned human woman fighter and soldier, weathered expressive face, dark brown hair tied back, practical mail over quilted gambeson, longsword and scarred wooden shield, calm protective posture.
- Wizard: older high-elf man wizard and sage, intelligent lined face, long silver hair, travel-stained blue-grey robes, ash quarterstaff, closed spellbook, and restrained violet reflected only in his eyes and fingertips.
- Rogue: young adult halfling woman rogue and former criminal, compact athletic build, practical moss-green wool and worn leather, shortbow, quiver, shortsword, alert half-smile, uncovered face.
- Cleric: one middle-aged hill-dwarf man cleric and acolyte, broad kind face, braided auburn beard, worn scale armour, iron mace, battered shield with a simple hammered sun emblem, no companion.

Shared portrait constraints: square head-and-torso classic high-fantasy oil and gouache portrait, visible brushwork, natural anatomy, worn materials, warm directional light, quiet dark background, readable eyes, no text, watermark, technology, decorative runes, plastic rendering, or extra equipment.

### Combat tokens

Each token used its approved portrait as an image-edit reference with this shared instruction:

> Preserve this exact character's face, ancestry, age, clothing, armour and equipment. Recompose as a top-down three-quarter full-body tabletop combat token, complete readable silhouette inside a circular crop-safe area, neutral grounded stance, painterly edge detail matching the source portrait, exactly one character on a perfectly uniform bright chroma green background. No ring, base, text, external cast shadow, extra weapon, watermark, or second person.

The chroma background was converted to a soft alpha matte with the bundled `remove_chroma_key.py` helper, then each token was encoded at 1024 x 1024. The final PNGs were inspected again after alpha conversion.
