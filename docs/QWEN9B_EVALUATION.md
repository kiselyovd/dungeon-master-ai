# Qwen3.5-9B gameplay evaluation

Status: installed with explicit user approval; tested candidate, **not accepted**
as the default Dungeon Master. Ordinary application settings/saves are unchanged.

Source: [LM Studio Community GGUF repository](https://huggingface.co/lmstudio-community/Qwen3.5-9B-GGUF).
Pinned revision 1379f25c6b505a3fc737bd7818cb09389cf807c1,
Qwen3.5-9B-Q4_K_M.gguf, 5,627,044,256 bytes, SHA256
cd76ec205963b3b33350093e6904d9de16c4e666fd104e1f632d25c7f15f2a13.
The downloaded local file matches size and full SHA256. The CLI resumed a broken
connection and completed; an alternate download helper refused to overwrite the
already completed file. No new production dependency was installed.

Runtime: existing pinned Prism llama-server (SHA256
1e1140d5d71da6eadc25c7e58720eae487ad02a7e01ac12a30526a0548302d4f),
RTX 3080 / 10 GB, all GPU layers, one slot, context 16,384, top-k 20 / top-p 0.8 / min-p 0,
presence 0, repeat 1.0; application temperature 0.7. Observed GPU use on load was
7,828 MiB including other desktop processes, not a measured peak/performance bound.
Backend was freshly built b4950c7 with preserved inherited worktree changes;
this is not clean-HEAD release evidence. Independent encrypted QA profile,
loopback ports 31425 / 31809, no cloud provider or physical-device client.

| Configuration | Replies | Actual result | Verdict |
| --- | --- | --- | --- |
| Nonthinking | 10 complete, 25.564 s | 8 actions; rescue / 10 gold / sealed sluice, no ending | FAIL |
| Thinking, 1,024-token budget | 7 complete, 50.189 s | 9 actions, 1 rejection; unchosen extra choices | FAIL |
| Thinking combat | 2 attempts, 1 complete, 107.955 s | unchosen EndTurn + attack; final turn interrupted | FAIL |

Nonthinking narrator omitted town return and finish, and said the bell was
repaired despite only sealing the sluice. The language is more readable than
the installed Bonsai Q1_0 observation, but these runs do not establish a general
model ranking. Thinking added acknowledge_echo after entering the belfry,
returned to archive after sealing without instruction, and recovered a roster
instead of stopping after the unavailable repeated move. Initial human choices
cannot be inferred from successful receipts or AgentDone.

Combat used a real level 1 fighter, owned longbow/arrows/longsword/chainmail and
explicit trained defense (AC 16, HP 11). Initial setup used standard authored
player-action HTTP commands; subsequent DM turns used the real Qwen provider.
After start_defense, the model rolled initiative six times even though canonical
initiative already existed. A bow shot was blocked by wall (6, 2), not range.
It repeated rejected shots, then ended the turn despite the player's explicit
instruction not to do so. An NPC moved, round 2 began and a later shot hit for 5:
enemy HP 6 -> 1, arrows 20 -> 19, hero HP 11 unchanged. More attacks were rejected;
no final narration/AgentDone arrived. Shared QA database records 18 complete
operations and 1 interrupted operation; combat itself is 1 complete / 1 interrupted.
Mechanical guards prevented invalid shots/resource overspend but did not
prevent the model choosing an additional hero action.

Independent read-only review rejects all three configurations. This motivates
an application resolution/narration phase, receipt-only recovery, and safe
refusal reasons; it does not justify model promotion or full-goal completion.
The first interpretation of a player intention still needs semantic acceptance.

Evidence: .artifacts/qwen9b/source.json, download-proof.json, proof.json,
solo-report.json/solo-final.json, thinking/{proof,solo-report,solo-final}.json,
combat/{proof,report,final}.json, build/runtime logs and cleanup.json.
Weights remain in the user's LM Studio model directory. QA processes/listeners
are stopped and verified separately. A cancelled earlier unapproved attempt
left 196 bytes of metadata; automatic approval review rejected its cleanup. The
user subsequently approved this specific download. See BONSAI_EVALUATION.md for
installed-model failures and the active AI Factory plan for remaining work.

## Direct chat execution with a separate intention check

The user selected direct execution, including genuine compound requests, without
a confirmation dialog. The new application guard checks each proposed hero action
against the immutable human message, keeps confirmed outcomes across recovery,
and withholds final prose while requested mechanics remain unresolved. Raw legacy
mutations also require source authorization; completed receipts are recovered
before any new check. JSON/terminal failures interrupt before a new effect.

Real guarded runs used freshly built, uncommitted work on top of 8f1fdf2 with
inherited changes preserved. These are loopback HTTP/model checks, not native
session, physical LAN, clean-HEAD or release evidence. No model was promoted.

| Run | Actual outcome | Acceptance |
| --- | --- | --- |
| Nonthinking | First move confirmed; archive move omitted; following request interrupted | FAIL |
| Thinking, 1,024 tokens | 10 completed replies, all 10 chosen adventure actions, rescue / 10 gold / closed_crossing, revision 13 | Mechanics pass; narration FAIL |
| Thinking combat | First start-defense request interrupted before any tool effect; revision stays 5 | FAIL |
| Thinking compound move + inspect | Intention verification interrupted before any tool effect | FAIL |

The thinking route resolved town return and finish that earlier runs omitted, and
added no unrequested hero actions. One quick save occurred after locating Tikhon;
it is not a final snapshot. Independent SQLite/source/narrative review still
rejects the run: the narrator changes the requested dry route into waist-deep
water (index 1), leaves the already rescued Tikhon waiting for another trip
(index 3), invents a permanently flooded quay then describes its normal boards
(indexes 5/7), assigns feelings to the hero and introduces Russian wording errors.
The final epilogue accurately states the closed ferry, damaged ward, surviving
echo and one-time reward. Correct receipts do not make preceding prose truthful.

Direct checker probes also exposed weak decisions without thinking and malformed
or truncated output with thinking. A response granting permission with no
candidate is now invalid at the application boundary. Experimental shorter
prompts were only diagnostic; they were not promoted into production.

Evidence: .artifacts/qwen9b/guarded/{solo-report,checker-probe,checker-probe-object,
checker-probe-compact}.json, guarded-thinking/{solo-report,solo-final}.json,
guarded-thinking/combat/report.json and guarded-thinking/compound/solo-report.json,
plus build, model, backend, wire and focused-test logs. Isolated QA processes were
stopped after these runs; installed weights and ordinary settings remain intact.

## Structured permission and completion ledger

The provider port now requests JSON Schema, both OpenAI-compatible adapters
forward it, and RetryableProvider preserves it on each attempt. A wrapped-provider
wire regression reproduced the missing format before the retry fix. Complete
terminal metadata and application-side shape validation remain mandatory even
when an endpoint ignores the preference.

Permission and completion are separate questions. Denying a model proposal does
not finish the human request. Completion reads finished attempts (performed or
refused), asks for remaining positive commands quoted from the original message,
and cannot authorize a new action. It omits availability menus and maps, retains
reference labels/exit coordinates, skill-check success/total/DC and combat events
for explicit conditional requests. Empty, invented, oversized or extra-field
responses are rejected. These contracts improve guard reliability; they do not
prove universal natural-language accuracy. Every controlled Stop rechecks the
ledger even when the earlier permission forecast predicted no remaining action.
A pending compound step restores the canonical tool catalog without persisting
or publishing the premature reply (RED-to-GREEN workflow regression).

Fresh real observations on hash-verified QA executables over 04f1a89 plus current
uncommitted work, with inherited baseline preserved:

| Scenario | Confirmed observation | Limit |
| --- | --- | --- |
| Move to quay AND inspect, no further travel | Both selected actions succeed in order, then one save; location quay, revision 5, 65.604 s | Narration invents tool marks on railings rather than the ward |
| Start defense with an NPC escape motive | Start commits once; a faulty future-action decision loops until the 180 s client timeout | Original request is retained for continuation |
| Same-request defense continuation | Complete reply in 42.064 s; revision stays 6, entire combat projection unchanged | Russian and location descriptions remain imperfect |
| Shot at first water echo, explicitly no EndTurn | Correct target/weapon; engine refuses combat_obstructed; no turn advancement or resource change | Completion initially loops despite the finished refusal |
| Same-request refused-shot continuation with ledger | Complete reply in 27.461 s; revision stays 6, entire combat projection unchanged | Semantic FAIL: arrow described as flying and a water wall invented |

The obstruction reminder supplies the confirmed refusal cause after context
refresh/recovery. It still did not make the latest prose faithful: the engine
refused the action before a shot, and the map does not establish a water wall.
Model promotion remains blocked by semantic acceptance, not by passing fixtures.

Ledger diagnostics distinguish a finished refused shot from an explicitly
requested movement fallback and a still-pending inspection. These direct model
probes are narrower than full session acceptance. Paired skill success/failure
tests verify payload preservation, not real-model accuracy for every conditional.

Evidence: .artifacts/qwen9b/schema/{captures,verified,final,cue,ledger},
resume-combat.json, resume-shot.json, ledger-list-probe.json, versioned executable
hash proofs and focused/wire logs. Cold-start attempts before model readiness,
the attempted overwrite of a still-mapped old executable, and a resume before
reapplying QA settings are setup failures; they are excluded from semantic claims.
Bootstrap deliberately uses a mock until settings are applied. Current QA uses
versioned copies and verifies their hashes before launch. Backend/model/proxy
processes and ports 31425, 31809 and 31810 were stopped and verified; ordinary
settings and installed weights remain untouched.

Checkpoint verification: 26 workflow, 9 provider-wire and 5 retry tests pass;
final consolidated gates pass with 876 Rust and 1117 frontend tests, including
architecture/static checks and Clippy. Independent staged-source review closed
the checked implementation; independent semantic review confirmed the prose
failures above. This is local verification, not native or physical LAN acceptance.

Narration-check prototype: the installed Qwen classified four supplied public
fact/draft pairs correctly (actual failed shot/quay text rejected; paired faithful
rewrites accepted). It recognized invented_mechanism for the water wall, and
clue_mismatch plus required_discovery_omitted for the quay. Diagnostic request
latencies were 17.761, 16.961, 20.677 and 20.869 seconds. Evidence lives in
.artifacts/qwen9b/schema/narration-probe.json. This is not wired into publication;
paired examples and manually supplied evidence do not prove session acceptance.

## Narration publication gate (integration verification in progress)

The application now buffers the final story until a separate strict fact check
accepts it. It can repair prose once with no tools, then checks again. Failed or
incomplete checks interrupt without storing/publishing the bad draft; confirmed
attempts survive continuation. Checks use public scene/outcome evidence, not the
private DM prompt or whole tool-result history. Known discovered Bell facts and
call-scoped movement/weapon/spell obstruction facts constrain the story. The
effective saved model is used on room-resume even after changing settings.

Focused 30 workflow, canonical-public-fact and 40 room-turn regressions pass;
independent source review closed the movement-refusal and stale-model findings.
Real model compound/refused-shot probes and consolidated gates are running on
a hash-verified executable; fixture success is not semantic session acceptance.

Integrated observations on executable f57395810b37 over003077f plus the
narration-gate source:

- Blocked longbow shot: AgentDone128.584s, correct target/refusal, full combat
  projection unchanged (revision6, arrows20, HP/positions/action unchanged), no
  EndTurn. Story says the arrow never left the bow and invents no water wall.
  Independent semantic PASS for this scenario; two closing questions and latency
  remain limitations. The shared host also ran consolidated gates during this probe.
- Compound move+inspect: both effects once, quay/revision4, but no final text or
  AgentDone at160.662s. Narration rejected; correction did not finish. Same-request
  resume preserves the entire adventure, replays the two recorded actions and
  adds one quicksave (revision4 to5); it also publishes no bad text.
- Captured correction ends with finish_reason=length and continued reasoning in
  content. Direct reasoning_effort=none is ineffective on this installed server;
  an isolated chat_template_kwargs enable_thinking=false probe stops in2.355s
  without reasoning, but merely repeats the wrong draft. It does not prove a fix.

Evidence: .artifacts/qwen9b/schema/narration/{build-proof.json,shot.json,
compound/solo-report.json,resume-compound.json,captures,probe-repair.json,
probe-repair-no-thinking.json}. Per-request template parameters were checked
against the [llama.cpp server documentation](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md)
and tested against the installed binary. No production reasoning preference or
ordinary model setting was changed.

Final local verification: 882 Rust and1117 frontend tests, architecture/static
checks and Clippy pass. Independent source review closed both findings. These
results establish a publication/recovery boundary, not full narration or native/
physicalLAN/session acceptance. Original32 dirty-file hashes remain unchanged.

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

Human-command live evaluation, 2026-10-09 (HEAD a76db0c plus owned changes):
strict planner schemas now couple Exact versus NeedsResolution fields, the prompt
specifies full adventure arguments, and the catalog contains only the four typed
human action tools. Qwen thinking-off parsed and executed move quay + inspect
quay exactly once while retaining the no-further-movement constraint. Initial
inner-argument and mixed-null proposals were rejected before effects.

The diagnostic-list checker falsely classified a canonical ward description as
an invented mechanism. Publication now separates a strict faithful boolean from
presentation and per-required-fact coverage. Negative classification cannot be
overridden by empty or cleared diagnostics; malformed/truncated checks fail
closed. No canon was changed. Independent source review found no actionable
issue. Small paired probes are calibration only, not universal model accuracy.
Thinking-on controls reject wrong object, water wall, new hero action/emotion,
recovery, reward and healing; its omission false acceptance is caught by the
separate required-fact coverage check. Internal planning fails presentation.

Executable SHA256 45affc7f6bf4cceecd739496732aeb1f944999182e594d97a2ecd5aa489557f6
resumed the original request after a real backend restart. Thinking-off still
failed delivery (8.616s). Thinking-on delivered a verified correction and Done
(71.936s); move/inspect receipts were replayed, not executed again. Adventure
history, location and inventory stayed unchanged. A new quick_save alone advanced
revision4 to5. Independent semantic review accepts minimal discovery/agency for
this episode but flags masculine address to Nora and weak clue phrasing.

Complete rescue/ending acceptance remains FAIL: the next request to enter the
archive along the dry upper path was rejected by manifest verification before
any effect (22.501s, revision5 unchanged). Adding public-route facts and schemas
in isolated probes did not resolve this rejection; those experimental inputs
were not added to production. Do not infer complete playability from one Done.
No model/default reasoning preference was promoted. Quantified commands, native
AI-led sessions and physical separate-device LAN/VPN acceptance remain open.
Evidence: .artifacts/qwen9b/human-a76db0c/{build-proof.json,captures,
compound-v3-report.json,resume-off-report.json,resume-thinking-report.json,
continuation-report.json,thinking-*.json,gates.log}.

Focused36 workflow and5 planning groups pass. Frozen consolidated gates pass:
942 Rust tests,1127 frontend tests, architecture/static checks and all-feature
Clippy. Backend build passes. This backend-only scope has no new browser/native
acceptance. User-owned source changes remain separate.
