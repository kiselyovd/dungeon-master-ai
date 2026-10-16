# Room invitation links

An organizer creates a link in the existing **Room** panel. Use an address that
the players can reach over LAN or VPN, including its port. Listening on
`0.0.0.0` or `[::]` does not provide an address to share. The application asks for
a reachable address rather than guessing a network adapter. Refresh replaces
the invitation; expiry and single use are enforced by the existing room server.

Players paste the link, choose **Check link**, review the address and room code,
enter their player name, and explicitly choose **Join a room**. The raw pasted
link is cleared after validation. Manual connection fields remain under
**Enter connection details manually**. Listening configuration is under
**Advanced network settings**.

Native opening uses **Enable opening invitation links** in the room screen.
The approved deep-link plugin registers the protocol for the current executable
on Windows/Linux. Registration is opt-in; paste works without it. Other systems
can use paste when runtime registration is unsupported. No installer or other-OS
runtime acceptance is implied by local Windows verification.

Opening a link focuses the existing main window and shows a preview. It does
not join, switch a connected room or interrupt a pending operation. Cancel keeps
the existing state. An active room must be left explicitly before opening another
join form. Draft preparation and current destination scope are checked before
navigation. An uncertain native join retains its existing join identity and
recovery paths; retry/recover or postpone the old attempt deliberately.

The URI contract is `dmai://join/v1` with exactly three query fields: `address`
(numeric IP and nonzero port), `room` (non-nil UUID), and `invitation` (the existing
64 hexadecimal character single-use token). There is no member capability,
resume proof, provider configuration or campaign content in the link. The native
parser bounds the input to1024 bytes, rejects unknown/duplicate fields, versions,
credentials, fragments, ambiguous paths and invalid addresses, and never fetches
a URL. The canonical room server still decides whether the token is valid.

Only `main` can parse, create, claim or register an invitation. Early native
events stay in a bounded in-memory inbox; the event bus carries an event UUID
only. Unknown raw arguments and URI contents are never logged. The plugin's
automatic CLI processing is disabled by omitting configured schemes, avoiding
its retained raw-URL state and debug URL logging. The application handles
bounded cold/warm arguments itself; single-instance owns IPC. QA identifiers
register their own `dmai-qa-*` scheme, separate from the production protocol.

Screenshots and diagnostics must redact invitation fields. Links are sensitive
even though their tokens expire; share them only with intended players.
