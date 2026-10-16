# Adventure Book UI Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the approved calm, classical adventure-book polish across first-run language, onboarding, title bar, settings, and the VTT/chat control surfaces.

**Architecture:** Add small presentation primitives under `src/ui`, keep Zustand and Tauri calls at their current boundaries, and migrate only the visible high-impact consumers. CSS reuses the existing token system and component style locations; no wire or backend contracts change.

**Tech Stack:** React 19, TypeScript, CSS Modules/global component CSS, Zustand, react-i18next, Vitest/Testing Library, Playwright, Tauri WebView2 CDP.

## Global Constraints

- No new production dependencies.
- Reuse existing design tokens, fonts, radii, shadows, spacing, icons, and animation timings.
- Human-visible strings stay in both English and Russian locale catalogs; machine contracts are unchanged.
- Persisted settings override new-profile defaults.
- Every task uses test-first development and preserves unrelated worktree changes.
- Final evidence separates unit checks, production build, browser Playwright, and real Tauri CDP.

---

### Task 1: Russian clean-profile default and segmented language control

**Files:**
- Create: `src/ui/LanguageSegmentedControl.tsx`
- Create: `src/ui/LanguageSegmentedControl.module.css`
- Create: `src/ui/__tests__/LanguageSegmentedControl.test.tsx`
- Modify: `src/state/settings.ts`
- Modify: `src/state/__tests__/settings.test.ts`
- Modify: `src/state/__tests__/persistStorage.test.ts`
- Modify: `src/components/Onboarding.tsx`
- Modify: `src/components/__tests__/Onboarding.test.tsx`
- Modify: `src/app/__tests__/productionRoot.test.tsx`

**Interfaces:**
- Produces: `LanguageSegmentedControl({ value: Language, onChange: (value: Language) => void, ariaLabel: string })`.
- Preserves: `settings.setUiLanguage` and persisted `settings.uiLanguage` contracts.

- [ ] **Step 1: Write failing tests** asserting a clean store defaults both languages to `ru`, persisted `en` survives hydration, the segmented control exposes `РУ` and `EN` buttons, and onboarding marks the current language with `aria-pressed`.
- [ ] **Step 2: Run focused tests** with `bun test src/state/__tests__/settings.test.ts src/state/__tests__/persistStorage.test.ts src/ui/__tests__/LanguageSegmentedControl.test.tsx src/components/__tests__/Onboarding.test.tsx src/app/__tests__/productionRoot.test.tsx`; expect failures against English defaults and the missing primitive.
- [ ] **Step 3: Implement the minimal behavior** by changing only clean-store defaults, creating the controlled two-button primitive, and replacing the private onboarding picker.
- [ ] **Step 4: Re-run focused tests** and confirm all pass.
- [ ] **Step 5: Commit** only Task 1 files as `feat(ui): default new adventures to Russian`.

### Task 2: Accessible application select and settings controls

**Files:**
- Create: `src/ui/Select.tsx`
- Create: `src/ui/Select.module.css`
- Create: `src/ui/__tests__/Select.test.tsx`
- Modify: `src/components/SettingsForm.tsx`
- Modify: `src/components/settings/LocalLlmTab.tsx`
- Modify: `src/ui/Button.module.css`
- Modify: `src/ui/Modal.module.css`
- Modify: `src/components/SettingsForm.module.css`
- Modify: `src/components/SettingsModal.module.css`
- Modify: affected settings tests under `src/components/__tests__` and `src/components/settings/**/__tests__`

**Interfaces:**
- Produces: `Select<T extends string>({ id, value, options, onChange, disabled?, ariaLabel? })`, where `options` is `readonly { value: T; label: string; disabled?: boolean }[]`.
- Consumes: existing controlled settings values and callbacks without changing domain types.

- [ ] **Step 1: Write failing Select tests** for pointer selection, ArrowDown/ArrowUp navigation, Enter selection, Escape close/focus return, selected marker, disabled state, and outside-click dismissal.
- [ ] **Step 2: Run `bun test src/ui/__tests__/Select.test.tsx`** and confirm the missing component fails.
- [ ] **Step 3: Implement Select** with a button trigger, `role="listbox"`, `role="option"`, `aria-activedescendant`, document outside-pointer cleanup, and existing token styles.
- [ ] **Step 4: Replace visible settings native selects** for provider, UI language, narration language, reasoning budget, and VRAM strategy; keep submitted values unchanged.
- [ ] **Step 5: Polish buttons/modal footer/action rows** using existing `Button`, `Icons`, and CSS tokens; preserve loading/error/disabled semantics.
- [ ] **Step 6: Run focused settings tests** and update interaction helpers from `selectOptions` to opening the listbox and choosing an option by accessible name.
- [ ] **Step 7: Commit** Task 2 files as `feat(ui): refine settings controls`.

### Task 3: Onboarding chapter header and modal-safe title bar

**Files:**
- Modify: `src/components/Onboarding.tsx`
- Modify: `src/styles/onboarding.css`
- Modify: `src/ui/Modal.module.css`
- Modify: `src/app/App.tsx`
- Modify: `src/styles/shell.css`
- Modify: `src/ui/Icons.tsx`
- Modify: `src/components/__tests__/Onboarding.test.tsx`
- Modify: `src/app/__tests__/productionRoot.test.tsx`
- Modify: `src/ui/__tests__/Modal.test.tsx`

**Interfaces:**
- Produces: title-bar buttons with stable `data-testid="window-minimize|window-maximize|window-close"` selectors for native acceptance.
- Preserves: `tauriWindowAction('minimize' | 'toggleMaximize' | 'close')`.

- [ ] **Step 1: Write failing tests** for chapter-header semantics, compact route states, branded crest, and window-action callbacks while a modal is mounted.
- [ ] **Step 2: Run focused tests** and capture the expected failures.
- [ ] **Step 3: Restructure onboarding header markup** into context/header and horizontally scrollable route while retaining step computation and body rendering.
- [ ] **Step 4: Make the title bar a top interaction layer** and offset modal/onboarding backdrops below `var(--header-height)` so no overlay intercepts native controls.
- [ ] **Step 5: Refine crest/navigation/window icon presentation** with existing SVG infrastructure and tokens.
- [ ] **Step 6: Re-run focused tests** and confirm behavior remains accessible.
- [ ] **Step 7: Commit** Task 3 files as `feat(ui): polish onboarding and title bar`.

### Task 4: Unified VTT tool rail and composer

**Files:**
- Modify: `src/components/VttCanvas.tsx`
- Modify: `src/styles/vtt.css`
- Modify: `src/components/ChatPanel.tsx`
- Modify: `src/components/ChatPanel.module.css`
- Modify: `src/components/__tests__/VttCanvas.test.tsx`
- Modify: `src/components/__tests__/ChatPanel.test.tsx`

**Interfaces:**
- Preserves: zoom, fit, grid, measure, layers, attachments, streaming, keyboard send, and `onMoveToken` behavior.
- Produces: stable `aria-pressed` states for toggle tools and a grouped composer action surface.

- [ ] **Step 1: Write failing tests** for VTT toggle `aria-pressed`, layers association, and composer disabled/busy semantics.
- [ ] **Step 2: Run focused VTT/chat tests** and confirm failures.
- [ ] **Step 3: Add semantic pressed/expanded state** without moving local presentation state or combat authority.
- [ ] **Step 4: Polish rail, layer popover, scale, composer, focus, active, hover, and reduced-motion styles** using existing tokens.
- [ ] **Step 5: Re-run focused tests** and confirm map/chat routing behavior is unchanged.
- [ ] **Step 6: Commit** Task 4 files as `feat(ui): unify tabletop controls`.

### Task 5: Full verification and real Tauri CDP visual acceptance

**Files:**
- Modify only if required by evidence: `e2e/smoke.spec.ts`, `scripts/tauri-cdp-e2e.ts`, or focused test helpers.
- Evidence output: `.artifacts/` screenshots and logs (not committed).

**Interfaces:**
- Consumes: stable accessible labels/testids from Tasks 1-4.
- Produces: local evidence only; no deployment mutation.

- [ ] **Step 1: Run `bun run lint`, `bun run typecheck`, and `bun run build`**; fix only regressions introduced by this plan.
- [ ] **Step 2: Run focused Vitest plus full `bun run test`** and record results separately.
- [ ] **Step 3: Run `bun run e2e`** against the built/deployed browser surface and verify Russian first-run, settings select, VTT rail, and composer.
- [ ] **Step 4: Start the real Tauri app with WebView2 CDP** using the repository script, connect to the actual WebView, and capture onboarding/settings/VTT screenshots.
- [ ] **Step 5: With onboarding or settings open, click maximize/restore controls through CDP** and verify the window geometry changes while the dialog remains open; test close only at the end of the dedicated run.
- [ ] **Step 6: Run `bun run gates`** and report any unrelated pre-existing failure separately.
- [ ] **Step 7: Commit any acceptance-test adjustments** as `test(ui): cover adventure book surfaces`.
- [ ] **Step 8: Push the branch with `git push`** only after reviewing the exact outgoing commits and worktree scope.

## Self-review

- Spec coverage: every stated surface maps to Tasks 1-4; accessibility and real native verification map to Tasks 2, 3, and 5.
- Placeholder scan: no deferred implementation placeholders remain; each task identifies behavior, files, commands, and commit boundary.
- Type consistency: `Language` and `Select<T extends string>` values remain controlled strings; no task changes backend DTOs or application ports.
- Scope: character wizard native selects are excluded from this high-impact pass because the approved screenshots target onboarding/settings/titlebar/VTT; they can consume the same `Select` later without blocking this deliverable.
