[Bell scenes](BELL_SCENES.md) · [Confirmed gameplay motion](CONFIRMED_MOTION.md)

# Scene transitions and motion preferences

Ordinary movement updates the public location painting and caption within the
table. Its short arrival does not cover the conversation, disable actions, or
move focus. The scene-name classifier and fullscreen 3.2-second interruption
are removed. Routine movement also no longer creates an unsolicited video-setup
toast.

The existing preference now describes cinematic moments at the beginning/end
of confirmed combat. These optional accents are small status cards inside the
map viewport, with pointer passthrough and an immediate Skip action. Small
windows use a compact text presentation. Cards do not behave as dialogs and do
not focus themselves. Initial hydration, restored timelines, repeated encounter
events and focus return do not replay a moment.

Presentation scope is separate from request invalidation. Confirmed adventure
continuations retain their presentation generation; loading/restoring a campaign
resets it. The request generation continues advancing, preserving the existing
protection against late responses. These fields are transient and are not
workspace preferences or portable campaign data.

The shared presentation-motion hook observes the OS reduced-motion preference
and window/document activity. Decorative ambient work pauses outside an active
window. Reduced motion uses static art, immediate committed movement, immediate
overlay closing and no delayed cinematic. Requests and canonical operations
continue normally. The CSS overrides target decorative surfaces and do not
remove map, miniature or ruler transforms.

## Art bindings

`transition-art-bindings.json` maps the four generic legacy roles to selected
illustrations: combat to ruins, dialog to tavern, exploration to forest road,
and dungeon to dungeon. A canonical public location painting takes precedence
when available. These mappings introduce no new masters or terrain semantics.

The four original transition paintings retain their hashes in the documented
archive under `.artifacts/visual-production/legacy-scene-transitions/`. They are
excluded from shipping catalogue imports. The asset check verifies the active
bindings and archived bytes. The active catalogue has 145 entries and totals
21,128,951 encoded bytes; the selected-master and final consumer audit remain
separate from this shipping count.

## Evidence boundaries

Focused tests cover ordinary movement/focus, canonical moments, scope/replay,
reduced and background behavior, close timers and preserved art hashes. The
production browser exercises a confirmed real-backend encounter while retaining
an unsent draft. A delayed real combat response is delivered with a test focus
sensor set inactive, then focus returns without replay. This sensor is explicit:
headless Chromium does not reproduce native OS focus semantics. Native focus and
fresh-build acceptance are verified separately at checkpoint9.

The full frontend suite passed 1,304 tests in 209 files. Focused motion/art tests
passed 55 cases, with 18 additional projection/closing checks. Actual-backend
scene journeys passed in RU/EN at 960/1280/1920 in both normal and reduced motion;
they retain drafts through real save/restore/reload and cover missing art. Three
Archive DPR checks retained alignment, ruler and server movement behavior. Fast
gates and production build passed. The isolated backend was stopped afterward.
