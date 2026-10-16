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
At the earlier checkpoint, defense ended with the guest at zero HP while the host stayed at twelve. The
session was restored to its initial snapshot for those restart checks. The historical
post-victory recovery gap is superseded by the native recovery acceptance below;
combat death-save behavior remains pending.
The checks invoke gameplay through real API/IPC; full wizard, pointer/dialog
combat interaction and interruption of a real model require additional UI passes.
Provider settings were supplied through the API for this fixture run; durable
user-configured model startup after restart is not proved by this test.
Busy-port host abandon, guest archive management, remaining solo mechanics,
real-model quality and a physical LAN/VPN session are still open. The complete
playability goal is not achieved by this checkpoint.

## Independent evidence review

The earlier read-only review confirmed bounded native/loopback behavior and identified a P1:
the downed guest has no supported out-of-combat recovery path after held defense,
and group Long Rest rolls back. That specific co-op gap is now closed by the native acceptance below; the complete
playability claim still requires the remaining scope.
The initial failed renderer request UUID was not captured, so the actual recovery
flag alone does not prove equality to that original UUID; membership count stayed
two. Restart-script booleans are literal and not independent process evidence.
Separate manual launch/CIM observations record host app PID 12300 -> 25264 and
guest app PID 46116 -> 39776, with only the other backend remaining after each
close and none after final close. These observations are saved separately in
process-restart-observations.json. Battle proof records held as an observed value;
the harness did not explicitly require held or a specific wrong-turn error code.
These distinctions remain part of the acceptance limits.

## Native held-zero-HP recovery acceptance

Verified with rebuilt c6871be backend and both isolated debug Tauri applications.
To exercise the recovery path, a test baseline starts the host injured at one HP
out of twelve. This is set through the existing character API before combat, not
by forcing zero HP after combat. Real defense actions then leave the host at zero
HP with held outcome. Combat/movement use real production API/IPC; both WebViews
render the fight. Recovery uses actual DOM controls in the production room screen.

In the successful run, guest Тара clicks aid for Мира. The server records Medicine
14 + 0, stabilizes the host and schedules three game hours. Typed movement actions
bring the shared scene to town. The host clicks Wait for recovery and its modal
confirmation; canonical HP becomes one. Host clicks group Long Rest; both clients
and SQLite report twelve out of twelve HP. No save is loaded from the selected
battle start through recovery/rest. Independent SQLite timestamp/reset-cursor checks
confirm that claim, rather than relying only on a literal script flag.

An earlier separate attempt failed aid and death saves: Мира died with three
failures, zero HP, no recovery actions and the real UI showing Погиб. That is a
recorded death branch, not a successful recovery. Subsequent tests reset their
starting baseline before a new encounter. The successful scenario has no post-battle
rollback; it does not claim ordinary healing can resurrect a dead character.

Sanitized evidence: .artifacts/two-native/recovery-battle-proof.json,
recovery-ui-proof.json (including actual aid receipt), recovery-durable-proof.json,
recovery-death-proof.json and masked recovered/death screenshots. Logs:
.artifacts/recovery-native-directed-battle.log, recovery-native-ui-current.log and
the earlier failed recovery-native-ui.log. Native applications/backends and Vite
are closed afterward. This is real localhost/debug native evidence; physical LAN,
real-model narration, attack/spell dialogs and full solo acceptance remain separate.
The observed co-op held-zero-HP continuation now succeeds; it does not complete the
whole project goal. Independent read-only evidence review confirmed the selected
encounter and revisions: held 135, aid 136, movement 137-138, passive recovery 139,
group rest 140; no intervening restore and both persisted HP12. It also confirmed
the distinct failed-death branch (Medicine9/9, saves4 then1) and process cleanup.
The specific co-op held-zero-HP continuation P1 is closed locally.
