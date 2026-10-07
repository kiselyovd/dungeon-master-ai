# Rest, resources and shared adventure time

Short Rest, Long Rest and waiting are typed intentions. The server reads the
accepted hero, current session revision, location and combat state; the client
cannot supply healing, a replacement clock or restored resources. Rest requires
at least1HP and a safe location outside combat. In the authored Bell adventure,
the town is the safe rest location. Legacy active combat rows also block rest
when their typed projection is absent.

The domain implements1-hour Short Rests with chosen class Hit Point Dice and
Constitution (minimum1HP per die),8-hour Long Rests and the16-hour wait before
starting another Long Rest. Long Rest restores HP, Hit Point Dice and slots;
Short Rest restores Pact Magic slots. Neither replenishes ammunition. Waiting
advances an explicitly chosen bounded duration. Full, half-caster and Pact
slot tables include higher slot levels, allowing the existing core effects to
upcast using actual resources. Rules were checked against the official
[SRD5.2.1](https://media.dndbeyond.com/compendium-images/srd/5.2/SRD_CC_v5.2.1.pdf),
printed pages71,77,185 and187.

Migration0007 adds session-owned `adventuring_states` and immutable
`rest_operations`. Game time is stored in seconds. Authored travel and scene
actions advance prepared durations; completed combat rounds advance6 seconds.
Snapshot creation captures the clock, and restoration rewinds it with the
character resources while invalidating old rest receipts. Snapshot restoration
remains an explicit timeline change; ordinary sheet edits preserve host-owned
`combatResources` and cannot refill spent stocks.

The rest transaction checks identity/revision/safety, derives the outcome,
persists the sheet and clock, advances the campaign revision and records the
exact result together. Failure rolls everything back. Simultaneous identical
requests share one result. Retry after a process restart returns the recorded
rolls and result without adding time or another resource effect. The HTTP
adapter exposes GET and POST `/campaigns/{id}/rest`, and mutations share the
campaign lane with AI turns and restoration.

The character sheet opens a rest dialog built from existing controls and
English/Russian catalogs. It displays HP, Hit Point Dice, slots, elapsed game
time and rest eligibility. An immutable public request outbox contains only
session/character identity, revision, causation UUID and intention; it contains
no provider credentials or character biography. It survives a renderer reload
and lets an unconfirmed rest retry its original identity. Scope and revision
guards protect both the store and dialog against late replies. Confirmed
resource updates preserve chat and narration state.

## Evidence and limits

Domain rest/table checks, SQLite transaction rollback/retry/concurrency,
authored travel/safety, legacy combat blocking, stale sheet updates and snapshot
restoration passed. Hook tests cover recovery after unmount, foreign-campaign
late replies and delayed status reads after successful retry. Independent
read-only review found the delayed-read race; it was reproduced and fixed, and
follow-up found no remaining actionable correction issue.

The production-root browser used a real isolated backend and SQLite. It
completed a Short Rest, deliberately lost the response, reloaded and recovered
the exact request without another hour or die. It then completed Long Rest,
retained19 arrows and the16-hour cooldown, and reloaded those values. After a
real backend process restart, both Short and Long receipts replayed exactly;
current time and resources remained unchanged. The dialog screenshot was
visually inspected. Weapon/spell/NPC browser regressions also passed.

Final collection on2026-10-07 passed the integrated full gate: architecture,
formatting, lint, TypeScript, Clippy, workspace Rust and1041 frontend tests.
The final backend/production builds and all four real-backend production-root
browser scenarios passed. A real process restart retained clock/resources and
exact Short/Long receipts. The isolated backend was stopped. Evidence applies
to the complete attached worktree; inherited changes remain outside this scoped
checkpoint, and the original32 source baseline hashes were preserved.

Remaining work includes interrupted/unsafe rest encounters, other class recharge
features, prepared-spell effects/concentration, AI rest/combat tools and context,
complete native solo sessions and LAN/VPN party play. Rest buttons and these
checks are not evidence of a complete AI-led or cooperative session.
