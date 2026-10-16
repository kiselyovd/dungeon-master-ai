# Combat action receipts and timeline recovery

Combat intentions have an encounter-scoped request ID and a typed signature
covering their action and expected revision. Migration0006 stores an immutable
receipt in SQLite. The receipt, projection revision, HP/action budgets and token
coordinates commit in one transaction. Failure to write any receipt rolls back
the action. The application has no process-local idempotency cache.

An exact retry returns the recorded rolls and projection, including after
closing and reopening the database. A different intention under the same ID is
rejected with `combat_request_mismatch`. Concurrent exact requests share one
result; different writers still require the expected-revision comparison.
HTTP rolls use a host-generated seed. The legacy `rng_seed` transport field
does not control production outcomes. Deterministic application tests can still
inject a seed into the inward-owned command.

Rehydrating initiative preserves its persisted order, current combatant and
round. It does not advance through round1 to reconstruct a pointer. An absent
current turn or zero round is rejected instead of indexing an empty encounter.

Restoration invalidates receipts for every encounter in the affected session,
including when the save has no combat or another encounter. The restored
encounter receives a revision above both its current and saved revisions.
Delayed pre-restore intentions cannot mutate the restored state. Historical
snapshot data stays immutable; the current campaign-state response contains the
restored combat revision.

The renderer gives each new combat intention a UUID. Retrying an unchanged
failed intention retains the original ID and body while that hook remains
mounted. It ignores replies from an earlier campaign/session transition or
encounter. Pending transport recovery across a renderer reload is still future
work; fresh UUIDs prevent new actions colliding with earlier receipts.

This foundation does not complete canonical actor ownership, equipped attacks,
spell/rest/resource/range rules, encounter outcomes or LAN/VPN party control.
Those requirements remain in the solo/co-op plan.

## Focused evidence

- Four initial regressions reproduced and fixed: new-resolver replay,
  concurrent duplicates, round preservation and token coordinates.
- Real file-backed pool close/reopen replay and rollback on receipt insertion
  failure passed.
- Same, absent and different-encounter restore cases reject old intentions.
- HTTP replay returns identical recorded rolls/result; changed action409.
- Hook tests cover UUID identities, unchanged retry and delayed reply rejection.
- Independent review identified ID collisions and session-wide restore receipt
  invalidation; regression fixes passed the limited follow-up review.
- Full integrated gates passed on 2026-10-07: architecture, formatting, lint,
  typecheck, Clippy, workspace Rust and1028 frontend tests. Production build and
  the production-root full-campaign browser scenario passed. Browser transport
  uses scripted responses; this is not native/live-model/co-op acceptance.
- These checks describe the complete attached worktree. Unrelated inherited
  edits remain outside the scoped checkpoint; the original32 baseline files
  retained their recorded hashes.

## Provider terminal failures

The application turn service validates terminal rounds for every provider, beyond
the OpenAI-compatible adapter's complete-response checks. Length/Error, empty or
blank Stop, tools paired with Stop, ToolUse without completed calls, and unclosed
tool buffers cannot persist a final assistant reply or dispatch tools from that
invalid round. Previously streamed public draft text can appear transiently, but
the turn remains interrupted and existing continuation removes unfinished events.
Logs contain static diagnostic categories rather than provider content.

Exhausting the agent round budget also interrupts the turn. Confirmed earlier tool
results remain in its checkpoint; continuation uses them without repeating dispatch.
The same original request is retained. This does not promise semantic correctness
of a model's complete prose or repair arbitrary malformed tool JSON.

Evidence after6cb6720 on the preserved worktree baseline:
- RED application regression reproduced Length emitting AgentDone.
- Eight invalid terminal cases now reject without assistant commit/tool dispatch.
- One-round confirmed roll survives exhaustion; same-request continuation dispatches
  the tool once, calls the provider twice and stores one human message.
- Three real HTTP/SQLite cases persist interrupted status with only the human
  message, then resume to complete with one confirmed reply and no unfinished draft.
- Application/server regression suite passes; independent bounded review finds
  no remaining actionable issue. Existing success fixtures now supply real final
  text rather than relying on empty Stop being a successful answer.

Local artifacts: .artifacts/browser-session/terminal-red.log,
terminal-backend-final.log and terminal-clippy.log. No release, native/live-model
quality or physical LAN acceptance is claimed by these provider-port regressions.
