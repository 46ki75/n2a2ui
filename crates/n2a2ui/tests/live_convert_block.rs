//! Live integration test that exercises `Client::convert_block` against
//! the real Notion API.
//!
//! Gated with `#[ignore]` so default `cargo test` skips it. Opt in with
//! `cargo test -- --ignored` (or `just test-live`). Reads `NOTION_API_KEY`
//! and `BLOCK_ID` from the environment or a `.env` at the workspace root
//! via `dotenvy`; both must be set.

use futures::TryStreamExt;
use n2a2ui::client::Client;
use n2a2ui::id::ROOT_ID;
use n2a2ui_a2ui::v0_9::{Component, MessageBody};

fn make_client() -> (Client, String) {
    let _ = dotenvy::from_path("../../.env");
    let notion_api_key =
        std::env::var("NOTION_API_KEY").expect("NOTION_API_KEY must be set for live tests");
    let block_id = std::env::var("BLOCK_ID").expect("BLOCK_ID must be set for live tests");
    let client = Client {
        notionrs_client: notionrs::client::Client::new(notion_api_key),
        reqwest_client: reqwest::Client::new(),
        enable_unsupported_block: true,
        enable_fetch_image_meta: false,
        enable_fetch_bookmark_meta: false,
    };
    (client, block_id)
}

#[tokio::test]
#[ignore = "live: hits Notion API, requires NOTION_API_KEY + BLOCK_ID"]
async fn convert_block_against_live_notion() {
    let (client, block_id) = make_client();

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

    // Sequence: createSurface, then >=1 updateComponents (initial empty
    // root + one per top-level sibling group).
    assert!(
        messages.len() >= 2,
        "expected at least createSurface + initial empty root"
    );
    assert!(
        matches!(messages[0].body, MessageBody::CreateSurface(_)),
        "first message must be createSurface"
    );
    for (i, m) in messages.iter().enumerate().skip(1) {
        assert!(
            matches!(m.body, MessageBody::UpdateComponents(_)),
            "message {i} should be updateComponents, got {:?}",
            m.body
        );
    }

    println!("--- messages (JSONL) ---");
    for message in &messages {
        let line = serde_json::to_string(message).expect("serialize message");
        println!("{line}");
    }

    let out_path = std::path::Path::new("../../.out/test.json");
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).expect("create .out directory");
    }
    let pretty = serde_json::to_string_pretty(&messages).expect("serialize messages");
    std::fs::write(out_path, pretty).expect("write messages JSON");
    println!(
        "wrote {} messages to {}",
        messages.len(),
        out_path.display()
    );
}

#[tokio::test]
#[ignore = "live: hits Notion API, requires NOTION_API_KEY + BLOCK_ID"]
async fn stream_yields_growing_root() {
    let (client, block_id) = make_client();

    let streamed: Vec<_> = client
        .convert_block_stream(&block_id, "notion-page")
        .try_collect()
        .await
        .expect("stream should not error");

    // createSurface + initial empty root + at least one chunk for any
    // non-empty page.
    assert!(
        streamed.len() >= 2,
        "expected at least createSurface + initial empty root, got {}",
        streamed.len()
    );
    assert!(
        matches!(streamed[0].body, MessageBody::CreateSurface(_)),
        "first message must be createSurface"
    );

    // Walk the chunks: every updateComponents must carry the root
    // Column, and its children list grows monotonically by at most one
    // id per chunk (a chunk corresponds to one top-level sibling group).
    let mut prev_root_children = 0usize;
    for (i, m) in streamed.iter().enumerate().skip(1) {
        let MessageBody::UpdateComponents(uc) = &m.body else {
            panic!("message {i} should be updateComponents");
        };
        let root = uc
            .components
            .iter()
            .find(|c| c.id() == ROOT_ID)
            .unwrap_or_else(|| panic!("message {i} must include the root component"));
        let Component::Column(col) = root else {
            panic!("root component should be a Column");
        };
        let n = match &col.children {
            n2a2ui_a2ui::v0_9::ChildList::Static(ids) => ids.len(),
            other => panic!("root.children must be Static, got {other:?}"),
        };
        assert!(
            n == prev_root_children || n == prev_root_children + 1,
            "root.children should grow by 0 or 1 per chunk (was {prev_root_children}, now {n} at message {i})"
        );
        prev_root_children = n;
    }

    println!("stream yielded {} messages", streamed.len());
}
