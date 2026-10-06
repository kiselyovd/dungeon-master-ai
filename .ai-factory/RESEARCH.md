# Playable solo and LAN/VPN cooperative sessions

Date: 2026-10-06
Status: proposed design and authored adventure draft; not executable game content.

## Agreed intent

- Finish a complete playable adventure with an AI Dungeon Master.
- Use one authoritative campaign system for solo and cooperative play.
- Cooperative players use separate devices, initially over LAN or VPN.
- Prepare world history, characters, secrets and consequences in advance rather than delegating their invention to a small runtime model.
- Evaluate the installed Ternary Bonsai 2 27B as a replacement candidate.
- Preserve existing worktree changes. Add no production dependencies without approval.

Working assumptions: 1 player in solo; 2-4 players in cooperative play; AI is the Dungeon Master; a player hosts campaign storage and inference. These assumptions remain adjustable. Initial host migration, public matchmaking, voice chat and public Internet relay are outside this first delivery.

## Current evidence and gaps

- Production build and TypeScript passed in the preceding audit.
- Architecture guard and Rust formatting passed after selecting installed toolchains explicitly.
- 43 selected frontend/script tests passed. The entire Vitest suite did not complete in the audit; it must not be reported as passed.
- Fresh production browser checks: 6/7 passed. Full-campaign save interception expects 127.0.0.1 while the client uses localhost; fix the harness before using this scenario to assess actual save behavior.
- Biome reported one formatting error in src/api/__tests__/agentEvents.test.ts and six warnings.
- Campaign activation currently changes campaign/session pointers without restoring the entire campaign projection.
- App sets worldReady from pc.heroClass, skipping independent world preparation.
- Player token selection currently uses name equality.
- AgentTurnRequest has campaign/session identity but no authenticated participant identity.
- Client combat request IDs use a process-local counter. Cooperative command identities must be globally unique and scoped to campaign/participant; two clients must not collide on combat-1.
- Combat already has revision checks and compare-and-set persistence. Reuse these, adding ownership and session-scoped ordering rather than duplicating the engine.
- The archived full-campaign live result is failed with missing combat_ended evidence. A fresh real solo and cooperative playthrough is still required.

## Proposed authority and content contract

Host-owned state: campaign, participant membership, character ownership, current scene, encounter, resources, discovered clues, NPC facts, journal, rewards, save revisions and adventure flags.

Clients send intentions. The server checks membership, character ownership, turn, resources, range, request identity and expected revision. Attack modifiers and equipment effects are derived from canonical sheets, not trusted client values. The AI proposes narrative and permitted tools; the engine accepts or rejects changes and supplies their actual outcomes.

One ordered execution lane per campaign prevents two simultaneous AI turns from interpreting the same obsolete state. Chat discussion may remain concurrent. In combat, only the current actor can commit its turn actions. Outside combat, participants may submit individual actions or explicitly agree a group action; the system does not require every player to answer before every narration.

An adventure package should separate:

1. immutable world canon and actual secrets;
2. player-visible facts;
3. each NPC's knowledge, beliefs, agenda and possible lies;
4. authored scene descriptions, connections, clues and encounters;
5. runtime facts and explicit consequence rules;
6. optional flavor the model may invent without changing canon.

The model receives the current scene, relevant canon, participant sheets, known clues and the engine's recent outcomes. Hidden information is supplied only when needed for DM reasoning, never copied into ordinary player events. The whole adventure is not injected into every turn.

The model may vary wording, gestures and nonessential background detail. It may not invent new quest-critical objects, rewrite secrets, grant resources, resurrect characters, reveal undiscovered clues as already known, or silently change scene. Unsupported actions receive an honest response with alternatives. No player action is committed merely because prose describes it.

Narrative proposals and their mechanical events need a recoverable operation record. Cancellation before commit changes no resources; interruption after commit resumes from the recorded outcome without rerolling or duplicating effects. The policy for partial multi-tool turns must be explicit in the implementation specification.

## Authored world: The Reed March / Тростниковая марка

Tone: grounded frontier fantasy, river trade, old civic promises, modest supernatural danger. People have practical interests and conflicting loyalties. Danger is discoverable and avoidable; conversation and investigation matter as much as fighting.

### Immutable history

- Forty years ago the town of Вереск survived a spring flood by opening the abbey's lower sluice, sacrificing the riverside archive instead of the inhabited upper town.
- Ferryman Савелий kept the last evacuation crossing open. He died rescuing the final boat. There are no surviving people trapped from that historical flood.
- The town promised to maintain the river memorial bell and publish the names of the rescued. The bell is a real boundary ward: its inscription binds river spirits to the abandoned lower quay when sounded with the memorial roster.
- A working copy of the roster was preserved at the town shrine. The drowned original is in the archive. Either copy can satisfy the ward after the damaged inscription is repaired.
- The archive contains a later land ledger proving that merchant Варден illegally obtained the old quay. This ledger is separate from the memorial roster.
- Варден recently cut the bell's clapper loose to strip and sell its silver fitting. This damaged the ward. He concealed the clapper in his warehouse; it is bent but repairable.
- Current missing ferryman Тихон is alive in the upper archive chamber after his boat failed. He has food for a day. He did not cause the ward's failure and is not secretly Савелий.
- The disturbances are echoes of Савелий, not an army waiting to invade the entire world. The echoes repeat his last rescue route and can be addressed by acknowledging the rescue.

The model must preserve these distinctions: historic Савелий / present Тихон, memorial roster / land ledger, cause of ward failure / consequences of flood.

### Locations

| ID | Visible name | Function |
|---|---|---|
| town | Вереск | Safe hub, shrine, inn, council and return point |
| quay | Старый причал | First physical clues and access to the river route |
| warehouse | Склад Вардена | Recover clapper; social, stealth or openly negotiated route |
| archive | Затопленный архив | Rescue Тихон and optionally recover original records |
| belfry | Нижняя звонница | Resolve the damaged ward and choose consequences |

World expansion seeds: a drought upriver that changes the ferry economy; a council election after the ledger is published; a missing bellmaker whose repairs conceal older inscriptions. These are future hooks, not extra objectives of the first adventure.

## First adventure: The Bell Beneath the Water / Колокол под водой

Target: level-1 characters, one complete session. A compressed acceptance route should take roughly 30-45 minutes; a conversational game may take 60-90 minutes. These are design targets, not measured results.

Player-facing premise:

> В Вереске третий вечер слышен колокол из затопленной звонницы. Сегодня не вернулся перевозчик Тихон. Его дочь ждёт у пустой лодки, а на мокрых досках причала появляются следы босых ног — против течения.

Primary objective: return Тихон safely and resolve the dangerous river disturbance. Secondary choice: whether to expose Варден's land theft. Publishing the ledger is never required to rescue Тихон or restore the ward.

### NPC cards

| ID / name | Public role and voice | True agenda | Knows / does not know | Disclosure conditions |
|---|---|---|---|---|
| mila / Мила | Ferryman's adult daughter; concise and practical | Bring her father home | Knows last crossing and missing boat; does not know who stole the clapper | Immediately gives last route and a dry rope; recognizes father on rescue |
| agata / Агата | Innkeeper; remembers debts and travelers | Keep the town safe without panic | Saw warehouse porters carrying a wrapped metal object; cannot identify it as the clapper | A conversation gives the observation without a mandatory roll |
| leon / Леон | Shrine caretaker; patient, exact about names | Restore the memorial and avoid vigilante punishment | Knows the ward, has roster copy; does not know the clapper's location | Explains ritual once players describe the bell or quay marks |
| varden / Варден | Merchant; courteous, always offers terms | Conceal theft and land fraud | Knows clapper location and ledger relevance; wrongly believes the haunting can be bought off | Evidence, repayment offer, public pressure or an agreed exchange can produce the clapper |
| tikhon / Тихон | Living ferryman; exhausted, dislikes ceremony | Leave the archive alive | Saw the echo's rescue route; found ledger but cannot read its legal detail | Once reached, reveals safe upper exit and offers recovered ledger |
| savely / Савелий | Echo; repeated fragments about the last boat | Complete the remembrance of the rescue | Remembers historical flood, not modern council politics | Acknowledgment and roster allow a peaceful resolution |

NPC agenda changes are explicit: Варден may cooperate without confessing the land theft; Мила's gratitude is not blanket agreement with every later decision; Савелий does not become a general quest oracle.

### Optional authored hero histories

These are story hooks to attach to valid existing SRD characters, not new mechanical classes or fixed identities. Players may edit names and histories before play.

- **Дарен Вейл, fighter hook:** former caravan guard who left a employer after a preventable bridge collapse. Агата once sheltered his caravan. Wants to save someone without accepting an order blindly; has no prior knowledge of this town's culprit.
- **Лисса Ренн, rogue hook:** courier returning an unpaid letter to Тихон. The letter is a mundane request for a family visit, not the solution to the ward. Wants to repay a personal debt; has no automatic access to the warehouse.
- **Мира Сол, cleric hook:** apprentice tasked with copying the shrine's damaged memorial records. Knows remembrance has local religious importance, but learns this ward's exact procedure from Леон.
- **Элин Торн, wizard hook:** scholar studying inscriptions changed by flowing water. Recognizes the inscription as a ward after inspecting it; cannot conjure the missing clapper or invent a replacement ritual without an engine-supported resolution.

In solo any hook can reach every essential outcome. Cooperative hooks create reasons to discuss decisions without forcing PvP, a traitor or exclusive clues. Companions are optional future work, not necessary for the solo acceptance path.

### Scene graph and authored beats

| Scene | Entry and purpose | Guaranteed progress | Optional challenge / consequence | Exit |
|---|---|---|---|---|
| S1 Empty ferry | Мила asks for help at the inn/quay | Missing ferryman's route and a rope | Agree on a reward or promise; record only an explicit agreement | Investigate quay, shrine or merchant |
| S2 Broken ward | Inspect bell marks and footprints at quay | Ward visibly damaged; archive route discoverable | Investigation DC 12 identifies tool marks; failure costs time but not the essential clue | Shrine explanation, warehouse or archive |
| S3 Merchant's terms | Recover clapper from warehouse | At least negotiation, public intervention and physical retrieval remain possible | Persuasion DC 12 may improve terms, stealth DC 12 may avoid guards; failure changes route/pressure rather than deleting the object | Clapper recovered or knowingly proceed without it |
| S4 Upper chamber | Reach living Тихон in archive | Speech locates him; upper stair route avoids a forced swimming check | Athletics DC 10 shortens a risky crossing; failure uses the safe route. Recovering ledger adds an optional complication | Rescue to town, explore records, or approach belfry |
| S5 Last crossing | Resolve echo and damaged ward | Peaceful restoration, mechanical sealing and defensive combat are distinct supported routes | Restoring needs repaired clapper + either roster + acknowledgment. Sealing closes lower sluice and protects upper town but loses quay access. Defensive combat buys time for rescue/escape; combat alone does not repair ward | Resolution chosen, or explicit unresolved retreat |
| S6 Return to town | Apply outcomes and conclude session | Journal summary, reward once, save and continuation hook | Exposure of ledger alters Варден/council relations; concealment preserves leverage | Closed adventure or explicitly resumable unresolved state |

Critical-clue rule: failed checks never erase the missing person's location, ward explanation or all paths to the clapper. Checks affect speed, cost and leverage. No essential clue depends on one NPC surviving or one arbitrary phrase in player dialogue.

Physical topology: town connects to quay, shrine and warehouse; quay has a maintained upper footpath to archive; archive connects to belfry by upper gallery; every dangerous location has a return path. Flooded lower passages are optional shortcuts. A player can visit S4 before S3 and return after rescuing Тихон.

### Runtime facts and ending predicates

Proposed flags:

`tikhon_located`, `tikhon_rescued`, `ward_understood`, `clapper_recovered`, `clapper_repaired`, `roster_available`, `ledger_recovered`, `ledger_published`, `echo_acknowledged`, `ward_restored`, `lower_sluice_sealed`, `reward_granted`.

`roster_available` becomes true when shrine copy or drowned original is obtained. `ward_restored` requires repaired clapper, roster and acknowledgment; it cannot become true from combat victory. `ledger_published` requires the recovered ledger plus a deliberate public action. Rescue does not automatically resolve the haunting.

- **Remembered river:** rescued ferryman + restored ward. Quay reopens; shrine adds the rescuers' names with their consent.
- **Closed crossing:** rescued ferryman + sealed lower sluice. Immediate danger controlled; trade route closes; the ward remains damaged. Future repair is possible.
- **Unfinished debt:** no completed ward/seal resolution, or ferryman remains missing. Return/retreat is allowed; campaign records unresolved goals, with no false victory or success reward.

Ledger exposure is an independent consequence layered over each ending. It does not unlock a mandatory fight. Reward is issued once for verified rescue; no XP or gold is fabricated by the narrator. Exact rewards are to be balanced with the verified equipment/economy catalog before executable content is written.

### Combat and solo/cooperative balancing

- Defensive encounter is opt-in after readable warning, failed de-escalation or deliberate aggression. Peaceful completion remains valid.
- Use verified existing creature/encounter templates with stable IDs; the echo's flavor must not silently grant new stat-block abilities.
- Create explicit encounter variants for solo and 2/3/4 participants. Validate damage, action economy and escape options for level 1 before declaring any variant balanced.
- No automatic difficulty scaling during an encounter and no extra waves because the model wants drama.
- Solo route has accessible safe exits and no puzzle requiring two simultaneous physical actors.
- Cooperative map actions, dialogue and quest decisions credit the actual participant. A disconnect pauses that player's unresolved combat turn; the host may explicitly pause the game or grant temporary character control, with the decision visible to everyone.
- Defeat enters an explicit recoverable outcome: retreat, capture or rescue as authored. The model cannot silently reset HP or invent resurrection.

### Prepared media direction

Required scenes: Вереск quay, warehouse exterior/interior, upper archive chamber, belfry gallery. Maps require readable floor levels and dry return paths; illustrations portray the same topology and NPC appearance. Use existing project art conventions. Media availability never gates mechanics; reusable authored map geometry and a bundled illustration are sufficient for acceptance. New image assets have not been generated in this exploration.

## Model candidate: installed Ternary Bonsai 2 27B

Observed local files:

- D:/lmstudio/models/prism-ml/Ternary-Bonsai-2-27B-gguf/Ternary-Bonsai-2-27B-PTQ1_0.gguf: 5,946,648,928 bytes.
- D:/lmstudio/models/prism-ml/Ternary-Bonsai-2-27B-gguf/Ternary-Bonsai-2-27B-mmproj-BF16.gguf: 931,145,856 bytes.
- NVIDIA GeForce RTX 3080 and about 63.9 GiB physical RAM observed. Actual VRAM/free memory and sustainable context still need measurement.
- lms CLI is installed. No listener on ports 1234/1235 was observed; this is not proof that no model server exists on another port.
- File presence is not inference or application compatibility evidence.

Official documentation identifies Bonsai 2 as Qwen3.8-27B-based. This installed ternary PTQ1_0 packing is approximately 1.75 bits/weight, not the separate binary 1-bit variant. Official format documentation requires the PrismML fork's activation transforms. Stock GGUF runtime compatibility must not be assumed.

Known-issues documentation describes malformed/repeated tool calls, long reasoning and output-budget exhaustion. It lists reasoning_effort medium as a shorter-reasoning option and documents special handling for system messages and reasoning history. These are candidate evaluation concerns, not evidence this installed model has failed our game.

The app currently starts its bundled local model through mistralrs. An initial Bonsai experiment should use a separate official PrismML local OpenAI-compatible endpoint through the existing provider adapter, keeping campaign mechanics unchanged. Compatibility with mistralrs is unverified. Do not copy model files, change the default provider, install a runtime dependency or download more weights as part of this research.

The existing agent output cap of 2048 tokens needs evaluation against this candidate's reasoning. Do not blindly select the model-card maximum context or output budget: measure memory, latency and completed tool/narration turns on the actual GPU. Text-only acceptance comes first; vision projector loading and simultaneous image generation are separate memory/performance checks.

Acceptance corpus: Russian narration; preserving named canon over repeated turns; not revealing unknown secrets; four distinct character identities; typed tool selection and schema-correct arguments; an intentionally rejected action; multi-tool continuation; interrupted stream/retry; a long-history turn; completed answer latency. Compare against the current model on the same cases. Promote only after real app turns and the authored adventure pass, not parameter count or publisher benchmark scores.

Sources checked 2026-10-06:

- https://huggingface.co/prism-ml/Ternary-Bonsai-2-27B-gguf
- https://huggingface.co/prism-ml/Ternary-Bonsai-2-27B-gguf/raw/main/KNOWN_ISSUES.md
- https://github.com/PrismML-Eng/Bonsai-demo/blob/main/MODEL-FORMATS.md
- https://raw.githubusercontent.com/PrismML-Eng/Bonsai-demo/main/BACKEND-SUPPORT.md

## Delivery sequence and acceptance gates

1. Reconcile incomplete existing acceptance work and fix the deterministic test harness. Establish baseline gates without claiming historical results as current.
2. Specify and implement campaign/party ownership and full projection restore, including migration from the current single-PC persistence. Prevent state leakage between campaigns.
3. Convert the reviewed adventure into validated data: scene connections, fact visibility, NPC knowledge, predicates, encounter variants, rewards and localization. Contract tests check required routes and reachable endings.
4. Implement bounded/recoverable AI turn orchestration over server-supplied context and validated tools. Show human-visible errors and retry/continue states; keep tool-debug JSON out of ordinary adventure prose.
5. Complete supported combat, exploration, rest and save/restore paths. Real solo acceptance reaches both peaceful resolution and a resolved defensive encounter, then restarts and resumes.
6. Evaluate Bonsai separately through the existing provider port. Model changes cannot substitute for fixes to campaign state or rules.
7. Add LAN/VPN rooms, participants, authenticated invitations, server-derived character ownership, command ordering, shared event delivery and reconnect snapshots. Guest access must exclude host settings, secrets and runtime-control endpoints.
8. Run two real independent clients: simultaneous submissions, duplicate request, stale revision, guest attempts to control another hero, reconnect during stream/combat, host restart and save restoration. Everyone must converge on the same state without duplicate rewards/damage.

For each implementation checkpoint: focused tests, relevant gates, production build and real browser/Tauri evidence. Full Rust suites, CI, packaging and deployment remain separate claims. No new production dependency or publication is authorized by this research document.

## Next design decisions

- Approve/refine the adventure tone, canon and intended supported level-1 mechanics.
- Fix host/guest transport and participant credential contracts before networking edits.
- Define the durable operation/recovery model and atomic save boundaries.
- Decide whether the first guest is a Tauri client, browser client, or both; browser joining is a proposal, not an agreed requirement.
- Define measurable response-time targets after a bounded Bonsai benchmark.

This document captures exploration and authored content only. It does not mark implementation tasks complete and does not make the adventure selectable in the application.
