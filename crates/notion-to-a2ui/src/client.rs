use a2ui::v0_9::{BLOCK_CATALOG_ID, ChildList, Column, Message, Surface};
use futures::TryStreamExt;
use notionrs::PaginateExt;
use notionrs::types::prelude::BlockResponse;

use crate::convert;
use crate::error::Error;
use crate::id::ROOT_ID;

#[derive(Debug)]
pub struct Client {
    pub notionrs_client: notionrs::client::Client,
    pub reqwest_client: reqwest::Client,

    /// If true, unsupported blocks render as `Unsupported` components.
    /// If false, they are skipped.
    pub enable_unsupported_block: bool,

    /// If true, image blocks are fetched once to read their intrinsic
    /// dimensions and emit `width`/`height` on the resulting `BlockImage`.
    pub enable_fetch_image_meta: bool,
}

impl Client {
    /// Convert a Notion block (typically a page id) into an A2UI surface.
    ///
    /// Walks `block_id`'s direct children via `get_block_children` (paginated),
    /// converts each known block into an A2UI component, and wraps them in a
    /// `Column` root. Currently only `paragraph` and `heading_1..4` are
    /// recognized — see `crate::convert`. Other kinds either become
    /// `Unsupported` placeholders or are skipped based on
    /// `enable_unsupported_block`.
    pub async fn convert_block(&self, block_id: &str) -> Result<Surface, Error> {
        let blocks: Vec<BlockResponse> = self
            .notionrs_client
            .get_block_children()
            .block_id(block_id)
            .into_stream()
            .try_collect()
            .await?;

        let mut surface = Surface::new(ROOT_ID);
        let mut root_children: Vec<String> = Vec::new();
        let mut pending: Vec<a2ui::v0_9::Component> = Vec::new();

        for block in &blocks {
            let Some((child_id, components)) =
                convert::convert_block(block, self.enable_unsupported_block)
            else {
                continue;
            };
            root_children.push(child_id);
            pending.extend(components);
        }

        surface.insert(
            Column {
                id: ROOT_ID.into(),
                children: ChildList::from_ids(root_children),
                ..Default::default()
            }
            .into(),
        );
        for component in pending {
            surface.insert(component);
        }

        Ok(surface)
    }

    /// Convert a Notion block and render it as the v0.9 message sequence
    /// (`createSurface` + `updateComponents`) bound to the Elmethis block
    /// catalog. Equivalent to `convert_block(...).to_messages(...)`.
    pub async fn convert_block_to_messages(
        &self,
        block_id: &str,
        surface_id: impl Into<String>,
    ) -> Result<Vec<Message>, Error> {
        let surface = self.convert_block(block_id).await?;
        Ok(surface.to_messages(surface_id, BLOCK_CATALOG_ID))
    }
}
