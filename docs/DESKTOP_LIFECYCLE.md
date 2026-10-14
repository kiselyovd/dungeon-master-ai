# Desktop windows and application exit

Closing an auxiliary window disposes that view. The main window, backend,
model/media runtime, room listeners and ongoing canonical operations continue.
Closing the main window exits the application; hosting does not continue in a
tray or background process.

The production root binds one main-window close listener to the same policy
used by its title-bar Close button. Hosts, guests and pending operations get a
confirmation dialog with **Cancel** focused first. It explains that a host
disconnects the room's players, a guest leaves the room, and an interrupted
response may require recovery. Cancel neither departs the room nor stops a
process. Repeated close requests share one dialog and one in-flight exit.

Before exit, mounted composers flush their latest scoped text directly through
the existing draft store. Preferences and secret-bearing settings use their
existing allowlisted storage adapters and Stronghold. A failed draft barrier
keeps the application open. Room departure calls the existing `stopHostRoom`
or `leaveGuestRoom` operation; a confirmed departure is not repeated if the
final exit needs another attempt.

If departure cannot be confirmed, the application remains open. The user can
retry or separately choose **Close and keep recovery**. This preserves current
data and the guest's existing postponed recovery selection before closing the
local application. It does not claim that the remote server accepted departure.
A stale dialog cannot close a different campaign or connection.

Native `CloseRequested` does not stop processes. `main` destruction requests
application exit, while auxiliary destruction only disposes its view.
`RunEvent::Exit` performs cleanup once: stop accepting control requests, cancel
room streams, stop runtime children, then stop the backend. A partial startup
failure after creating the control server uses the same cleanup. Runtime
shutdown prevents queued restarts and disposes a late child from an obsolete
generation. Only the main WebView can invoke `desktop_confirm_exit`.

On Windows, a private Job Object is established before creating windows or
spawning children. Its non-inheritable handle remains open until process
termination, and its `KILL_ON_JOB_CLOSE` limit contains the child process tree
even if the app is killed without Rust exit callbacks. Normal cleanup still
runs explicitly. Windows child inheritance and the limit behavior follow
[Microsoft's Job Objects documentation](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)
and the [basic limit definition](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_basic_limit_information).
This adds no production dependency. Other platforms retain explicit normal
cleanup; the recorded crash containment proof is Windows only.

Local verification records the production-root dialog and guest departure,
browser host/guest cancellation and unconfirmed-departure flows, native scoped
auxiliary close/destroy/reopen, actual host stop, healthy GUI crash, partial
startup failure and exactly-once cleanup. A native interruption case with a
deterministic protocol fixture preserves two receipts, 600 seconds, request
identity and the unsent draft across confirmed close and cold restart. This
fixture proves recovery transport and persistence, not real-model quality.

QA window-creation/destroy permissions and the blank auxiliary fixture belong
only to the isolated QA configuration. Production capabilities remain scoped
to the main window; production detached views are a separate plan task.

Repeated launch uses the official single-instance plugin before backend setup.
It restores and focuses the existing main window without replacing its campaign,
draft or pending operation. Unknown launch arguments are rejected. Native logs
contain event types only, never raw arguments or working directories. An early
focus request waits for the main window. Windows plugin initialization is
serialized through Ready to prevent a second backend in the plugin's short
mutex/message-window startup gap. The plugin owns command forwarding.

Application identity is the configured bundle identifier. Concurrent native QA
clients use different identifiers, databases and profiles; changing a database
override alone does not create a second application identity.
