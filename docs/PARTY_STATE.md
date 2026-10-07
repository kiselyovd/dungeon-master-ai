# Durable party foundation

Party identity uses member and character UUIDs. Up to four members share one
campaign; equal display names are allowed. The host keeps the authoritative
existing solo sheet. Accepted guest sheets have separate storage and cannot
overwrite it. Enrollment is a host-authorized application operation; its trusted
host identity is not deserializable from a wire body.

The domain rejects invalid IDs, duplicate ownership and character reassignment.
Reconnecting to an existing binding does not create another member. SQLite
enrollment commits sheet, campaign revision and immutable receipt atomically.
Exact retries return fresh party state; changed requests under the same ID fail.

Internal state contains all sheets. `view_for` supplies public name/class/HP/AC
summaries for every member and the complete sheet only for the selected owner.
The caller must authenticate that owner before selecting a view. This is an
ownership/projection boundary; network authentication is a separate required
layer that is not exposed yet.

Snapshots optionally include the host member identity and accepted guest sheets.
Older solo snapshots remain readable. Restore requires the same member/character
bindings and rejects an older snapshot that would lose a new guest. Duplicate,
oversized or mismatched guest entries are rejected. All guest restores share the
existing snapshot transaction; failure on a later guest rolls back earlier
updates and the campaign revision. Membership credentials never belong in the
game snapshot.

## Verification and remaining work

Domain ownership tests, snapshot validation and seven real SQLite cases cover
same-name heroes, private projections, immutable retry, receipt rollback, solo
identity protection, guest restore, malformed snapshots, failed multi-guest
restore and actual pool shutdown/reopening. Historical migration fixtures include
the new roster migration without rewriting their old snapshot data.

This foundation has no host/join UI or public LAN endpoint. Authentication,
multi-hero combat/resource synchronization, ordered room events, reconnect and
native two-client acceptance remain required. Follow
`.ai-factory/plans/codex-lan-coop.md` before exposing the party to remote clients.
