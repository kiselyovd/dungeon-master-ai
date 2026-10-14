# Detached hero and journal

The desktop hero and journal views offer **Open in separate window**. The hero
sheet also exposes this action in its header. Secondary views are read-only:
game commands, rest, setup, provider configuration and campaign file operations
remain in the main window. **Return to game** focuses the main window. Closing
a secondary view keeps the game running; exiting the main window closes all
owned views and backend processes.

## Projection and access

`src/auxiliary.tsx` and `src/app/AuxiliaryApp.tsx` are the detached production
entrypoint. They reuse `CharacterSheetView` and `JournalViewer` without mounting
the game store, provider startup, room controller or a second backend.

The main composition publishes a bounded version-one projection. Room data comes
from the participant-scoped room controller and `roomContext`; the accepted
character must match the member's actor. Journal entries must belong to the
selected campaign. The native adapter checks the selected room/member scope,
binds each reader to its actual window label, and supplies only that view's
payload. The journal reader never receives the hero document.

Sequence numbers reject delayed publications. Readers request changes once per
second; unchanged revisions return only a sequence acknowledgement. Five-second
heartbeats carry no campaign payload. A lease expires after fifteen seconds if
the main composition stops refreshing it. Scope departure, unavailable state or
expired access replaces the view with an unavailable message. A save timeline
change or campaign switch publishes the current projection under a new scope.

## Native boundaries

The main and detached windows have distinct command capabilities. Detached views
can read their projection, focus the main window and close themselves. Their
close command receives the caller window from Tauri and accepts no target label.
They cannot open other windows, obtain the backend port, mutate rooms, read the
settings store or Stronghold, or select campaign files.

Detached WebViews use an isolated profile with private browsing and a restrictive
CSP. They cannot issue loopback or external network requests. Projection data is
ephemeral; window placement uses separate versioned hero/journal keys in the
existing native desktop preferences. Main-window geometry has its own key.

## Verification limits

Windows QA has exercised both actual WebView roots, per-window command denial,
loopback blocking, absence of a main-storage canary, close/reopen/focus, an actual
library campaign switch and main-exit process cleanup. Component and production
browser tests cover owned/guest projections, invalid identity, revoked access and
sanitized journal HTML. Browser fixtures are UI evidence, not native authority.

Physical second-monitor and 125/150/200-percent scale checks, fresh native guest
revocation, and detached uploaded/generated portrait rendering remain acceptance
items in the active implementation plan. Other desktop operating systems are not
verified by the Windows run.
