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
