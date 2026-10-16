# Adventure Book UI Polish - Design Specification

## Status

Approved in conversation on 2026-08-12. The chosen direction is a calm, classical “adventure book” interface: restrained parchment-and-brass ornament, clear hierarchy, and tactile controls without visual noise.

## Goals

- Make Russian the default language for a genuinely new profile while preserving an existing saved language.
- Turn onboarding into a readable chapter header plus a compact journey through the remaining steps.
- Keep native window controls usable above every modal and replace text-like/default browser symbols with the project icon language.
- Replace visibly native settings controls with reusable, accessible application primitives.
- Give the title bar a recognizable Dungeon Master AI crest and one consistent engraved icon treatment.
- Unify VTT tools, zoom, layers, measurement, composer, and send action so they feel like one instrument panel.
- Preserve the current architecture, tokens, fonts, color palette, keyboard behavior, and machine contracts.

## Visual Direction

The interface should feel like a well-kept campaign journal rather than a generic web dashboard. Existing dark plum surfaces remain the background. Brass is reserved for identity, focus, selected state, and primary actions. Borders and shadows stay within the existing token set. Motion is calm and functional: short fades, small translations, and restrained highlight changes; reduced-motion preferences disable nonessential movement.

No new production dependencies, color families, spacing scales, radii, typography scales, shadow systems, or icon libraries are introduced.

## Information Architecture

### Native title bar

- Left: a compact crest, product wordmark, and subtle divider.
- Center/right navigation: Saves, Journal, NPCs, Settings with matched icon size, stroke weight, alignment, hover, focus-visible, and active treatment.
- Far right: minimize, maximize/restore, and close controls using the same SVG icon system.
- The title bar remains the topmost interactive layer. Modal backdrops start below it, so dragging and all three window actions continue to work while onboarding, settings, or other dialogs are open.

### Onboarding

- The header becomes a two-level composition.
- Top line: current chapter title and “step N of M” context on the left; a segmented `РУ / EN` language control on the right.
- Second line: a compact horizontal route of step labels. Completed, current, and future steps are visually distinct without shouting.
- The content area remains stable when optional steps change. Existing step bodies and state-machine behavior are preserved.
- On narrow layouts the route scrolls horizontally instead of wrapping into an uneven multi-row header.

### Settings

- Tabs retain the existing structure, but use consistent focus/active treatment.
- Selects become a shared accessible listbox/combobox primitive with a styled trigger, chevron, option panel, selected marker, keyboard navigation, Escape, and outside-click dismissal.
- Download management and Hugging Face search become shared action rows: leading icon, label, compact count/status, and trailing affordance.
- Footer actions sit on a separated sticky surface. Cancel is secondary; Save is the clear brass primary action with comfortable hit areas.
- Disabled, loading, validation, and no-model states remain explicit and readable.

### VTT and chat controls

- The vertical VTT rail becomes a single grouped control surface rather than unrelated square buttons.
- Zoom, fit, grid, measure, and layers share dimensions, icon alignment, tooltip/focus behavior, and active state.
- The measurement readout is visually attached to its tool instead of floating ambiguously.
- Composer and Send form one action row. The text area has a clear focus state; Send uses the project icon plus localized label and communicates disabled/busy state.

## Components and Ownership

- `src/ui/Select.tsx`: reusable controlled select/listbox primitive; presentation only.
- `src/ui/LanguageSegmentedControl.tsx`: reusable two-option language control used by onboarding and settings where appropriate.
- Existing `Button` and `Icons` remain the base for actions and glyphs; missing project-specific marks are added to the existing icon surface.
- `src/components/TitleBar.tsx` (or the current title-bar composition): owns only presentation and window-action callbacks.
- `Onboarding`, settings views, `VttCanvas`, and `ChatPanel` consume shared primitives and keep existing feature/state ownership.
- CSS stays colocated with the existing component/style structure and uses existing custom properties.

## State and Data Flow

- A new store starts with `uiLanguage: 'ru'` and `narrationLanguage: 'ru'`.
- Persisted settings win during hydration; changing language still updates i18n through the existing application controller.
- Custom selects remain controlled by their parent and emit the same domain values as native selects.
- Window actions remain callbacks into the existing Tauri adapter; visual changes do not move native logic into presentation components.
- No user setting, API payload, event name, storage key, or backend DTO changes.

## Accessibility and Interaction

- All icon-only buttons have localized accessible names and visible tooltips where the existing UI pattern supports them.
- Minimum practical hit targets are preserved for title-bar and VTT controls.
- The custom select supports pointer and keyboard use: Enter/Space opens, Arrow keys move, Enter selects, Escape closes, and focus returns to the trigger.
- Focus-visible treatment is consistent and uses existing brass focus tokens.
- Dialog labels and modal focus behavior remain intact.
- Motion respects `prefers-reduced-motion`.

## Error and Edge Cases

- Running outside Tauri keeps the current safe warning behavior for native window actions.
- Empty or unavailable model lists render a disabled trigger or explicit empty option without creating an invalid setting.
- Optional onboarding steps may change after preset selection; current index remains clamped as today.
- Narrow windows keep critical title-bar controls visible and allow secondary navigation to compress before controls overlap.
- Hidden/background modal layers never intercept pointer input intended for the title bar.

## Verification

- Unit/component tests for Russian clean-profile defaults, persisted-language preservation, language segmented control, custom select pointer/keyboard behavior, title-bar actions, and onboarding header semantics.
- Production-root render test asserts Russian first-run labels and the Dungeon Master AI identity.
- Focused component tests, `bun run lint`, `bun run typecheck`, and `bun run build`.
- Playwright browser smoke for onboarding, settings, custom select, and VTT controls.
- Real Tauri WebView2 CDP run: open a modal, exercise maximize/restore and verify title-bar controls remain clickable; capture screenshots for onboarding, settings, and the main VTT surface.
- Run the consolidated project gates after focused verification. Report mocked browser and real Tauri evidence separately.

## Self-review

- The design reuses current tokens, fonts, icons, and ownership boundaries.
- It explicitly covers clean-profile versus persisted language behavior.
- It treats the title bar as a native interaction boundary, not only a z-index cosmetic fix.
- It defines keyboard and reduced-motion behavior for the new interaction primitives.
- It preserves current settings values and backend/wire contracts.
- The scope is intentionally a coherent high-impact pass rather than an unbounded redesign of every screen.
