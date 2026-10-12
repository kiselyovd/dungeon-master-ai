# Model readiness

Settings first shows the selected model, whether its files are installed, the
confirmed process state, and Start/Stop/Check actions. The quick local-mode
window reuses this same controller and view. No download starts merely by opening
settings or choosing a provider. A current installation check is required before
Start; an unavailable catalog offers Retry.

The local status response includes the native owner's loaded model ID. A running
port alone does not establish that the selected model is running. Failed status
reads invalidate the claim while preserving the last snapshot for recovery. A
five-second request bound prevents a hung poll from remaining active forever.
Cold startup restores the confirmed preset selection and context preference;
it does not choose or download another model. Model selection and settings Save
are gated during solo and room turns. The backend's full model/environment guard
remains authoritative for inference.

Advanced sections hold memory strategy, optional image model, context limit,
reasoning controls, Dungeon Master instructions and agent limits. Context accepts
integer values from 512 through 65536, defaulting to 8192. Starting or restarting
passes the actual `--max-seq-len` flag supported by the installed Mistral binary.
The Start action stops an existing process before applying changed arguments.
Larger limits consume more memory; a context preference does not prove successful
loading or inference.

Starting chat requests a media process only for enabled providers that declare a
native runtime requirement. Local adapters and their retry/fallback wrappers
preserve that requirement. Bundled and cloud providers do not require it. Optional
images/video do not block campaign preparation or owned offline art.

Cloud forms contain the server address, credential and model selection. Check
connection requests the existing model-list endpoint; a curated fallback is
identified as a catalog, not as a confirmed provider connection. Editing the
draft invalidates older checks, and late replies cannot replace a newer catalog.
Provider response bodies and errors are masked in the ordinary UI. Credentials
remain in the existing private transport and Stronghold flow.

The current preset start bridge supports its registered preset IDs. Other HF
download records remain available in download management; they are not displayed
as working preset choices. Complete custom-model native setup and successful
model-led play remain separate acceptance work in the existing mechanics plans.

## Verification

Frontend regressions cover unavailable/malformed status, stale loaded identity,
late checks, cloud fallback/error masking, advanced controls and room turn gates.
Real HTTP/native-port tests verify bounded context arguments and media start flags
without downloading weights or spawning inference. Production-browser tests cover
RU/EN at 960x640, status retry, context configuration and existing Save/Cancel.
Fresh Tauri evidence for entry, scoped hero, cold restoration and these settings
is recorded in the grouped Tasks 12-15 checkpoint. None of these fixture checks
establishes semantic inference quality or physical LAN acceptance.
