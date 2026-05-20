//! Convert a Notion `rich_text` array into A2UI inline components.
//!
//! Each Notion rich-text entry becomes either a `RichText` (plain run
//! with optional decoration / color) or a `LinkText` (when `href` is
//! set). Equation runs are emitted as `RichText` with the `Katex`
//! decoration carrying the LaTeX source as their text.

use a2ui::v0_9::{Component, ComponentId, Decoration, LinkText, RichText};
use notionrs::types::prelude::{Color, RichText as NotionRichText, RichTextAnnotations};

use crate::id::child_id;

/// Convert a Notion rich-text array into a list of synthesized A2UI
/// inline components. Returns the child ids in source order plus the
/// fully realized components ready for `Surface::insert`.
pub fn convert_rich_texts(
    parent_id: &str,
    slot: &str,
    items: &[NotionRichText],
) -> (Vec<ComponentId>, Vec<Component>) {
    let mut ids = Vec::with_capacity(items.len());
    let mut components = Vec::with_capacity(items.len());

    for (index, item) in items.iter().enumerate() {
        let id = child_id(parent_id, slot, index);
        components.push(convert_single(&id, item));
        ids.push(id);
    }

    (ids, components)
}

fn convert_single(id: &str, item: &NotionRichText) -> Component {
    let (plain, annotations, href, is_equation) = match item {
        NotionRichText::Text {
            plain_text,
            annotations,
            href,
            ..
        } => (plain_text.clone(), *annotations, href.clone(), false),
        NotionRichText::Mention {
            plain_text,
            annotations,
            href,
            ..
        } => (plain_text.clone(), *annotations, href.clone(), false),
        NotionRichText::Equation {
            equation,
            annotations,
            href,
            ..
        } => (
            equation.expression.clone(),
            *annotations,
            href.clone(),
            true,
        ),
    };

    if let Some(href) = href {
        return LinkText {
            id: id.into(),
            text: plain.into(),
            href,
            ..Default::default()
        }
        .into();
    }

    let mut decoration = decorations_from(&annotations);
    if is_equation {
        decoration.push(Decoration::Katex);
    }
    let decoration = if decoration.is_empty() {
        None
    } else {
        Some(decoration)
    };
    let color = match annotations.color {
        Color::Default => None,
        c => Some(c.to_string()),
    };

    RichText {
        id: id.into(),
        text: plain.into(),
        decoration,
        color,
        ..Default::default()
    }
    .into()
}

fn decorations_from(annotations: &RichTextAnnotations) -> Vec<Decoration> {
    let mut out = Vec::new();
    if annotations.bold {
        out.push(Decoration::Bold);
    }
    if annotations.italic {
        out.push(Decoration::Italic);
    }
    if annotations.underline {
        out.push(Decoration::Underline);
    }
    if annotations.strikethrough {
        out.push(Decoration::Strikethrough);
    }
    if annotations.code {
        out.push(Decoration::Code);
    }
    out
}
