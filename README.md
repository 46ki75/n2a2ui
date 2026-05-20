# notion-to-a2ui

Convert Notion blocks into [A2UI](https://a2ui.dev) v0.9 components, using
the [Elmethis Block Catalog](https://46ki75.github.io/elmethis/a2ui/v0_9/block_catalog.json)
as the component vocabulary.

This workspace contains two crates:

- [`a2ui`](./crates/a2ui) — Rust types for the A2UI v0.9 Elmethis Block
  Catalog (components, surface envelope, dynamic-value helpers).
- [`notion-to-a2ui`](./crates/notion-to-a2ui) — the converter; walks a
  Notion block tree and emits an A2UI `Surface`.

## Example

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let notion_api_key = std::env::var("NOTION_API_KEY")?;
    let block_id = std::env::var("BLOCK_ID")?;

    let notionrs_client = notionrs::client::Client::new().secret(notion_api_key);
    let reqwest_client = reqwest::Client::new();

    let client = notion_to_a2ui::client::Client {
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
