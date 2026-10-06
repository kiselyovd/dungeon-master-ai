# Durable solo hero binding

A new combat in a real campaign reads its accepted character sheet from
SQLite. The hero receives an encounter token UUID and a separate persistent
`character_id`; another NPC may legitimately have the same name. The token
uses the durable name, current/max HP and AC. Initiative uses Dexterity rather
than a supplied initiative bonus; speed is validated. Both HTTP and agent
creation return the stored projection, including the server-added hero.

Agent `start_combat` entries explicitly have `role: npc`. The saved hero is
added by the engine. Ambiguous older player-plus-enemy entries are rejected in
a real hero session. Compatibility fixtures without a durable hero retain
their existing behavior. Known campaign/session pairs are checked before
selecting the HTTP lane or executing compatibility combat tools.

Bound hero HP, the campaign revision, combat state and the action receipt commit
in one transaction. A failed receipt rolls them back together. A bound sheet
cannot be replaced during combat. Saving in a bound encounter captures the
canonical sheet, even if the request includes another identity or HP values.
Legacy token-only writes are rejected for bound encounters; they cannot bypass
the projection and character transaction. End combat is atomic and includes its
confirmed projection; the compatibility end tool selects the requested session.

The renderer validates `campaign_scope` and mirrors the matching character's HP
and revision without resetting a chat stream. Campaign continuation and snapshot
restoration use the same combat parser, preserving the owner fields. Portraits,
the action bar, drag controls and player keyboard commands use character IDs
when available. Name fallback is retained only for old unbound fixture data.

## Equipment source correction

The existing sixteen catalog weapons had a generic `physical` damage type;
ranges and versatile values were incomplete. These fields and armor Dexterity
caps/Chain Mail Strength were checked against the official
[SRD5.2.1](https://media.dndbeyond.com/compendium-images/srd/5.2/SRD_CC_v5.2.1.pdf),
printed pages91-92. The catalog attribution remains unchanged. This prepares
authoritative equipment actions; it does not itself implement them.

## Evidence and remaining work

- Application hero mapping tests, SQLite binding/receipt rollback/sheet-save
  guards, real HTTP campaign/session rejection and explicit NPC tool creation.
- Renderer tests for atomic HP updates, narration preservation, same-name NPC
  ownership, continuation and player keyboard gating.
- Production-root browser against a real isolated backend/SQLite: same-name
  hero and NPC, actual turn command, two renderer reloads and retained binding.
- Independent review's four findings were reproduced and fixed; its limited
  follow-up found no remaining actionable issue in those corrections.

Full integrated gates passed on 2026-10-07: architecture, formatting, lint,
typecheck, Clippy, workspace Rust and1033 frontend tests. Production build and
the real-backend continuation browser scenario passed. The isolated backend was
stopped after verification. This describes the complete attached worktree;
unrelated inherited changes remain outside the scoped checkpoint.
This phase does not establish a complete playable session. Weapon/spell/resource
intent derivation, NPC turns, the authored defensive encounter and map, native
AI-directed acceptance, real model evaluation and LAN/VPN party control remain
in the active plan.
