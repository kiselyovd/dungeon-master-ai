[Previous: Room invitations](ROOM_INVITATIONS.md) · [Back to README](../README.md) · [Next: Workspace preferences](WORKSPACE.md)

# Desktop notifications

Notifications start disabled. Enable them in the advanced behavior settings;
the same control is available in room settings. Windows can independently block
the application in **Settings → System → Notifications**. The application reports
the actual Windows notifier setting; enabling the in-app control does not change
an OS refusal. Unsupported platforms keep the control unavailable.

Only confirmed metadata can produce a notification: the living owned actor's
turn, a completed native model start, a completed model download with its server
receipt, or an actionable failure after an established room connection. Another
player's turn, unchanged status, download progress, streaming tokens and private
narration produce none. Bodies contain only generic English/Russian text. Changing
the UI language also updates subsequent native model-ready notifications.

Any focused application window suppresses delivery. Observed events are consumed
even while focused, disabled or denied, so they do not appear later as a backlog.
A persisted monotonic ledger rejects duplicate and older turn/connection events
across reconnect and application restart. Its bounded capacity fails explicitly
rather than forgetting replay protection.

Clicking restores the existing main window. A typed activation opens the relevant
chat, download settings or connection view only if its scope is still permitted.
It cannot replace a campaign, join a room or run a game command. The Windows
adapter registers a per-profile COM activator and an identity-specific Start-menu
shortcut after opt-in; it restores that registration on subsequent launches.
Activation arguments contain an opaque single-use UUID. The bounded native outbox
stores only event kind, canonical IDs and destination, never message bodies,
provider credentials or invitation links.

Local Windows QA verified real delivery to Notification Center, a human click
reaching the COM callback and restoring the main window, and actual OS refusal
returning `denied` without sending. Earlier sender-only delivery did not receive
the click and was replaced. Focus/replay policy, host/guest ownership, language
updates and native serialization have separate automated coverage. Installer,
other-OS and real-model gameplay acceptance remain separate from this adapter QA.

## See also

- [Desktop lifecycle](DESKTOP_LIFECYCLE.md)
- [Commands and role limits](COMMANDS.md)
- [Room invitations](ROOM_INVITATIONS.md)
