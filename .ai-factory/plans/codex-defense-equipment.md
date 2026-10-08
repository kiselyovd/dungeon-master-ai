# Equipment defense and armor

Continue the approved solo/co-op goal, addressing ChainMail/AC12 observed in real
native character creation. Preserve human control and server-authoritative rules.

- [x] Correct/complete SRD armor catalog and add explicit owned defense derivation.
- [x] Bind canonical solo/party/combat defense; protect budget/retry/rollback paths.
- [ ] Add explicit worn-armor/shield controls and correct live preview/committed choice.
- [ ] Verify frontend-server parity, real native creation/combat/reopen and review.
- [ ] Apply Stealth disadvantage and decide supported untrained-armor penalties.
- [ ] Modernize the legacy full-campaign UI fixture with actual ownership/contracts.

Backend foundation: see DEFENSE_EQUIPMENT.md and .artifacts/armor logs. Pure4/app4
cases, full core/storage regression and HTTP owned-armor case pass. Independent
read-only review confirmed budget-boundary read compatibility and normalized-write
rollback. Unupgradeable oversized legacy preserves old values until an explicit edit
frees room. UI and native armor acceptance remain open; no release or full-goal claim.
