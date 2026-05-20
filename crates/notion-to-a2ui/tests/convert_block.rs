//! Integration test that exercises `Client::convert_block` against the
//! live Notion API.
//!
//! Reads `NOTION_API_KEY` and `BLOCK_ID` from the environment (or from a
//! `.env` file at the workspace root via `dotenvy`). When either is
//! absent the test is skipped silently, so `cargo test` stays a no-op
//! for contributors without credentials and CI (which runs `--lib` only)
//! is unaffected.
//!
//! Run with output:
//!     cargo test -p notion-to-a2ui --test convert_block -- --nocapture

use a2ui::v0_9::MessageBody;
use notion_to_a2ui::client::Client;

#[tokio::test]
async fn convert_block_against_live_notion() {
    let _ = dotenvy::from_path("../../.env");

    let Ok(notion_api_key) = std::env::var("NOTION_API_KEY") else {
        eprintln!("skipping: NOTION_API_KEY not set");
        return;
    };
    let Ok(block_id) = std::env::var("BLOCK_ID") else {
        eprintln!("skipping: BLOCK_ID not set");
        return;
    };

    let notionrs_client = notionrs::client::Client::new(notion_api_key);
    let reqwest_client = reqwest::Client::new();

    let client = Client {
        notionrs_client,
        reqwest_client,
        enable_unsupported_block: true,
        enable_fetch_image_meta: false,
    };

    let surface = client
        .convert_block(&block_id)
        .await
        .expect("convert_block should succeed");

    assert!(
        !surface.root.is_empty(),
        "surface root id should not be empty"
    );
    assert!(
        surface.components.contains_key(&surface.root),
        "surface.components must contain the root id `{}`",
        surface.root
    );

    let json = serde_json::to_string_pretty(&surface).expect("serialize surface as JSON");
    println!("--- surface ---\n{json}");

    let messages = client
        .convert_block_to_messages(&block_id, "notion-page")
        .await
        .expect("convert_block_to_messages should succeed");

    assert_eq!(
        messages.len(),
        2,
        "expected createSurface + updateComponents"
    );
    assert!(
        matches!(messages[0].body, MessageBody::CreateSurface(_)),
        "first message must be createSurface"
    );
    assert!(
        matches!(messages[1].body, MessageBody::UpdateComponents(_)),
        "second message must be updateComponents"
    );

    println!("--- messages (JSONL) ---");
    for message in &messages {
        let line = serde_json::to_string(message).expect("serialize message");
        println!("{line}");
    }
}
