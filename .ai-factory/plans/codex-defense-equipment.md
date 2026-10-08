# Equipment defense and armor

Continue the approved solo/co-op goal, addressing ChainMail/AC12 observed in real
native character creation. Preserve human control and server-authoritative rules.

- [x] Correct/complete SRD armor catalog and add explicit owned defense derivation.
- [x] Bind canonical solo/party/combat defense; protect budget/retry/rollback paths.
- [x] Add explicit worn-armor/shield controls and correct live preview/committed choice.
- [x] Verify frontend-server parity, real native creation/combat/reopen and review.
- [ ] Apply Stealth disadvantage and decide supported untrained-armor penalties.
- [x] Modernize the legacy full-campaign UI fixture with actual ownership/contracts.

Backend foundation: see DEFENSE_EQUIPMENT.md and .artifacts/armor logs. Pure4/app4
cases, full core/storage regression and HTTP owned-armor case pass. Independent
read-only review confirmed budget-boundary read compatibility and normalized-write
rollback. Unupgradeable oversized legacy preserves old values until an explicit edit
frees room. UI and native armor acceptance remain open; no release or full-goal claim.

Frontend creation checkpoint: controls/shared inventory/preview/commit and draft/PC
persistence implemented. 194 focused tests and production-root reload browser case
pass; independent reviewer confirmed two fixes. Existing hero recreation and native
armor acceptance remain open. See DEFENSE_EQUIPMENT.md.

Native Chain Mail acceptance: real wizard AC16, stored versioned choice, server-bound
human in battle, actual NPC15 misses against16, cold reopen same UUID/round2/rev4.
Read-only SQLite and independent review confirm one commit/no snapshots/cleanup.
Shield/other armor native coverage and existing hero recreation remain follow-ups.

Dice prerequisite completed: formerly ignored advantage reproduced and fixed;
disadvantage, cancellation, typed schema, damage-mode rejection and replay-compatible
serialization verified. Canonical skill modifiers and automatic armor penalties
remain the next implementation step; see DEFENSE_EQUIPMENT.md.

Canonical solo check checkpoint: strict identified skills, current sheet lookup,
server ability/proficiency, automatic worn-armor Stealth/Poisoned disadvantage,
DC result and bounded context implemented. Domain/application/SQLite/server suite
passes; independent review closed portrait-context and legacy-budget findings.
Co-op check authority/receipts and native/model check delivery remain next; the
combined Stealth/untrained task stays open.

Co-op check authority/receipts implemented using existing prepared-tool records.
File-backed two-member check/replay/reopen, rollback, reset and separate revocation
regressions pass. Model/native delivery and untrained armor remain follow-ups.
The combined Stealth/untrained task stays open until those limitations are resolved.

Two-native scripted-provider check delivery accepted: guest UI Send -> canonical
roll4+modifier2=6/DC15 failure -> same answer in both clients; full close/restart
and completed native replay preserve revision141/dice/transcript, provider calls2
unchanged. Independent SQLite/review/cleanup confirmed. This case is unarmored;
real-model quality, physical LAN and remaining mechanics are still open.

Browser fixture checkpoint, 2026-10-08: canonical cleric ownership, prepared spell
slots, weapon intentions, current-state save restoration and fresh scoped combat
projections replace the legacy full-campaign fixture. The stale-projection check
was proved negative by disabling both revision guards in a temporary build: the
test failed on token position60px versus270px, then passed after exact restoration.
Other chat/movement/tabletop fixtures now configure the provider before sending.
Full production-browser suite:62 passed,1 existing vision skip. Four authored Bell
cases use the real isolated SQLite/backend for peaceful, distinct endings, restore
which rolls back the reward, followed by a new rescue granting10gold, and defensive combat/map/rescue. The full-campaign test
uses scripted transport; real-model narrative quality and physical LAN stay open.
See docs/BROWSER_SESSION_ACCEPTANCE.md. Independent review closed the timing race
and found no remaining actionable fixture issue. No production guards weakened.
Fast gates pass (architecture, Cargo formatting, Biome, TypeScript and em-dash);
production build and full browser suite pass. Backend process/listener stopped.
