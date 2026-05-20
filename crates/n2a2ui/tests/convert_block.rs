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
//!     cargo test -p n2a2ui --test convert_block -- --nocapture

use a2ui::v0_9::{Component, MessageBody};
use futures::TryStreamExt;
use n2a2ui::client::Client;
use n2a2ui::id::ROOT_ID;

fn make_client() -> Option<(Client, String)> {
    let _ = dotenvy::from_path("../../.env");
    let notion_api_key = std::env::var("NOTION_API_KEY").ok()?;
    let block_id = std::env::var("BLOCK_ID").ok()?;
    let client = Client {
        notionrs_client: notionrs::client::Client::new(notion_api_key),
        reqwest_client: reqwest::Client::new(),
        enable_unsupported_block: true,
        enable_fetch_image_meta: false,
    };
    Some((client, block_id))
}

#[tokio::test]
async fn convert_block_against_live_notion() {
    let Some((client, block_id)) = make_client() else {
        eprintln!("skipping: NOTION_API_KEY or BLOCK_ID not set");
        return;
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
    println!("wrote {} messages to {}", messages.len(), out_path.display());
}

#[tokio::test]
async fn stream_yields_growing_root() {
    let Some((client, block_id)) = make_client() else {
        eprintln!("skipping: NOTION_API_KEY or BLOCK_ID not set");
        return;
    };

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
            a2ui::v0_9::ChildList::Static(ids) => ids.len(),
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
