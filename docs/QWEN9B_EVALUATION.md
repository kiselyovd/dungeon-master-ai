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

Quantified live continuation (a62e907 plus owned planner fixes): executable
0dbde38118bd296821bcfa5b89b84b49d344d6ab28194b56d99478b60e313b51
completed the original "Жду минуту дважды. Ход не завершаю." after an interrupted
planning attempt and backend restart. Two distinct canonical rest receipts contain
Wait1 minute, campaign clock60 then120 seconds, revision2→4. Location, inventory,
HP and spell slots stayed unchanged; no end_turn was executed. Thinking-on took
105.745s and needed one prose correction to remove an invented feeling. This is
a real quantity episode, not a latency or full-session acceptance result.
Thinking-off initially proposed60-minute waits plus a move from the prohibition;
thinking-on before the prompt fix duplicated the prohibition as NeedsResolution.
Both were rejected before any effect. Positive-only clarification and duration
example are added; paired duration control stays one2-minute wait.

The next compound request failed on source occurrence indices and a duplicate
constraint. Renaming the unresolved wire field/adding source descriptions fixed
one probe but broke quantity/duration controls; that experiment is NOT promoted.
Instead, source/structure errors get exactly one fresh plan generation using the
protected original input and quoted rejected proposal. All typed arguments in
ALL commands are checked first, so a source error cannot mask an argument error.
Malformed/truncated responses, invalid arguments and semantic denials still fail;
the repaired proposal must pass structure and independent semantic approval
before persistence or dispatch. Nine planner groups pass, including mixed-error,
false-approval and bounded/truncated-repair regressions. Independent review closed
the mixed-error finding and has no remaining source issue.

Executable fe53127c0b10f5da9a70be423be62b5776b67aab055bf08efe160b7f19e08012
actually repaired the original quay request (captures15→16→17), then executed move
and inspection once (revision4→6, locationquay). Delivery still failed after97.403s:
faithful=false correctly rejected invented feeling/interpretation, but diagnostics
reported only a false omission, which coverage cleared. The gate failed closed
before prose correction. Independent semantic review confirms this distinction.
Next: a valid negative decision without localized diagnostics must remain an
explicit non-publishable internal verdict, allowing the existing one tool-free
prose correction and fresh checks. Do not fabricate a fact ID/offending quotation,
accept the original draft or retry game effects; malformed checks remain errors.
This next publication transition is not implemented in this checkpoint.
Evidence: .artifacts/quantified-0f6d4ae/{build-proof-a62e907.json,
build-proof-prohibition.json,build-proof.json,captures,quantity-fixed-report.json,
quantity-sql-proof.json,session-resume-report.json,source-final-*.json}.
Full rescue/ending, native sessions and physicalLAN remain open; no model/default
promotion or user-profile mutation.

Source-repair checkpoint gates pass:951 Rust and1127 frontend, architecture/static
checks and all-feature Clippy; final backend build passes. Original32 dirty source
hashes preserved, owned evaluation processes stopped. No browser/native/physical
LAN/full-adventure acceptance is inferred from these backend and model episodes.

Unlocalized negative checkpoint (8d7f569 plus owned changes): publication requires
an internal verified decision, not merely an empty violations list. A validated
negative without localized claims can receive exactly one fresh tool-free prose
correction and full reassessment. No fabricated fact/quote is generated. The
model diagnostics deserialize into a separate closed type; the internal decision
cannot be injected by wire data or promoted by clearing claims.39 workflow groups
cover empty/cleared diagnostics, second denial/no third generation, corrupted/
truncated replies, absent bad text/history/private data and unchanged dispatcher
count. Independent source review has no actionable finding.

Executable ff30325b7b4f6aa63292732c3b879c0bf4112e9bbbe355debb629e63155ffe81
resumed the old quay request and delivered Done in36.938s, revision6 unchanged,
replaying prior receipts. This draft passed immediately, so it did NOT exercise
the new unlocalized branch. Independent review passes mechanics/agency/discovery
but FAILS story quality: rust from a blade and bent copper plates are unsupported
investigation/repair clues. The automated positive verdict missed them; no model
promotion is justified. The next archive/upper-route request again failed
manifest verification before effects (21.036s, revision6 unchanged). A normal
read-only question was also misclassified as an inspection and rejected.

A separate controlled-provider HTTP/SQLite fixture explicitly supplied four
expected decisions: empty read-only plan, semantic approval, negative fidelity,
empty diagnostics. Real Qwen generated the one fresh correction, then its new
fidelity/presentation decisions ran normally. Done in79.479s; no tool steps,
revision6 unchanged, one bound final message matching the delivered text, operation
complete, all state except chat unchanged and the discarded draft absent from
history. Current campaign clock720 seconds includes the previously committed
travel; the fixture does not change it. This is recovery/adapter proof with
controlled decisions, not actual intent extraction or overall model accuracy.
Evidence: .artifacts/unlocalized-8d7f569/{build-proof.json,session-resume-report.json,
captures,continuation-report.json,fault-live-report.json,fault2-live-report.json,
fault2-sql-proof.json,fault2-captures,gates.log}.

Full gates pass (954 Rust,1127 frontend, architecture/static/all-feature Clippy)
and backend build passes. Next quality work: supply grounded public references
for route-qualified intentions, distinguish DM questions from hero actions, and
reject invented investigative object details. Native/full solo/physicalLAN
acceptance remains open; no user/default-model settings were changed.

Grounded reference episode,3b75a3a plus owned data changes:
Public route descriptions now explicitly map dry upper-path qualifiers to full
canonical MoveArchive args. Discovered facts and safe meanings of offered actions
are also available to both intent phases; no undiscovered NPC outcome/secret is
added and references do not grant permission. Actual route request now passes:
revision6→7, oneMoveArchive, Done86.860s. Initial repair relocated the historical
ward discovery to archive; independent review marks narration FAIL. The actual
Locate mapping now passes after an offered-action meaning is supplied: one
LocateTikhon, revision7→8, Done57.839s. Its prose incorrectly damaged the sluice
instead of the ward; independent narrative acceptance remains FAIL.

Ward fact now explicitly identifies the old-quay discovery, rules out unconfirmed
material/plate/tool/extra-infrastructure damage, and keeps later observations and
restoration distinct. Rescue reward text consistently identifies the rescuing
player-controlled hero rather than the ferryman/daughter. Bell-only presentation
glossary supplies exact EN/RU object labels to the existing fresh-prose repair.
A false roster prerequisite in AcknowledgeEcho reference was found by independent
review and removed; the domain allows acknowledgment without that record.

Legacy rescue request committed once: revision8→9, Tikhon home, Nora stillarchive,
exactly10gold to Nora; delivery failed after108.112s with wrongrecipient and
memory/new-route claims. Retry after recipient clarification kept revision9 and
still failed after27.870s: its prose stated an unconfirmed sluice closure, then
diagnostics incorrectly required an optional ward discovery. The DM brief was
itself incorrectly using past closure language whenever ward damage was known;
that source bug is fixed using the actual LowerSluiceSealed fact, including no
unexamined cause disclosure on early unfinished endings.

Independent requirement review identifies an additional FAIL: personally
escorting Tikhon home requires hero travel, which legacy rescue_tikhon does not
perform. This cannot be closed with NPC-only receipt or patched in narration.
VERIFY now expressly checks effect sufficiency; a model-only replay of the actual
wrong manifest produces valid=false. This paired probe is not universal intent
accuracy, and the old saved binding/receipt remains truthful and unchanged.
Next canonical capability must preserve old NPC-return and separately represent
physical escort, with genuine hero arrival and existing replay/once-reward guards.
Evidence: .artifacts/public-routes-3b75a3a/{build-proof-routes-only.json,
build-proof-actions.json,build-proof-clarity.json,build-proof.json,captures,
session-resume-report.json,locate-resume-report.json,remaining-report.json,
rescue-before-closure-fix-report.json,escort-insufficiency-probe.json}.
No default-model/reasoning promotion, native/physicalLAN/full-adventure acceptance.

Final grounding checkpoint:18 lib,11 planning and40 workflow groups pass; frozen
consolidated gates pass (960 Rust,1127 frontend, architecture/static/all-feature
Clippy), final backend build succeeds. Latest source fixes are not promoted as a
successful live narrative/physical escort run. Existing real failures and partial
legacy intent remain documented. Original32 source hashes preserved; isolated QA
processes stopped. Final-source binary proof: build-proof-final.json; executed
older-binary evidence retains its separate proofs and captures.

Canonical personal escort checkpoint (050f0ad plus owned implementation): separate
escort_tikhon_home takes the hero to Town as well as returning Tikhon, while
legacy rescue keeps Archive. Guarded route cost600s and once-reward share the
existing transactional/replay path; no migration or production dependency added.
SQL identity/stale/replay/time/map/restore/rollback runs both variants. Roomguest
runs both with rollback/replay and no other hero-sheet change. Production browser
with realHTTP/SQLite selects escort, confirms Town/mapremoved/10gold, completes
peaceful ending and reloads it. This is manual UI/engine proof, not AI narration.

Targeted Qwen with THREE canonical fixture setup actions (MoveQuay, MoveArchive,
LocateTikhon) then original "Провожу Тихона домой по безопасному пути": model
selected new escort, separate checks approved, one canonical effect revision5→6,
Town, exactly10gold and clock1500 (900 setup+600escort), Done46.506s. Full AI session
is not proved. Independent review passes selection/physical intent and FAILS
agency in prose: "Вы передали ему последние слова" invents hero speech. The
future-choice menu also deviates from the intended one-question style.

A separate agency boolean correctly flags invented emotion and accepts declared
speech, but falsely accepts the unrequested farewell and rejects the NPC-only
control. This probe is rejected, not a new production validator or override.
No model/default promotion. Evidence .artifacts/escort-050f0ad/{build-proof.json,
compound-report.json,captures,model-sql-proof.json,choices.png,browser-final.log,
agency-probe-*.json,gates-final.log}. UI fixture controls only native environment;
campaign state/actions come from the isolated real server and SQLite.

Escort frozen verification: consolidated gates GREEN (961 Rust, 1128 frontend;
architecture/static/all-feature Clippy). All four affected production-browser
scenarios pass, including legacy rescue, snapshot recovery and defensive ending.
Evidence: .artifacts/escort-050f0ad/browser-all.log and gates-final.log.

Agency diagnostic continuation after 1d1c833 (isolated Qwen9B, no production changes):
- Per-claim extraction/support with engine context, reasoning budget1024:
  4/6 expected accept/reject decisions. It falsely accepts original farewell
  as engine_outcome and rejects the NPC-only control. Exact-source fields also
  contain fabricated/nonexact quotations. Rejected as a production validator.
- Same six inputs with reasoning budget4096/max_tokens8192: 5/6 decisions;
  original farewell still falsely accepted. More reasoning alone is insufficient.
- Direct/no-thinking same six inputs: 4/6; original farewell and an invented
  explicit hero goodbye are falsely accepted. Some evidence quotes copy draft
  rather than source. No runtime/default reasoning promotion.
- Explicit bilingual hero-speech generation instruction, three captured-context
  replays: independent review rejects all three. One changes route/retells arrival,
  one assigns readiness for new tasks, one relocates the granted reward to a purse
  on a table. No production prompt patch accepted on these results.
- Isolated source-only entailment of one hero claim against original human text,
  without engine context, direct/no-thinking: 4/4 paired decisions (reject original
  farewell and new emotion; accept declared speech and faithful paraphrase).
  This is a narrow diagnostic, not full-draft extraction or session acceptance.
  Hypothesis: mixing effect evidence with human authorization encourages the model
  to treat a completed action as permission for additional hero behavior. Follow-up
  must verify actor identification/exact claim extraction and source support on
  whole drafts, NPC controls, refusals and compound requests before integration.
Evidence: .artifacts/agency-1d1c833/{farewell,npc_only,declared_speech,
invented_emotion,npc_speech,hero_speech,generation-*,direct-*,source-*}.json,
budget4096/*.json. Captured original prompts and outputs are preserved. No engine
writes, user campaigns, defaults or production dependencies changed.

Full-draft source-support continuation after ef164fe:
Fourteen paired controls include the actual farewell, NPC-only control, explicit
and paraphrased hero speech, new emotion, compound actions/extra promise, refused
attack/substitute spell, miss, involuntary damage and an NPC compliment.
Direct/no-thinking exact hero-claim extraction then source-only entailment:11/14;
misses actual farewell and substitute spell and rejects declared speech. Whole
source-only draft assessment:11/14, falsely denies NPC-only, involuntary damage
and NPC compliment. Explicit hero/NPC roster variant:10/14, also falsely allows
new hero speech and denies requested paraphrase. The earlier4/4 single-claim pilot
therefore does NOT justify production integration. All three variants rejected.
Evidence .artifacts/agency-full-ef164fe/{probe*.mjs,*.json}; no new validator,
runtime mode or model defaults promoted.

Grounding repair: completed EscortTikhonHome disappears from available intentions,
so its human-readable route meaning was missing after success. A public historical
route fact now requires Bell package, rescued fact and explicit escort in history.
It describes the anonymous initiating hero's Archive→Quay→Veresk arrival; later
movement/current shared scene remain authoritative, and it neither assigns that
trip to another human nor confirms extra hero speech/emotion. Same text reaches
DM context and public narration evidence. Legacy NPC-only rescue/preescort restore
cannot acquire it. Missing action in bounded history means unavailable historical
proof, not proof the action never occurred.
Regression RED then GREEN; all19 application lib tests pass. Independent source
review found no actionable issue. Three frozen successful-context Qwen replays
with the current source fact produced no new hero speech but remain insufficient
for full quality acceptance: final prose still adds unsupported world/location
or reward details. They are context replays, not fresh engine/full-session proof.
Initial model-loading503 captures route-generation-*.json are excluded; only
route-generation-ready-*.json have successful responses and Stop endings.
Full AI-led peaceful/defensive, native/recovery and physical LAN/VPN acceptance
remain open. Original32 source hashes and unrelated worktree edits preserved.

Historical-route checkpoint final verification:962 Rust and1128 frontend tests
pass; architecture/static/all-feature Clippy GREEN, final backend build passes.
Logs .artifacts/agency-full-ef164fe/{gates.log,build.log}. Independent source
review clean; independent narrative review still FAILS broad acceptance. Owned
QA model stopped and its port free. No UI layout or transport change in this step.

Fresh full-session attempts after880e9bf:
- Latest source backend64f902cceddf24b9, fresh isolated DB, canonical fixture escort
  PASS before model startup. Fresh AI campaign3d55b5d8-87a3-4211-b451-a20c8e3d5777,
  original first message "Иду к старому причалу и осматриваю оберег."
  stops after22.567s, revision2 unchanged. Correct two-command generation is
  semantically rejected: currentTown public_actions lacks InspectQuay meaning.
- Supplying only canonical schemas/normalizedgoal explanation still rejects it;
  captured reasoning says inspect_quay inspects a quay instead of the requested
  ward. Supplying trusted proposed-action definition fixes that captured positive
  case; omission, wrongtarget and structurally valid swapped-goals controls deny.
  A raw reversed-source-order control was falsely accepted by the model, but
  validate_structure rejects it before semantic verification in production.
- Implemented shared canonical-purpose helper; proposed_action_meanings only for
  decoded Exact Adventure goals after structural validation, Bell-gated. Availability
  stays truthful/current; definitions do not reveal discoveries or authorize actions.
  Both initial and clarification VERIFY receive definitions and canonical schemas.
  Current public_actions/discovered facts/engine eligibility remain unchanged.
- Regression RED missing payload then GREEN:12 planner,19lib,40workflow groups.
  Independent source review clean; full gates963Rust/1128frontend GREEN and buildPASS.
- New-source183e77bbd2e616dd backend, separate fresh DB and canonical fixturePASS:
  first full-session request fails after26.154s, revision2 unchanged. This time
  planning/one structural repair repeat incorrect occurrence1 and invented overlapping
  constraint before semantic verification. Reasoning budget1024 is exhausted.
- Same captured generation with thinking disabled produces correct two commands
  and empty constraints. However current-verifier direct-mode controls fail positive
  case and falsely allow a question mapped to InspectQuay (only3/5 expected decisions).
  Fresh direct-mode full-session first request likewise stops after3.107s, revision2
  unchanged. Direct-mode/runtime default is NOT promoted. All3 AI campaigns have
  no gameplay effects; request planning remains blocked, not a completed session.
Evidence .artifacts/full-session-880e9bf (initial/prototype captures),
.artifacts/meaning-880e9bf (new binary/build-proof, fixture, reports, captures,
gates.log, build.log), .artifacts/direct-meaning-880e9bf (controls/runtime/report).
The SSE contains only keepalive on these failures; Done/errors arrays in diagnostic
reports do not imply success: authoritative backend code is
human_command_planning_unavailable, durable revision/history remain unchanged.
No rejected semantic/narrative validator is integrated or overridden. A stronger
local-model test is proposed separately; approval for9B is not reused for a new
16.5GB27B download. GPU actually RTX3080/10GB, RAM68,565,700,608bytes;27BQ4 cannot
fit wholly on GPU. Neither speed nor quality of27B is verified or promised.
