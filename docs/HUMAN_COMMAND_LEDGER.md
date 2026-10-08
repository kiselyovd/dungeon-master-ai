# Durable human commands

This protocol is being implemented to stop reinterpreting a player's completed
actions after reconnect. It preserves direct execution from chat and compound
requests. The AI remains the Dungeon Master; heroes belong to human players.

The application foundation currently defines an ordered manifest, exact source
spans, separate negative constraints, typed intentions, conditions, prepared-step
bindings and confirmed outcomes. Spans use UTF-8 bytes and are derived by the
server from quotes in the saved original message; repeated phrases have distinct
positions and command IDs. Structural validation does **not** establish semantic
permission. Both the initial manifest and each concrete candidate still need
independent intention verification.

A prepared step is not a finished attempt. Only a scope/signature-verified
canonical receipt or a definitive engine refusal closes it. Intermediate travel
does not finish another destination. Unknown check outcomes keep conditions open;
a specific refusal guard matches its actual code. The same canonical nonce cannot
close two commands. Exact replay retains its original binding, parameters and
nonce. Unresolved future targets remain visible and cannot be silently completed.

`HumanCommandLedgerRepository` is an inward-owned private port. Its future
adapters must derive actor/operation scope from protected storage and current
authority, validate against the original request, and allocate binding identities
atomically with tool preparation. Existing tool `command_json` signatures remain
unchanged. Client history and narration cannot supply completion evidence.

The solo SQLite adapter now persists the private manifest and atomically binds
steps with existing tool preparation. It verifies owner, actor, original request
and restore generation; exact canonical receipts repair missing result metadata
after effect-before-transcript interruption. Prepared world/combat revisions and
actor-token identity are captured separately from old command_json signatures.
Stale uncommitted contexts and mismatched full signatures are rejected in the
canonical write transaction. Dice/refusal settlement also checks prepared context.

Room persistence, model manifest generation/verification, trusted future-target
resolution and replacement of the runtime completion loop are **not wired yet**.
Legacy unbound operations need conservative recovery; this foundation does not
make old effects repeatable or claim live session/LAN acceptance.

Focused checks:

```powershell
cargo test -p app-application --test human_commands_tests --test workflow_tests
cargo test -p adapter-sqlite --test human_commands --test agent_operations
bun run architecture:check
```
