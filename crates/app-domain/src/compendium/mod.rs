//! Structured access to the SRD 5.1 character-creation data shipped in the binary.
//!
//! Unlike the sibling `app_domain::srd` module (which embeds raw SRD prose
//! for LLM retrieval), this module exposes typed records of races, classes,
//! backgrounds, spells, equipment, feats, and weapon properties for use by
//! the character-creation wizard and downstream rules logic.

mod loader;
pub mod types;

pub use loader::{compendium, Compendium};
