# Provider configuration after startup

The native solo acceptance found that a fresh backend began as mock/empty while
hydrated frontend settings still displayed the selected model. Manually saving
unchanged settings restored communication. Source inspection confirmed that only
onboarding and Settings Save previously posted the configuration.

Production bootstrap now restores configured values after storage hydration and
backend readiness. An unconfigured guest has no local configuration to publish.
The first solo DM request awaits provider preparation; cancellation releases the
chat immediately. Preparation failure preserves an unissued intention, so retry
starts it without claiming to resume a nonexistent server turn.

Settings writes use immutable snapshots and a shared FIFO. Manual Save always
writes; routine preparation reuses an acknowledged URL/body. A new backend
invalidates that acknowledgement. An old process's late response is rejected
rather than declaring the new process ready. Fetch and response consumption have
a ten-second limit, releasing the queue after failure. Credentials stay in the
existing Stronghold-backed flow and private transport; the acknowledgement is
memory-only. Native runtime remains the process owner; an embedded local provider
requires an already ready runtime before configuration. Cold embedded-model loading
and real-model quality have separate acceptance requirements.

Verification: provider lifecycle/ordering/cancel/timeout tests, actual production
browser configuration-before-first-turn scenario, all167Vitest files/1102tests,
TypeScript/build and architecture pass. Independent review reproduced and verified
the late-ACK and cancellation repairs.

Two real Tauri cold launches use the same isolated QA profile with fresh backend
processes/ports58889 and58074. Both automatically select openai-compat/gpt-4o before
any Settings interaction. Native UI questions produce new durable message pairs
3->5 and5->7 through a live deterministic local HTTP fixture behind the actual
provider adapter. Independent SQLite inspection finds each new question once,
its reply and no reasoning sentinel; hero HP11 remains unchanged. The first harness
assertion expected a user content string but the real record has text parts; readback
of the committed unique question corrected the proof without another send.

Local artifacts: .artifacts/solo-native/startup-fixed-proof.json,
startup-fixed-repeat.json, cold-dm-proof.json, cold-dm-repeat.json and
startup-cleanup-proof.json. They verify configuration/transport and cleanup, not
Bonsai quality or physical LAN. Armor, finished VTT presentation and the broader
solo/co-op session requirements remain open. No new production dependency or release.

Four focused production-browser startup/recovery scenarios pass. The full-campaign
fixture now uses canonical character ownership, campaign scope, casting/loadout
and current snapshot contracts. The complete browser suite passes62 cases with
one existing vision skip; see BROWSER_SESSION_ACCEPTANCE.md for real-backend
versus scripted-transport evidence. Real-model/full native session and physical
LAN acceptance remain open.
