# n2a2ui

Convert Notion blocks into [A2UI](https://a2ui.dev) v0.9 components, using
the [Elmethis Block Catalog](https://46ki75.github.io/elmethis/a2ui/v0_9/block_catalog.json)
as the component vocabulary.

This workspace contains two crates:

- [`n2a2ui-a2ui`](./crates/n2a2ui-a2ui) — Rust types for the A2UI v0.9 Elmethis Block
  Catalog (components, surface envelope, v0.9 message envelope,
  dynamic-value helpers). Pure data model, no I/O.
- [`n2a2ui`](./crates/n2a2ui) — the converter; walks a
  Notion block tree with `notionrs` and emits either an A2UI `Surface`
  or the v0.9 message sequence that renders it.

## Example: convert a Notion page to a Surface

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let notion_api_key = std::env::var("NOTION_API_KEY")?;
    let block_id = std::env::var("BLOCK_ID")?;

    let notionrs_client = notionrs::client::Client::new(notion_api_key);
    let reqwest_client = reqwest::Client::new();

    let client = n2a2ui::client::Client {
        notionrs_client,
        reqwest_client,
        enable_unsupported_block: true,
        enable_fetch_image_meta: false,
    };

    let surface = client.convert_block(&block_id).await?;
    println!("{}", serde_json::to_string(&surface)?);
    Ok(())
}
```

## Example: emit the v0.9 rendering messages (eager)

`convert_block_to_messages` returns the full chunked render sequence,
already bound to the Elmethis catalog — `createSurface`, then an
`updateComponents` carrying an empty root `Column` (so the surface
mounts immediately on the client), then one `updateComponents` per
top-level Notion sibling group (a single block, or a consecutive run
of list-items collapsed into one `List`). Each chunk re-sends the root
`Column` with a growing `children` array — `updateComponents` is
upsert-by-id per the v0.9 spec, so this is the canonical streaming
shape, not redundant traffic.

```rust
let messages = client
    .convert_block_to_messages(&block_id, "notion-page")
    .await?;

for message in &messages {
    println!("{}", serde_json::to_string(message)?);
}
// {"version":"v0.9","createSurface":{"surfaceId":"notion-page","catalogId":"..."}}
// {"version":"v0.9","updateComponents":{"surfaceId":"notion-page","components":[{"id":"root","component":"Column","children":[]}]}}
// {"version":"v0.9","updateComponents":{"surfaceId":"notion-page","components":[...,{"id":"root","component":"Column","children":["<id-1>"]}]}}
// {"version":"v0.9","updateComponents":{"surfaceId":"notion-page","components":[...,{"id":"root","component":"Column","children":["<id-1>","<id-2>"]}]}}
// ...
```

## Example: stream the v0.9 rendering messages

For progressive rendering, drive the underlying stream directly — it
yields each `Message` as soon as the corresponding Notion sibling group
is converted, instead of buffering everything until the walk finishes:

```rust
use futures::TryStreamExt;

let mut stream = client.convert_block_stream(&block_id, "notion-page");
while let Some(message) = stream.try_next().await? {
    println!("{}", serde_json::to_string(&message)?);
}
```

`convert_block_to_messages` is literally `convert_block_stream(...).try_collect().await`,
so both APIs emit the same sequence — streaming just lets the renderer
display each chunk as it arrives.

## Behavior toggles

- `enable_unsupported_block` — when `true`, block types the converter
  doesn't know about render as an `Unsupported` component carrying a
  `details` string; when `false`, they are silently dropped.
- `enable_fetch_image_meta` — when `true`, each image block is fetched
  once with `reqwest` + `imagesize` so the emitted `BlockImage` carries
  intrinsic `width` / `height`. Adds a network round-trip per image; off
  by default.

## What gets converted

Paragraphs, headings (1–4), quotes, callouts (with their leading icon
rendered inline), toggles, dividers, code (Mermaid is detected),
equations (Katex), images / files / pdf / audio / video, bookmarks
(embed / link_preview / child_page / child_database all normalise to
`Bookmark`), columns + column_list, tables, synced blocks, and tabs.
Consecutive `bulleted_list_item` / `numbered_list_item` / `to_do`
siblings collapse into a single A2UI `List`.

Rich-text runs convert to `RichText`, `LinkText`, or — for
`custom_emoji` mentions — an inline `Icon` pointing at the emoji's URL.
Equations inside rich text become a `RichText` carrying the LaTeX
source with the `Katex` decoration.

## Tests

`cargo test` runs the schema round-trip tests in `crates/n2a2ui-a2ui` plus a
live integration test in `crates/n2a2ui/tests/convert_block.rs`.
The integration test reads `NOTION_API_KEY` and `BLOCK_ID` from the
environment (or a `.env` at the workspace root) and skips silently when
either is missing, so contributors without credentials — and CI, which
runs `cargo test --lib` — are unaffected.
