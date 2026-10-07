# Room capability foundation

The application layer now provides persistent room creation, invitations, member
authentication, revocation and closure. This foundation is not yet connected to
HTTP routes, native host/join controls or multiplayer gameplay. It does not
establish a playable LAN session by itself.

A room belongs to one campaign/session and supports the existing four-member
party limit. Member identities are UUIDs, independent of display names. A host
can issue an invitation for a saved player, including a player in a full party;
joining replaces that player's room credential while preserving their character
ownership. Recreating a closed room preserves party identities.

Invitations expire after ten minutes and are consumed once. An invitation grants
entry, while a separate member capability authenticates later requests. Retrying
entry also requires the client's private resume proof; a public join UUID cannot
recover another player's credential. Revocation, closure and credential
replacement invalidate earlier authority leases. Administrative repository
mutations recheck the lease inside their transaction.

SQLite migration 0009 stores room metadata, credential references and operation
receipts. Capability values, invitation values and resume proofs use the existing
SecretsStore port. They are excluded from public room projections and campaign
snapshots. The actual SQLite/Stronghold reopen test verifies credential recovery.
Room secrets deliberately implement neither Debug nor Serialize; a future
transport must explicitly handle their delivery without logging them.

All service instances share a process-wide mutation coordinator. Production
composition must also retain the existing exclusive database/process authority;
this coordinator does not synchronize independent host processes. Room creation
is an internal local-owner entry point, not an unauthenticated network command.
Provider configuration remains outside room-host authority.

Focused verification:

```powershell
cargo test -p app-server --test room_capabilities
```

Coverage includes private-proof replay, two simultaneous service instances,
saved four-player recovery across new rooms, expiration, revocation, closure,
rejection before secret writes, public metadata privacy and real vault/database
reopening. HTTP authentication, party combat, event ordering and native/LAN
acceptance remain separate pending work in the co-op plan.
