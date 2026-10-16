# First entry and profile migration

First launch presents two real navigation actions: **Start adventure** and
**Join game**. The existing header supplies the language selector and native
window controls. The entry surface has no active campaign chat, tactical map,
provider preflight, global hero picker or media setup wizard behind it.

Start adventure opens campaign creation. Hero creation/selection belongs to that
campaign and is committed through its scope before play. Guest entry opens the
existing native room interface, without choosing a local model or solo hero.
The saved guest choice restores that interface on the next boot; saved native
guest/host recovery records remain under the original transport authority.

Onboarding version 2 persists completion and the selected entry separately.
Legacy completed profiles retain their settings and use adventure navigation.
Partial profiles retain existing character sheets and drafts; choosing an entry
does not clear them or fabricate a hero. An explicit return from the room updates
navigation intent and does not repeatedly force the guest screen back open.

Model configuration is requested when a player first tries to send an AI turn.
With no configured server, or no started local model, the composer opens the
appropriate settings tab while retaining unsent text and attachments. It sends
no request and creates no gameplay operation in that case. Backend environment
and loaded-model checks remain authoritative when a request is dispatched.
Media is optional; its missing configuration does not block browsing or drafts.

Validation uses the real root and production-browser entry at 960x640 and
1280x800 in RU/EN, guest restart, preservation of legacy data and the no-dispatch
composer gate. Browser fixtures verify routing, not native transport or inference.
Fresh Tauri entry/campaign/setup acceptance is grouped at Tasks 12-15.

Grouped native checkpoint for Tasks 12-15 passed on the fresh isolated debug QA
build: initial entry, scoped hero/full-wizard cancel, confirmed initialized world,
settings status/advanced controls and actual SQLite cold restoration. Native
portrait decoding was checked separately before accepting the art screenshot.
Model/media inference and physical LAN acceptance remain separate.
