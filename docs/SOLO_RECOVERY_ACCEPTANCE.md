# Native solo recovery acceptance

Verified on 2026-10-08 in the attached solo-coop-playability worktree, source
checkpoint9244286, with inherited workspace changes preserved. Local debug Tauri/WebView runs the actual backend; this is not
a packaged release or physical LAN test. The QA profile uses a distinct native
identifier, SQLite file, vault and WebView directory under .artifacts/solo-native.

## Observed path

Initial onboarding/manual provider configuration, bundled Bell campaign creation
and the fighter/Human/Soldier character wizard were driven through real controls.
The committed human hero is Daren, HP11/max11. For this recovery test, a real
character API edit deliberately starts the encounter injured at HP1. A generic
NPC encounter is created through the actual combat API; exactly one human token
is bound to the durable character. The combat-start response is SSE: an initial
harness JSON parse failed after commit, so the existing encounter was read back
rather than recreated.

The player presses EndTurn in the native UI. NPC damage drops the hero to zero;
the generic encounter ends without authored rescue. The actor remains alive and
already has one successful mandatory combat save. Rest UI exposes the owned save.

| Step | Campaign revision | HP | Game seconds |
| --- | ---: | ---: | ---: |
| Battle finished, dying | 5 | 0 | 12 |
| Native save button, die12 | 6 | 0 | 18 |
| Native save button, die10, stable | 7 | 0 | 24 |
| Native Wait + confirmation | 8 | 1 | 7224 |
| Native LongRest + confirmation | 9 | 11 | 36024 |

The independent SQLite audit found only two character commits, HP11 then HP1,
zero snapshots/restores/co-op rooms, an ended encounter with zero-HP living hero,
three solo receipts (two saves and Wait), and the LongRest receipt. Visible dice
12/10 match durable results. There is no health injection or snapshot workaround
after encounter creation. Recovery helpers only read state and click controls.

Full app/backend closure and reopening retain character identity, HP11 and the
16-hour LongRest cooldown. The new backend uses another private port. The reviewer
independently inspected the DB, scripts/screenshots and restart artifact; it did
not personally observe the first closure. Final cleanup was independently verified:
known app/backend/helper PIDs and listeners1438/9240/15440/49569/55466 are absent.

## Remaining defects found during the run

- Startup provider replay: after reopening, frontend remembers gpt-4o and the QA
  fixture URL, but /providers reports backend mock with empty default model. The
  actual DM request fails. Saving unchanged settings in the real SettingsModal
  installs openai-compat/gpt-4o and a subsequent real native/backend request succeeds.
  This identifies a missing startup synchronization path; the manual save is only
  a diagnostic workaround. Automatic readiness is not fixed or accepted yet.
- Armor: the wizard class package includes Chain Mail, while the committed/live
  AC remains12 from unarmored10+DEX. computeLiveSheet explicitly defers armor.
- Finished VTT: after recovery/rest, canonical hero and HUD are HP11, but the old
  ended encounter token remains HP0 with a skull until restart. Dying/dead display
  and finished-encounter presentation require correction.

The model response comes from a deterministic local HTTP fixture behind the real
provider adapter. One request reached it after manual resave; private reasoning
sentinel is absent from native UI and transcript. This proves transport recovery,
not quality of Bonsai or another real model. Natural encounter balance, full-story
playthrough, broader session UX and physical-device LAN remain separate acceptance.

## Local evidence

Artifacts under .artifacts/solo-native: battle-start.json, battle-proof.json,
recovered-proof.json, rest-proof.json, durable-proof.json, restart-proof.json,
config-diagnosis.json, config-after-resave.json, dm-after-resave-proof.json and
cleanup-proof.json, with native screenshots and build/process logs. These are QA
artifacts, not tracked production inputs. No new production dependency, release,
push or deployment was introduced. Whole-goal completion remains unproven.

## Provider startup correction

The native restart gap is fixed: production bootstrap restores hydrated provider
settings to the current backend, and the first solo DM request awaits that same
acknowledgement. FIFO/immutable requests, process invalidation, timeout and abort
handling protect it from late responses and a stuck setup request. A pre-request
failure retries as a new turn, retaining its nonce without an invalid resume flag.

Two fresh native launches automatically configure the saved provider and commit
new UI question/reply pairs without Settings Save. The fixture proves actual
transport; embedded cold-model startup, Bonsai quality and physical LAN remain
separate. See PROVIDER_STARTUP.md for evidence and limits. Armor and ended VTT
presentation are the next verified defects to repair.
