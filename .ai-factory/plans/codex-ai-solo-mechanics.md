# AI-directed solo session and recovery plan

Goal: preserve the full thread objective: complete an AI-led solo adventure and LAN/VPN co-op, rather than substituting procedural menu progression.
Execution: inline, authorized by 'Продолжай до конца'; preserve inherited source changes and add no production dependencies. Independent final review remains required. UI uses existing tabletop/chat/adventure primitives and EN/RU catalogs.

## Ordered work

- [x] Typed `adventure_action` tool: domain action-only payload, server-derived character/revision, operation identity scoped to request/call; exact receipt replay without duplicate rewards. Refresh current public outcome on replay rather than returning obsolete restored state.
- [x] Prepared-adventure context limits tools that could rewrite canon, scene, NPC memories or rewards independently. Server execution enforces the same restriction, including hallucinated non-advertised tools. Context failure is explicit.
- [x] Deliver confirmed adventure outcome to the renderer without interrupting its own active chat stream; validate identity and monotonic revision. Narration follows engine confirmation.
- [ ] Durable turn IDs/operations and ordered campaign execution; reconnect/cancel/retry resumes recorded effects without rerolling tools. Keep committed outcomes separate from incomplete narration.
- [ ] Canonical solo actor/token binding, attack/spell/rest/resource/range/turn behavior and a resolved defensive encounter. Prepared map geometry persists and appears on the real tabletop.
- [ ] Scripted-provider integration first, then real model/tool/narration acceptance; evaluate installed Bonsai through a verified compatible runtime without promoting untested weights.
- [ ] Full gates, production build/browser/native complete peaceful and defensive solo sessions, restart/recovery, documentation and scoped checkpoints.
- [ ] Continue party ownership and LAN/VPN room/event/reconnect/two-client acceptance; no narrowing of the original objective.

## Authored defensive session

Co-op foundation continuation: domain party identity/ownership, separate guest
storage and owner-scoped views implemented; migration0008 and atomic enrollment
receipts preserve the existing solo host sheet. Snapshots capture/restore guest
sheets while requiring unchanged ownership/roster. Domain3, snapshot1 and SQL7
checks pass, including real pool close/reopen, rollback after second guest failure
and malformed duplicate snapshots. Independent review closed the duplicate-entry
guard issue; historical migration fixtures/count updated. Final full gates passed
(workspace Rust and1042 frontend tests). No HTTP/LAN authentication, multi-hero combat/UI or native two-peer
acceptance claimed. Remaining sequence: .ai-factory/plans/codex-lan-coop.md;
storage notes: docs/PARTY_STATE.md. Reqwest approval still pending; no dependency
added. Proposed AGENTS rule: each embedded migration change must update both
manual historical-schema fixtures and migration/schema assertions.

Schema/model follow-up,2026-10-07: adventure_action oneOf now requires location
only for move; unit actions reject it. Real malformed seal intent reproduced
the old schema/decoder mismatch RED; schema regressions GREEN. Real Bonsai
completed ten requests/86.961s through HTTP/SQLite with zero tool errors,
Tikhon rescued, closed_crossing, exactly one ten-gold reward and confirmed quick
save. Independent schema review found no actionable issue; full gates pass
(workspace Rust and1042 frontend tests). Narrative review still finds roster
mistranslation and extra coins in prose; persisted inventory is correct. Keep
full model/native/defensive/co-op boxes open. Explicit reqwest approval is still
pending; no dependency added. Both isolated evaluation processes stopped.

Proposed AGENTS rule for repeated schema drift: validate published tool schemas
against typed commands and captured real-provider payloads; MockProvider-only
checks do not prove the outgoing wire schema or context.

Real-model continuation,2026-10-07: installed Bonsai PTQ failed under LM Studio's
stock runtime; checksum-verified isolated Prism CUDA runtime loaded it and passed
a native tool-call/confirmed Russian narration probe. Real application turns
reproduced missing cumulative tool capture and omitted system context in the
OpenAI-compatible adapter. Wire RED→GREEN fixes restore both; shared completion
now preserves observed tool order. With context restored, first five real steps
advanced one chosen action each. Full semantic/session acceptance remains open:
raw identifiers and misreading completed intentions still appeared. Context
clarification awaits rerun. Genai's same-frame multiple-tool gap remains open;
pure wire prototype passes three tests, production integration awaits explicit
reqwest approval under AGENTS. No provider promotion or full native/co-op claim.
Details and runtime provenance: docs/BONSAI_EVALUATION.md.

Canonical DM intention checkpoint, 2026-10-07: strict `combat_action` and
`rest_action`, server-derived actor/revision/seed, scoped immutable existing
combat/rest receipts, public result delivery and per-round context/catalog
refresh implemented. Decoder2, real SQLite3 and scripted-provider7 checks pass.
Independent review's terminal delivery, collaborative catalog and old active
receipt findings were reproduced/fixed; follow-up closed them. SQLite regression
covers old Move after A closes, old retreat while B is active, and old Move after
B closes. Production-browser3 scripted SSE cases confirm scene, rest HP and
combat closure during narration. Production build and final full gates pass
(workspace Rust and1042 frontend tests); original32 source hashes unchanged.
No real model/native AI-led/LAN acceptance claimed. Wider mechanics/recovery and
party requirements remain open. Live LM Studio inventory sees the installed
`ternary-bonsai-2-27b` PTQ file and no loaded models; next verify its real runtime
and tool/narration behavior before provider promotion.

Defense checkpoint collection, 2026-10-07: linked optional encounter, authored
12x8 map, host pathfinding/line of sight, exit retreat and nonlethal recovery
implemented. Domain/application/SQLite regressions cover outcomes, round cap,
receipt replay, rollback and active snapshot restoration. Production-preview
browser against real isolated HTTP/SQLite completed defense, rescue, sealed
ending and reload with one reward and spent slots. Catalog regression reproduced
missing start_defense and passed after correction. Independent read-only reviews
closed the round-cap issue and found no remaining actionable issues in this
scope. Final full gates passed (workspace Rust and 1041 frontend tests), production
build passed; original 32 baseline hashes remain unchanged. Owned backend stopped.
Native AI-directed play, wider spells/class features, AI combat/rest intentions,
real-model/Bonsai and LAN/VPN acceptance remain pending; broad tasks stay open.

Next AI interface: expose strict intention-only `combat_action` and `rest_action`
tools. Derive campaign, session, character, encounter, actor, revision and dice
seed on the server. The model selects an available weapon/spell/target, a map
destination, end-turn/retreat, or Short/Long/Wait; it cannot supply HP, damage,
resource balances or a replacement map. Reuse the application resolver and
transactional repositories, with operation/call-scoped immutable receipt IDs.
Supply current combat positions, budgets, supported choices and rest/time state
in context, refresh after confirmed tools, and deliver public projections through
the existing atomic frontend ports. First verify scripted-provider effects,
replay, rejection without spending, and narration following confirmed outcomes;
then exercise actual model and native play. The intention adapter is implemented;
actual model/native acceptance remains pending.

Add a voluntary Archive defense after locating Tikhon. The server creates a linked encounter and immutable12x8 upper-landing map in the same adventure transaction/revision/receipt. Two custom water-echo hazards use authored profiles and spawn positions; the saved hero keeps its canonical resources. Holding three rounds or dispersing the echoes confirms a rescue window; combat alone grants no rescue reward or ward restoration. Retreat requires reaching the mapped dry exit. Defeat is explicitly nonlethal: Mila/Agata recover the hero to town at1HP after an hour, retaining spent resources and unfinished objectives. Resolve story state, HP/time, combat projection and receipt atomically; snapshots/retries preserve the link and cannot fabricate victory.

Geometry is host-authoritative: bounded walkable cells, walls, occupied destinations, path costs, NPC pathfinding and line-of-sight checks. UI brief: draw the server map inside the existing VTT using existing grid/palette controls, label the dry exit and objective in EN/RU, add the existing-style retreat control only for a linked encounter. Reuse current action/weapon/spell panels and tokens; no new visual tokens or production dependencies. Verify domain outcomes/path rejection, real SQL compound creation/closure/rollback/replay/restore, then full peaceful and defensive browser/native sessions. AI intention delivery/context must carry the confirmed combat/story state; generic mutations cannot substitute authored outcomes. Full spell/class/model/recovery/party requirements remain active.

## Spell and rest execution

Rest/resource continuation: move the shared adventure clock into a session-owned `AdventuringState`, captured/restored with snapshots. Keep remaining Hit Point Dice and the last completed Long Rest in the canonical hero resources. Short Rest takes60 minutes, spends explicitly chosen class hit dice with Constitution (minimum1HP per die), and restores Pact slots. Long Rest takes480 minutes, requires16 hours since the previous completion before starting, restores HP/hit dice/slots, and never replenishes ammunition. Waiting is an explicit bounded time intention, not a client-supplied clock value. Host checks current session/character/revision, active combat and authored-location safety; the Bell's town is the safe rest location. Persist clock, sheet, revision and immutable outcome receipt atomically, replay without rerolls, and invalidate rest receipts on timeline restoration. Extend slot capacities to the class tables so higher-slot casting and Pact recharge operate on real resources.

Rest UI brief: one existing-style modal opened from character controls, with Short/Long/Wait choices, hit-dice/time selection, HP/slot stock, elapsed game time and cooldown/safety feedback. Reuse Modal/Field/Select/Button and existing layout/tokens, with EN/RU strings. No production dependencies. Verify real backend/browser/restart/snapshot and independent review before declaring this resource vertical complete; interruption encounters, class recharge features, full authored session and LAN remain part of the broader goal.

Rest/resource collection,2026-10-07: deterministic Short/Long/Wait, higher slot/Pact tables, shared second-resolution session clock, protected stocks, atomic resource/clock/revision/receipt commits and snapshot restoration implemented. Domain6, SQL7, migration/repository and hook3 checks GREEN. Renderer outbox recovered a lost committed response after reload with the same UUID. Four real-backend production-root browser scenarios and actual backend restart/exact Short+Long replay GREEN; no second effect, arrows unchanged and cooldown retained. Independent review's delayedGET/localstatus regression reproduced and fixed; follow-up closed it and legacy-active-combat blocking. Full gates GREEN (workspace Rust,1041 frontend tests and architecture/static checks), final builds GREEN. Original32 source hashes preserved; isolated backend stopped. Other spells/concentration/class features, unsafe/interrupted rests, AI tools/context/native complete solo and LAN/VPN party remain active requirements.

The current bound `Cast` consumes an action without a spell or effect. Replace it with a typed intent (actor, prepared spell ID, target, slot level), derived casting profile and persistent slots. Begin with Fire Bolt, Eldritch Blast, Sacred Flame, Cure Wounds, Healing Word and Magic Missile; unsupported prepared spells must fail before consuming anything and remain explicit pending work. Validate preparation/class, targets, range, action or bonus action and the one-slot-per-turn rule; cantrips consume no slot, healing caps at max HP, spell events follow the canonical dice. Preserve slots across encounters, reload and snapshot restore through the existing receipt transaction. Then add remaining selected spells/concentration and outside-combat healing/rest with time/safety rules; no claim of all-spell completion from this initial engine.

UI brief: keep the existing Cast control, use the current Modal/Field/Select/Button patterns for prepared spell, target and slot selection, show remaining slots and explicit unavailable spells, retain EN/RU locale catalogs. Rest uses the established character/adventure controls, with confirmed outcomes and no invented visual tokens. Verify the real production root and backend before completing this vertical. No new production dependencies.

Initial spell core progress, 2026-10-07: six typed domain effects and intention-only HTTP, class/preparation validation, slot consumption with atomic HP/receipt/campaign commits, snapshot stock/turn-flag restoration, EN/RU spell selection dialog and bonus-action availability implemented. Twelve domain checks, SQL replay/restore and dialog/hook/action-bar checks GREEN. Real-backend production-root browser healed with Healing Word, retained its action, reloaded and cast Sacred Flame without another slot. Independent core/edge review found no remaining actionable issue in the checked scope. Full gate/build/final browser collection follows. Higher slots, remaining spells/concentration/rest, AI combat tools, authored complete sessions and party/LAN remain required.

Final initial-spell collection: integrated full gates GREEN (workspace Rust,1038 frontend tests and static/architecture checks), final production/backend builds and three real-backend production-browser regressions GREEN. Independent core and condition/healing follow-up reviews report no remaining actionable checked-scope issue. Original32 source hashes preserved; isolated backend stopped. This closes core6 only; first-level slot initialization is explicit and higher slots/Pact recharge, remaining spells/concentration/rest, AI-directed native/model sessions and party/LAN continue in the same full goal.

## Weapon and NPC execution vertical

Derive a bound hero loadout from the stored class/abilities/catalog equipment, with proficiency, finesse, melee/thrown/ranged distances and ammunition/quantities. A typed weapon intention contains actor/target/weapon/mode only; supplied modifiers/damage do not control a bound attack. Persist consumed resources with the same receipt/character transaction and keep canonical snapshots from overwriting them. NPC profiles execute their movement/attack/end-turn through the domain engine, including opening initiative and transitions back to the player; defeated sides terminate the encounter. UI retains existing action-bar/modal conventions, offers target/weapon choices and confirmed resource counts, with no new tokens or dependencies. Verify deterministic rules/rollback/replay first, then actual server/browser complete combat. Spells/rest, detailed map geometry, authored defensive consequences, native/model/party/reload recovery remain required after this vertical.

## Durable hero binding vertical

Final weapon/NPC collection: full integrated gates GREEN (workspace Rust,1036 frontend tests, architecture/static checks), production build and two real-backend production-browser scenarios GREEN on the final backend. Modal screenshot visually inspected after animation. Review-found legacy restore contamination reproduced RED (current5 arrows replacing saved20) and fixed by deriving missing profiles from the snapshot sheet; final follow-up reports no remaining actionable correction issue. Original32 source hashes preserved; isolated backend stopped. This checkpoint closes the weapon/NPC vertical only; spell/rest, authored defense/map, AI-directed native/model/recovery and party/LAN tasks remain active.

Weapon/NPC progress, 2026-10-07: intention-only weapon transport, server-derived loadout/range/ammunition, transactional stock receipts, automatic NPC turns and durable defeated-side closure implemented. Production-root browser against real isolated backend/SQLite completed a longsword encounter and retained its result after reload; separate same-name hero test verified NPC turns return control. Independent review's opening closure, negative damage/temp-HP, final duplicate replay and thrown-versatile issues reproduced RED and passed GREEN. A final review also found old bound encounters without loadouts; file close/reopen plus old-snapshot restore now upgrades missing profiles without resetting round/budget/HP. Legacy bound raw attack cannot bypass canonical profiles. Full mechanics/AI-directed/native/model/party requirements remain pending; final compatibility gate collection follows.

Bind a real campaign's combat actor to the server-stored character UUID; create a unique encounter token ID and carry its character_id separately. Use accepted durable name/HP/AC, derive initiative from Dexterity and validate speed rather than trusting a model's replacement hero. Both HTTP and agent combat creation must return the stored projection after binding. Commit bound hero HP and campaign revision atomically with each combat receipt; deliver campaign scope so the renderer mirrors the correct sheet without interrupting chat. Reject sheet replacement while combat is active. Use character identity for portrait/turn gating so an NPC with the same name is not controlled as the hero. Keep existing UI controls/tokens and verify the real production root in a browser. Correct incomplete equipment mechanics against official SRD5.2.1, PDF pages91-92; this prepares subsequent authoritative weapon/spell/resource actions, not a claim those mechanics are already complete. Authored encounter trigger/map/outcome and all remaining broad tasks stay pending.

Progress: bound mapping/atomic HP+campaign revision and receipt rollback passed. Equipment regression reproduced unsupported physical damage and corrected existing16 weapon/armor mechanics from the official source. Independent review found lost continuation fields, duplicate model player entries, snapshot sheet replacement and mismatched campaign/session start; all reproduced RED and passed GREEN. A real isolated backend/SQLite production-browser scenario passed same-name NPC gating, actual player turn command and renderer reloads. Keyboard ownership RED->GREEN also prevents sending player intents for an unowned actor. Limited follow-up review found no remaining actionable correction issue. Full integrated gate/checkpoint collection follows; spell/resource/NPC/defense/native/model/party work remains pending.

Proposed AGENTS rule for repeated tooling mistakes: resolve source/output paths with rg --files or the build manifest before read/build calls; use rg -g patterns rather than PowerShell wildcard path arguments. Every command whose Git hook invokes Cargo must explicitly prepare the Cargo PATH and intended RUST_LOG, just like a direct Rust verification command.

Final hero-binding collection: full gates passed (workspace Rust and1033 frontend tests), production build and real-backend/SQLite production-root continuation scenario passed. Source/bin-map clarified Cargo app-server versus packaged dmai-server. Browser used native-transport shims but real campaign/combat persistence and actual turn commands; no native/real-model/co-op session acceptance claimed. The owned isolated backend was stopped. Authoritative weapon/NPC/spell/resource intent work is next; broad mechanics and recovery/party boxes stay unchecked.

## Combat foundation checkpoint, 2026-10-07

The existing HTTP service constructs a new combat resolver per request, losing its in-memory idempotency cache. Rebuilding initiative also resets round1 on every action. Before binding the authored solo hero and encounter, persist action intent/result receipts atomically with the combat projection, preserve restored round/current turn, and keep token coordinates consistent with the projection. Tests must cover a reopened resolver/store replay, changed payload under the same ID, concurrent duplicate/different requests, round2 surviving subsequent actions, and movement coordinates. This is a prerequisite of the complete mechanics/solo/co-op work above, not a substitute for it. No UI or dependency changes are required for this checkpoint.

Review-driven frontend assumptions: keep the existing combat controls and visual tokens. Each new intention uses a UUID; a failed identical intention retries the immutable UUID/body; replies from an earlier campaign transition or encounter are ignored. This changes transport coordination only, with no new layout, labels, controls, or dependencies. Verify hook regressions, production build and an existing production-browser combat scenario.

Progress: four initial regressions reproduced RED and passed GREEN. SQLite receipts now commit atomically with projection/token updates, survive real pool close/reopen, and roll back on receipt-insert failure. Restoring combat advances its revision. Independent review found restart command-ID collisions and receipts surviving no/different-combat restoration; SQL and hook RED cases reproduced both, then UUID retry identities, session-wide receipt invalidation and late-response guards passed. Limited follow-up review found no remaining actionable issue in those fixes. Full mechanics/actor/party tasks remain unchecked; final integrated gate/build/browser collection follows.

Final collection for this checkpoint: full gates passed (workspace Rust and1028 frontend tests), production build and full-campaign production-root browser scenario passed. SQL8+repository10, application9, HTTP4 and hook5 focused checks passed. Original32 baseline hashes unchanged. Browser responses are scripted; no native AI-directed/live-model/co-op acceptance is claimed. Next integrated vertical: bind the durable solo hero to its token, authoritative equipment/resources and authored defensive encounter outcome.

First bounded interfaces: `AdventureActionCommand { action: Action }` in application agent commands; `AdventureIntent {campaign_id, session_id, causation_id, action}` and `AdventureRepository::act_as_dm`; `AgentContext.adventure_active`; scoped catalog and executor restrictions. A tool outcome contains only public adventure state, identity/revision and necessary inventory updates, never portrait blobs, biographies, private canon or credentials.

Review focus: duplicate model call IDs across different turns; stale pre-restore tool replay; hallucinated legacy mutation tools; action completion while narration streams; context load failure; cancellation after a committed tool.

## Progress evidence

- 2026-10-06: decoder RED UnknownTool -> GREEN strict action-only payload; SQL intent-port RED -> GREEN server identity/revision and exact scoped replay returning latest public state.
- Real scripted-provider orchestrator executed adventure_action into SQLite and rejected hallucinated set_scene. Private portrait/biography omitted from tool outcome. Private DM reasoning leak reproduced RED and suppressed for authored campaign streams; legacy reasoning coverage remains GREEN.
- Atomic frontend outcome RED missing adapter -> GREEN preserves narration controller/messages, rejects malformed/cross-campaign/stale outcomes. Production browser scripted SSE scenario passed and kept narration while moving the visible adventure to the quay.
- TypeScript, production build and fast gates passed. Application/SQLite/server suite reached a host RUST_LOG=warn telemetry failure; the telemetry test rerun with explicit RUST_LOG=info passed. This is not reported as a completed final full gate for this ongoing phase.
- No live model, native AI-directed acceptance, durable turn recovery, complete solo mechanics or co-op acceptance claimed. No new dependencies, release, push or original-source modification. Ongoing work remains in the attached worktree for continuation.

## Durable-operation progress (not full recovery completion)

- Stable renderer UUID and immutable retry request; incomplete EOF is rejected. Hook/transport RED -> GREEN and production-browser replay scenario passed without duplicated user/assistant messages.
- Migration0004, application-owned operation port/service, SQLite event receipts and HTTP wrapper. Completed response replays identically and does not insert another user message; changed payload409. Same-campaign two-request HTTP concurrency probe observed peak one.
- A changed owner ID is treated as still running. Bootstrap must acquire the OS-backed exclusive database authority lock before reconciling abandoned records. Unit handle-release check and two real live-process rejection proved this; completed replay survived real backend restart.
- Disconnect-before-first-event and completion-before-terminal-marker failures reproduced RED -> GREEN. Partial effect/narration continuation still pending; ordered-work item remains unchecked.
- Workspace Rust and1025 frontend tests passed. Full gate caught lock open-mode warning, corrected with explicit truncate(false); CI-lint/static gate passed. Latest focused HTTP/SQLite checks pending final collection. No full native/live-model/co-op recovery claim.
- Proposed AGENTS rule: every Cargo test invocation must set the intended RUST_LOG explicitly when telemetry assertions depend on INFO, instead of inheriting the desktop shell filter.

## Interrupted-checkpoint review, 2026-10-07

- Migration0005 and per-operation transcript/tool receipts support explicit continuation after a committed authored action; HTTP scenario preserved revision3 and one user message.
- Independent read-only review found five issues: resume/restore TOCTOU, missing confirmed delivery, hanging-provider cancellation, absent-operation retry, and duplicate final narration. Four RED regression cases reproduced failures; queued-restore and real HTTP hanging-provider tests cover production coordination. Fixes share the lane from claim through finish, repair recorded delivery, wake both provider waits, claim absent immutable requests and deliver the already committed final reply.
- Follow-up review confirmed those fixes and found missing ToolCallStart during delivery repair. Reproduced RED and repaired start-before-media/result so real renderer cards can receive the recovered outcome.
- Full gate found test-module ordering in checkpoint_tools; moved the module to the end without changing behavior. Final integrated gate/review collection still pending. Durable-operation task remains unchecked because reload recovery and running-request handling remain unfinished.
- Final collection: full gates passed (workspace Rust plus1025 frontend tests), production build and two targeted production-browser scenarios passed. Tests now supply an explicit terminal provider round instead of treating empty EOF as success; separate regression verifies no final Assistant is persisted for an interrupted draft. Independent follow-up review closed all findings in this phase. Original32 source-baseline hashes remained unchanged; inherited edits excluded from the scoped checkpoint. Native AI-directed/live-model/full-mechanics/co-op acceptance still pending.

Tool transport checkpoint,2026-10-08: same-frame multi-tool loss reproduced in
installed genai0.6.5. Tool-enabled OpenAI-compatible rounds now use existing SDK
complete-response handling, preserving all IDs/args/order; no-tools remains SSE.
Seven wire regressions and full provider/server suites pass. Independent review
closed incomplete-final-response and mixed-legacy-markup findings. This supersedes
pending reqwest proposal; no dependency approval needed for the implemented path.
Real Bonsai/tool/narration session and native acceptance remain next.

Real Bonsai continuation,2026-10-08: installedmodel10-step route mechanically
completed with zeroerrors, correcteffects/reward, but independent semanticreview
failed discovery/role/localization/ending. Fact-dependent context and reasoning/
preamble transport fixes added. Real interrupted finish+save resumed same request
without repeatedeffects (revision13/one save/10gold); prosefail preserved. Latest
fresh ten-step route102.132s completed; Russianlanguage/unsupporteddetails gate
stillFAIL.474Rust tests/nine wirecases/Clippy/build pass; browser1pass3skips.
Do notpromote Bonsai ormark model/native/fullsession work complete. Runtime and
QAprocesses cleaned; see BONSAI_EVALUATION.md for provenance and openlimitations.

## Provider-independent terminal-round validation

- [x] Reproduce Length/Error, empty Stop, tool/finish mismatches and incomplete
  tool buffers becoming completed turns through the application provider port.
- [x] Reject invalid terminal rounds before persisting the assistant or dispatching
  their tools. Retain prior confirmed rounds for explicit continuation.
- [x] Treat exhausted agent rounds as interrupted; resume confirmed tool results
  without executing them again. Verify application and real HTTP/SQLite outcomes.
- [x] Run focused/full backend gates and independent review, then local checkpoint.

No UI/dependency/schema changes intended. Public drafts may already be streamed,
but cannot become durable completed replies. Existing interruption UI handles the
provider error. Preserve arbitrary provider content from diagnostic logs.

Application/server regression suite passes; real HTTP/SQLite3-case interruption
and same-request continuation confirmed. Independent bounded source/recovery review
finds no actionable issue. 427 tests and Clippy pass; final fast-gate/local checkpoint collection recorded below. See
COMBAT_RECOVERY.md for limits and local artifacts.

Terminal validation final checkpoint4d096a8:427 application/server tests, Clippy
alltargets and fast gates pass; independent source/recovery review closed. Original
32baseline hashes preserved. Successful test fixtures now provide nonblank final
text; this changes no production provider fallback.

Real Bonsai temperature0.4 repeat: ten completed turns103.889s, ten authoredactions,
revision12,10gold and closed_crossing; no quicksave. Independent semantic review
stillFAIL (location/time contradiction, inventedlock, wetdryroute, mistranslation).
No promotion or full-goal completion. Isolated processes/listeners verified stopped.
See BONSAI_EVALUATION.md. Next real-model work must address reliable narrative
fidelity rather than treating lower temperature as an accepted fix.


## Installed-model comparison and approved Qwen9B candidate

- [x] Compare the other installed Bonsai27B Q1_0 through real current backend.
  Verify local weight hash, isolated Stronghold profile and actual world outcomes.
- [x] Independent review of language, facts and intention fidelity; record failures
  without promoting model or treating agent_done as successful gameplay.
- [x] Obtain explicit approval before downloading an uninstalled large model.
  User approved Qwen3.5-9B Q4_K_M5.627GB after both installed candidates failed.
- [x] Download pinned weights and verify published SHA256; launch isolated runtime.
- [ ] Run real-model rescue/ending/combat checks, durable readback and independent
  semantic review. Keep ordinary model/settings and saves unchanged until accepted.
- [ ] Update evidence/docs and scoped local checkpoint; complete native session
  and physical LAN acceptance remain separately required.

Installed community Bonsai1 source sha71d68718..., revisionf4e1b84..., already
present3.803GB file. Nonthinking, temperature0.7, context16384 on RTX3080/10GB.
Five replies complete in21.666sec but route FAIL: index1 claims archive arrival
without action; index2 performs unchosen inspect_quay; index3 moves town instead
of unavailable rescue; index4 correctly rejects town->belfry. Three successful
world actions, one tool rejection, revision5, no rescue/reward/ending. Russian
has mixed script, raw IDs and invented actions. Different settings/single runs
support this scenario verdict only, not a general model ranking. Isolated
backend/model processes and ports stopped; original weights preserved.

Initial Qwen download was cancelled on finding the older no-auto-large-download
constraint. No weight bytes arrived;196B metadata remained because automatic
approval review rejected cleanup. User subsequently explicitly approved download,
so the same pinned revision resumes. No production dependency is added.


Qwen9B first current-runtime observations (no promotion): nonthinking10 completed
replies25.564sec, eight successful actions, rescued Tikhon/10gold/sealedsluice but
no finish/ending and a false repairedbell claim. Thinking1024: seven completed
replies50.189sec, nine actions/one rejection, with unchosen acknowledgment,
return and roster recovery. Combat: two attempts107.955sec, one completed,
6redundant initiative rolls, denied shot through a wall, unchosen EndTurn then
weapon effect; second operation interrupted after round limit. Canonical actor
HP11/arrows19 and revision8 persist. User input explicitly forbade ending the
hero turn. Independent semantic reviews reject these configurations. These are
real HTTP/model checks, not native full-session or physical LAN acceptance.

## Execute requested player intentions directly, then narrate

- [x] Reproduce multi-action drift from real Qwen thinking/actual combat, plus
  application RED regressions for same batch, next round, cross-kind and recovery.
- [x] Implement a separate semantic authorization check against the immutable
  human message. Execute directly without a per-action confirmation UI. Genuine
  compound requests can resolve multiple requested actions in order; a confirmed
  or definitively rejected attempt cannot authorize an unrequested follow-up.
  Keep decoding correction possible; storage/transport uncertainty interrupts.
  Check every proposed action, including same-batch entries, and withhold final
  prose while explicitly requested mechanics remain unresolved. Rules/media/save
  remain available. Do not replace a failed attack with
  EndTurn or another action the player did not choose.
- [x] Recover phase only from this request's checkpoint, never previous history.
  Add receipt-only dispatcher/capability ports and canonical repository lookups.
  Validate immutable args, owner/lease/session/timeline under one repository path.
  Recover already committed effects without dispatching new ones. Missing receipts
  cannot confirm an effect; unknown/read failure means interrupted. Never replay
  stale projections. Legacy raw mutations recover only completed steps; a pending
  non-idempotent effect remains uncertain.
- [ ] Regress A-confirmed+B-pending both without effect and with prior committed
  world effect but missing tool checkpoint/result, including SQLite reopening.
  Cover changed arguments, restore/revoke, rejected intent and old multi-effect
  checkpoints. Preserve all prior confirmed effects truthfully.
- [ ] Expose safe definitive combat refusal reasons to the narrator where current
  adapters erase them; classify uncertain repository errors without false refusal.
- [ ] Run focused/core/room/full gates, rebuild isolated backend and repeat real
  Qwen rescue/ending/combat plus independent source/semantic review. Record fresh
  native/session evidence separately; do not promote an unaccepted model.

User explicitly selected direct execution from text with a separate model check.
The checker receives public context and recorded outcomes, never DM-only lore;
strict JSON and complete terminal metadata are required before authorization.
NPC consequences inside canonical actions remain server-authoritative. The model
checker is not proof of narrative truth or universal semantic reliability; actual
model evaluation and independent review remain required. Historical model results
above used b4950c7 with inherited baseline preserved; guarded runs must record
their own build and evidence. Current implementation remains uncommitted.

Guarded evaluation: thinking Qwen completed all ten selected adventure actions in
129.012 seconds, returned to town and confirmed closed_crossing, rescue and ten
gold at revision13; one save followed locating Tikhon. Independent semantic review
still FAIL: dry-path/deep-water contradiction, already rescued Tikhon described as
awaiting transport, invented permanent flooding, assigned hero feelings and Russian
wording errors. Guarded combat-start and compound move+inspect interrupted before
effects (24.936/15.040 seconds, unchanged revisions5/2). No model promotion.

Focused verification:22 workflow tests,11 adventure-agent tests,3 room-agent tests,
39 room-DM tests and6 actual OpenAI-compatible wire tests pass. Independent bounded
source/recovery review closed after raw-effect/source checks, replay-before-policy,
immutable signatures and denied-result delivery repair. Full gates and remaining
model/native/physical LAN acceptance are recorded separately, never inferred from
these fixtures. QA processes and ports31425/31809 are verified stopped.

Next acceptance work, grounded in the failed runs:
- [x] Request provider-supported structured JSON for the separate checker using
  the installed SDK; retain strict terminal/schema validation even when a provider
  ignores the format. Verify actual wire payload and model outputs. Preserve a
  safe static diagnostic code for verification failure without logging model text.
- [ ] Repeat combat-start, explicitly forbidden EndTurn, real compound actions and
  same-request continuation. Require correct effects AND completed honest replies.
- [ ] Resolve the independently identified narrative contradictions; repeat the
  rescue/ending route and independent semantic review before any model promotion.

Checkpoint verification: full consolidated gates pass with869 Rust tests and1117
frontend tests, architecture guard, formatting, Biome, TypeScript and Clippy. A
Windows body-limit regression now tests32MiB+1 instead of an unread1MiB tail;
five exact repeats and the full suite pass without weakening production limits.
Final independent staged-source review finds no new actionable defect. Original
32 dirty-file hashes are preserved. This closes the local implementation check,
not model semantic acceptance, native sessions, physical LAN or the overall goal.

Structured-output checkpoint: production retry wrapper now forwards
JSON Schema (RED actual-wire missing-format regression, then GREEN). Permission
denial cannot consume a human request or publish phantom progress. Completion
uses quoted remaining commands and finished-attempt ledger, with skill outcome
metrics retained for conditional requests; no new command is authorized there.
Focused26 workflow/9 wire/5 retry tests pass. A RED-to-GREEN regression now
rechecks completion at Stop even after a false pre-execution forecast: pending
compound actions restore the current catalog; premature prose is neither stored
nor published. Independent staged-source review closed this fix.

Actual compound move+inspect executes both choices, quay/revision5 (65.604s).
Faulty future-action interpretation of an NPC motive interrupted defense start;
same-request continuation completes42.064s at unchanged revision6 and unchanged
combat. Refused bow shot also remains recoverable: ledger continuation completes
27.461s at unchanged revision6 and unchanged combat. No unrequested EndTurn or
new shot occurs. Narration stillFAIL: fabricated water wall/arrow flight despite
engine refusal. No promotion; native/physical LAN and full semantic session remain
required. QA processes and ports31425/31809/31810 verified stopped.

Final structured-output checkpoint: consolidated full gates pass (876 Rust tests,
1117 frontend tests, architecture/static checks and Clippy). Independent source
review reports no actionable issue in this checkpoint; independent semantic
review confirms the real narration failures remain. Original32 baseline hashes
are unchanged. Passing gates do not establish native, physical LAN or model
semantic session acceptance.

## Next: verified narration publication

- [x] Extract confirmed public narration evidence from the existing canon and
  current attempt receipts; keep private DM knowledge outside the verifier.
- [x] Add a separate structured narration-check port before final Assistant
  persistence and TextDelta publication. Allow atmosphere/dialogue, reject
  contradicted mechanics, invented investigative causes and omitted discoveries.
- [x] Allow one prose-only correction with no tools, then recheck; on exhaustion
  interrupt without publishing bad prose or repeating committed mechanics.
- [x] Prove draft privacy, compound no-repeat and same-request recovery through
  fixtures; wire both solo and room production paths.
- [ ] Repeat actual Qwen compound/refused-shot sessions and independent semantic
  review before promotion or full native/LAN acceptance.

Read-only independent architecture review identified the existing buffered Stop
publication point, canonical context and durable receipts as the insertion
boundaries. An isolated real-Qwen diagnostic classified four supplied cases
correctly: actual bad shot and actual bad quay rejected, paired faithful rewrites
accepted. Evidence: .artifacts/qwen9b/schema/narration-probe.json. This is a small
prototype using explicitly supplied public facts, not integrated production
verification or evidence of universal narration accuracy.

Narration publication implementation: strict known-fact verdict, one tool-free
correction and recheck before final message/events. Discovered Bell facts share
the existing ward canon; closed endings, rescue/reward and dry archive route
provide public constraints. Whole tool results/private prompt/history are omitted.
Obstruction facts distinguish movement, ranged/melee attacks and spells, with
call-scoped IDs; the main refusal reminder uses the same classification.
Checker receives the effective selected model per request, including room resume.
30 workflow tests, public canon tests and 40 room-turn tests pass. Independent
review closed both found P2s (move-as-shot, stale room-checker model). Final gates
and real-model integrated narration observations are in progress.

Final narration gate local collection: full consolidated gates pass (882 Rust,
1117 frontend tests; architecture/static checks and Clippy). Hash-verified QA
executable f57395810b37 covers this source over003077f. Actual blocked longbow
shot completes128.584s with unchanged full combat projection/revision6, ammo20,
no EndTurn or new effect. Independent semantic review passes this scenario only.
Compound move+inspect both commit once at quay/revision4, but prose is withheld
and correction ends incomplete (160.662s); same-request resume retains the entire adventure, adds one quicksave
(revision4 to5), and also withholds its bad draft. Captured correction exhausts the token limit
while reasoning continues in content. Direct QA-only no-thinking correction
stops in2.355s but repeats the bad draft: it is not an accepted repair or a
production change. Next: precise violation spans, sufficient public fact evidence
and bounded correction runtime, then actual compound/rescue/ending acceptance.
No model promotion; native/physicalLAN/full-game acceptance remains open.

Structured correction checkpoint: violations carry exact draft quotes (max400
characters), omissions can reference only required facts, and repair uses a
strict narration-only JSON envelope. Fresh generation receives public evidence
and rejected claims without the entire discarded draft. Shared EN/RU ward terms
separate the required clue from optional quay-route information.31 workflow
tests and final consolidated gates pass (883 Rust,1117 frontend, architecture/
static checks and Clippy); independent source review closed the omission finding.

Actual same-request continuation on executable b26cf12a8d33: revision5 stays5,
entire adventure unchanged, original move/inspect/save receipts replayed without
new effects. A faithful correction is generated, but the checker wrongly reports
required_discovery_omitted. No final text/AgentDone is delivered after129.640s.
Independent semantic review passes the generated correction and classifies the
checker rejection as false-positive; integrated delivery remains FAIL. Evidence:
.artifacts/qwen9b/schema/repair/{build-proof.json,captures,resume-compound.json}.

Isolated no-thinking diagnostics: a narrow contradiction boolean distinguishes
faithful ward marks from wrong railings/dangling clapper (3/3). A missing-ID list
still rejects the faithful discovery, while a positive conveyed boolean accepts
it and rejects atmosphere-only/wrong-object text (3/3). These are paired small
probes, not universal accuracy or production reasoning changes. Next: bounded
positive fact-coverage adjudication for reported omissions, followed by real
compound, rescue/ending/native and physicalLAN acceptance. No model promotion.

Positive coverage checkpoint: only validated required_discovery_omitted gets a
separate strict conveyed boolean, with the effective model and one public fact
plus draft. A true result clears only that omission; all other violations stay.
Duplicate IDs reuse one check, and invalid/incomplete replies fail closed.33
workflow tests and full consolidated gates pass (885 Rust,1117 frontend plus
architecture/static checks and Clippy); independent source review closed scope.

Actual executable98449221372f over472fa15: thinking-on same-request continuation
stops before narration verification with provider_public_response_missing
(31.426s); thinking-off continuation loops on a phantom pending inspection and
ends at the round limit (25.521s). Both keep revision5 and the entire adventure
unchanged. These runs do not establish live positive-coverage acceptance.
Evidence: .artifacts/qwen9b/schema/coverage/{build-proof.json,resume-compound.json,
resume-compound-off.json,captures}. Extra action labels did not fix that ledger
misclassification. A direct attempted-boolean probe falsely accepts inspection
when only movement is recorded; that approach is explicitly rejected, not added
to production. Empty-array structural probes confirm grammar permits[] here.
No production reasoning defaults/model promotion changed.

Next architectural work: bind each verified human command to its authoritative
tool attempt/receipt, preserving operation/owner/timeline scope and recovery
after effect-before-transcript interruption. Progress should use these trusted
bindings rather than repeatedly guessing whether a recorded effect occurred.
Keep direct execution, real compounds/conditionals and final independent review;
no confirmation UI or one-action cap. Legacy unbound history must never silently
authorize or manufacture a new effect. Native/physicalLAN/full-goal remain open.

## Durable human-command manifest and effect bindings

Independent read-only architecture review confirms that quotes alone cannot
prove no second command was omitted. Persist a scoped immutable command manifest
before the first effect, plus command-to-prepared-step bindings, not client
history or final prose. Existing command_json signatures remain unchanged.

- [ ] Define ordered command IDs/source spans, typed destinations/targets and
  dependencies; keep negated constraints separate. Validate against the saved
  human request, including repeated identical phrases and conditional fallback.
- [ ] Persist manifest privately with solo operation/generation and room member/
  lease/timeline scope; make changed-request/changed-manifest reuse conflict.
- [ ] Atomically bind a verified candidate before effect preparation, using
  CheckpointTools/claim_tool_step and room_agent::prepare. Validate the typed
  signature, command ID, scope and canonical nonce; do not rewrite old signatures.
- [ ] Derive finished attempts from confirmed canonical receipts or definitive
  rejections. Permission/decoder denials and uncertain repository failures do
  not complete commands. Prerequisite travel cannot finish a different destination.
- [ ] Extend existing agent_replay delivery repair for effect-before-transcript
  interruption; preserve raw non-idempotent uncertainty. Reject restore/revocation
  and cross-owner/cross-timeline metadata before using it.
- [ ] Drive pending progress from applicable unfinished manifest commands while
  preserving permission, ownership, legality and action/resource budgets per step.
  Conditions resolve from confirmed check outcomes; unknown conditions stay open.
- [ ] Keep conservative legacy recovery: replay existing outcomes honestly,
  never derive new authorization from unbound historical transcript.
- [ ] Regress move+inspect through reopen with a deliberately wrong completion
  verifier; crash before/after effect/before ToolResult, changed signature,
  double attack, intermediate movement, conditional success/failure, restore
  and cross-member cases. Then actual model/native/physicalLAN and final review.

This records authoritative progress; it does not magically verify initial NLP
extraction. Manifest generation and concrete permission still require independent
semantic acceptance. No confirmation UI, one-action cap or production dependency.

Human-command pure foundation implemented (not runtime integration): ordered
UTF8 source spans/occurrence helper, typed canonical human intentions, separate
constraints, unresolved-target placeholders, scope/generation/room epoch/reset
cursor, monotonic step identities, canonical nonce uniqueness, exact replay and
confirmed-outcome progress. Performed intermediate travel cannot close another
destination; a definitive refusal closes an attempt; prepared/uncertain states
stay open. Check guards use confirmed booleans, missing results stay unknown;
specific refusal guards match actual codes.11 focused ledger groups plus33
workflows pass. Independent review found/fixed nonce reuse across commands and
closed pure-foundation scope. Private inward repository port now defines protected
load, semantically-verified manifest save, and atomic preparation after permission.
Persistence, trusted receipt/authority adapters, semantic planning, unresolved
target resolution, legacy compatibility and runtime replacement remain pending.
Initial gates found clone-on-Copy RestIntent; corrected. Final post-port gates
are running. Protocol notes: docs/HUMAN_COMMAND_LEDGER.md.

Final pure-foundation collection: post-port consolidated gates pass (896 Rust,
1117 frontend tests, architecture/static checks and Clippy). Independent source
review closed nonce reuse and repository-contract scope. Original32 dirty-file
hashes remain unchanged. This checkpoint defines/tests the protocol only: no
production completion path, SQLite migration, model plan, live/native or physical
LAN acceptance is claimed. Next implementation is protected SQLite metadata
and atomic prepared-step binding with reopen/crash/scope regressions, followed
by runtime wiring and full session acceptance.

Solo SQLite journal implemented as private per-operation/current-owner adapter.
Migration0019 stores manifest/bindings separately from unchanged tool signatures.
Manifest save verifies saved request/actor/timeline and rejects retroactive legacy
authorization; preparation and binding are atomic. Canonical receipt lookup/
delivery repair closes effect-before-transcript gaps and real pool reopen with
new process owner preserves progress. Frozen campaign/combat revision, encounter
and actor token reject stale pending effects; actual canonical write transactions
verify the full signature/context too. Dice/refusal settlement rejects stale or
uncertain results without poisoning pending steps.12 SQL scenarios plus focused
operation/combat/repository/pure-ledger suites pass. Independent review found
fixed pending-context and incomplete-signature P2s, then closed checked scope.
Full gates running. Room storage, model planning and runtime completion wiring
remain pending; no live/native/physicalLAN/full-goal acceptance claimed.

Final solo-storage collection: consolidated gates pass (908 Rust,1117 frontend,
architecture/static checks and Clippy).12 actual SQL scenarios cover rollback,
reopen/new owner, effect-before-transcript repair, signature/timeline guards,
legacy rejection, stale encounter/turn, full commit signature, durable refusal
and dice/uncertain settlement. Independent review closed both discovered P2s.
Original32 source hashes preserved. This is private solo persistence and guarded
canonical execution support, not runtime/model-manifest wiring or live session
acceptance. Room metadata and full original solo/co-op/native/physicalLAN scope
remain active.

Room private journal checkpoint: migration0020 stores member/turn manifest and
bindings separately from unchanged tool signatures. Preparation/binding share
one transaction; owner, original request, lease, actor, epoch and reset cursor
come from protected storage. Canonical receipts recover effect-before-transcript;
pending world/combat context and durable refusal are enforced in actual effect
transactions. Legacy prepare cannot bypass a manifest-bearing turn. Seven new
SQL/application regressions pass: repeated goals/rollback/replay, source and owner
isolation, durable refusal, stale revision, legacy bypass, exclusive owner recovery
and timeline reset. Existing room suite also passes. Independent review found the
legacy bypass, confirmed its fix and closed the checked source scope. Consolidated
gates are running. Manifest generation/verification, future-target resolution,
runtime completion replacement and live/native/physical LAN acceptance remain
open. Direct execution of compound free-text commands remains the agreed behavior.

Final room-storage collection: all consolidated gates pass (915 Rust,1117
frontend, architecture/static checks and Clippy). Independent review closed the
legacy preparation bypass. This checkpoint provides private room persistence
and guarded effects; model planning/runtime integration and complete real-model,
native and separate-device LAN/VPN sessions are still required.

Runtime integration in progress: protected original_request port, independently
verified structured manifest planner, shared turn service journal initialization,
atomic binding and canonical progress/completion are implemented. Production solo
and room factories attach owner-scoped adapters for verified-model requests;
room planner uses effective saved model. Scripted real-SQLite runtime proves both
identical commands finish despite false model completion forecast, committed
effect-before-ToolResult recovery without replan, and exact pending permission
surviving later interruption and another resume. Review found permission denial
poisoning, strict schema open-object incompatibility, and impossible clarification
delivery. First two fixed with regression checks; durable refusal replay added.
Clarification lifecycle is explicitly OPEN: NeedsResolution must deliver a safe
question and retain unfinished intent without falsely completing the session.
Trusted future-target resolution/quantified commands, full post-change gates,
real Qwen complete sessions and native/separate-device LAN acceptance remain open.

Current focused verification: four structured planner groups and four real SQLite
room runtime scenarios pass, including pending permission and durable refusal
replay. Existing 33 workflow and11 pure-ledger groups plus solo journal/operation
suites pass. Architecture guard and focused Clippy(all targets, warnings denied)
pass. Independent follow-up closed permission-poisoning and strict-schema findings;
clarification-delivery finding stays OPEN. Changes remain in progress/uncommitted
until the explicit unfinished-command clarification lifecycle is implemented.

Clarification lifecycle continuation: migration0021 stores immutable private
question/reply and verified final-narration records. Current-reference resolution
and independently checked questions/goals are implemented. Needs-input delivers
without AgentDone and only after interrupted status commits; HTTP continuation
claims the same protected operation before accepting a reply. Solo in-memory
question/reply flow and native encrypted-outbox/private-question transitions are
wired; the room composer permits the owning player to answer. Questions and finals
now bind to chat messages atomically, closing lost-question-ACK duplication and
final-append-before-Done regeneration.16 room metadata/runtime checks and21 focused
frontend tests pass. Production build and browser fixture question/reply flow pass,
with visual inspection; focused all-target Clippy passes. Full parallel room run
had53/54 pass with one departure transport timeout; that exact test passed alone.
Do not claim full gates yet. New native parser/transition unit run is in progress.
Independent review's remaining P2: solo reload/remount loses the pending handle.
Next: protected pending recovery/read and reload regression, native protocol
acceptance, semantic resolution tests, full frozen gates/review, scoped checkpoint,
then quantified intentions and real Qwen/native/separate-device session acceptance.

Post-review recovery fixes: immutable question UUID now also binds its transcript
message; verified final receipt binds its final message. Both commit atomically
with private metadata. Lost-question-ACK and final-append-before-Done regressions
pass among16 journal/runtime cases; independent follow-up closed both P2s. Four
native parser/transition tests pass, focused all-target Clippy and architecture
guard pass. Solo reload/remount pending recovery remains the one open reviewed
clarification finding. No frozen full-gate/checkpoint or live/native session claim
is made until that recovery path and the remaining acceptance work are complete.

Solo pending recovery continuation: protected GET /agent/pending reads only the
latest incomplete verified operation for current actor/session/timeline, excludes
room state and older operations, and preserves exact original wire history and
request parameters. It restores unanswered-question handles and exact accepted
reply retries, including an interrupted operation with saved final. Uncertain
stream outcomes trigger a fresh probe, closing lost-NeedsInput and accepted-reply
remount windows. Independent follow-up closed both P2s; endpoint status semantics
are running/interrupted, not client-ACK tracking. All16 solo SQL cases,17 room
journal/runtime cases, HTTP new-server/private-room exclusion and11 API/hook checks
pass. Production build and2 fixture browser flows (including reload) pass. Unchecked
ToolUse prose is now omitted from protected transcript; regression passes. Frozen
full gates are running before checkpoint. Real Qwen complete sessions, quantified
intentions, native and physical separate-device LAN/VPN acceptance remain active.

Consolidated gate follow-up: biome found one owned API-line formatting issue,
corrected. Parallel room tests reproduced a departure timeout; deterministic
delayed-join regression proved the global guest-selection mutex blocked unrelated
AppState/vault contexts. Selection serialization now belongs to shared AppStateInner:
clones preserve ordering, independent compositions do not share the mutex. Reviewer
checked all seven lock sites and archive order; no actionable finding. Corrected
fixture uses independent rooms/invitations. Focused regression and all58 parallel
room tests pass. Frozen final consolidated gates are running. User-owned original
changes remain separate; only the three owned API semantic edits will be staged
over HEAD for src/api/agent.ts.

Final runtime/clarification collection: consolidated gates pass (938 Rust,1127
frontend, architecture/static checks and all-feature Clippy). All58 parallel room
cases pass after scoping the guest selection lane. Production build and2 browser
fixture scenarios pass, including remount/reload with the exact original operation.
Independent reviews closed discovered semantic-permission, schema, clarification,
transcript/final-replay, solo recovery and native lock findings. Original32 file
hashes preserved. Scoped staging includes only three owned src/api/agent.ts changes;
inherited transport edits remain unstaged. This checkpoint is runtime integration
and recovery proof, not complete real-Qwen/native/physical-LAN session acceptance.

Live human-command continuation,2026-10-09: strict coupled planner schema and
full adventure argument examples fix two real malformed Qwen proposals without
relaxing the independent manifest/per-action checks. Actual move+inspect with a
negative movement constraint committed exactly once. Separate publication
classification, presentation and required-fact coverage avoid a reproduced false
invented-mechanism diagnosis; inconsistent negative diagnostics cannot publish.
36 workflow and5 planning groups pass; independent source review has no open
finding. Frozen gates pass (942 Rust,1127 frontend, architecture/static, Clippy).

Real backend-restart resume: thinking-off narration still fails; thinking-on
finishes in71.936s, replays original effects and only creates one new quick save
(revision4→5). Independent semantic review accepts agency/minimal discovery but
flags gender and unclear clue phrasing. Next archive request is rejected by
manifest verification before effects; schema/route-context probes remain failed.
No default-model/reasoning promotion and no full Qwen session acceptance.
Next bounded implementation: quantified repetitions from one source clause,
with ordered per-attempt bindings/receipts and no duplicate retry or quota
bypass; then richer conditional/refusal cases and full model/native/LAN proofs.
Keep the broad goal and acceptance boxes open.


Quantified-command continuation:
- [x] Expand one explicitly quantified source clause into complete ordered
  per-attempt commands, preserving full source and private ordinal/count metadata.
- [x] Reject incomplete/mixed/overlapping groups; read legacy manifests without
  the optional metadata; retain independent count and per-action verification.
- [x] Preserve original action budgets and refusal/replay semantics. Real SQLite
  Wait twice survives file-pool reopen/new owner with exactly120 seconds; fighter
  attack twice resolves once then refuses without projection/turn changes.
- [x] Room attempts use distinct canonical nonces and repeated delivery creates
  no additional effect. Independent source/test review has no open finding.
- [x] Frozen full gates and scoped local checkpoint (948 Rust,1127 frontend;
  architecture/static checks and all-feature Clippy pass).
- [ ] Real-model quantity/conditional interpretation and complete session proof.
No model, native or physical-LAN acceptance is inferred from these protocol tests.

Quantified scope verification:18 solo SQLite,14 protocol,6 planner and the new
room regression pass; consolidated gates pass with948 Rust and1127 frontend
checks. Final staged independent review has no actionable finding. All32 original
source hashes are preserved; only owned files are staged. No frontend behavior
changed, no dependencies added, no model/default promotion or publication.
