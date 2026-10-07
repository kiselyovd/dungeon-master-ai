# Room capability foundation

The application layer now provides persistent room creation, invitations, member
authentication, revocation and closure. This foundation is not yet connected to
the production listener, native host/join controls or multiplayer gameplay. A
dedicated HTTP router is verified against a real loopback listener in tests; it
does not establish a playable LAN session by itself.

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
reopening. Production HTTP composition, party combat, event ordering and
native/LAN acceptance remain separate pending work in the co-op plan.

## Dedicated HTTP surface

`adapter_http::rooms::router` composes room routes separately from the desktop
router. It is not merged into or publicly bound by production bootstrap yet.

| Method | Path | Authority |
| --- | --- | --- |
| POST | `/rooms/{room_id}/join` | One-use invitation or private-proof replay |
| GET | `/rooms/{room_id}` | Active member capability |
| GET | `/rooms/{room_id}/party` | Active member; own sheet and public actors |
| POST | `/rooms/{room_id}/invitation` | Host; optional saved target member |
| DELETE | `/rooms/{room_id}/members/{member_id}` | Host |
| DELETE | `/rooms/{room_id}` | Host |

Member requests use `Authorization: Bearer ...` and `x-room-member-id`. Duplicate
credential/identity headers are rejected. Join JSON contains `join_id`,
`display_name`, `invitation` and `resume_proof`; clients must retain the private
proof before the first request to recover a lost response. JSON rejects unknown
fields, request bodies are limited to 8 KiB, and responses use `Cache-Control:
no-store`. No secrets travel in route/query parameters.

The party route returns null for a room member without an accepted hero. Accepted
members receive the existing PartyView allowlist: their own complete sheet,
stable roster identities and public mechanical actor summaries. Host authority
does not grant access to another player's full sheet. The route does not expose
the complete campaign snapshot. Provider, save, campaign-library and local-model
administration routes are absent from this router; room creation remains internal.

The real HTTP test covers identity spoofing, host/guest restrictions, private-proof
replay, both directions of biography privacy, unknown fields, oversized requests,
duplicate headers, revocation and closure. Hero submission/approval, shared game
projection, gameplay mutations, events and production/native wiring remain pending.
