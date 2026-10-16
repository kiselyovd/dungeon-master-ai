# Project Base Rules

> Auto-detected conventions from the current codebase. Update this file when project-wide conventions change.

## Naming Conventions

- **TypeScript/React files:** Use `PascalCase.tsx` for components, `useCamelCase.ts` for hooks, and `camelCase.ts` for state, API, library, and utility modules.
- **TypeScript identifiers:** Use `camelCase` for variables and functions, `PascalCase` for components, classes, types, and interfaces, and `SCREAMING_SNAKE_CASE` for true constants.
- **Rust files and identifiers:** Use `snake_case` for modules, files, functions, and variables; use `PascalCase` for structs, enums, traits, and error types.
- **Python files and identifiers:** Use `snake_case` for modules, functions, variables, and pytest tests; use `PascalCase` for classes.
- **Tests:** Use `*.test.ts` / `*.test.tsx` or colocated `__tests__/` for Vitest, `tests/*.rs` and `#[cfg(test)]` modules for Rust, and `test_*.py` for pytest.
- **Styles:** Keep component-specific CSS Modules beside their component; keep shared tokens and application-wide styles under `src/styles/`.

## Module Structure

- `crates/app-domain` contains deterministic D&D rules and domain behavior. It must not depend on Axum, SQLx, Tauri, model providers, or frontend concerns.
- `crates/app-application` owns use cases, typed models and inward-owned ports; its only workspace dependency is `app-domain`.
- `crates/adapter-http`, `adapter-sqlite`, `adapter-llm`, `adapter-media` and `adapter-secrets` translate their respective boundaries without importing sibling adapters.
- `crates/app-bootstrap` owns backend composition and the `dmai-server` entry point. `app-llm` and `app-server` remain compatibility facades, not new homes for policy or concrete wiring.
- `src-tauri` owns native process lifetime, capabilities, desktop integration and native startup credentials.
- `src/app` owns frontend composition; `src/features` owns feature controllers. `src/api`, `src/state`, `src/components` and `src/ui` retain transport, projections, product views and reusable presentation responsibilities.
- `sidecar` is an independent FastAPI process. Keep heavyweight media/model dependencies and backend dispatch inside this boundary.
- Cross-boundary communication must use typed Rust interfaces, validated HTTP/SSE contracts, Tauri events/commands, or explicit frontend schemas.

## Error Handling

- Use stable error codes or typed errors for failures that cross a boundary; keep user-visible localization separate from machine contracts.
- In TypeScript, normalize unknown failures at API boundaries and preserve the original `cause` where useful. Do not silently convert failures into success-like defaults.
- In Rust, use specific `thiserror` enums for expected domain, provider, and boundary failures. Use `anyhow` for outer application context, not as a replacement for domain error types.
- In FastAPI, return deliberate HTTP errors for invalid requests or unavailable backends and let unexpected failures remain observable.
- Log failures with operational context, but never include credentials, Stronghold contents, or unnecessary private campaign/model payloads.

## Control Flow

- Prefer flat, readable control flow over deeply nested conditionals. Use guard clauses, early `return`/`continue`, small named helper methods, or explicit classification logic when they make the code easier to follow. Handle edge cases and irrelevant branches early so the main path stays visible.
- Keep deterministic rules pure where possible; move I/O and orchestration to the system boundary that owns it.
- Make degraded and unavailable runtime states explicit. Do not report a model, sidecar, save, or tool call as ready or successful without evidence.

## Logging

- Use structured `tracing` fields in Rust server code and the existing `log` facade in the Tauri shell.
- Use `console.warn` and `console.error` sparingly at frontend recovery boundaries; avoid routine `console.log` in production UI code.
- CLI and build scripts may write concise progress and machine-readable results to stdout/stderr.
- The Python sidecar may use Uvicorn/FastAPI logging and startup status output; keep protocol markers stable for process launchers.

## Testing and Verification

- Frontend behavior uses Vitest, Testing Library, and user-visible assertions; browser flows use Playwright or the Tauri CDP scripts.
- Rust domain behavior uses focused unit/integration tests; route, persistence, and wire contracts use `app-server` integration tests.
- Python sidecar tests inject stub backends by default. GPU/model smoke tests must stay explicitly separable from the offline suite.
- Run the narrowest relevant tests during iteration, then `bun run gates` for consolidated release confidence. Run `bun run e2e` or the appropriate Tauri smoke path for visible UI or desktop integration changes.
- A successful focused test is not evidence that the full gate, browser application, Tauri runtime, packaging, or CI passed.

## Repository Operations

- Use the `gh` CLI for GitHub repository, pull request, workflow, and release operations. Do not add or depend on GitHub MCP for this project.
- Preserve unrelated worktree changes. Stage or modify only files owned by the active task.
- Run shell operations as separate commands when failure handling matters; do not hide intermediate failures in compound command chains.
