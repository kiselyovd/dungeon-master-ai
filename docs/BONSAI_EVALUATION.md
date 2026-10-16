# Bonsai runtime and playability evaluation

Status: runtime compatibility proved; full game/narration acceptance remains open.
The model has not replaced the user's configured provider.

The installed `Ternary-Bonsai-2-27B-PTQ1_0.gguf` fails to load in LM Studio's
selected llama.cpp CUDA runtime 2.52.0. The authors explain that this packing
needs their activation-transform runtime. See the
[model card](https://huggingface.co/prism-ml/Ternary-Bonsai-2-27B-gguf) and
[tested setup](https://github.com/PrismML-Eng/Bonsai-demo).

An isolated evaluation uses the
[Prism release prism-b10754-2459f68](https://github.com/PrismML-Eng/llama.cpp/releases/tag/prism-b10754-2459f68).
The Windows CUDA 12.4 runtime and CUDA-library archives match the SHA256 digests
published in its release metadata:

| Archive | SHA256 |
| --- | --- |
| runtime | `200842b4c689fe092a8a1684ef1e185d8d025a0bfae6908737607a0f29560b96` |
| CUDA libraries | `8c79a9b226de4b3cacfd1f83d24f962d0773be79f1e7b75c6af4ded7e32ae1d6` |

On the RTX 3080 10 GiB, the model loaded with an 8192-token context, one slot,
full GPU offload, medium reasoning effort and a 1024-token reasoning budget.
Observed GPU allocation was 7451 MiB, including other desktop use. A native
OpenAI tool call selected the quay movement, and a second request narrated its
confirmed result in Russian; requests took 2223 and 1277 ms respectively.

## Findings in the application

Real gameplay exposed defects that scripted model tests did not exercise:

- OpenAI-compatible streaming did not enable genai's cumulative tool capture.
  Continuation fragments acquired `call_0`, separating them from the original
  tool ID. The wire regression failed before correction and passed afterward.
- `ChatRequest.system_prompt` was omitted from the actual HTTP body, losing
  authored canon and canonical mechanics. A wire-body assertion reproduced it;
  forwarding the prompt corrected it.
- Shared tool completion used HashMap order. Completion now follows the original
  observed call order, with order-sensitive coverage.
- Genai 0.6.5 loses additional calls in a single normal SSE tool-call frame.
  A native wire-codec prototype handles multiple entries, numeric index order,
  fragmented UTF-8/CRLF and incomplete streams. Production integration awaits
  explicit approval to use the existing workspace `reqwest` dependency in
  `adapter-llm`; the prototype is tested in the server's existing test environment.

With system context restored, the first five real gameplay steps advanced by
one chosen action each. A later malformed intent was rejected before the model
corrected it. Narration still sometimes printed API identifiers or interpreted
completed actions as unavailable future actions. The authored context now
clarifies that confirmed outcomes must be narrated as completed and identifiers
are not player-facing prose.

The next real-model rerun completed ten player requests in 86.961 seconds with
zero tool errors: locate and rescue Tikhon, seal the sluice, return to town and
finish with `closed_crossing`. Inventory contains exactly one ten-gold rescue
reward. The final turn also made a confirmed quick save. A discriminated tool
schema now requires location for movement and forbids it for unit actions;
the former schema permitted the malformed seal request that failed decoding.
Wire/schema regressions and the actual runtime confirmed this correction.

This proves the mechanical peaceful route through real inference/HTTP/SQLite.
Narrative acceptance remains open: the model called the memorial roster a
genealogy and invented extra coins in final prose, although the stored inventory
remained correct. Do not treat that route as full native or narrative acceptance.

Prepared world state, identity, HP and rewards remain engine-owned. A successful
load or tool probe is not proof of a complete playable session. Next acceptance
must cover the complete peaceful and defensive routes, narrative fidelity,
native application/restart, and then LAN/VPN co-op. Evaluation processes were
stopped; weights and original project files were preserved.

Tool transport reliability checkpoint, 2026-10-08:
The installed genai0.6.5 streaming adapter was reproduced dropping the second
native tool when both calls arrive in one OpenAI SSE delta. A wire fixture now
serves both protocol forms and asserts both IDs/argument payloads/order.
Tool-enabled OpenAI-compatible requests use the existing SDK exec_chat complete
response API, then publish the usual provider chunk contract. Plain no-tools chat
retains incremental streaming. This supersedes the proposed reqwest workaround:
no new production dependency, SDK upgrade or download is required.

All tools in a validated complete batch retain their IDs, JSON args and order.
Invalid/duplicate IDs and missing/unknown terminal metadata fail before delivery.
Truncated/filtered responses also fail before any chunk, because the current agent
must not persist partial unconfirmed prose as a final answer. Native+legacy mixed
responses keep native IDs and suppress leaked markup without duplicate execution;
legacy-only local-model markers use the shared existing parser. Reasoning/text
keep their existing event types and downstream privacy controls.

Evidence: wire RED -> GREEN, seven focused cases (multiple calls, termination,
identity, text/reasoning, legacy, mixed and cancellable waiting), full adapter and
server suites pass. Independent review found terminal-state and mixed-markup
issues; both repairs were confirmed. Additional waiting for a complete tool-enabled
response is the tradeoff; it may include final narration because the catalog is
still offered for that round. No real Bonsai semantic/native acceptance claimed
by this transport checkpoint. See .artifacts/tool-round logs and provider tests.

Real-model continuation, 2026-10-08 (no default-provider promotion):
Three fresh QA-route/ending observations used the installed PTQ1_0 weights and
pinned Prism runtime, context8192, one slot, medium reasoning/budget1024 and
application temperature1. The current complete-response transport completed the
first ten-request route in80.656seconds with zero tool errors, one chosen adventure
action per step, rescued Tikhon, closed_crossing and exactly10gold. Independent
review rejected narration: premature discoveries, rescued Tikhon put back upstairs,
raw identifiers/poor translations and a stray closing think tag.

Repairs: the DM context now carries fact-dependent located/rescued/reward/discovery
and ending briefs, scene-scoped knowledge, EN/RU names, and human-agency guidance.
UI labels no longer reveal the old echo's name before discovery. Embedded/stray
thinking markers are separated before legacy-tool parsing, so hidden thought text
cannot synthesize a tool call. Empty final public replies fail; tool-bearing prose
is suppressed until the subsequent confirmed final response. Safe provider error
categories are logged without response bodies, credentials or thought text.

The next brief route improved rescue/discovery/markup, but its final request was
interrupted after finish and quick_save committed. No agent_done was delivered;
errors=[] in the raw report was NOT success. The runtime generated2048tokens in
that last request, consistent with an output-cap interruption, but its exact
finish metadata was not captured then. A strict same-request resume on a rebuilt
backend completed in6.196seconds with revision13, ten actions, one save and one
10gold reward unchanged. That resume's prose incorrectly reopened the ferry;
its semantic result remains FAIL despite mechanically safe recovery.

An explicit closed-crossing brief repaired the main ending distinction. A final
fresh ten-request route completed all replies in102.132seconds with zero tool
errors, one save, exactly10gold, Tikhon home, closed trading route and damaged ward.
Independent review still rejects stable Russian narration: mixed Chinese/Latin
characters, distorted names, an unsupported no-detour assertion and wet-path
inconsistency. Its duplicate epilogue/technical save prose prompted the final
preamble suppression, verified on wire tests but not re-run through real inference
yet. Do not erase earlier failures or call this full native/model acceptance.

Evidence: .artifacts/bonsai/current/{solo-report,solo-final}.json;
.artifacts/bonsai/brief/{solo-report,resume-proof,epilogue-proof}.json;
.artifacts/bonsai/finalbrief/{solo-report,solo-final}.json and model/backend logs.
Final validation:474application/provider/server tests, nine focused wire cases,
Clippy, production build pass. Production Playwright:1passed/3existing skipped;
this is not a completed full-session browser gate. Independent source/semantic
review and explicit runtime/proxy/process cleanup confirmed. Installed weights
and original project32-file baseline preserved. Russian model quality, complete
peaceful/defensive native sessions and physical LAN/co-op acceptance remain open.

## Lower-temperature repeat after4d096a8

A fresh isolated ten-step real-model route used the same installed PTQ1_0 weights,
pinned runtime/context/reasoning settings, current complete-response/preamble
guards and application terminal validation, changing temperature1 to0.4. All ten
turns completed in103.889seconds with nonblank public narration, no visible thinking
markup and zero tool errors. Independent SQLite read confirms ten complete
operations, ten authored actions, revision12, rescued Tikhon, closed_crossing
and exactly10gold. No quick save was requested: saves/snapshots remain zero.

Independent semantic review still returns FAIL:
- Step3 narrates a one-hour arrival in Veresk, then says the hero remains in the
  archive. The authoritative result keeps archive and advances the rescue5minutes.
- Step4 adds a rusty lock blocking the sluice without a confirmed world fact.
- Step6 puts the hero in ankle-deep water despite the selected dry upper route.
- Step9 mistranslates quarterstaff and turns route closure into a permanent claim.

The old echo's identity remains undiscovered, reward is not repeated and the ward
is not declared repaired. No additional mechanical action is chosen for the hero.
A suspicious parchment detail in step1 is not treated as a proven discovery leak
because it has not been presented as an actionable clue or loot.

Temperature reduction alone does not make this model an accepted Dungeon Master.
The default provider remains unchanged. Evidence under .artifacts/bonsai/lowtemp:
solo-report.json, solo-final.json, proof.json, backend/runtime logs and cleanup.json.
Both isolated backend/model processes and ports31424/31808 were stopped and
verified absent. Full native/peaceful/defensive/co-op semantic acceptance remains
open. Separately, the current production browser suite now passes62 tests with
one existing vision skip; see BROWSER_SESSION_ACCEPTANCE.md for its scope.

## Other installed Bonsai Q1_0 comparison

The installed lmstudio-community/Bonsai-27B-GGUF/Bonsai-27B-Q1_0.gguf also fails
this real gameplay scenario. Local SHA256 matches published revision
f4e1b84fac8d99a340cc87d06cd572dbe8a974d5 (source metadata in
.artifacts/bonsai1/source.json). Current backend b4950c7 used an isolated encrypted
profile, same Prism runtime, context16384, nonthinking and temperature0.7.
Five replies completed in21.666sec, but only three adventure actions succeeded;
one later action was correctly rejected. Index1 narrates archive arrival without
a move; index2 executes unchosen inspect_quay; index3 returns town rather than
rescuing Tikhon. The route stops at index4's unavailable belfry transition. Actual
SQLite state is revision5/town, no located/rescued Tikhon, reward or ending.
Independent review returns FAIL for intent fidelity and damaged mixed-language
Russian. agent_done and complete operation receipts do not prove player intent
was fulfilled. Different settings and single runs do not establish a general
ranking against Prism Bonsai2. No model promotion; processes/listeners stopped.

User explicitly approved a separate Qwen3.5-9B Q4_K_M comparison afterward.
Pinned weight metadata and approval/download evidence are under .artifacts/qwen9b.
The earlier unapproved attempt was stopped before weight bytes arrived; metadata
cleanup was rejected by automatic approval review. No general model-download
permission or production-provider change is inferred from this narrow approval.
