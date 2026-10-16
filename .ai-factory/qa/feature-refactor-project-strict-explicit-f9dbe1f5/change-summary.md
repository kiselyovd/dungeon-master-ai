## Change Summary

**Commits:** 13
**Changed files:** 419
**Risk level:** High

---

### What Changed

The branch refactors the complete desktop application into strict Explicit Architecture boundaries and replaces the former arcane-tech visual identity with a calmer living-tabletop presentation. The requested QA slice focuses on the production desktop path from first launch through character creation and the first playable campaign turn.

---

### Affected Areas

| Component | Change type | Description |
|---|---|---|
| Tauri process ownership | Changed | Tauri now owns the backend, model, and media process lifecycle and exposes the backend port to the WebView. |
| Character creation | Changed | The onboarding hero choice, advanced character wizard, persisted PC projection, and production composition root were reorganized. |
| Game session | Changed | Agent turns now cross typed frontend ports, HTTP/SSE adapters, application use cases, repositories, and local runtime control. |
| Presentation | Changed | Production artwork, materials, animation, overlays, and product-root composition were replaced or extracted. |

---

### Evidence

| Finding | Evidence |
|---|---|
| Production React root moved behind explicit composition | `src/main.tsx`, `src/app/bootstrap.tsx`, `src/app/App.tsx` |
| Character completion commits a fully derived PC | `src/components/wizard/ReviewTab.tsx::beginConfirmed` |
| First chat action creates campaign/session identifiers and streams an agent turn | `src/features/agent-turn/useAgentTurn.ts::send`, `src/state/session.ts::ensureSession` |
| Tauri owns runtime processes | `src-tauri/src/lib.rs`, `src-tauri/src/processes/` |
| Backend persistence and transport were extracted | `crates/adapter-http`, `crates/adapter-sqlite`, `crates/app-application` |

---

### Risks

**Critical:**

- A clean WebView profile may fail to hydrate onboarding or Stronghold-backed persistence.
- Character wizard completion may leave required PC stats empty or fail to dismiss the wizard.
- Runtime-control, local LLM startup, settings application, SSE streaming, or SQLite persistence may fail across the extracted boundaries.

**Medium:**

- Layered overlays (onboarding, settings, preflight, character wizard) may block one another.
- The new artwork/material styling may break usable layout in the real WebView.

**Low:**

- Labels or screenshots may differ between English and Russian while machine contracts remain stable.

---

### Testing Recommendations

**First priority:**

- [ ] Launch the actual Tauri application with WebView2 CDP and an isolated local profile.
- [ ] Create a named character from scratch and verify non-zero derived statistics.
- [ ] Start the local model runtime, apply settings, submit a first game action, and observe a rendered DM response.

**Regression:**

- [ ] Confirm backend port discovery, HTTP health, runtime state, session identifiers, and message persistence.
- [ ] Confirm there are no uncaught WebView exceptions during the flow.
