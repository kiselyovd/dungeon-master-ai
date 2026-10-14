# Visual performance evidence

These are local Windows measurements for the active product UI implementation,
not release or hardware GPU acceptance. The repeatable QA scripts and raw rows
are under `.artifacts/product-uiux/task-43/`.

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

## Remaining acceptance

The hardware repetition proves this measured browser workload, with concurrent
Cargo/frontend load. Real GPU/model contention, repeated scene/actor transitions,
and packaged offline local/cloud runtime checks remain open in Task 43. The
checkpoint comparison also does not reconstruct an unrecorded twenty-actor
baseline from before the full UI/art implementation. No measurement here is a
hard process/GPU memory cap or a guarantee against every long frame.
