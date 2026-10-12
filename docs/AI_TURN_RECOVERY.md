# AI turn delivery and recovery

[Back to README](../README.md)

The renderer gives every new turn a UUID `request_id`. A failed transport retry
retains the original request body, history, board and ID. It replaces its
incomplete local narration, rather than appending a second user message.
EOF without `agent_done` or legacy `done` is an incomplete stream, not success.

Migration `0004_agent_operations.sql` adds durable request records and ordered
public events. A completed request replays its recorded response without calling
the provider again or inserting another user message. Reusing its ID with a
changed request is rejected. Public tool/media IDs include the turn UUID, while
provider conversation IDs remain unchanged internally.

The terminal marker is delivered only after the completion receipt commits.
An insert/update failure remains visibly incomplete. Disconnect cancellation is
observed even before the first public model event. Confirmed gameplay effects
are not rolled back by transport cancellation.

Distinct turns in one campaign run through one shared lane. Character commits,
adventure intentions and save restoration coordinate with that lane. Other
campaigns have independent lanes. SQLite expected revisions remain required.
The lane is acquired before claiming or resuming an operation and held until
its completion receipt commits. Checkpoint validation cannot precede a queued
restore and then execute on the restored timeline.

Migration `0005_agent_checkpoints.sql` records messages and immutable tool
results separately from public delivery. An explicit retry uses `resume=true`
with the original body and UUID. A request that never reached the server starts
safely under that same UUID. An interrupted request resumes recorded calls and
confirmed results; confirmed dice rolls are reused. Missing tool start, media
and result delivery is repaired from the checkpoint. Partial narration is
discarded; an already committed final assistant reply is delivered without
generating or appending another reply. Provider EOF without a terminal chunk
does not persist a draft as a finished reply.

Restoration increments the session's timeline generation. An interrupted turn
from the old timeline is rejected rather than applying its remaining effects
to the restored state. Pending authored actions and quick saves use their
original causation IDs. Legacy mutation/media calls with an ambiguous pending
receipt fail conservatively and require a fresh intention.

Cancellation wakes both the provider connection wait and the next-chunk wait.
It lets the actual nested engine exit before releasing the campaign lane;
setting a flag or aborting only the outer recorder would not establish that.

Production bootstrap holds an OS-backed exclusive authority lock beside the
database for the entire backend process lifetime. A second backend cannot take
authority merely because an old operation record looks stale. Only after the OS
lock is acquired does startup mark abandoned running operations interrupted.
The lock-file's presence and a changed owner UUID alone are not liveness proof.

## Read-only player progress

`GET /agent/recovery?campaign_id=<uuid>&session_id=<uuid>` reads the latest
unfinished solo operation for the current accepted character and restored
timeline. An active cooperative campaign is excluded from this solo endpoint.
`GET /rooms/{room_id}/dm/recovery` authenticates membership and reads only that
member's current operation. The renderer uses the registered native room
operation `agent_recovery`; credentials and destination remain native-owned.

The version-1 projection carries scope/request identity, the pinned model,
confirmed attempt outcomes, command progress and a permitted continuation:

| Phase | Meaning | Next action |
|---|---|---|
| `running` | The current process still owns delivery | Wait |
| `progress_unknown` | No validated human-command ledger is available | Resume the original request; do not infer zero effects |
| `actions_pending` | Some canonical command attempts remain unfinished | Resume |
| `needs_input` | A stored clarification awaits its owner's reply | Clarify using its original identity |
| `narration_pending` | Command attempts closed; final narration is not recorded | Resume narration |
| `delivery_pending` | Verified final narration is recorded but delivery is incomplete | Replay delivery |

The projection copies no original request text, history, source spans, private
questions/targets, tool JSON, model reasoning, final narration, biography or
credentials. A refusal is distinct from a performed effect. Unknown progress
has no invented completion counters. The projection grants no mutation authority;
actual continuation still validates the original request and current engine scope.

Canonical receipt inspection is read-only, including the effect-before-transcript
case. It does not repair tool delivery or append messages; repair remains part of
the authorized resume path. Restored timelines and mismatched actor metadata
cannot supply old progress. An unavailable/missing endpoint is an error, not
evidence that no pending request exists.

Migration `0022_agent_actor_scope.sql` captures the original character when a
new operation is claimed, before model planning. Existing protected manifests
provide the only legacy backfill. A replacement hero cannot resume the old
operation. The additive owner-only `/agent/recovery-input` endpoint supplies
the original request for a validated manifest-free interruption; it is used for
continuation, never rendered as public progress. `/agent/pending` preserves its
previous verified-manifest contract.

Focused application, SQLite, real room HTTP and renderer-contract tests verify
these boundaries. This establishes read/projection behavior; proactive production
UI, actual Tauri lifecycle and real-model session acceptance remain separate.

## Current limits

`running` and `interrupted` are explicit states. Incomplete operations require
an explicit continuation. HTTP running-request subscription, persisted renderer
recovery after reload, wider cancellation/effect ordering and legacy mutation
recovery remain ongoing work. These primitives do not establish a completed
recoverable solo/co-op session or live-model acceptance.

## Recorded evidence

- SQLite tests: completed replay after reopening, changed-payload rejection,
  running-owner safety, disconnect before any model event and failed completion
  receipt withholding the terminal marker.
- Checkpoint regression tests: request absent at first retry; confirmed result
  missing from delivery; restored timeline rejection; final reply replay without
  a second provider call; cancellation during both kinds of provider wait.
- Real HTTP tests: identical completed response without duplicate messages;
  two simultaneous distinct turns in one campaign have provider concurrency one.
- Real HTTP regression tests: queued restoration invalidates resume before its
  claim; disconnect from a genuinely pending provider stream interrupts the
  worker and frees the campaign lane.
- Production browser: same request ID/body after partial EOF, one user message,
  replacement narration; confirmed adventure updates while narration continues.
- Isolated file-backed backend: second live process rejected by OS lock;
  completed response replayed after actual shutdown/restart.
- Final integrated full gate on 2026-10-07 passed architecture, formatting,
  lint, typecheck, Clippy, workspace Rust tests and all 1025 frontend tests.
  Production build and two targeted production-browser scenarios passed.
  Independent review findings were fixed and the limited follow-up review had
  no remaining findings. Earlier gate failures exposed test-module ordering
  and fixtures relying on empty provider EOF; both were corrected.
- Validation describes the complete attached worktree, including preserved
  inherited changes. Scoped checkpoints exclude unrelated baseline edits.

## See Also

- [Campaign continuation](CAMPAIGN_STATE.md)
- [Durable human commands](HUMAN_COMMAND_LEDGER.md)
- [Native cooperative acceptance](COOP_ACCEPTANCE.md)
