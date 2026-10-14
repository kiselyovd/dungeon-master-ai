# Workspace and native window preferences

The shared table remembers chat/context widths, the selected context panel,
reading size, expanded context and split/table/chat view. The versioned
`workspace_v1` record projects a fixed whitelist of display fields. Unknown panel
or view values, invalid numbers and unsupported versions recover safely. Older
chat-width and reading preferences remain supported. These settings never contain
campaign content, drafts, room proofs or credentials.

Use the view buttons above the table to choose Split, Table or Chat. The context
divider supports dragging, arrow-key nudges, Shift for larger steps and Home/End.
Escape, blur and visibility loss cancel a drag. Reset layout restores the default
panes, reading size and view while keeping the current campaign and draft. In
Tauri it also invokes the main-window geometry reset; a native failure is reported
without pretending that the window changed.

Native `desktop.json` stores the versioned normal logical size, monitor-relative
position and maximized state. The existing store plugin supplies persistence.
Restore clamps to an available monitor's work area, including nonclient borders.
Invalid geometry uses defaults; a missing monitor falls back safely. Only the
main window captures geometry. Minimized/fullscreen values do not replace normal
bounds, and maximizing preserves the previous normal size. Native reset rejects
an auxiliary caller.

Browser checks passed panel/size/reading/view restoration after reload, draft
preservation, reset and malformed preferences. The frontend suite passed 1,306
tests in 210 files; focused persistence/workspace checks passed 74 tests.
Rust geometry tests passed simulated 125/150/200% DPI and removed-monitor cases.
Fresh Tauri checks at the available OS scale 100% passed resizing to 1100x700,
maximize/cold restart/unmaximize, real UI reset, malformed geometry and a recorded
non-existent monitor. Actual window bounds stayed within the current work area;
uncaught JavaScript exceptions were zero. Owned test processes were closed.

The physical/native 125/150/200% display matrix remains unverified: this QA
environment exposes only a 100% monitor. Simulated calculation coverage is not
equivalent to those native display checks. The plan keeps this acceptance item
open. Grouped full/native verification and an owned checkpoint commit follow the
command-palette task.
