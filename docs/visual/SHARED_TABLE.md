# Shared table presentation

`features/game-table` owns the role-independent presentation contract and views.
`app/tableAdapters.ts` projects solo and room controller snapshots into that
contract. `GameTableExperience` binds the appropriate controller callbacks; the
feature receives no transport, Zustand store, rules engine or sibling-feature
internals. Existing game commands, revision checks and recovery identities remain
in their controllers and the server.

The shared model includes public actor IDs, vitals and appearance references,
player-visible conversation/outcomes, connection/readiness/recovery status and
action availability. It does not carry private sheets, proposals, invitation
credentials, system messages, arbitrary tool arguments or provider payloads.
Own-actor identity comes from canonical character/member IDs, not display names.
A guest's connected transport does not prove that the host model passed inference;
that readiness remains unknown. Local sending requires both confirmed provider
configuration and the matching native loaded model.

Scenes explicitly distinguish exploration from tactical geometry. An encounter
flag cannot manufacture a map. The UI scope key contains campaign/session, role,
room/epoch, restored timeline and encounter IDs. Ordinary revision updates retain
the key. Changed timelines invalidate selections without remounting text controls
and losing unsent drafts. The chat width reuses the existing preference bounds.

Bound presentation ports forward UI intentions to existing role controllers.
Optional context actions are omitted until their real panels are available.
Debug scope logs are opt-in through `VITE_TABLE_TRACE=1` and contain safe IDs only.

The scope surface is integrated in the production solo/host/guest composition.

## Persistent layout

The shared `GameTable` keeps a compact public party strip above scene/map and
conversation columns. Narration scrolls within its panel; the composer stays at
the bottom. Both role adapters supply existing scene/chat controls as slots, so
the table does not reproduce transport or game commands. The keyboard/pointer
resizer uses the original shared chat preference and limits. At minimum width,
secondary titlebar labels collapse into the existing named icons.

Room credentials, invitation refresh and leaving/closing the room live in the
Room modal. Hero approval, authored actions, rest and wounded-hero help live in
the Adventure context modal. The existing recovery notice remains immediately
reachable. Host configuration is exposed through its real port; guest model
setup is not fabricated. Modal nesting handles Escape/Tab only at the top dialog
and returns focus to the parent, retaining unsent text.

Layout acceptance uses the real production root at 960x640, 1280x800 and
1920x1080 in RU/EN, host/guest/solo flows, map/composer bounds, keyboard resizing,
credential masking and context focus return. The native checkpoint additionally
uses actual solo and host controllers, production assets and isolated SQLite.
Browser layout success does not establish separate-device LAN or model semantics.

## Combat identity and selection

The mechanical hit cell and drag coordinates remain independent of the whole
miniature. The selected portrait resolves a distinct matching miniature; custom
portraits retain a portrait fallback. Unknown appearances use a neutral pawn.
All public room actors resolve by character ID, including equal display names.
Selection is shared between the map and initiative, persists through revisions
and is invalidated by a changed encounter/timeline or removed actor. It never
grants command authority. Dialogs accept eligible selected targets and retain
choices on failed ACK; obsolete revision scopes close the dialog. Both roles
use the same resource/action bar, with only supported room commands exposed.
Existing server range, action, resource and ownership checks remain authoritative.

Production browser acceptance covers 30/48/64 screen pixel miniatures through
real wheel zoom, pointer-free silhouettes, stable world cells and shared selection.
The frontend suite at the combat checkpoint passed 195 files / 1212 tests.

## Conversation and input

Solo and room messages use the same safe narrative rendering and speaker labels.
Room wire data identifies a user role, not its author, so no guessed player name
is attached. Truncated public messages are marked. Tool outcomes expose only the
existing allowlisted confirmed numbers; diagnostics remain in the inspector.
Sticky scrolling respects scroll-away and offers a latest-message action.
Recovery sits beside the composer. Existing publication gates remain unchanged.

Unsent text uses a versioned `conversation-drafts.json` file, bounded to 32 scopes
and 32000 characters each. Solo scopes use campaign/session; room scopes include
room/epoch/member. Late hydration cannot overwrite typed input. Rejected reads
or failed disk flushes retain the input and show a safe status. Accepted dispatch
retains the original controller pending identity. Attachments survive navigation
in the current process; their bytes are deliberately absent from the cold-start
text draft store. Browser acceptance proves long RU narration, scroll-away,
panel focus return, Shift+Enter and scoped text restoration after reload.

## Context and canonical public data

The Scene/Hero/Journal/NPC tabs use a dock over the scene and retain the mounted
composer and map selection. The same contents expand into the existing accessible
Modal. Tab arrows, Escape and focus return work at minimum and larger widths.
Journal and save dates follow the selected language and actual stored timestamps.
Own hero context reads only canonical fields from the owned document; room
adapters check member/character identity and campaign IDs. A malformed context
is isolated from chat. Unknown/private fields and undiscovered world canon are
not used as dossier content.

The application projection publishes `public_goals` from the currently offered
scene actions, excluding travel and session finish. Legacy documents default to
an empty goal list; no future quest or hidden condition is guessed. Existing
revision-checked controllers dispatch all actions. Completed discoveries remain
in the public fact list. NPC records are keyed by ID, and ambiguous same-name
fact additions are refused. The cosmetic `npc-appearance.json` ledger freezes
the prior eight-way portrait assignment by campaign/record ID; catalog growth
does not reshuffle known faces. Later art waves upgrade these stable references.

Entering co-op from a cold solo session refreshes the host campaign catalog.
Catalog loading/failure/retry is shown beside the host selector; guest joining
does not depend on this local catalog or local inference setup.

The actual Tauri QA checkpoint enters Archive defense through four authored UI
actions and renders the selected fighter miniature at 30/48/64 screen pixels
under the real desktop CSP. A normal close and cold launch retain HP 12, AC 18,
inventory, combat fingerprint, 900 seconds and four adventure receipts, plus the
scoped text draft. Actual native host creation on an ephemeral loopback port,
public appearance, owned context and confirmed room close also pass. No separate
physical guest/LAN, generative model, GPU media or release installation is implied.
The debug media sidecar is an empty packaging placeholder in this QA environment.

Final checkpoint checks: consolidated full gates pass (1003 Rust tests and
199 frontend files / 1222 tests), production build passes, and 43 production
browser scenarios pass. Checks use the working tree with preserved inherited
changes; they do not claim CI or deployment validation. The spell target picker
excludes confirmed dead actors and retains aid to downed humans. An absent
controlled selection remains empty until explicitly selected, instead of
visually substituting the first option. Speaker and message body remain separate
semantic elements, preserving exact public text and retry/clarification history.
