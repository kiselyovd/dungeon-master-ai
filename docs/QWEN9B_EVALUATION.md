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
