## Test Cases: Character Creation and First Game Session

---

### TC-001: Launch the complete desktop infrastructure

**Priority:** High
**Type:** Positive
**Execution surface:** hybrid

**Precondition:**

The application is launched with an isolated Tauri identifier, `DATABASE_URL=sqlite::memory:`, local-sidecar dev wiring, and WebView2 CDP on port 9222.

**Steps:**

1. Start the Tauri development application.
2. Connect to the application page over CDP.
3. Read the backend port through the Tauri `backend_port` command.
4. Request backend health and inspect the first-run UI.

**Expected result:**

The actual Tauri runtime is present, the production shell renders, the backend reports healthy on a dynamic loopback port, first-run onboarding is visible, and no uncaught WebView exception occurs.

---

### TC-002: Create a complete character from scratch

**Priority:** High
**Type:** Positive
**Execution surface:** browser-ui

**Precondition:**

TC-001 passed and the isolated profile is on first-run onboarding.

**Steps:**

1. Select the manual setup preset and choose `Build from scratch (advanced)`.
2. Close the automatically opened Settings dialog to reveal the character wizard.
3. Select Fighter, Human, Acolyte, and Standard Array.
4. Select starting-gold equipment mode, enter `Aldric Emberward`, and navigate to Review.
5. Click `Begin Adventure`.

**Expected result:**

The wizard closes, the application shell remains usable, the PC is `Aldric Emberward` with the selected class/race/background and positive derived HP/HP max, and the isolated Tauri persistence contains the completed onboarding and PC state.

**Test data:**

```text
Name: Aldric Emberward
Class: Fighter
Race: Human
Background: Acolyte
Abilities: Standard Array
Equipment: Starting gold
```

---

### TC-003: Start a real local game session and persist the first turn

**Priority:** High
**Type:** Positive
**Execution surface:** hybrid

**Precondition:**

TC-002 passed and the Tauri-controlled mistralrs runtime is staged.

**Steps:**

1. Open Settings and select the local mistralrs provider.
2. Register and select `Qwen3-0.6B (ISQ Q4K, text-only)` through the Local LLM UI, then start the runtime.
3. Wait until the backend reports the LLM ready, then save Settings.
4. Submit the player action in the main composer.
5. Wait for a non-empty Dungeon Master reply.
6. Query the session message endpoint using the campaign/session IDs observed from the WebView state.

**Expected result:**

The runtime becomes ready under Tauri ownership, `/settings/v2` succeeds, `/agent/turn` succeeds, a non-empty DM reply renders, secure campaign/session IDs are created, and the backend returns both the user action and assistant response for that session without uncaught WebView exceptions.

**Test data:**

```text
I enter the Lantern & Stag tavern, greet the innkeeper, and ask about trouble near the old watchtower.
```

## Test Data

### Positive

* Isolated first-run Tauri profile.
* Public Qwen3-0.6B model resolved by the staged local runtime.
* Deterministic named Fighter character.

### Negative

* Any runtime startup failure must remain visible in the Settings UI and be recorded as a failed or blocked case, never treated as a pass.
