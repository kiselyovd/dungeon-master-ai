# Campaign Documents and DM Memory Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a dedicated local PDF/DOCX campaign library, access-scoped retrieval, a continuity ledger, and a proactive DM context pipeline without treating documents as chat attachments.

**Architecture:** Application-owned document/memory ports define ingestion, retrieval, and commits. Extraction and embeddings remain outward adapters; SQLite stores versioned provenance and ACL; the agent turn consumes a bounded context bundle and validates all memory proposals.

**Tech Stack:** Rust, SQLx/SQLite, existing embedding adapter, Axum, React, TypeScript, Valibot, Vitest, Playwright.

**Spec:** `docs/superpowers/specs/2026-08-13-campaign-platform-srd52-design.md`

## Global Constraints

- Documents are host-local campaign sources, never chat attachments or P2P payloads.
- Treat extracted text as untrusted data, never system instructions.
- Do not add PDF/DOCX parsing packages until the user explicitly approves the exact production dependencies.
- No source content, excerpts, embeddings, prompts, or filenames with private content in logs.
- Player responses may cite discovered/public provenance but may never serialize `dm_only` records.

---

### Task 1: Define document, knowledge, and continuity contracts

**Files:**
- Create: `crates/app-application/src/models/knowledge.rs`
- Create: `crates/app-application/src/models/continuity.rs`
- Create: `crates/app-application/src/ports/documents.rs`
- Create: `crates/app-application/src/ports/knowledge.rs`
- Create: `crates/app-application/src/campaigns/ingest_document.rs`
- Create: `crates/app-application/src/campaigns/remove_document.rs`
- Modify: `crates/app-application/src/models/mod.rs`
- Modify: `crates/app-application/src/ports/mod.rs`
- Modify: `crates/app-application/src/campaigns/mod.rs`
- Test: `crates/app-application/tests/knowledge_workflow.rs`

**Interfaces:**
- Produces `DocumentId`, `DocumentRevision`, `KnowledgeScope::{Public, Discovered, DmOnly}`, `SourceLocation`, `KnowledgeChunk`, `MemoryProposal`, `ContinuityRecord`.
- Produces ports `DocumentExtractor`, `KnowledgeRepository`, `ContinuityRepository`, `EmbeddingProvider`.

- [ ] **Step 1: Write failing ingestion workflow tests**

Cover an extractor returning two pages, an embedding port returning vectors, and a repository that records nothing until publish:

```rust
let result = use_case.execute(IngestDocumentCommand {
    campaign_id,
    display_name: "atlas.pdf".into(),
    media_type: DocumentMediaType::Pdf,
    bytes: b"fixture".to_vec(),
    scope: KnowledgeScope::DmOnly,
}).await.unwrap();

assert_eq!(result.chunk_count, 2);
assert_eq!(repo.published_revisions(), vec![result.revision]);
assert!(repo.chunks().iter().all(|chunk| chunk.scope == KnowledgeScope::DmOnly));
```

Also assert extraction or embedding error leaves `published_revisions()` empty.

- [ ] **Step 2: Run and verify missing contract failures**

Run: `cargo test -p app-application --test knowledge_workflow`
Expected: FAIL because knowledge contracts are missing.

- [ ] **Step 3: Define bounded extractor input and output**

```rust
#[async_trait]
pub trait DocumentExtractor: Send + Sync {
    async fn extract(&self, source: DocumentSource) -> Result<ExtractedDocument, DocumentError>;
}

pub struct ExtractedSection {
    pub ordinal: u32,
    pub page: Option<u32>,
    pub heading: Option<String>,
    pub text: String,
}
```

Set hard application limits: 20 MiB source bytes, 500 extracted sections, 2 MiB normalized UTF-8 text, 2,000 chunks, and non-empty text. Return stable codes `file_too_large`, `document_encrypted`, `no_extractable_text`, `document_malformed`, and `document_unsupported`.

- [ ] **Step 4: Define atomic publish/remove repository operations**

`publish_revision` receives metadata, all chunks, and embeddings in one call. `remove_document` atomically deletes source metadata, chunks, and index rows but leaves campaign events and continuity facts unchanged.

- [ ] **Step 5: Implement deterministic chunking in the application layer**

Create chunks of at most 1,200 Unicode scalar values with 150-character overlap, preserving section/page provenance. Never split an empty or whitespace-only chunk.

- [ ] **Step 6: Run application tests**

Run: `cargo test -p app-application --test knowledge_workflow`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/app-application/src/models/knowledge.rs crates/app-application/src/models/continuity.rs crates/app-application/src/ports/documents.rs crates/app-application/src/ports/knowledge.rs crates/app-application/src/campaigns/ingest_document.rs crates/app-application/src/campaigns/remove_document.rs crates/app-application/src/campaigns/mod.rs crates/app-application/src/models/mod.rs crates/app-application/src/ports/mod.rs crates/app-application/tests/knowledge_workflow.rs
git commit -m "feat(application): define campaign knowledge workflows"
```

### Task 2: Make and approve the parser dependency decision

**Files:**
- Create: `docs/architecture/document-extraction-decision.md`
- Inspect: `Cargo.lock`
- Inspect: `sidecar/pyproject.toml`
- Inspect: workspace `Cargo.toml` files

**Interfaces:**
- Produces one approved extraction choice before adapter manifests change.

- [ ] **Step 1: Inventory already-locked parser capabilities**

Run:

```text
rg -n "name = \"(lopdf|pdf-extract|pdfium-render|zip|quick-xml|docx-rs|pypdf|python-docx)\"" Cargo.lock sidecar/pyproject.toml
```

Expected: record exact existing versions or record that no usable production parser is present.

- [ ] **Step 2: Write the decision record with two concrete options**

Option R (preferred for process isolation): Rust `lopdf` plus `zip`/`quick-xml` in a new `adapter-documents`; bounded synchronous parsing runs in `spawn_blocking`.

Option P: Python `pypdf` plus `python-docx` in a dedicated extraction endpoint of the existing Python sidecar; this couples document availability to the media process and is not recommended.

The record must compare licenses, binary size, encrypted-file behavior, platform packaging, maintenance status, and offline tests.

- [ ] **Step 3: Request explicit dependency approval**

Stop plan execution and ask the user to approve exact package names and pinned version ranges from the decision record. Do not edit a manifest before approval.

- [ ] **Step 4: Record the approval in the decision document**

Add the date, selected option, packages, versions, and user approval reference. This is a decision record, not a secret-bearing transcript.

- [ ] **Step 5: Commit the approved decision**

```bash
git add docs/architecture/document-extraction-decision.md
git commit -m "docs(architecture): approve document extraction adapter"
```

### Task 3: Implement bounded PDF/DOCX extraction adapter

**Files (Option R after approval):**
- Create: `crates/adapter-documents/Cargo.toml`
- Create: `crates/adapter-documents/src/lib.rs`
- Create: `crates/adapter-documents/src/pdf.rs`
- Create: `crates/adapter-documents/src/docx.rs`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/app-bootstrap/Cargo.toml`
- Test: `crates/adapter-documents/tests/extraction.rs`
- Test fixture: `crates/adapter-documents/tests/fixtures/two-pages.pdf`
- Test fixture: `crates/adapter-documents/tests/fixtures/two-sections.docx`

**Interfaces:**
- Implements `DocumentExtractor` from Task 1.
- Produces normalized `ExtractedDocument` without filesystem or network access.

- [ ] **Step 1: Add minimal synthetic fixture files**

Fixtures contain only project-authored text: page/section one `Moonlit Gate`, page/section two `Silver Valley`. Add malformed, empty-text, and encrypted/unsupported fixtures only when their bytes can be generated deterministically by the selected parser's test tooling.

- [ ] **Step 2: Write extraction and bound tests**

Assert page/section provenance, normalized line endings, no NUL characters, stable error codes, size rejection before parsing, and no panic for 100 random byte slices of increasing length.

- [ ] **Step 3: Run and verify missing implementation**

Run: `cargo test -p adapter-documents`
Expected: FAIL because extraction functions are not implemented.

- [ ] **Step 4: Implement PDF extraction**

Parse only in-memory bytes, reject encryption, visit pages in document order, and emit one section per page. Do not render images or perform OCR.

- [ ] **Step 5: Implement DOCX extraction**

Read `word/document.xml` from the ZIP package, preserve paragraph order and heading style where available, ignore macros/external relationships, and reject ZIP entries whose expanded total exceeds 32 MiB.

- [ ] **Step 6: Run adapter tests and dependency guard**

Run: `cargo test -p adapter-documents && bun run architecture:check && cargo deny check licenses`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock crates/adapter-documents crates/app-bootstrap/Cargo.toml
git commit -m "feat(documents): extract bounded PDF and DOCX text"
```

### Task 4: Persist knowledge revisions and scoped retrieval

**Files:**
- Create: `crates/adapter-sqlite/src/knowledge.rs`
- Create: `crates/adapter-sqlite/src/continuity.rs`
- Modify: `crates/adapter-sqlite/migrations/0001_initial.sql`
- Modify: `crates/adapter-sqlite/src/lib.rs`
- Test: `crates/adapter-sqlite/tests/knowledge_contracts.rs`

**Interfaces:**
- Implements `KnowledgeRepository` and `ContinuityRepository`.
- Produces `search(campaign_id, query_embedding, allowed_scopes, limit)` ordered by score then stable ID.

- [ ] **Step 1: Write atomicity and ACL contract tests**

Insert public, discovered, and DM-only chunks. Assert a player-safe search returns no DM-only IDs or metadata, while a DM search may return all scopes. Force a duplicate revision to prove all rows roll back.

- [ ] **Step 2: Run and verify missing repository implementations**

Run: `cargo test -p adapter-sqlite --test knowledge_contracts`
Expected: FAIL.

- [ ] **Step 3: Add normalized knowledge tables**

Use `campaign_documents`, `knowledge_revisions`, `knowledge_chunks`, `knowledge_embeddings`, and `continuity_records`. Store vectors using the same validated representation as existing SRD embeddings; do not serialize source bytes into SQLite logs.

- [ ] **Step 4: Implement search with scope filtering in SQL before decoding rows**

Never load all scopes and filter in Rust for a player query. Limit candidates and return provenance with each result.

- [ ] **Step 5: Run SQLite suite**

Run: `cargo test -p adapter-sqlite`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/adapter-sqlite/migrations/0001_initial.sql crates/adapter-sqlite/src/knowledge.rs crates/adapter-sqlite/src/continuity.rs crates/adapter-sqlite/src/lib.rs crates/adapter-sqlite/tests/knowledge_contracts.rs
git commit -m "feat(sqlite): persist scoped campaign knowledge"
```

### Task 5: Add campaign document HTTP API and frontend library

**Files:**
- Create: `crates/adapter-http/src/routes/documents.rs`
- Modify: `crates/adapter-http/src/routes/mod.rs`
- Modify: `crates/adapter-http/src/services.rs`
- Modify: `crates/app-server/src/http_services.rs`
- Create: `src/api/contracts/documents.ts`
- Create: `src/api/documents.ts`
- Create: `src/features/campaigns/DocumentLibrary.tsx`
- Create: `src/features/campaigns/DocumentLibrary.module.css`
- Create: `src/features/campaigns/useDocuments.ts`
- Modify: `src/features/campaigns/CampaignCreate.tsx`
- Modify: `src/locales/en/campaigns.json`
- Modify: `src/locales/ru/campaigns.json`
- Test: `crates/adapter-http/tests/document_wire_contract.rs`
- Test: `src/api/__tests__/documents.test.ts`
- Test: `src/features/campaigns/__tests__/DocumentLibrary.test.tsx`

**Interfaces:**
- Produces `GET /campaigns/{id}/documents`, `POST /campaigns/{id}/documents`, `DELETE /campaigns/{id}/documents/{document_id}`.
- Upload accepts bounded `application/pdf` and DOCX MIME bytes; response never echoes bytes or extracted text.

- [ ] **Step 1: Write backend wire tests**

Assert `413` over 20 MiB, `415` wrong MIME, stable extraction error codes, atomic success summary, safe list fields, and delete semantics.

- [ ] **Step 2: Write frontend transport and component tests**

Cover file picker and drop, per-file indexing status, error retry, delete confirmation, scope selection, and the absence of any chat message mutation.

- [ ] **Step 3: Run tests and verify failures**

Run: `cargo test -p adapter-http --test document_wire_contract && bun run test -- src/api/__tests__/documents.test.ts src/features/campaigns/__tests__/DocumentLibrary.test.tsx`
Expected: FAIL.

- [ ] **Step 4: Implement thin HTTP mapping and validated frontend schemas**

Do not log filenames or bodies. Use request IDs and stable codes for safe diagnostics.

- [ ] **Step 5: Implement the library with existing UI primitives/tokens**

The library is a campaign-creation step and later campaign-management screen. It is not a modal attachment composer. Show explicit `strict canon` and `free inspiration` policy, mapping internally to retrieval weights while preserving ACL.

- [ ] **Step 6: Run frontend and HTTP checkpoints**

Run: `cargo test -p adapter-http && bun run lint && bun run typecheck && bun run build && bun run test`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/adapter-http/src/routes/documents.rs crates/adapter-http/src/routes/mod.rs crates/adapter-http/src/services.rs crates/adapter-http/tests/document_wire_contract.rs crates/app-server/src/http_services.rs src/api/contracts/documents.ts src/api/documents.ts src/api/__tests__/documents.test.ts src/features/campaigns/DocumentLibrary.tsx src/features/campaigns/DocumentLibrary.module.css src/features/campaigns/useDocuments.ts src/features/campaigns/CampaignCreate.tsx src/features/campaigns/__tests__/DocumentLibrary.test.tsx src/locales/en/campaigns.json src/locales/ru/campaigns.json
git commit -m "feat(campaigns): add document knowledge library"
```

### Task 6: Build scoped DM context and validated continuity commits

**Files:**
- Create: `crates/app-application/src/agent/context_bundle.rs`
- Create: `crates/app-application/src/agent/memory.rs`
- Modify: `crates/app-application/src/agent/context.rs`
- Modify: `crates/app-application/src/agent/turn.rs`
- Modify: `crates/app-application/src/models/agent.rs`
- Modify: `crates/app-application/src/agent/tools.rs`
- Modify: `crates/app-bootstrap/src/wiring.rs`
- Test: `crates/app-application/tests/dm_memory.rs`
- Test: `crates/adapter-http/tests/agent_wire_contract.rs`

**Interfaces:**
- Produces `DmContextBundle`, `TurnMode`, `MemoryProposal`, and `MemoryCommitResult`.
- Preserves existing SSE event names; memory commits are reflected through existing tool results or a versioned additive event only after frontend schema support is added in the same task.

- [ ] **Step 1: Write context privacy and budget tests**

Assert a DM bundle may include DM-only chunks, a player-safe projection cannot, source ordering is deterministic, and total text obeys a configured character/token estimate budget.

- [ ] **Step 2: Write memory proposal validation tests**

Reject unknown scope, empty fact, invalid source IDs, cross-campaign IDs, duplicate causation, and a proposal contradicting strict source canon without an explicit conflict record.

- [ ] **Step 3: Run and verify failures**

Run: `cargo test -p app-application --test dm_memory`
Expected: FAIL.

- [ ] **Step 4: Implement mode classification and bounded retrieval orchestration**

`exploration`, `social`, `combat`, and `downtime` are application enums. Retrieve SRD, current projection, discovered continuity, source canon, and hidden plans through separate port calls and compose them in a fixed priority order.

- [ ] **Step 5: Implement validated commit after successful turn completion**

Tool/domain events commit first. Memory proposals are validated and appended with causation IDs; a memory failure emits a safe warning/result but does not roll back already-confirmed combat/domain events.

- [ ] **Step 6: Strengthen the DM policy**

Update RU/EN prompt sources so player text is intent, uncertain consequential actions require checks, clocks/NPC agendas advance, free-form action remains possible, and undiscovered content never appears. Keep prompts in `prompts/`, not inline route code.

- [ ] **Step 7: Run application and wire suites**

Run: `cargo test -p app-application && cargo test -p adapter-http --test agent_wire_contract`
Expected: PASS with unchanged existing SSE names and order guarantees.

- [ ] **Step 8: Commit**

```bash
git add crates/app-application/src/agent/context_bundle.rs crates/app-application/src/agent/memory.rs crates/app-application/src/agent/context.rs crates/app-application/src/agent/turn.rs crates/app-application/src/agent/tools.rs crates/app-application/src/models/agent.rs crates/app-application/tests/dm_memory.rs crates/app-bootstrap/src/wiring.rs crates/adapter-http/tests/agent_wire_contract.rs prompts/dm_system.en.txt prompts/dm_system.ru.txt
git commit -m "feat(dm): add scoped campaign memory and continuity"
```

### Task 7: Prepare the first playable world and create the initial autosave

**Files:**
- Create: `crates/app-application/src/campaigns/preparation.rs`
- Create: `crates/app-application/tests/world_preparation.rs`
- Modify: `crates/app-application/src/campaigns/mod.rs`
- Modify: `crates/app-application/src/models/campaign.rs`
- Modify: `crates/app-application/src/ports/events.rs`
- Modify: `crates/app-application/src/ports/repositories.rs`
- Modify: `crates/adapter-http/src/routes/campaigns.rs`
- Create: `crates/adapter-http/tests/campaign_routes.rs`
- Create: `src/api/contracts/preparation.ts`
- Create: `src/api/preparation.ts`
- Create: `src/api/__tests__/preparation.test.ts`
- Create: `src/features/campaigns/WorldPreparation.tsx`
- Create: `src/features/campaigns/WorldPreparation.module.css`
- Create: `src/features/campaigns/model/useWorldPreparation.ts`
- Create: `src/features/campaigns/__tests__/WorldPreparation.test.tsx`
- Modify: `src/features/campaigns/CampaignCreate.tsx`
- Modify: `src/locales/en/campaigns.json`
- Modify: `src/locales/ru/campaigns.json`

**Interfaces:**
- Produces `PrepareWorldCommand`, `PreparationEvent`, `PreparedWorld`, and `PreparationFailure`.
- The first confirmed preparation commits campaign bible, opening scene, map specification, NPC seeds, clocks, character anchors, and an autosave at the resulting confirmed campaign revision in one application transaction.

- [ ] **Step 1: Write deterministic preparation tests**

Use a scripted LLM port. Assert progress event order, campaign/source scope, schema validation, generated opening choices, a complete map spec, and exactly one initial autosave. Assert malformed LLM output and cancellation leave no half-prepared campaign state.

- [ ] **Step 2: Write the exact SSE wire contract**

Cover `preparation_started`, `preparation_progress`, `preparation_completed`, and `preparation_failed`. Payloads carry campaign ID, stage enum, monotonic sequence, safe message code, and final revision; they never carry hidden prompt text or document bytes.

- [ ] **Step 3: Run and verify failures**

Run: `cargo test -p app-application --test world_preparation && cargo test -p adapter-http --test campaign_routes`
Expected: FAIL because the preparation use case and stream are absent.

- [ ] **Step 4: Implement preparation as one application use case**

Build the context from campaign settings, character, scoped source chunks, and SRD. Ask the LLM for typed sections, validate each section, accumulate them outside persistence, then commit one `WorldPrepared` event and the initial snapshot transactionally. A retry reuses the command causation ID and cannot create duplicate events or saves.

- [ ] **Step 5: Implement the preparation screen**

Show existing-token progress states for rules, source, world, map, NPCs, and opening scene. The user may cancel safely or retry a failed stage. Navigate to `game` only after `preparation_completed`; never mount the VTT beneath the preparation screen.

- [ ] **Step 6: Add frontend protocol and UI tests**

Cover fragmented SSE frames, duplicate/out-of-order sequence rejection, cancellation, retry, localized errors, unmount cleanup, and successful route transition.

- [ ] **Step 7: Run focused and production-root gates**

Run: `cargo test -p app-application --test world_preparation && cargo test -p adapter-http --test campaign_routes && bun run test -- src/api/__tests__/preparation.test.ts src/features/campaigns/__tests__/WorldPreparation.test.tsx src/app/__tests__/productionRoot.test.tsx && bun run lint && bun run typecheck && bun run build`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/app-application/src/campaigns/preparation.rs crates/app-application/src/campaigns/mod.rs crates/app-application/src/models/campaign.rs crates/app-application/src/ports/events.rs crates/app-application/src/ports/repositories.rs crates/app-application/tests/world_preparation.rs crates/adapter-http/src/routes/campaigns.rs crates/adapter-http/tests/campaign_routes.rs src/api/contracts/preparation.ts src/api/preparation.ts src/api/__tests__/preparation.test.ts src/features/campaigns/WorldPreparation.tsx src/features/campaigns/WorldPreparation.module.css src/features/campaigns/model/useWorldPreparation.ts src/features/campaigns/CampaignCreate.tsx src/features/campaigns/__tests__/WorldPreparation.test.tsx src/locales/en/campaigns.json src/locales/ru/campaigns.json
git commit -m "feat(campaigns): prepare the first playable world"
```

### Task 8: Phase acceptance

**Files:**
- Create: `e2e/campaign-documents.spec.ts`
- Modify: `e2e/full-campaign.spec.ts`

**Interfaces:**
- Proves document ingestion is separate from chat and the DM uses discovered context without leaking DM-only content.

- [ ] **Step 1: Add deterministic browser fixtures and tests**

Use project-authored minimal PDF/DOCX fixtures. Test success, malformed file, removal, strict scope, no attachment bubble, and safe visible provenance.

- [ ] **Step 2: Run full local gates**

Run: `bun run architecture:check && bun run gates && bun run e2e`
Expected: PASS.

- [ ] **Step 3: Run real Tauri campaign-document smoke**

Start `bun run dev:all`, then run `bun run e2e:tauri` with the deterministic document fixture path supplied by the script configuration.

Expected: campaign creation indexes the document, reaches the game table, and the first DM turn uses fixture lore without showing a raw document in chat.

- [ ] **Step 4: Commit acceptance tests**

```bash
git add e2e/campaign-documents.spec.ts e2e/full-campaign.spec.ts
git commit -m "test(dm): cover campaign documents and memory"
```
