# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Workspace layout

Cargo workspace (edition 2024, resolver 3) with two member crates:

- `crates/a2ui` — Rust types for the [A2UI](https://a2ui.dev) v0.9 Elmethis Block Catalog. Pure data model; no I/O. The wire schema is vendored at `crates/a2ui/schemas/v0_9/block_catalog.json`.
- `crates/notion-to-a2ui` — converter that walks a Notion block tree (via `notionrs`) and emits an A2UI `Surface`. Currently a stub: `Client::convert_block` returns an empty `Column` root. The full conversion is being ported in phases.

## Commands

```bash
# Build everything
cargo build

# Run all tests (lib + integration). CI only runs --lib (see below).
cargo test

# What CI runs (.github/workflows/unit-test.yml)
cargo test --lib

# Run one crate's tests
cargo test -p a2ui
cargo test -p notion-to-a2ui

# Run a single test by name
cargo test -p a2ui surface_round_trip_preserves_order

# Lint / format
cargo clippy --all-targets
cargo fmt
```

Note: `crates/a2ui/tests/schema.rs` `include_str!`s the vendored catalog and asserts its `$id` matches `a2ui::v0_9::BLOCK_CATALOG_ID` — if you bump the catalog version, both the URL constant and the vendored file must move together or that test fails.

## Architecture

### A2UI surface model (`a2ui::v0_9`)

A `Surface` is `{ root: ComponentId, components: IndexMap<ComponentId, Component> }` — a flat adjacency list. Parents reference children by id; the `IndexMap` preserves insertion order across serde round-trips (asserted in tests). Use `Surface::insert(component)` rather than touching the map directly — it keys by `component.id()`.

`Component` is a `#[serde(tag = "component")]` tagged union over every block type in the catalog. The discriminator field name (`component`) and per-variant casing are dictated by the v0.9 wire format, so don't rename them. A couple of serde quirks worth knowing before adding fields:

- `Callout::callout_type` serializes as `"type"` (`#[serde(rename = "type")]`).
- `HeadingLevel` round-trips as `u8` 1..=6, not a string.
- All optional fields use `skip_serializing_if = "Option::is_none"` — keep this when adding new fields so the wire stays minimal.
- `DynamicString` / `ChildList` are `#[serde(untagged)]` unions of literal vs. binding vs. template — order of variants matters for deserialization.

Adding a new component variant requires four edits in lockstep: define the struct in `block_catalog.rs`, add it to the `Component` enum, add it to the `component_impls!` macro list (this generates `From<T>` and `Component::id()`), and add a round-trip test in `tests/schema.rs`.

### Notion → A2UI conversion (`notion_to_a2ui`)

`Client` holds a `notionrs::client::Client` and a `reqwest::Client` plus two behavior toggles:

- `enable_unsupported_block` — when false, unknown block types are dropped; when true, they become `Unsupported` components carrying a `details` string.
- `enable_fetch_image_meta` — when true, image blocks are fetched once with `reqwest` + `imagesize` to populate `width`/`height` on `BlockImage`. This adds a network round-trip per image.

**Component id strategy** (`src/id.rs`): Notion block UUIDs are reused verbatim as `ComponentId`s. For components the converter *synthesizes* (the page-level root, the per-run `RichText`/`LinkText`/`Icon` inside a rich-text array, per-row table cells, etc.), ids are minted via `child_id(parent, slot, index)` → `"{parent}::{slot}/{index}"`. This is load-bearing: the same Notion page must always produce the same A2UI ids so downstream diffs stay stable. The synthesized page root uses the constant `ROOT_ID = "root"`.

## Conventions

- PRs target `develop` or `release/*`, not `main` (see `.github/pull_request_template.md`).
- Workspace deps (`serde`, `serde_json`, `indexmap`) are declared once in the root `Cargo.toml` and pulled into members via `{ workspace = true }` — add new shared deps there, not per-crate.
- The `a2ui` crate must stay I/O-free (no `reqwest`, `tokio`, etc.) — it's the pure schema crate consumed by the converter and potentially other producers.
