//! Local LLM manifest types shared by the backend HTTP surface and any future
//! domain consumers (e.g. HF search adapters in Tasks 15-19).
//!
//! These types intentionally live in `app-domain` (not `app-server`) so the
//! same struct definitions can be reused by other crates without pulling in
//! axum / sqlx. The wire shape mirrors the frontend
//! `src/state/local_llm/manifest.ts` `SystemEntry` / `UserEntry`.

pub mod manifest;
