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
