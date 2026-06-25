## Summary

<!-- What does this PR change, and why? One or two sentences is fine. -->

## Type of change

- [ ] feat (new user-facing capability)
- [ ] fix (bug fix)
- [ ] docs (documentation only)
- [ ] refactor (no behaviour change)
- [ ] chore (tooling, deps, CI, config)
- [ ] test (adds or improves tests)

## Related issues

<!-- e.g. Closes #123, Relates to #456. -->

## Test plan

<!-- How was this verified? Note manual steps and which suites you ran. -->

- [ ] `bun run gates` passes locally (cargo fmt, clippy, biome ci, tsc, cargo test, vitest, em-dash)
- [ ] Manual verification (describe):

## Checklist

- [ ] Gates pass (`bun run gates`, or `bun run gates:fast` for docs-only)
- [ ] Docs updated if behaviour or setup changed (README / ARCHITECTURE / CHANGELOG)
- [ ] No em-dash or en-dash anywhere (plain hyphen `-` only); the em-dash gate is blocking
