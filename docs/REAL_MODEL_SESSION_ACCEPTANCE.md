# Real-model native session acceptance

Status on2026-10-14: **partial; complete AI-led sessions remain unaccepted**.
Task44 stays open. This record extends the native cold-recovery proof in
[Solo recovery acceptance](SOLO_RECOVERY_ACCEPTANCE.md) and keeps it separate
from deterministic [browser session acceptance](BROWSER_SESSION_ACCEPTANCE.md).

## Environment and method

The actual Tauri main WebView ran an isolated QA profile with real SQLite and
the installed Qwen3.5-9B Q4_K_M through a loopback llama-server. Reasoning was
disabled for this QA process. Source checkpoint522646e includes the earlier
5f1edc9 implementation; the protected inherited dirty baseline is unchanged.
Production defaults, validators and authority checks were unchanged.

Fixtures created only a fresh bundled Bell campaign and fighter through the
supported creation APIs. Gameplay used the rendered Russian composer and real
mouse clicks. Canonical state and recovery endpoints were read only. The
transparent model proxy forwarded actual requests/responses unchanged. Its
diagnostic log retains schema keys, canonical action kinds and verdict booleans;
it excludes prompts, narration, keys and raw model responses.

Each command waits for its actual `/agent/turn` network completion. The driver
permits at most two same-request UI continuations, then stops at a failure.
Later diagnostic continuations are recorded separately and do not turn the
initial attempt into a pass.

## Peaceful route

Eleven commands completed with delivered narration: Agata, quay, ward inspection,
town, shrine, Leon, town, warehouse, clapper negotiation, repair and town.
Repair required two continuations; its canonical action committed once.

The twelfth command, returning to the quay, remained `actions_pending` after
the bounded run. Its durable manifest correctly specified `move:quay`, with no
effect receipt. A separate same-UUID diagnostic continuation subsequently
committed the one requested move, then withheld narration after faithfulness
verification and repair. Final state: revision14, quay, HP12, twelve adventure
history entries, one confirmed effect on this pending request. The adventure is
open. No rescue, final ward restoration or peaceful ending is claimed.

## Defensive route

All five introductory commands completed through the model/UI: quay, ward
inspection, archive, locating Tikhon and starting defense. The first move needed
one continuation; the other four needed none.

The player then held position using the actual End Turn button three times.
The server settled opponents automatically. Observed combat revisions0→1→2
reached rounds2 and3; the third click resolved defense and removed the active
combat projection. The canonical outcome is `defense_held`; the hero survived
with4/12 HP. These button actions demonstrate the native combat path, not model
selection of combat commands.

The next natural command, rescuing Tikhon, failed before effects on the initial
attempt, two continuations and two separate diagnostic continuations. The
diagnostic exchange returned HTTP200 with a complete structured proposal for
`adventure_action / rescue_tikhon`, followed by an independent `valid=false`.
No manifest or effect was persisted for that request. Final campaign revision11,
archive, HP4 and `defense_held` remained unchanged across the diagnostic resumes.
The adventure is open; the complete defensive session is not accepted.

## Evidence and remaining acceptance

Local metadata, screenshots, bounded failures and diagnostic observations are
under `.artifacts/product-uiux/task-44-sessions/`. Important files include
`peaceful/verdict.json`, `peaceful/diagnostic-resume.json`,
`defensive/verdict.json` (introductory setup only),
`defensive/verdict-finish.json`, `defensive/combat-uncovered.log`,
`defensive/diagnostic-resume{,-first}.json`, `model-structure.jsonl` and
`final-snapshot.json`.

The first combat harness incorrectly matched button text including its keyboard
hint. The next attempt clicked an obscured button under the open hero panel.
After closing the panel, all three clicks committed. The harness then assumed
the combat projection would remain present after victory; it was corrected to
accept the canonical cleared projection plus held outcome. The original logs
remain. The final held observation has no new combat rows because it reads the
already completed battle; the two earlier revision observations and terminal
canonical snapshot supply the actual evidence.

Existing successful full Rust/frontend/Python/static/package layers are recorded
in the active UI/UX plan; they do not establish live model quality. No new full
suite was necessary for this documentation and artifact-only checkpoint.
Compatible managed inference/generation still needs matching model weights and
real runtime execution. A concrete Gemma download proposal is pending separate
authorization; package installation is already authorized. Manual DPI repetition
was explicitly waived and physical two-device LAN/VPN testing deferred by the
user. No publication is authorized.

The QA native app, model and transparent proxy were stopped after the snapshot;
their owned ports9243/31809/31810 are closed. All43 protected inherited paths
still match the checkpoint13 byte hashes. `cleanup-proof.json` and
`checkpoint-14-receipt.json` record these checks.

## Second installed model,2026-10-15

An inventory found the already installed Ternary Bonsai2 27B PTQ1_0 model.
The existing compatible Prism runtime loaded it with context16384, one slot,
reasoningoff and authenticated loopback access. Model SHA256 is
`53107f530aa52eb00912263ab1ee29bd199261c87cd7b4ad4ca1318c1fe33ee3`;
the file is5,946,648,928bytes. No weights were downloaded. The model was selected
through normal settings in the same isolated QA profile, on a fresh campaign.

The bounded peaceful run delivered eight commands. Its ninth action recovered
the clapper once, but narration remained pending after two continuations. A
separate diagnostic continuation localized an `unconfirmed_effect` against
`public_state`; the bounded prose repair then passed `faithful=true` and delivered
the answer without repeating the action. The initial bounded failure remains a
failure in `peaceful/verdict.json`.

The same campaign continued with repair, town, quay, archive and locating Tikhon.
All five completed; the return to town required one continuation. Thus fourteen
commands eventually completed in this campaign, with the extra clapper narration
continuation explicitly included. This is not an uninterrupted-session pass.

The fifteenth command, `rescue_tikhon`, reproduced the pre-effect rejection seen
with Qwen. The initial request and both same-UUID continuations generated the
correct structured action, then independently returned `valid=false`. The final
canonical state is revision17, Archive, HP12, fourteen adventure history entries,
no rescue fact and zero effects on the pending rescue request. The adventure is
open. A specific programming cause for the negative semantic decision has not
been established; no validator was weakened to force completion. A separate
full Bonsai defensive session is not claimed.

Evidence is in `.artifacts/product-uiux/task-44-bonsai/`: provenance, the initial
and continued verdicts/progress, same-request diagnostic metadata, screenshots,
safe model-structure records and final snapshot. QA settings were restored to
the preceding Qwen alias through the normal UI. The native app, model and proxy
were stopped; the runtime/profile process count is zero and ports9243/31809/31810
are closed. No production profile or new package/source dependency was changed.
New matching managed weights still await the existing download question, and
physical two-device LAN remains explicitly deferred. Task44 remains open.
