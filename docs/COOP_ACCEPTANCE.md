# Local native co-op acceptance

Verified at local checkpoint 73439c9 using two debug Tauri apps on one Windows
machine. Host and guest use separate application identifiers, WebView profiles,
SQLite files, encrypted Stronghold vaults and backend processes. Real production
API modules invoke registered Tauri commands; no Tauri mock or Zustand injection
is used. The UI is served by Vite for these native checks. This is not a packaged
release or physical-device LAN/VPN acceptance.

## Evidence and results

- Enrollment: guest membership recovered after an uncertain initial join;
  two accepted human heroes (Мира and Тара); distinct backend ports; private hero
  biographies isolated in both directions. Guest approval is denied with HTTP 401.
- Shared DM: an actual HTTP fixture provider runs behind the host genai adapter.
  Guest turn completes through native streaming. Both clients read identical public
  messages. The provider sees both biographies and the shared-DM role; private
  reasoning does not enter player events or shared persisted narration. This
  establishes transport/context behavior, not real-model narrative quality.
- Peaceful adventure: 24 alternating native intentions finish remembered_river.
  The rescuing guest receives ten gold, host receives none; exact rescue replay
  changes neither campaign revision nor clock. Both real SSE subscriptions observe
  25 updates ending at the same head and ending.
- Defense: owner snapshot restore returns the same roster and advances the reset
  marker. Both actual WebViews render map/initiative. Six human end-turn intentions
  yield between players; another player's action is denied without advancing combat.
  Exact movement replay does not repeat its effect. Both views report held with
  identical public actors and adventure state.
- Restart: each native application and child backend is closed and relaunched.
  Saved host recovery rebinds the original nonzero port; guest recovery retains
  its member UUID. Roster identity and restored HP match across both clients.

Sanitized local proof artifacts are under .artifacts/two-native/: enrollment-proof,
dm-proof, provider-proof, peaceful-proof, battle-proof and restart-proof JSON,
with enrollment-verified, dm, peaceful-current, battle, restart and
guest-restart-current logs. Native battle screenshots mask the host invitation.
These ephemeral test profiles and executable copies are excluded from Git.
Both native apps, their backends, fixture provider and Vite server are closed after
verification. Original dirty checkout remains preserved.

## Remaining acceptance

The first native join timed out after remote commit; explicit recovery succeeded
without another member. Its timing needs diagnosis before claiming a smooth join.
Defense ended with the guest at zero HP while the host stayed at twelve. The
session was restored to its initial snapshot for restart checks; post-victory
zero-HP stabilization, recovery and death-save behavior remain to be completed.
The checks invoke gameplay through real API/IPC; full wizard, pointer/dialog
combat interaction and interruption of a real model require additional UI passes.
Provider settings were supplied through the API for this fixture run; durable
user-configured model startup after restart is not proved by this test.
Busy-port host abandon, guest archive management, remaining solo mechanics,
real-model quality and a physical LAN/VPN session are still open. The complete
playability goal is not achieved by this checkpoint.


## Independent evidence review

Read-only review confirms bounded native/loopback behavior and highlights a P1:
the downed guest has no supported out-of-combat recovery path after held defense,
and group Long Rest rolls back. This must be fixed before a playable-session claim.
The initial failed renderer request UUID was not captured, so the actual recovery
flag alone does not prove equality to that original UUID; membership count stayed
two. Restart-script booleans are literal and not independent process evidence.
Separate manual launch/CIM observations record host app PID 12300 -> 25264 and
guest app PID 46116 -> 39776, with only the other backend remaining after each
close and none after final close. These observations are saved separately in
process-restart-observations.json. Battle proof records held as an observed value;
the harness did not explicitly require held or a specific wrong-turn error code.
These distinctions remain part of the acceptance limits.
