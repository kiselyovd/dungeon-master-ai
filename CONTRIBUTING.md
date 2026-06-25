# Contributing to Dungeon Master AI

Thanks for your interest in contributing. This guide covers the local setup, the quality gates, and the conventions the project expects. For the system design, read [ARCHITECTURE.md](ARCHITECTURE.md).

## Prerequisites

- Rust stable: `rustup default stable` (the workspace pins `rust-version = "1.81"`).
- [Bun](https://bun.com/) 1.3+ (package manager and script runner).
- Tauri prerequisites for your OS: https://tauri.app/start/prerequisites/
- For the cloud path: any OpenAI-compatible endpoint plus an API key (OpenRouter recommended).
- For the local path: a built `mistralrs` server binary plus the Python image sidecar (see the prebuild CI workflows and `docs/RELEASE.md`). Models download from HuggingFace on first use.

## Setup

```bash
bun install
bun run install-hooks   # one-time: pre-commit (fast gates) + pre-push (full gates)
```

`install-hooks` wires `scripts/gates.sh` into git: pre-commit runs the fast subset, pre-push runs the full set. You can bypass once with `--no-verify`, but CI runs the same checks, so a bypass only defers the failure.

## Running the app

```bash
bun run tauri dev       # Vite + the Rust backend (dmai-server) - enough for CLOUD mode
```

`bun run tauri dev` builds and spawns the Rust backend automatically. It does NOT build or start the model sidecars (the `mistralrs` LLM and the Python image sidecar); those are on-demand:

```bash
bun run setup:local            # build the mistralrs server + create the image-sidecar .venv
bun run setup:local --cuda     # GPU build (needs the CUDA toolkit / nvcc on PATH)
bun run dev:all                # preflights the sidecars, then runs tauri dev
```

For cloud mode (OpenAI-compatible / OpenRouter) plain `bun run tauri dev` is all you need - no sidecars.

## Quality gates

`scripts/gates.sh` is the single source of truth for the gates. The same script backs the git hooks and the CI `lint` job, so local and CI cannot drift.

```bash
bun run gates        # full: cargo fmt --check, cargo clippy, biome ci, tsc, cargo test, vitest, em-dash
bun run gates:fast   # fast subset: cargo fmt --check, biome ci, tsc, em-dash (no compile)
bun run e2e          # Playwright (its own CI job; not part of gates.sh - needs a browser download)
```

What the gates check:

- `cargo fmt --all --check` - Rust formatting.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` - Rust lints (warnings are errors).
- `biome ci .` - TypeScript / frontend lint and format.
- `bun run typecheck` (`tsc --noEmit`) - TypeScript types.
- `cargo test --workspace` and `bun run test` (vitest) - the test suites.
- `scripts/check-no-em-dash.sh` - the em-dash gate (see code style below).

Run `bun run gates` before opening a PR; it mirrors CI's blocking checks.

## Code style

- TypeScript / frontend: Biome for both lint and format (`biome.json`). Use `bun run check:fix` to auto-fix.
- Rust: rustfmt for formatting and clippy for lints; clippy warnings are treated as errors.
- Writing: use the plain hyphen `-` only. Do not use the em-dash or en-dash anywhere in code, comments, docs, or commit messages. A dedicated gate fails the build on either character.
- Markdown: do not hard-wrap prose. Keep one line per paragraph and per bullet; there is no fixed column width.

## Tests

- Rust tests live next to each crate under `crates/<crate>/tests/` (integration) and inline `#[cfg(test)]` modules.
- Frontend tests use vitest and live in `__tests__/` folders and `*.test.ts(x)` files under `src/`.
- End-to-end Playwright specs live in `e2e/`.

When adding a feature or fixing a bug, add or update the matching tests so `bun run gates` stays green.

## Commit and PR flow

1. Branch off `main`.
2. Make your change with tests and docs as needed.
3. Run `bun run gates` and confirm it passes.
4. Open a PR; the [pull request template](.github/PULL_REQUEST_TEMPLATE.md) walks you through summary, change type, related issues, test plan, and the checklist.
5. Keep PRs focused. Update `CHANGELOG.md` when the change is user-facing.
