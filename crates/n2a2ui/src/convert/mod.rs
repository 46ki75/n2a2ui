//! Notion → A2UI block dispatch.
//!
//! [`Converter`] holds the notionrs + reqwest clients plus the runtime
//! toggles, so per-block handlers can recurse (via `BoxFuture`) into
//! `has_children` subtrees and optionally fetch image dimensions.
//!
//! Sibling grouping happens in [`Converter::convert_siblings`]:
//! consecutive `bulleted_list_item` / `numbered_list_item` / `to_do`
//! blocks collapse into a single `List` so the wire output matches the
//! A2UI block model rather than Notion's flat sibling layout.

use futures::TryStreamExt;
use futures::future::BoxFuture;
use n2a2ui_a2ui::v0_9::{
    BlockImage, BlockQuote, Bookmark, Callout, CalloutType, ChildList, CodeBlock, Column,
    ColumnList, Component, ComponentId, ContentTab, ContentTabs, Divider, File as FileComponent,
    Heading, HeadingLevel, Icon, Katex, List, ListItem, ListStyle, Mermaid, Paragraph, RichText,
    Table, TableCell, TableRow, Toggle, Unsupported,
};
use notionrs::PaginateExt;
use notionrs::types::prelude::{
    Block, BlockResponse, BookmarkBlock, BulletedListItemBlock, CalloutBlock, EmojiAndIcon,
    EquationBlock, File as NotionFile, Language, NumberedListItemBlock, ParagraphBlock, QuoteBlock,
    RichText as NotionRichText, TableRowBlock, ToDoBlock, ToggleBlock,
};

use crate::error::Error;
use crate::id::child_id;

pub(crate) mod color;
pub mod rich_text;

#[cfg(test)]
mod tests;

/// Stateful Notion → A2UI converter. Holds the network clients so
/// per-block handlers can recurse into children and (optionally) probe
/// image dimensions.
pub(crate) struct Converter<'a> {
    pub notionrs: &'a notionrs::client::Client,
    pub reqwest: &'a reqwest::Client,
    pub enable_unsupported_block: bool,
    pub enable_fetch_image_meta: bool,
}

impl<'a> Converter<'a> {
    /// Fetch `parent_id`'s direct children, convert them, and return the
    /// ids the parent should reference plus every synthesized component
    /// to insert into the surface.
    pub async fn convert_children(
        &self,
        parent_id: &str,
    ) -> Result<(Vec<ComponentId>, Vec<Component>), Error> {
        let blocks = self.fetch_children(parent_id).await?;
        let mut bag = Vec::new();
        let ids = self.convert_siblings(&blocks, &mut bag).await?;
        Ok((ids, bag))
    }

    pub(crate) async fn fetch_children(
        &self,
        parent_id: &str,
    ) -> Result<Vec<BlockResponse>, Error> {
        let blocks: Vec<BlockResponse> = self
            .notionrs
            .get_block_children()
            .block_id(parent_id)
            .into_stream()
            .try_collect()
            .await?;
        Ok(blocks)
    }

    /// Convert a single non-list sibling into its own chunk: a fresh
    /// component bag plus the id the parent should reference.
    ///
    /// Returns `None` when the block is unsupported and
    /// `enable_unsupported_block` is off — the streaming orchestrator
    /// skips those without emitting an `updateComponents`.
    pub(crate) async fn convert_single_block_to_chunk(
        &self,
        notion: &BlockResponse,
    ) -> Result<Option<(ComponentId, Vec<Component>)>, Error> {
        let mut bag = Vec::new();
        let id = self.convert_block(notion, &mut bag).await?;
        Ok(id.map(|id| (id, bag)))
    }

    /// Convert a consecutive run of list-item siblings (same style) into
    /// one `List` chunk, returning the list-group id and every component
    /// synthesized for it.
    pub(crate) async fn convert_list_group_to_chunk(
        &self,
        items: &[BlockResponse],
        style: ListStyle,
    ) -> Result<(ComponentId, Vec<Component>), Error> {
        let mut bag = Vec::new();
        let id = self.emit_list(items, style, &mut bag).await?;
        Ok((id, bag))
    }

    /// Convert a sequence of sibling blocks, grouping consecutive list
    /// items into a single `List` so the structure matches the A2UI
    /// adjacency model rather than Notion's flat layout.
    fn convert_siblings<'b>(
        &'b self,
        blocks: &'b [BlockResponse],
        bag: &'b mut Vec<Component>,
    ) -> BoxFuture<'b, Result<Vec<ComponentId>, Error>> {
        Box::pin(async move {
            let mut ids = Vec::new();
            for group in top_level_groups(blocks) {
                match group {
                    SiblingGroup::List { range, style } => {
                        let list_id = self.emit_list(&blocks[range], style, bag).await?;
                        ids.push(list_id);
                    }
                    SiblingGroup::Single { index } => {
                        if let Some(id) = self.convert_block(&blocks[index], bag).await? {
                            ids.push(id);
                        }
                    }
                }
            }
            Ok(ids)
        })
    }

    fn convert_block<'b>(
        &'b self,
        notion: &'b BlockResponse,
        bag: &'b mut Vec<Component>,
    ) -> BoxFuture<'b, Result<Option<ComponentId>, Error>> {
        Box::pin(async move {
            let id = notion.id.clone();
            let component: Component = match &notion.block {
                Block::Paragraph { paragraph } => {
                    self.paragraph(&id, paragraph, notion.has_children, bag)
                        .await?
                }
                Block::Heading1 { heading_1 } => {
                    self.heading(&id, HeadingLevel::H1, &heading_1.rich_text, bag)
                }
                Block::Heading2 { heading_2 } => {
                    self.heading(&id, HeadingLevel::H2, &heading_2.rich_text, bag)
                }
                Block::Heading3 { heading_3 } => {
                    self.heading(&id, HeadingLevel::H3, &heading_3.rich_text, bag)
                }
                Block::Heading4 { heading_4 } => {
                    self.heading(&id, HeadingLevel::H4, &heading_4.rich_text, bag)
                }
                Block::Quote { quote } => self.quote(&id, quote, notion.has_children, bag).await?,
                Block::Callout { callout } => {
                    self.callout(&id, callout, notion.has_children, bag).await?
                }
                Block::Toggle { toggle } => {
                    self.toggle(&id, toggle, notion.has_children, bag).await?
                }
                Block::Divider { .. } => Divider {
                    id: id.clone(),
                    ..Default::default()
                }
                .into(),
                Block::Code { code } => {
                    let body = code
                        .rich_text
                        .iter()
                        .map(rich_text_plain)
                        .collect::<String>();
                    let caption = if code.caption.is_empty() {
                        None
                    } else {
                        Some(code.caption.iter().map(rich_text_plain).collect::<String>())
                    };
                    if matches!(code.language, Language::Mermaid) {
                        Mermaid {
                            id: id.clone(),
                            code: body,
                            ..Default::default()
                        }
                        .into()
                    } else {
                        CodeBlock {
                            id: id.clone(),
                            code: body,
                            language: Some(code.language.to_string()),
                            caption,
                            ..Default::default()
                        }
                        .into()
                    }
                }
                Block::Equation { equation } => katex(&id, equation),
                Block::Image { image } => self.image(&id, image).await,
                Block::File { file } => file_component(&id, file),
                Block::Pdf { pdf } => file_component(&id, pdf),
                Block::Audio { audio } => file_component(&id, audio),
                Block::Video { video } => file_component(&id, video),
                Block::Bookmark { bookmark } => self.bookmark(&id, bookmark),
                Block::Embed { embed } => Bookmark {
                    id: id.clone(),
                    url: embed.url.clone(),
                    ..Default::default()
                }
                .into(),
                Block::LinkPreview { link_preview } => Bookmark {
                    id: id.clone(),
                    url: link_preview.url.clone(),
                    ..Default::default()
                }
                .into(),
                Block::ChildPage { child_page } => Bookmark {
                    id: id.clone(),
                    url: notion_page_url(&id),
                    title: Some(child_page.title.clone()),
                    ..Default::default()
                }
                .into(),
                Block::ChildDatabase { child_database } => Bookmark {
                    id: id.clone(),
                    url: notion_page_url(&id),
                    title: Some(child_database.title.clone()),
                    ..Default::default()
                }
                .into(),
                Block::ColumnList { .. } => self.column_list(&id, notion.has_children, bag).await?,
                Block::Column { column } => {
                    let mut children = Vec::new();
                    if notion.has_children {
                        let (ids, comps) = self.convert_children(&id).await?;
                        bag.extend(comps);
                        children = ids;
                    }
                    Column {
                        id: id.clone(),
                        children: ChildList::from_ids(children),
                        width_ratio: Some(column.width_ratio),
                        ..Default::default()
                    }
                    .into()
                }
                Block::Table { table } => {
                    self.table(&id, table.has_column_header, table.has_row_header, bag)
                        .await?
                }
                Block::Tab { .. } => self.tab(&id, notion.has_children, bag).await?,
                Block::SyncedBlock { .. } => {
                    // Render the synced block transparently as a Column of its children.
                    let mut children = Vec::new();
                    if notion.has_children {
                        let (ids, comps) = self.convert_children(&id).await?;
                        bag.extend(comps);
                        children = ids;
                    }
                    Column {
                        id: id.clone(),
                        children: ChildList::from_ids(children),
                        ..Default::default()
                    }
                    .into()
                }
                other => {
                    if !self.enable_unsupported_block {
                        return Ok(None);
                    }
                    Unsupported {
                        id: id.clone(),
                        details: Some(unsupported_label(other)),
                        ..Default::default()
                    }
                    .into()
                }
            };
            bag.push(component);
            Ok(Some(id))
        })
    }

    // --- per-block handlers ------------------------------------------------

    async fn paragraph(
        &self,
        id: &str,
        block: &ParagraphBlock,
        has_children: bool,
        bag: &mut Vec<Component>,
    ) -> Result<Component, Error> {
        let (mut children, mut rt) =
            rich_text::convert_rich_texts(id, "rich_text", &block.rich_text);
        bag.append(&mut rt);
        if has_children {
            let (cs, comps) = self.convert_children(id).await?;
            bag.extend(comps);
            children.extend(cs);
        }
        Ok(Paragraph {
            id: id.into(),
            children: ChildList::from_ids(children),
            color: color::map_color(block.color),
            background_color: color::map_background_color(block.color),
            ..Default::default()
        }
        .into())
    }

    fn heading(
        &self,
        id: &str,
        level: HeadingLevel,
        rich_text_items: &[NotionRichText],
        bag: &mut Vec<Component>,
    ) -> Component {
        let (child_ids, mut rt) = rich_text::convert_rich_texts(id, "rich_text", rich_text_items);
        bag.append(&mut rt);
        Heading {
            id: id.into(),
            level,
            children: ChildList::from_ids(child_ids),
            ..Default::default()
        }
        .into()
    }

    async fn quote(
        &self,
        id: &str,
        block: &QuoteBlock,
        has_children: bool,
        bag: &mut Vec<Component>,
    ) -> Result<Component, Error> {
        let (mut children, mut rt) =
            rich_text::convert_rich_texts(id, "rich_text", &block.rich_text);
        bag.append(&mut rt);
        if has_children {
            let (cs, comps) = self.convert_children(id).await?;
            bag.extend(comps);
            children.extend(cs);
        }
        Ok(BlockQuote {
            id: id.into(),
            children: ChildList::from_ids(children),
            ..Default::default()
        }
        .into())
    }

    async fn callout(
        &self,
        id: &str,
        block: &CalloutBlock,
        has_children: bool,
        bag: &mut Vec<Component>,
    ) -> Result<Component, Error> {
        let mut children = Vec::new();
        if let Some(icon) = block.icon.as_ref() {
            let icon_id = child_id(id, "icon", 0);
            if let Some(component) = inline_icon_component(&icon_id, icon) {
                bag.push(component);
                children.push(icon_id);
            }
        }
        let (rt_ids, mut rt) = rich_text::convert_rich_texts(id, "rich_text", &block.rich_text);
        bag.append(&mut rt);
        children.extend(rt_ids);
        if has_children {
            let (cs, comps) = self.convert_children(id).await?;
            bag.extend(comps);
            children.extend(cs);
        }
        Ok(Callout {
            id: id.into(),
            children: ChildList::from_ids(children),
            callout_type: callout_type_from_icon(block.icon.as_ref()),
            ..Default::default()
        }
        .into())
    }

    async fn toggle(
        &self,
        id: &str,
        block: &ToggleBlock,
        has_children: bool,
        bag: &mut Vec<Component>,
    ) -> Result<Component, Error> {
        let (summary_ids, mut summary_rt) =
            rich_text::convert_rich_texts(id, "summary", &block.rich_text);
        bag.append(&mut summary_rt);
        let mut children = Vec::new();
        if has_children {
            let (cs, comps) = self.convert_children(id).await?;
            bag.extend(comps);
            children = cs;
        }
        Ok(Toggle {
            id: id.into(),
            summary: ChildList::from_ids(summary_ids),
            children: ChildList::from_ids(children),
            ..Default::default()
        }
        .into())
    }

    async fn image(&self, id: &str, file: &NotionFile) -> Component {
        let Some(src) = file_url(file) else {
            return Unsupported {
                id: id.into(),
                details: Some("image:api_uploaded".into()),
                ..Default::default()
            }
            .into();
        };
        let alt = file_name(file);
        let caption = file_caption(file).map(|rts| rts.iter().map(rich_text_plain).collect());

        let (width, height) = if self.enable_fetch_image_meta {
            self.fetch_image_dimensions(&src).await
        } else {
            (None, None)
        };

        BlockImage {
            id: id.into(),
            src,
            alt,
            width,
            height,
            caption,
            ..Default::default()
        }
        .into()
    }

    async fn fetch_image_dimensions(&self, url: &str) -> (Option<f64>, Option<f64>) {
        let Ok(response) = self.reqwest.get(url).send().await else {
            return (None, None);
        };
        let Ok(bytes) = response.bytes().await else {
            return (None, None);
        };
        match imagesize::blob_size(&bytes) {
            Ok(dim) => (Some(dim.width as f64), Some(dim.height as f64)),
            Err(_) => (None, None),
        }
    }

    fn bookmark(&self, id: &str, block: &BookmarkBlock) -> Component {
        // A2UI `Bookmark.description` is reserved for the OG `meta
        // description`. Notion's `caption` is user-authored text and is
        // semantically distinct, so we deliberately do not populate
        // `description` from it. OG enrichment is a separate concern.
        Bookmark {
            id: id.into(),
            url: block.url.clone(),
            ..Default::default()
        }
        .into()
    }

    async fn column_list(
        &self,
        id: &str,
        has_children: bool,
        bag: &mut Vec<Component>,
    ) -> Result<Component, Error> {
        let mut children = Vec::new();
        if has_children {
            let (cs, comps) = self.convert_children(id).await?;
            bag.extend(comps);
            children = cs;
        }
        Ok(ColumnList {
            id: id.into(),
            children,
            ..Default::default()
        }
        .into())
    }

    async fn table(
        &self,
        id: &str,
        has_column_header: bool,
        has_row_header: bool,
        bag: &mut Vec<Component>,
    ) -> Result<Component, Error> {
        let rows = self.fetch_children(id).await?;
        let (header_ids, body_ids) =
            self.classify_table_rows(&rows, has_column_header, has_row_header, bag);
        Ok(Table {
            id: id.into(),
            body: body_ids,
            header: if header_ids.is_empty() {
                None
            } else {
                Some(header_ids)
            },
            has_column_header: Some(has_column_header),
            has_row_header: Some(has_row_header),
            ..Default::default()
        }
        .into())
    }

    /// Classify pre-fetched table children into `(header_ids, body_ids)`,
    /// pushing the synthesized `TableRow` / `TableCell` components into
    /// `bag`. Non-`TableRow` children are filtered out *before*
    /// enumeration so the column-header row stays at index 0 even when
    /// `fetch_children` yields stray non-row siblings.
    pub(crate) fn classify_table_rows(
        &self,
        rows: &[BlockResponse],
        has_column_header: bool,
        has_row_header: bool,
        bag: &mut Vec<Component>,
    ) -> (Vec<ComponentId>, Vec<ComponentId>) {
        let table_rows = rows.iter().filter_map(|row| match &row.block {
            Block::TableRow { table_row } => Some((row.id.clone(), table_row)),
            _ => None,
        });
        let mut header_ids = Vec::new();
        let mut body_ids = Vec::new();
        for (row_index, (row_id, table_row)) in table_rows.enumerate() {
            let cell_ids = self.table_row_cells(&row_id, table_row, has_row_header, bag);
            bag.push(
                TableRow {
                    id: row_id.clone(),
                    children: cell_ids,
                    ..Default::default()
                }
                .into(),
            );
            if has_column_header && row_index == 0 {
                header_ids.push(row_id);
            } else {
                body_ids.push(row_id);
            }
        }
        (header_ids, body_ids)
    }

    fn table_row_cells(
        &self,
        row_id: &str,
        row: &TableRowBlock,
        has_row_header: bool,
        bag: &mut Vec<Component>,
    ) -> Vec<ComponentId> {
        let mut cell_ids = Vec::with_capacity(row.cells.len());
        for (col_index, cell_runs) in row.cells.iter().enumerate() {
            let cell_id = child_id(row_id, "cell", col_index);
            let (rt_ids, mut rt) = rich_text::convert_rich_texts(&cell_id, "rich_text", cell_runs);
            bag.append(&mut rt);
            let is_header = has_row_header && col_index == 0;
            bag.push(
                TableCell {
                    id: cell_id.clone(),
                    children: rt_ids,
                    is_header: if is_header { Some(true) } else { None },
                    ..Default::default()
                }
                .into(),
            );
            cell_ids.push(cell_id);
        }
        cell_ids
    }

    /// Convert a Notion `Tab` block (a tabbed container whose direct
    /// children are paragraphs serving as tabs) into a `ContentTabs`
    /// component, with one `ContentTab` per child paragraph.
    ///
    /// Per the Notion spec, each child paragraph's `rich_text` is the
    /// tab's label and its `children` are the tab's panel content.
    /// Non-paragraph children are ignored.
    async fn tab(
        &self,
        id: &str,
        has_children: bool,
        bag: &mut Vec<Component>,
    ) -> Result<Component, Error> {
        let mut tab_ids = Vec::new();
        if has_children {
            let children = self.fetch_children(id).await?;
            for child in &children {
                let Block::Paragraph { paragraph } = &child.block else {
                    continue;
                };
                let tab_id = child.id.clone();
                let (label_ids, mut label_rt) =
                    rich_text::convert_rich_texts(&tab_id, "label", &paragraph.rich_text);
                bag.append(&mut label_rt);
                let mut content_ids = Vec::new();
                if child.has_children {
                    let (cs, comps) = self.convert_children(&tab_id).await?;
                    bag.extend(comps);
                    content_ids = cs;
                }
                bag.push(
                    ContentTab {
                        id: tab_id.clone(),
                        label: ChildList::from_ids(label_ids),
                        content: ChildList::from_ids(content_ids),
                        ..Default::default()
                    }
                    .into(),
                );
                tab_ids.push(tab_id);
            }
        }
        Ok(ContentTabs {
            id: id.into(),
            children: tab_ids,
            ..Default::default()
        }
        .into())
    }

    async fn emit_list(
        &self,
        items: &[BlockResponse],
        style: ListStyle,
        bag: &mut Vec<Component>,
    ) -> Result<ComponentId, Error> {
        let list_id = list_group_id(&items[0].id);
        let mut item_ids = Vec::with_capacity(items.len());
        for item in items {
            let item_id = self.emit_list_item(item, bag).await?;
            item_ids.push(item_id);
        }
        bag.push(
            List {
                id: list_id.clone(),
                children: item_ids,
                style: Some(style),
                ..Default::default()
            }
            .into(),
        );
        Ok(list_id)
    }

    async fn emit_list_item(
        &self,
        item: &BlockResponse,
        bag: &mut Vec<Component>,
    ) -> Result<ComponentId, Error> {
        let id = item.id.clone();
        let (rt_items, has_children, todo_mark) = match &item.block {
            Block::BulletedListItem {
                bulleted_list_item: BulletedListItemBlock { rich_text, .. },
            } => (rich_text.as_slice(), item.has_children, None),
            Block::NumberedListItem {
                numbered_list_item: NumberedListItemBlock { rich_text, .. },
            } => (rich_text.as_slice(), item.has_children, None),
            Block::ToDo {
                to_do: ToDoBlock {
                    rich_text, checked, ..
                },
            } => {
                let mark = if *checked { "☑ " } else { "☐ " };
                (rich_text.as_slice(), item.has_children, Some(mark))
            }
            _ => unreachable!("emit_list_item called on a non-list block"),
        };

        let mut children: Vec<ComponentId> = Vec::new();

        if let Some(mark) = todo_mark {
            let mark_id = child_id(&id, "todo_mark", 0);
            bag.push(
                RichText {
                    id: mark_id.clone(),
                    text: mark.into(),
                    ..Default::default()
                }
                .into(),
            );
            children.push(mark_id);
        }

        let (rt_ids, mut rt) = rich_text::convert_rich_texts(&id, "rich_text", rt_items);
        bag.append(&mut rt);
        children.extend(rt_ids);

        if has_children {
            let (cs, comps) = self.convert_children(&id).await?;
            bag.extend(comps);
            children.extend(cs);
        }

        bag.push(
            ListItem {
                id: id.clone(),
                children,
                ..Default::default()
            }
            .into(),
        );
        Ok(id)
    }
}

// --- free helpers ----------------------------------------------------------

/// One top-level sibling group from a `&[BlockResponse]`: either a
/// single non-list block, or a consecutive run of list items of one
/// style. Used by both the eager `convert_siblings` walk and the
/// streaming orchestrator so the two paths cannot drift on chunk
/// boundaries.
pub(crate) enum SiblingGroup {
    Single {
        index: usize,
    },
    List {
        range: std::ops::Range<usize>,
        style: ListStyle,
    },
}

pub(crate) fn top_level_groups(blocks: &[BlockResponse]) -> Vec<SiblingGroup> {
    let mut groups = Vec::new();
    let mut i = 0;
    while i < blocks.len() {
        if let Some(style) = list_style(&blocks[i].block) {
            let mut j = i + 1;
            while j < blocks.len() && list_style(&blocks[j].block) == Some(style) {
                j += 1;
            }
            groups.push(SiblingGroup::List { range: i..j, style });
            i = j;
        } else {
            groups.push(SiblingGroup::Single { index: i });
            i += 1;
        }
    }
    groups
}

pub(crate) fn list_style(block: &Block) -> Option<ListStyle> {
    match block {
        Block::BulletedListItem { .. } | Block::ToDo { .. } => Some(ListStyle::Unordered),
        Block::NumberedListItem { .. } => Some(ListStyle::Ordered),
        _ => None,
    }
}

fn list_group_id(first_item_id: &str) -> ComponentId {
    format!("{first_item_id}::list")
}

fn rich_text_plain(rt: &NotionRichText) -> String {
    match rt {
        NotionRichText::Text { plain_text, .. }
        | NotionRichText::Mention { plain_text, .. }
        | NotionRichText::Equation { plain_text, .. } => plain_text.clone(),
    }
}

fn katex(id: &str, block: &EquationBlock) -> Component {
    Katex {
        id: id.into(),
        expression: block.expression.clone(),
        ..Default::default()
    }
    .into()
}

fn file_url(file: &NotionFile) -> Option<String> {
    match file {
        NotionFile::External(f) => Some(f.external.url.clone()),
        NotionFile::NotionHosted(f) => Some(f.file.url.clone()),
        _ => None,
    }
}

fn file_name(file: &NotionFile) -> Option<String> {
    match file {
        NotionFile::External(f) => f.name.clone(),
        NotionFile::NotionHosted(f) => f.name.clone(),
        _ => None,
    }
}

fn file_caption(file: &NotionFile) -> Option<&[NotionRichText]> {
    match file {
        NotionFile::External(f) => f.caption.as_deref(),
        NotionFile::NotionHosted(f) => f.caption.as_deref(),
        _ => None,
    }
}

fn file_component(id: &str, file: &NotionFile) -> Component {
    let Some(src) = file_url(file) else {
        return Unsupported {
            id: id.into(),
            details: Some("file:api_uploaded".into()),
            ..Default::default()
        }
        .into();
    };
    FileComponent {
        id: id.into(),
        src,
        name: file_name(file),
        ..Default::default()
    }
    .into()
}

fn notion_page_url(block_id: &str) -> String {
    let stripped: String = block_id.chars().filter(|c| *c != '-').collect();
    format!("https://www.notion.so/{stripped}")
}

/// Render a Notion `EmojiAndIcon` as an inline A2UI component:
/// - `Emoji` becomes a `RichText` carrying the Unicode emoji character
///   (the catalog has no glyph component, and a literal emoji string
///   renders inline correctly without needing an image fetch).
/// - `CustomEmoji` and `File` become an `Icon` pointing at their URL.
/// - `Icon` (Notion's named built-in icons) has no URL we can serve,
///   so it's skipped.
fn inline_icon_component(id: &str, icon: &EmojiAndIcon) -> Option<Component> {
    match icon {
        EmojiAndIcon::Emoji(emoji) => Some(
            RichText {
                id: id.into(),
                text: emoji.emoji.clone().into(),
                ..Default::default()
            }
            .into(),
        ),
        EmojiAndIcon::CustomEmoji(custom) => Some(
            Icon {
                id: id.into(),
                src: custom.custom_emoji.url.clone(),
                alt: Some(custom.custom_emoji.name.clone()),
                ..Default::default()
            }
            .into(),
        ),
        EmojiAndIcon::File(file) => {
            let src = file_url(file)?;
            Some(
                Icon {
                    id: id.into(),
                    src,
                    alt: file_name(file),
                    ..Default::default()
                }
                .into(),
            )
        }
        EmojiAndIcon::Icon(_) => None,
    }
}

/// Use the callout's emoji icon as a hint for the A2UI `CalloutType`.
/// Notion doesn't model a discrete type — we map the common emojis used
/// for callouts to the catalog's enum and fall back to `None` otherwise.
fn callout_type_from_icon(icon: Option<&EmojiAndIcon>) -> Option<CalloutType> {
    let EmojiAndIcon::Emoji(emoji) = icon? else {
        return None;
    };
    match emoji.emoji.as_str() {
        "ℹ️" | "📝" | "🗒️" | "💡" => Some(CalloutType::Note),
        "✅" | "✔️" | "🟢" => Some(CalloutType::Tip),
        "⭐" | "❗" | "📌" => Some(CalloutType::Important),
        "⚠️" | "🚧" | "🟡" => Some(CalloutType::Warning),
        "🛑" | "❌" | "🔴" | "☠️" => Some(CalloutType::Caution),
        _ => None,
    }
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
