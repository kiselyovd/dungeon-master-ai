# Contributing to Dungeon Master AI

This guide covers development environments, quality gates and contribution boundaries. For the system design, read [ARCHITECTURE.md](ARCHITECTURE.md); for application use, start with [Getting started](docs/GETTING_STARTED.md).

## Prerequisites

- Rust 1.94 or newer, matching `Cargo.toml` and the locked dependencies.
- Bun 1.3+ for package management/scripts, and a Node runtime compatible with the locked frontend tools. Use `bun.lock` without an incidental dependency upgrade.
- Python 3.12 for the sidecar's independent offline checks.
- For native desktop work, the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for that target OS in an approved native environment or CI runner.
- An OpenAI-compatible endpoint and account key only when performing a real-provider acceptance run. Fixture checks do not require a paid provider.

## Container-first setup

Use Docker/dev containers for dependency installation, code generation, builds, lint, tests and browser automation. Reuse a compatible approved environment. The repository currently has no project Dockerfile or dev-container definition, so record the toolchain/image recipe used by the task rather than implying that `docker compose up` is available.

Keep `node_modules`, Python environments, Cargo targets and disposable caches inside the container or task-owned named volumes. Source can be mounted from the checkout; dependency trees must not be created on that host bind mount. Install from the project's manifests and lockfiles. For example, in the approved container workspace with a separate dependency mount:

```bash
bun install --frozen-lockfile
```

Do not install or change host runtimes/dependencies as a fallback. Record ownership and purpose for task-created containers, volumes, networks and large scratch outputs; remove disposable task resources when their consumers finish. Preserve other sessions' resources, model stores, keys, databases and release outputs. Check affected free space before allocating a build/download of 1 GiB or more and retain the workspace's 20 GiB reserve.

`bun run install-hooks` installs fast pre-commit and full pre-push gates. Use it only when those hooks execute through the approved toolchain environment. The hooks do not replace CI or browser/native acceptance.

## Browser and native desktop development

The frontend dev server and browser automation can run in a container. Production-browser verification builds the frontend first and sets the preview flag; plain `bun run e2e` starts the development server:

```bash
bun run build
CI=1 DMAI_E2E_PREVIEW=1 DMAI_E2E_PORT=15165 bun run e2e -- --workers=1
```

Use a free task-owned port. Browser fixtures exercise the real React root with controlled transport; they do not launch Tauri or prove a real model/network session.

A target-OS GUI, WebView, native signing or GPU requirement may need a native environment. Before running outside containers, obtain explicit approval for the concrete commands, expected disk use and cleanup. Preserve existing host sessions. Once that environment is approved, the native source entry is:

```bash
bun run tauri dev
```

Tauri builds/stages and owns the game backend. Standard cloud builds do not include embedded model/media runtimes. Optional local setup is a separate, potentially large native task:

```bash
bun run setup:local
bun run setup:local --cuda
bun run dev:all
```

`setup:local` builds mistralrs and creates a Python environment; it must not be run on the host without that separate approval. `dev:all` preflights the optional artifacts before starting Tauri. Production backend vault credentials and provider configuration are described in [Configuration](docs/CONFIGURATION.md). Windows x64, macOS arm64 and Linux x64 native cloud bundles are built on their corresponding CI runners; a container frontend build is not evidence for all three packages.

## Quality gates

`scripts/gates.sh` owns the consolidated source checks and is used by hooks and CI lint. Run the following in the approved container/toolchain environment. Coverage, sidecar, security, browser and native bundle jobs have additional independent checks in `.github/workflows/`.

```bash
bun run gates
bun run gates:fast
bun run test -- --coverage
```

What the gates check:

- `cargo fmt --all --check` - Rust formatting.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` - Rust lints (warnings are errors).
- `biome ci .` - TypeScript / frontend lint and format.
- `bun run architecture:check` - strict Rust workspace edges and frontend import boundaries.
- `bun run assets:check` - versioned visual assets and declared raster metadata.
- `bun run typecheck` (`tsc --noEmit`) - TypeScript types.
- `cargo test --workspace` and `bun run test` (vitest) - the test suites.
- `scripts/check-no-em-dash.sh` - the em-dash gate (see code style below).

The Python 3.12 sidecar has an independent offline gate. Use the minimal test dependencies from `.github/workflows/test.yml` in its container; no model download or GPU is needed:

```bash
python -m ruff check sidecar
RUN_GPU_TESTS=0 python -m pytest sidecar/tests -q
```

GPU tests remain opt-in with `RUN_GPU_TESTS=1`.

For native changes, run `bun run e2e:tauri` only in the approved actual Tauri/WebView environment. Record focused checks, full gates, CI, production-browser, native packaging and real-provider/LAN results separately. Never reuse a dated QA report as evidence that the current revision passed.

## Code style

- TypeScript / frontend: Biome for both lint and format (`biome.json`). Use `bun run check:fix` to auto-fix.
- Rust: rustfmt for formatting and clippy for lints; clippy warnings are treated as errors.
- Writing: use the plain hyphen `-` only. Do not use the em-dash or en-dash anywhere in code, comments, docs, or commit messages. A dedicated gate fails the build on either character.
- Markdown: do not hard-wrap prose. Keep one line per paragraph and per bullet; there is no fixed column width.

## Tests

- Rust tests live next to each crate under `crates/<crate>/tests/` (integration) and inline `#[cfg(test)]` modules.
- Frontend tests use vitest and live in `__tests__/` folders and `*.test.ts(x)` files under `src/`.
- End-to-end Playwright specs live in `e2e/`.

For architecture changes, keep application/domain types inward, translate at adapter boundaries, and update `scripts/architecture-boundaries.json` only when adding a deliberate workspace crate. Do not add an exception for a forbidden edge.

When adding a feature or fixing a bug, add or update the matching tests so `bun run gates` stays green.

## Commit and PR flow

1. Branch off `main`.
2. Make your change with tests and docs as needed.
3. Run `bun run gates` and confirm it passes.
4. Open a PR; the [pull request template](.github/PULL_REQUEST_TEMPLATE.md) walks you through summary, change type, related issues, test plan, and the checklist.
5. Keep PRs focused. Update `CHANGELOG.md` when the change is user-facing.
