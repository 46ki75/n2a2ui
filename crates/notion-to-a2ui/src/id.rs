//! Deterministic id generation for synthesized A2UI components.
//!
//! Notion blocks already carry stable UUIDs that we reuse as component ids.
//! For components synthesized by the converter (the page root, the
//! `RichText` / `LinkText` / `Icon` runs inside a Notion rich-text array,
//! per-row table cells, etc.) we mint deterministic ids derived from the
//! parent id, a slot label, and an index — so the same Notion page always
//! produces the same A2UI ids and diffs stay stable.

/// The id used for the synthesized page-level root container.
pub const ROOT_ID: &str = "root";

/// Mints a deterministic child id from a parent id, a slot label, and an
/// index within that slot.
pub fn child_id(parent: &str, slot: &str, index: usize) -> String {
    format!("{parent}::{slot}/{index}")
}
