# Visual performance evidence

These are local Windows measurements for the active product UI implementation,
not release or hardware GPU acceptance. The repeatable QA scripts and raw rows
are under `.artifacts/product-uiux/task-43/`.

The tested builds include source checkpoint `5f1edc9` and the preserved inherited
worktree changes recorded in `checkpoint-12-dirty-baseline.json`. Package hashes
and the four-file portable manifest are in `task-43-package/package-manifest.json`.

## Shipping art

The production frontend contains 21,615,012 encoded art/font bytes (20.614 MiB).
Adding every backend bundled raster, conservatively counting duplicates, gives
27.915 MiB against the 32 MiB target. `bun run assets:check` verifies hashes,
dimensions, provenance, selected decorative consumers and per-category limits
for the 145 catalog entries. One hundred selected version-two masters have
image-generation provenance. Source production artifacts are outside packaged
frontend imports. An installer/runtime check is still required separately.

## Comparable production builds

On 2026-10-14, the production frontend rebuilt from commit `6345b9a` was compared
with the current frontend on this same computer. Five fresh browser contexts per
build alternated baseline/current order. Both rendered the actual production
root with the same validated twenty-actor wire fixture, at 1920 × 1080 and DPR 2.
Each sample included a real pointer pan and 180 animation-frame intervals.
Cargo builds and the frontend suite were running concurrently.

| Metric | Checkpoint baseline | Current frontend |
|---|---:|---:|
| Median time until table and actors are ready | 730.2 ms | 759.3 ms |
| Median frame interval | 16.7 ms | 16.7 ms |
| Median sample p95 frame interval | 16.7 ms | 16.8 ms |
| Frame intervals over 50 ms across 900 samples | 11 | 13 |
| Unique fetched raster dimensions × four | 38,367,488 bytes | 38,367,488 bytes |

The measured ready/frame/p95 regressions were +4.0%/0%/+0.6%, within the 10%
comparison target. The 36.59 MiB raster estimate bounds the unique loaded art in
this fixture and is below 96 MiB. It does not measure decoder caches, GPU texture
allocation, browser process memory or fonts. Asset sharing/release for owned
images is covered separately by `useMediaImage` tests: one URL per shared scope,
revocation after the last consumer, abort on scope departure, and no delivery of
a late result from an old scope. Current tokens and map artwork are DOM/SVG
images; the Pixi application renders tactical geometry.

## Hardware GPU repetition

The initial browser reported ANGLE SwiftShader. A separate repeat used Chromium's
documented [`--enable-gpu`](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/docs/gpu/using-gpu-hardware-in-headless-chrome.md)
and Windows [`--use-angle=d3d11`](https://chromium.googlesource.com/angle/angle/+/refs/heads/main/doc/DebuggingTips.md)
options. Every one of the ten contexts had to report NVIDIA GeForce RTX 3080;
software fallback fails this hardware check. The actual native WebView separately
reported the same RTX 3080/D3D11 renderer.

With the identical twenty-actor fixture and sampling protocol, baseline/current
ready medians were 594.0/582.9 ms, and frame medians/p95 were 16.7/16.7 ms in both
builds. Each build had zero intervals over 50 ms across its 900 sampled frames.
Loaded raster dimensions remained 38,367,488 bytes. Raw hardware rows are in
`comparative-hardware-proof.json`; the software results above are preserved.

## Repeated transitions and offline packages

The cloud QA NSIS installer is 47,732,283 bytes. Its extracted release executable
and backend booted with external HTTP/HTTPS denied for the QA processes. All
166 packaged art/font files matched the frozen frontend SHA-256 values:
21,615,012 bytes, including 144 successfully decoded rasters. Twenty fonts and
two static SVGs account for the remaining files. The native frontend retained
the same index hash as the current production build.

Ten real cloud-package campaign switches checked different actors, owned
portraits and canonical scene paintings. Every visible portrait matched its
source Blob SHA-256, and retired object URLs were revoked. A longer forty-switch
local-package run created 41 URLs, revoked 40, and retained only the current one.
Both canonical campaign projections stayed unchanged. After warm-up at fourteen
switches, comparable game-view snapshots had 391 DOM nodes both times and JS heap
9.78/10.53 MiB at fourteen/forty switches. These are snapshots, not a hard memory
cap or proof about every browser cache. Ordinary QA preferences contained no
Blob URLs or encoded image bytes. Cold exit removed all nine captured cloud
package processes before its returning-player fixture was prepared.

The local QA portable directory includes the real backend, mistralrs executable
and a 3,238,260,781-byte Python media executable. The media build uses Python 3.12.12,
the exact diffusers lock commit and matching declared dependency versions, plus
the documented official Nunchaku CUDA 12.8/PyTorch 2.8 wheel. Ruff and offline pytest
passed 23 cases with five GPU cases skipped. The packaged media executable itself
served health/capabilities/backends offline from an empty weights directory;
its bootloader and worker exited after the smoke test. The mistralrs executable
passed its real help invocation. The local Tauri release also booted offline and
matched all 166 art/font files. These QA packages were not installed or published.

## Concurrent model generation

An additional repetition ran while the installed Qwen3.5-9B Q4_K_M generated
2,048 tokens in 39.7 seconds. Four matched baseline/current sample pairs fit
entirely inside that generation interval. For those pairs, ready medians were
785.4/659.2 ms, frame medians were 16.7 ms in both builds, p95 medians were
16.7/16.8 ms, and each build had zero intervals over 50 ms across 720 frames.
The separate GPU snapshot observed 94% combined utilization. Raw timestamps,
all five original samples and the four matched pairs are retained in
`comparative-model-contention-proof.json`. This is concurrent rendering/inference
evidence; the synthetic load prompt does not prove game semantics. The owned
model server was stopped after the test to release its video memory.

## Remaining acceptance

The hardware repetition proves this measured browser workload. Managed media
startup through the app requires an installed compatible managed LLM first.
The empty local QA profile correctly refused its missing curated Qwen weights
with HTTP 400 and left both runtimes off. The approved LM Studio GGUF used for
the separate recovery test differs from the curated download hash and was not
relabelled as installed. Full managed inference, image/video generation and
whole-adventure model semantics therefore remain Task44 acceptance items.
The checkpoint comparison does not reconstruct an unrecorded twenty-actor
baseline from before the full UI/art implementation. No measurement here is a
hard process/GPU memory cap or a guarantee against every long frame.
