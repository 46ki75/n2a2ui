# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Workspace layout

Cargo workspace (edition 2024, resolver 3) with two member crates:

- `crates/a2ui` — Rust types for the [A2UI](https://a2ui.dev) v0.9 Elmethis Block Catalog. Pure data model; no I/O. The wire schema is vendored at `crates/a2ui/schemas/v0_9/block_catalog.json`.
- `crates/n2a2ui` — converter that walks a Notion block tree (via `notionrs`) and emits an A2UI `Surface` (or the v0.9 message sequence that renders it).

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
cargo test -p n2a2ui

# Run a single test by name
cargo test -p a2ui surface_round_trip_preserves_order

# Live integration test (skipped without env vars; .env at workspace root works)
NOTION_API_KEY=... BLOCK_ID=... \
  cargo test -p n2a2ui --test convert_block -- --nocapture

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
- `ContentTab` uses `label: ChildList` and `content: ChildList` (singular, both ChildList). Older shapes with `title`/`labels[]`/`contents[]` are gone — keep the rename in sync with `@elmethis/core`'s `ContentTabApi`.

Adding a new component variant requires four edits in lockstep: define the struct in `block_catalog.rs`, add it to the `Component` enum, add it to the `component_impls!` macro list (this generates `From<T>` and `Component::id()`), and add a round-trip test in `tests/schema.rs`.

### v0.9 message envelope (`a2ui::v0_9::message`)

`Message { version, #[serde(flatten)] body: MessageBody }` serializes as `{"version":"v0.9","createSurface":{...}}` — the v0.9 wire shape uses the body key as the discriminator (externally-tagged enum, camelCase). Body variants: `CreateSurface`, `UpdateComponents`, `UpdateDataModel`, `DeleteSurface`. `Surface::to_messages(surface_id, catalog_id)` emits the canonical `[createSurface, updateComponents]` pair that renders the whole surface in one round-trip.

### Notion → A2UI conversion (`n2a2ui`)

`Client` holds a `notionrs::client::Client` and a `reqwest::Client` plus two behavior toggles:

- `enable_unsupported_block` — when false, unknown block types are dropped; when true, they become `Unsupported` components carrying a `details` string.
- `enable_fetch_image_meta` — when true, image blocks are fetched once with `reqwest` + `imagesize` to populate `width`/`height` on `BlockImage`. This adds a network round-trip per image.

Three entry points, layered around the same `Converter` core:

- `Client::convert_block(block_id) -> Surface` — eager, returns the whole adjacency-list surface in one shot.
- `Client::convert_block_stream(block_id, surface_id) -> impl Stream<Item = Result<Message, Error>>` — the streaming primitive. Emits `createSurface`, then an `updateComponents` carrying an empty root `Column` (so the surface mounts immediately and the all-skipped case still produces a valid root), then one `updateComponents` per top-level sibling group. Each chunk carries that group's synthesized components plus an updated root `Column` whose `children` array grows by one id. `updateComponents` is upsert-by-id (v0.9 spec §`updateComponents` / Adjacency List), so re-sending the root is cheap and replay-safe; the empty-root mount is required because v0.9 buffers all updates until `root` exists.
- `Client::convert_block_to_messages(block_id, surface_id) -> Vec<Message>` — defined as `convert_block_stream(...).try_collect().await`. Eager wire shape is therefore `createSurface + updateComponents(empty root) + N × updateComponents(chunk)`, **not** the 2-message pair `Surface::to_messages` produces from a finished `Surface`. Callers asserting `messages.len() == 2` were relying on the prior shape and will need updating.

Conversion lives in `src/convert/`. The eager `Converter::convert_children` walks each level and groups consecutive `bulleted_list_item` / `numbered_list_item` / `to_do` siblings into a single `List` (group id `format!("{first_item_id}::list")`) so the wire shape matches A2UI's nested-list model rather than Notion's flat sibling layout. The streaming path replays the same grouping in the orchestrator (see `client::convert_block_stream`) by calling `Converter::convert_single_block_to_chunk` or `convert_list_group_to_chunk` per group — each owns a fresh bag, delegates to the existing per-block primitives, and returns the chunk's `(id, components)`. To-do checkboxes render via a synthesized `RichText` prefix child (`☐` / `☑`).

**Component id strategy** (`src/id.rs`): Notion block UUIDs are reused verbatim as `ComponentId`s. For components the converter _synthesizes_ (the page-level root, the per-run `RichText`/`LinkText`/`Icon` inside a rich-text array, per-row table cells, callout/page leading icons, list-group wrappers, to-do prefix markers, etc.), ids are minted via `child_id(parent, slot, index)` → `"{parent}::{slot}/{index}"`. This is load-bearing: the same Notion page must always produce the same A2UI ids so downstream diffs stay stable. The synthesized page root uses the constant `ROOT_ID = "root"`.

**Inline icon mapping** (`src/convert/rich_text.rs` + `inline_icon_component` in `src/convert/mod.rs`):

- A `Mention::CustomEmoji` rich-text run becomes an A2UI `Icon { src = custom_emoji.url, alt = name }` instead of a plain `RichText`. This applies inside every container that uses `convert_rich_texts` (paragraph, heading, quote, callout, toggle summary, list item, table cell, content-tab label).
- A callout's leading `block.icon` is rendered as a synthesized child prepended to the callout's children — `Emoji` → `RichText` carrying the Unicode glyph, `CustomEmoji` / `File` → `Icon` with the URL. The same icon still feeds `callout_type_from_icon` for the `CalloutType` hint.

**Tab → ContentTabs mapping**: one Notion `tab` block becomes one `ContentTabs`; each child paragraph becomes one `ContentTab` where the paragraph's `rich_text` is the `label` ChildList and its children are the `content` ChildList.

## Conventions

- PRs target `develop` or `release/*`, not `main` (see `.github/pull_request_template.md`).
- Workspace deps (`serde`, `serde_json`, `indexmap`, `async-stream`) are declared once in the root `Cargo.toml` and pulled into members via `{ workspace = true }` — add new shared deps there, not per-crate.
- The `a2ui` crate must stay I/O-free (no `reqwest`, `tokio`, etc.) — it's the pure schema crate consumed by the converter and potentially other producers.
- When changing a component's schema in `crates/a2ui`, mirror the same change in the upstream TypeScript catalog at `/home/ikuma/org/46ki75/elmethis/packages/core/src/a2ui/v0_9/block-catalog.ts` (and the matching Qwik renderer/story/spec under `packages/qwik/src/components/a2ui/catalog/`). The Rust schema is a vendored mirror of that source of truth.
