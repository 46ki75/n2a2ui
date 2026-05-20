//! Notion → A2UI block dispatch.
//!
//! Phase 1 only handles `paragraph` and `heading_1..4`. Everything else
//! either becomes an `Unsupported` placeholder (when the client has
//! `enable_unsupported_block = true`) or is skipped.

use a2ui::v0_9::{
    ChildList, Component, ComponentId, Heading, HeadingLevel, Paragraph, Unsupported,
};
use notionrs::types::prelude::{Block, BlockResponse, RichText as NotionRichText};

pub mod rich_text;

/// Convert one Notion block into an A2UI component plus any synthesized
/// descendants. The returned `ComponentId` is the id the parent should
/// reference; the `Vec<Component>` is everything to insert into the
/// surface (root of the subtree first).
///
/// Returns `None` to signal "skip this block" — used for unsupported
/// kinds when `enable_unsupported_block` is false.
pub fn convert_block(
    notion: &BlockResponse,
    enable_unsupported_block: bool,
) -> Option<(ComponentId, Vec<Component>)> {
    let id = notion.id.clone();
    let mut bag: Vec<Component> = Vec::new();

    let component: Component = match &notion.block {
        Block::Paragraph { paragraph } => {
            let (child_ids, mut children) =
                rich_text::convert_rich_texts(&id, "rich_text", &paragraph.rich_text);
            bag.append(&mut children);
            Paragraph {
                id: id.clone(),
                children: ChildList::from_ids(child_ids),
                ..Default::default()
            }
            .into()
        }
        Block::Heading1 { heading_1 } => {
            heading(&id, HeadingLevel::H1, &heading_1.rich_text, &mut bag)
        }
        Block::Heading2 { heading_2 } => {
            heading(&id, HeadingLevel::H2, &heading_2.rich_text, &mut bag)
        }
        Block::Heading3 { heading_3 } => {
            heading(&id, HeadingLevel::H3, &heading_3.rich_text, &mut bag)
        }
        Block::Heading4 { heading_4 } => {
            heading(&id, HeadingLevel::H4, &heading_4.rich_text, &mut bag)
        }
        other => {
            if !enable_unsupported_block {
                return None;
            }
            Unsupported {
                id: id.clone(),
                details: Some(unsupported_label(other)),
                ..Default::default()
            }
            .into()
        }
    };

    bag.insert(0, component);
    Some((id, bag))
}

fn heading(
    id: &str,
    level: HeadingLevel,
    items: &[NotionRichText],
    bag: &mut Vec<Component>,
) -> Component {
    let (child_ids, mut children) = rich_text::convert_rich_texts(id, "rich_text", items);
    bag.append(&mut children);
    Heading {
        id: id.into(),
        level,
        children: ChildList::from_ids(child_ids),
        ..Default::default()
    }
    .into()
}

#[allow(deprecated)]
fn unsupported_label(block: &Block) -> String {
    match block {
        Block::Audio { .. } => "audio".into(),
        Block::Bookmark { .. } => "bookmark".into(),
        Block::Breadcrumb { .. } => "breadcrumb".into(),
        Block::BulletedListItem { .. } => "bulleted_list_item".into(),
        Block::Callout { .. } => "callout".into(),
        Block::ChildDatabase { .. } => "child_database".into(),
        Block::ChildPage { .. } => "child_page".into(),
        Block::Code { .. } => "code".into(),
        Block::ColumnList { .. } => "column_list".into(),
        Block::Column { .. } => "column".into(),
        Block::Divider { .. } => "divider".into(),
        Block::Embed { .. } => "embed".into(),
        Block::Equation { .. } => "equation".into(),
        Block::File { .. } => "file".into(),
        Block::Heading1 { .. } => "heading_1".into(),
        Block::Heading2 { .. } => "heading_2".into(),
        Block::Heading3 { .. } => "heading_3".into(),
        Block::Heading4 { .. } => "heading_4".into(),
        Block::Image { .. } => "image".into(),
        Block::LinkPreview { .. } => "link_preview".into(),
        Block::NumberedListItem { .. } => "numbered_list_item".into(),
        Block::Paragraph { .. } => "paragraph".into(),
        Block::Pdf { .. } => "pdf".into(),
        Block::Quote { .. } => "quote".into(),
        Block::SyncedBlock { .. } => "synced_block".into(),
        Block::TableOfContents { .. } => "table_of_contents".into(),
        Block::Tab { .. } => "tab".into(),
        Block::Table { .. } => "table".into(),
        Block::TableRow { .. } => "table_row".into(),
        Block::Template { .. } => "template".into(),
        Block::ToDo { .. } => "to_do".into(),
        Block::Toggle { .. } => "toggle".into(),
        Block::MeetingNotes { .. } => "meeting_notes".into(),
        Block::Transcription { .. } => "transcription".into(),
        Block::Video { .. } => "video".into(),
        Block::Unsupported { unsupported } => format!("unsupported:{}", unsupported.block_type),
    }
}
