# Party combat persistence

Present accepted campaign party characters enter a new encounter together. Each uses
its durable character UUID and a distinct encounter-token UUID, derives its
mechanical profile from its own stored sheet, and receives a separate spawn tile.
Names do not determine identity or ownership. NPC automation yields at each
living player turn; the same rules support solo and groups of up to four.

The existing CombatCampaignScope remains anchored to the host character for wire
compatibility. It identifies campaign/session/revision; its character_id is not
authority to control all party actors. The room combat interface now derives
the acting character from the authenticated member roster and checks the lease
inside its commit transaction. Production LAN/native composition remains pending.

Every combat persistence transaction requires the encounter's exact recorded character set.
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
verify backend mechanics and persistence. Authenticated room command coverage is
described in ROOM_AUTH.md. Synchronized clients, a native LAN UI and physical-device
co-op acceptance remain separate pending work.

## Present and retained heroes

New encounters use the active members of the current room. Retained party sheets
remain durable after a member is revoked or a room closes, but those heroes do
not enter the next solo encounter. The host remains required. A trusted party
created through the internal pre-room API retains its existing behavior until
the campaign first has room history; this is not an unauthenticated network path.

Existing encounters keep their recorded owner/token UUID bindings even after
room changes. Hydration/resource synchronization selects those recorded owners
from retained sheets; writes cannot add/drop/reassign them, including by removing
campaign scope. Save restore likewise selects recorded owners from the full
snapshot roster, requires the host, and keeps missing/duplicate identity checks.
This lets a new host-only battle coexist with retained guest sheets and survive
save/restore/reopen without rewriting historical encounters.

Group long rest applies only to active accepted members. A revoked downed hero
cannot block the remaining group and its retained HP/resources remain unchanged.
Replay returns the receipt without another rest or revision. Existing solo rests
and recovery clock behavior remain unchanged.

Evidence: two RED regressions (guest in new fight; revoked downed guest blocking
rest), then file-backed revoked/closed-room -> new host-only fight -> save/restore
-> full SQLite close/reopen, with preserved guest sheet and rejected forged owner
removal. An independent reviewer found the initial restore mismatch; separate
RED reproduction and repair are retained. Logs under .artifacts/browser-session:
roster-{red,restore-red,green,reopen,backend-final,clippy}.log.

Open lifecycle work: removing a player during an active battle, restoring an old
co-op encounter while its players are absent, retired roster capacity and rejoining
retained heroes. This checkpoint does not prove native/two-device LAN behavior
or full session completion. No UI, schema or production dependency was added.

Final local collection after7842315:440 SQLite/server tests, changed-crate Clippy
alltargets, architecture, formatting, Biome and TypeScript pass. Independent
follow-up review closed the restore finding. The verified attached worktree keeps
its inherited baseline; original32source hashes are preserved. No new frontend or
real model/native session was exercised by this backend-only checkpoint.
