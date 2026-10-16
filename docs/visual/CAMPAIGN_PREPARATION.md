# Campaign preparation

Prepared play follows three real stages: choose an adventure, confirm a hero in
that campaign, and enter its initialized first scene. The library is always
reachable through Back. The custom route saves a title and opening description;
it does not claim to generate an authored quest sequence. Document import is not
offered until there is a working importer.

Four prepared heroes reuse the existing ability and HP presets, with explicit
canonical equipment and class-legal spells from the bundled SRD catalog. Their
defense choice is checked and derived by the server. Full customization remains
available through the existing wizard; cancelling keeps its scoped draft.
Selection and name edits never alter the active character before confirmation.

## Interrupted preparation

Non-secret source/title/premise drafts and mutation identities are stored in
`campaign-preparation.json`. Creation persists its UUID before POST; retries use
the same UUID, including after a lost response or language change for the same
bundled adventure. The server compares original inputs and creates campaign,
session, event and opening scene in one transaction. Concurrent retries return
the existing aggregate. Changed inputs under the same UUID are rejected.

Character preparation flushes the existing scoped wizard draft before sending.
Only the request identity, actor/session IDs, original revision and document hash
are added to the preparation file; portrait bytes and credentials are excluded.
The controller reads canonical scope before dispatch. A matching retry preserves
the original revision and causation ID, allowing the existing receipt to replay.
Back and reopening the exact campaign retain the draft. Another campaign resets
it. Late responses cannot apply after a campaign transition.

Malformed persisted attempts are retained and block mutation rather than being
silently replaced. A failed disk write prevents dispatch. Attempts are removed
only after canonical server acknowledgement. A cold load discovers a hero that
was committed before its response was lost.

World readiness checks an initialized authored adventure package or a real stored
opening scene. Having a class or a premise alone does not establish readiness.

## Evidence

SQLite integration covers concurrent creation, lost ACK replay, transactional
rollback and all four production hero fixtures. Frontend tests compare those
fixtures with the actual SRD catalog, retain drafts and identities, and prevent
cross-campaign leakage. Production-browser coverage includes RU/EN at 960x640
and 1280x800, lost create/hero responses, rejected confirmation, Back and cold
reload. Native proof for entry and settings is recorded at the grouped checkpoint
for Tasks 12-15; browser fixtures do not establish native/model inference quality.

Grouped native checkpoint for Tasks 12-15 passed on the fresh isolated debug QA
build: initial entry, scoped hero/full-wizard cancel, confirmed initialized world,
settings status/advanced controls and actual SQLite cold restoration. Native
portrait decoding was checked separately before accepting the art screenshot.
Model/media inference and physical LAN acceptance remain separate.
