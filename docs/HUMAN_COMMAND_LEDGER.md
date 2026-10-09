# Durable human commands

This protocol is being implemented to stop reinterpreting a player's completed
actions after reconnect. It preserves direct execution from chat and compound
requests. The AI remains the Dungeon Master; heroes belong to human players.

The application foundation currently defines an ordered manifest, exact source
spans, separate negative constraints, typed intentions, conditions, prepared-step
bindings and confirmed outcomes. Spans use UTF-8 bytes and are derived by the
server from quotes in the saved original message. Separate repeated phrases have
distinct positions and command IDs. An explicitly quantified clause can share
one full source span across a complete contiguous repetition group, with
one-based `index` and `total` metadata on each command. Missing ordinals, mixed
counts, changed spans and overlaps outside the group are rejected. The optional
metadata defaults to absent for existing saved manifests; no migration is needed. Structural validation does **not** establish semantic
permission. Both the initial manifest and each concrete candidate still need
independent intention verification.

A prepared step is not a finished attempt. Only a scope/signature-verified
canonical receipt or a definitive engine refusal closes it. Intermediate travel
does not finish another destination. Unknown check outcomes keep conditions open;
a specific refusal guard matches its actual code. The same canonical nonce cannot
close two commands. Exact replay retains its original binding, parameters and
nonce. Unresolved future targets remain visible and cannot be silently completed.

`HumanCommandLedgerRepository` is an inward-owned private port. Its SQLite
adapters derive actor/operation scope from protected storage and current
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

The room SQLite adapter stores member/turn-scoped manifests and binds preparation
in the same transaction. Current process owner, lease, actor, room epoch and reset
cursor come from protected storage. Canonical receipts recover effects without
requiring a transcript; repeated identical commands retain distinct nonces.
New-effect transactions reject stale prepared context, durable refusals and
unbound legacy preparation for turns that already have a manifest. Truly legacy
turns and manual room commands retain their existing signatures and contracts.

The current runtime integration obtains the protected original request, generates
one ordered manifest and independently checks its meaning before persistence.
The planner uses closed structured-output schemas; canonical arguments travel as
a JSON string and still pass the existing typed decoder. Production solo and room
factories attach the owner-scoped repository when player verification is enabled.
The shared turn service derives completion from canonical journal progress rather
than asking the model to reinterpret completed attempts. Every new effect still
needs semantic permission and an atomic binding. Exact prepared permission is
reused after current authority/signature/context checks; a recorded refusal can be
delivered again without a second effect or a fresh permission guess.

Migration0021 adds private clarification and final-narration metadata. An unresolved
goal can now resolve against current public reference labels and authenticated
replies, with an independent semantic check. If a choice is still missing, an
independently checked question is delivered with `agent_needs_input`, rather than
`agent_done`. Durable delivery sends that terminal only after the incomplete
operation status commits. Replies refer to the server-generated question UUID
and resume the original request; they cannot rewrite a resolved/bound goal.

Question metadata and its bound chat message commit atomically. Re-delivery uses
the same question identity without appending another message. Verified final
narration and its bound message also commit atomically; a crash before AgentDone
does not require another provider call. Native transport preserves its encrypted
outbox and private question marker for needs-input, validates exact original
request/reply transitions, and clears the outbox only after final completion.

The read-only `/agent/pending` endpoint restores the latest incomplete verified
solo operation for the current accepted actor/session/restore generation. Active
room state is excluded. It preserves the original wire history, model, images,
board and either an unanswered question or an already accepted reply. After an
uncertain stream outcome the hook rechecks this protected state before starting
another request. Remount can retry the exact accepted reply and original UUID.
The endpoint selects running/interrupted operations; it does not track client ACKs
for operations already marked complete. Checked final text remains in chat history.

This integration is still under full verification.
Quantified repeated actions, conservative legacy recovery, complete post-change
gates and real-model/native/separate-device session acceptance remain outstanding.
Focused SQL/runtime, frontend and production-browser fixture checks do not prove
a complete live Qwen or physical LAN session.

Focused checks:

```powershell
cargo test -p app-application --test human_commands_tests --test workflow_tests
cargo test -p app-application --test command_planning_tests
cargo test -p adapter-sqlite --test human_commands --test agent_operations
cargo test -p app-server --test room_dm_turns
bun run architecture:check
```

Quantified requests still use one binding and canonical receipt per attempt.
A performed or refused first attempt does not finish a second requested attempt.
Each new attempt uses a distinct tool call and, in rooms, a distinct canonical
nonce. Exact replay returns the old outcome. Quantity does not grant actions,
resources, successful outcomes or turn advancement: a second attack can receive
an authoritative budget refusal without damage or moving to another turn.
The existing manifest bound is32 commands; larger quantities must remain
explicitly unresolved rather than being silently truncated. Semantic extraction
and verification of the count remain model-dependent.

Verification covers a file-backed Wait twice after effect-before-transcript,
pool close/reopen and a new process owner (+120 seconds exactly), a real fighter
attack followed by budget refusal with an unchanged combat projection, and two
room attempts with different nonces and no third effect on replay. These checks
establish protocol/engine behavior, not real-model quantity interpretation or
native/physical separate-device LAN acceptance.
