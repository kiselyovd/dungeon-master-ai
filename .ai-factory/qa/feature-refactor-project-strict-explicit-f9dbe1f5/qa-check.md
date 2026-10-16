## QA Check: Character Creation and First Game Session

**Mode:** agent
**Environment:** local isolated Tauri development profile, real WebView2 over CDP
**Tested revision:** `b4cedc277cd92b5b2b68c1a836a2d1d769fea3a1`
**Worktree digest:** `1a74a047e8cd1d0ed044fc0dcee29f7214b5546d`
**Source digest:** `2ad864d5a4621e8ae6111eff334009e4269a2d29`

### Source bindings

| Case | Digest |
|---|---|
| TC-001 | `de000cbee80914c9668fe13595a20b3c2268b479` |
| TC-002 | `15089275d947b47548d76925707fddb00ff67325` |
| TC-003 | `68644bc61a3ef8ee28e0a2f10fd2f57d8c4149ca` |

### Results

- [x] **TC-001: Launch the complete desktop infrastructure** — Passed
  - Real Tauri WebView2 was reachable on CDP port 9222.
  - `bun run e2e:tauri` passed 7/7 checks against the real WebView.
  - The backend reported healthy on dynamic loopback port 63151, and no uncaught page exception was observed.
  - Execution correction: the final persistence run used the backend's isolated default file database instead of `sqlite::memory:` because a pooled SQLite in-memory URL creates connection-local databases.

- [x] **TC-002: Create a complete character from scratch** — Passed
  - The advanced character wizard completed in the real Tauri WebView.
  - The committed sheet showed `Aldric Emberward`, Human Fighter level 1, Acolyte, HP 12/12, AC 12, standard-array abilities, inventory, and 80 gold.
  - Evidence screenshot: `.artifacts/character-session/character-created.png`.

- [x] **TC-003: Start a real local game session and persist the first turn** — Passed
  - The local provider was selected in the real Settings UI and the Tauri-owned `mistralrs-server` reached ready on dynamic port 59553 with Gemma 4 E2B.
  - `/settings/v2` selected `local-mistralrs`; the main WebView composer submitted the player action through `/agent/turn`.
  - A distinct assistant bubble rendered: `You enter the Lantern and Stag tavern...` and the composer returned to its idle state.
  - Read-only SQLite evidence showed the same session contained one user row followed by one assistant row, with non-empty serialized text parts.
  - Final screenshot: `.artifacts/character-session/game-session-final.png`.
  - Execution correction: Gemma 4 E2B was used because it is the configured, staged local model in the current UI/runtime path; Qwen3-0.6B is not exposed by the current model picker.

### Supporting automated checks

- `bun run gates` — exit 0; architecture guard, Cargo fmt, Biome, TypeScript, em-dash guard, workspace Clippy, all Rust workspace tests, and Vitest passed.
- The stock `scripts/tauri-cdp-play.ts` reported a response as soon as the reasoning placeholder appeared. The final pass decision did not rely on that early signal; it used the completed assistant bubble and database rows.

### Summary

| Status | Count |
|---|---:|
| Passed | 3 |
| Failed | 0 |
| Blocked | 0 |
| Pending | 0 |
