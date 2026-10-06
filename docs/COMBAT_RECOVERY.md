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
