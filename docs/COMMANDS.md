# Commands and shortcuts

Open **Commands** in the title bar or table controls, or press **Ctrl+K**
(**Meta+K** on macOS). Search uses the current English or Russian labels.
Tab reaches the matching actions; Arrow Down from search focuses the first
enabled action. Enter runs a single enabled search result. Escape closes only
the top dialog and returns focus to its opener.

| Command | Windows / Linux | macOS |
| --- | --- | --- |
| Quick save | Ctrl+S | Meta+S |
| Saves | Ctrl+Shift+S | Meta+Shift+S |
| Hero | Ctrl+Shift+H | Meta+Shift+H |
| Journal | Ctrl+Shift+J | Meta+Shift+J |
| NPCs | Ctrl+Shift+N | Meta+Shift+N |
| Settings | Ctrl+, | Meta+, |
| Recovery | Ctrl+Shift+R | Meta+Shift+R |
| Focus chat | Ctrl+Shift+C | Meta+Shift+C |
| Fullscreen | F11 | F11 |

Map focus, workspace reset, room controls and ending your turn are available
through the palette. Runtime and tool diagnostics remain under **Advanced
commands**, with the existing Ctrl/Meta+Shift+M and Ctrl/Meta+Shift+I shortcuts.

Commands are composed in `src/app/commandRegistrations.ts` and bound by the
solo or connected-room composition to the existing owning controllers. Guests
cannot configure the host or invoke local saves. Local saves are unavailable
inside a room, including on the host. Ending a turn requires a living owned
actor, its confirmed current turn and no pending operation or recovery. The
controllers and server still enforce request identity, role and revision.

Typing and IME do not invoke navigation commands. Palette, quick save and
fullscreen are the explicit shortcuts usable while editing. A dialog blocks
other global commands; the palette may be opened above it, and fullscreen
remains available. Held keys and in-flight duplicate invocations are ignored.
Queued commands are canceled when their campaign or room timeline unmounts,
and availability is checked again before dispatch. Palette searches, drafts,
room credentials and exception payloads are never logged or persisted.

Browser tests exercise the production React root with fixture transport; native
fullscreen and filesystem behavior require a separate Tauri run. Platform
modifier tests simulate Windows/Linux and macOS and do not claim an actual
macOS device check.
