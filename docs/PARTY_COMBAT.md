# Party combat persistence

Accepted campaign party characters now enter a new encounter together. Each uses
its durable character UUID and a distinct encounter-token UUID, derives its
mechanical profile from its own stored sheet, and receives a separate spawn tile.
Names do not determine identity or ownership. NPC automation yields at each
living player turn; the same rules support solo and groups of up to four.

The existing CombatCampaignScope remains anchored to the host character for wire
compatibility. It identifies campaign/session/revision; its character_id is not
authority to control all party actors. Room gameplay authorization must derive
the acting character from the authenticated member roster, which remains pending.

Every combat persistence transaction requires the exact accepted character set.
Missing, repeated and foreign bindings fail before a partial outcome can commit.
Host state stays in campaign_characters, guest state in party_characters. HP,
ammunition/quantity profiles and spell slots update together with the encounter,
campaign revision and combat receipt. Shared time advances once for the round,
regardless of party size. Failed guest writes roll back all related effects.

Raw sheet queries are used during combat-profile recovery to avoid recursive
campaign capture. Legacy missing profiles derive from the corresponding actor's
sheet. Snapshot restoration uses the saved host/guest sheets, preserving saved
resources, token IDs and timeline. Reading current guest sheets during restoration
would contaminate the saved state and is explicitly covered by a regression case.
The authored archive defeat recovery applies its existing nonlethal rescue outcome
to every bound hero instead of only the first actor.

Focused verification:

```powershell
cargo test -p adapter-sqlite --test party_combat --test combat_hero --test spell_intents --test weapon_intents
cargo test -p app-application --test party_combat --test hero_binding --test authored_defense
```

Coverage includes two/four same-name heroes, actual resolver turn handoffs and NPC
automation, group resources/time/replay, guest-write rollback, malformed identity
sets, real database close/reopen and legacy-profile snapshot recovery. These checks
verify backend mechanics and persistence. They do not establish authenticated room
gameplay, synchronized clients, a native LAN UI or physical-device co-op acceptance.
