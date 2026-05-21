//! Unit tests reproducing bugs surfaced in the codebase review.
//!
//! Fixtures are built by deserializing minimal JSON into
//! `notionrs`'s `BlockResponse`, then handed directly to the converter
//! primitives. No HTTP traffic and no Notion credentials are required —
//! every block has `has_children = false`, so `fetch_children` is never
//! invoked.

use a2ui::v0_9::{ChildList, Column, Component, ComponentId, Surface, UpdateComponents};
use notionrs::types::prelude::BlockResponse;

use super::{Converter, SiblingGroup, top_level_groups};
use crate::id::ROOT_ID;

// --- fixture helpers --------------------------------------------------------

fn make_converter<'a>(
    notionrs_client: &'a notionrs::client::Client,
    reqwest_client: &'a reqwest::Client,
) -> Converter<'a> {
    Converter {
        notionrs: notionrs_client,
        reqwest: reqwest_client,
        enable_unsupported_block: false,
        enable_fetch_image_meta: false,
    }
}

fn block_response(id: &str, type_tag: &str, payload: serde_json::Value) -> BlockResponse {
    let json = serde_json::json!({
        "object": "block",
        "id": id,
        "parent": { "type": "page_id", "page_id": "00000000-0000-0000-0000-000000000000" },
        "created_time": "2024-01-01T00:00:00.000Z",
        "last_edited_time": "2024-01-01T00:00:00.000Z",
        "created_by": { "object": "user", "id": "00000000-0000-0000-0000-000000000000" },
        "last_edited_by": { "object": "user", "id": "00000000-0000-0000-0000-000000000000" },
        "has_children": false,
        "archived": false,
        "in_trash": false,
        "type": type_tag,
        type_tag: payload,
    });
    serde_json::from_value(json).expect("BlockResponse JSON should deserialize")
}

fn rich_text_text(text: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "text",
        "text": { "content": text, "link": null },
        "annotations": {
            "bold": false, "italic": false, "strikethrough": false,
            "underline": false, "code": false, "color": "default"
        },
        "plain_text": text,
        "href": null
    })
}

fn paragraph(id: &str, text: &str) -> BlockResponse {
    block_response(
        id,
        "paragraph",
        serde_json::json!({ "rich_text": [rich_text_text(text)], "color": "default" }),
    )
}

fn bulleted(id: &str, text: &str) -> BlockResponse {
    block_response(
        id,
        "bulleted_list_item",
        serde_json::json!({ "rich_text": [rich_text_text(text)], "color": "default" }),
    )
}

fn numbered(id: &str, text: &str) -> BlockResponse {
    block_response(
        id,
        "numbered_list_item",
        serde_json::json!({ "rich_text": [rich_text_text(text)], "color": "default" }),
    )
}

fn bookmark_with_caption(id: &str, url: &str, caption: &str) -> BlockResponse {
    block_response(
        id,
        "bookmark",
        serde_json::json!({ "url": url, "caption": [rich_text_text(caption)] }),
    )
}

/// Replay the streaming orchestrator on a pre-built `&[BlockResponse]`
/// using the same `top_level_groups` helper that `client.rs` consumes,
/// so this test exercises the real grouping dispatch (not a copy of it).
async fn streaming_chunks(
    converter: &Converter<'_>,
    blocks: &[BlockResponse],
) -> Vec<(ComponentId, Vec<Component>)> {
    let mut chunks = Vec::new();
    for group in top_level_groups(blocks) {
        let chunk = match group {
            SiblingGroup::List { range, style } => Some(
                converter
                    .convert_list_group_to_chunk(&blocks[range], style)
                    .await
                    .expect("convert_list_group_to_chunk should succeed"),
            ),
            SiblingGroup::Single { index } => converter
                .convert_single_block_to_chunk(&blocks[index])
                .await
                .expect("convert_single_block_to_chunk should succeed"),
        };
        if let Some(c) = chunk {
            chunks.push(c);
        }
    }
    chunks
}

fn dummy_clients() -> (notionrs::client::Client, reqwest::Client) {
    (
        notionrs::client::Client::new("dummy"),
        reqwest::Client::new(),
    )
}

fn root_column(children: &[ComponentId]) -> Component {
    Column {
        id: ROOT_ID.into(),
        children: ChildList::from_ids(children.to_vec()),
        ..Default::default()
    }
    .into()
}

// --- Bug #4: bookmark caption is misrouted to description ------------------

/// `Converter::bookmark` puts the Notion-side `caption` (user-authored
/// text under the URL) into A2UI's `Bookmark.description`. Per the
/// catalog, `description` is reserved for the OG `meta description`
/// (sibling of `title`/`image`), so this conflates two semantically
/// distinct fields. The fix is either to leave `description` empty (and
/// route caption somewhere else, when the catalog grows one) or to
/// actually fetch and populate OG metadata.
#[tokio::test]
async fn bug4_bookmark_caption_should_not_be_set_as_description() {
    let (nclient, rclient) = dummy_clients();
    let converter = make_converter(&nclient, &rclient);

    let bm = bookmark_with_caption("bk-1", "https://example.com", "user caption");
    let (id, components) = converter
        .convert_single_block_to_chunk(&bm)
        .await
        .expect("convert_single_block_to_chunk must succeed")
        .expect("bookmark must produce a chunk");

    assert_eq!(id, "bk-1");
    assert_eq!(components.len(), 1);
    let Component::Bookmark(out) = &components[0] else {
        panic!("expected Bookmark variant, got {:?}", components[0]);
    };

    assert_eq!(
        out.description, None,
        "bookmark.description must not be populated from the Notion caption \
         (caption is user-authored; description is reserved for OG meta)"
    );
}

// --- Bug #2: list-grouping duplicated between eager and streaming paths ----

/// `Client::convert_block_stream` re-implements the consecutive-list-item
/// grouping loop that lives in `Converter::convert_siblings`. If the two
/// loops ever drift apart, the streaming wire shape will silently disagree
/// with the eager `Surface`. This test pins them to the same fixture.
#[tokio::test]
async fn bug2_streaming_and_eager_paths_produce_identical_chunking() {
    let (nclient, rclient) = dummy_clients();
    let converter = make_converter(&nclient, &rclient);

    let blocks = vec![
        paragraph("p-1", "intro"),
        bulleted("b-1", "first bullet"),
        bulleted("b-2", "second bullet"),
        paragraph("p-2", "middle"),
        numbered("n-1", "first item"),
        numbered("n-2", "second item"),
        paragraph("p-3", "outro"),
    ];

    let mut eager_bag: Vec<Component> = Vec::new();
    let eager_ids = converter
        .convert_siblings(&blocks, &mut eager_bag)
        .await
        .expect("eager convert_siblings should succeed");

    let streamed = streaming_chunks(&converter, &blocks).await;
    let streamed_ids: Vec<ComponentId> = streamed.iter().map(|(id, _)| id.clone()).collect();
    let streamed_flat: Vec<Component> = streamed
        .iter()
        .flat_map(|(_, comps)| comps.clone())
        .collect();

    assert_eq!(
        eager_ids, streamed_ids,
        "chunk ids should match between eager convert_siblings and the \
         streaming orchestrator's loop"
    );

    let eager_component_ids: Vec<&str> = eager_bag.iter().map(|c| c.id()).collect();
    let streamed_component_ids: Vec<&str> = streamed_flat.iter().map(|c| c.id()).collect();
    assert_eq!(
        eager_component_ids, streamed_component_ids,
        "component ids must be emitted in the same order by both paths"
    );

    // Sanity: the two consecutive bullets must collapse into one List
    // group, and the two consecutive numbered items into another. With
    // three paragraphs, we expect 5 top-level chunks.
    assert_eq!(
        streamed_ids.len(),
        5,
        "expected paragraph + bullet-list + paragraph + numbered-list + paragraph"
    );
    assert!(streamed_ids[1].ends_with("::list"));
    assert!(streamed_ids[3].ends_with("::list"));
}

// --- Bug #1: IndexMap ordering across streaming reconstruction -------------

/// A consumer that reconstructs a `Surface` by replaying the
/// `updateComponents` messages from `convert_block_stream` should end up
/// with the same `IndexMap` key order as the eager `convert_block`
/// produces directly. This test pins that invariant — if it fails, the
/// claim in the review (streaming reconstruction places root at the tail
/// of chunk 1 rather than at index 0) is real.
#[tokio::test]
async fn bug1_streaming_reconstruction_preserves_eager_component_order() {
    let (nclient, rclient) = dummy_clients();
    let converter = make_converter(&nclient, &rclient);

    let blocks = vec![
        paragraph("p-1", "alpha"),
        bulleted("b-1", "bravo"),
        bulleted("b-2", "charlie"),
        paragraph("p-2", "delta"),
    ];

    // Eager surface — exactly the construction in
    // `client::Client::convert_block`.
    let mut eager_bag: Vec<Component> = Vec::new();
    let eager_root_children = converter
        .convert_siblings(&blocks, &mut eager_bag)
        .await
        .expect("eager convert_siblings should succeed");
    let mut eager_surface = Surface::new(ROOT_ID);
    eager_surface.insert(root_column(&eager_root_children));
    for c in eager_bag {
        eager_surface.insert(c);
    }

    // Streamed wire shape, reconstructed by `Surface::insert` per
    // component in message order. Mirrors `convert_block_stream`:
    //   1. createSurface (carries no components — omitted here)
    //   2. updateComponents { components: [empty root] }
    //   3..N. updateComponents { components: [chunk inner..., updated root] }
    let mut streamed_msgs: Vec<UpdateComponents> = Vec::new();
    let mut accumulated: Vec<ComponentId> = Vec::new();
    streamed_msgs.push(UpdateComponents {
        surface_id: "s".into(),
        components: vec![root_column(&accumulated)],
    });
    let chunks = streaming_chunks(&converter, &blocks).await;
    for (chunk_id, mut comps) in chunks {
        accumulated.push(chunk_id);
        comps.push(root_column(&accumulated));
        streamed_msgs.push(UpdateComponents {
            surface_id: "s".into(),
            components: comps,
        });
    }
    let mut streamed_surface = Surface::new(ROOT_ID);
    for uc in &streamed_msgs {
        for c in &uc.components {
            streamed_surface.insert(c.clone());
        }
    }

    let eager_order: Vec<&str> = eager_surface
        .components
        .keys()
        .map(|s| s.as_str())
        .collect();
    let streamed_order: Vec<&str> = streamed_surface
        .components
        .keys()
        .map(|s| s.as_str())
        .collect();
    assert_eq!(
        eager_order, streamed_order,
        "reconstructing a Surface from convert_block_stream messages must \
         preserve the eager IndexMap component order"
    );
}

// --- Bug #6: table row_index over unfiltered iterator ----------------------

fn table_row(id: &str, cells: &[&str]) -> BlockResponse {
    let cells_json: Vec<serde_json::Value> = cells
        .iter()
        .map(|t| serde_json::json!([rich_text_text(t)]))
        .collect();
    block_response(id, "table_row", serde_json::json!({ "cells": cells_json }))
}

/// `Converter::classify_table_rows` must filter out non-`TableRow`
/// children *before* enumerating, so that with `has_column_header =
/// true` the first real row lands in the header even when a stray
/// non-row block precedes it. Prior to the fix the index advanced past
/// the skipped block, sending the first real row into the body.
#[tokio::test]
async fn bug6_table_filters_non_row_children_before_indexing_header() {
    let (nclient, rclient) = dummy_clients();
    let converter = make_converter(&nclient, &rclient);

    let rows = vec![
        paragraph("stray-1", "not a row"),
        table_row("row-a", &["A1", "A2"]),
        table_row("row-b", &["B1", "B2"]),
    ];

    let mut bag: Vec<Component> = Vec::new();
    let (header_ids, body_ids) = converter.classify_table_rows(&rows, true, false, &mut bag);

    assert_eq!(
        header_ids,
        vec!["row-a".to_string()],
        "the first TableRow should be the header row when has_column_header=true, \
         regardless of any non-row siblings preceding it"
    );
    assert_eq!(body_ids, vec!["row-b".to_string()]);
}
