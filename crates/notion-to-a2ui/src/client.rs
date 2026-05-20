use a2ui::v0_9::{BLOCK_CATALOG_ID, ChildList, Column, Message, Surface};

use crate::convert::Converter;
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
    /// Walks `block_id`'s direct children via `get_block_children`
    /// (paginated), recursively converts each block to its A2UI component
    /// equivalent, and wraps the top-level result in a `Column` root.
    /// Unrecognized block types either become `Unsupported` placeholders
    /// or are skipped based on `enable_unsupported_block`.
    pub async fn convert_block(&self, block_id: &str) -> Result<Surface, Error> {
        let converter = Converter {
            notionrs: &self.notionrs_client,
            reqwest: &self.reqwest_client,
            enable_unsupported_block: self.enable_unsupported_block,
            enable_fetch_image_meta: self.enable_fetch_image_meta,
        };
        let (root_children, components) = converter.convert_children(block_id).await?;

        let mut surface = Surface::new(ROOT_ID);
        surface.insert(
            Column {
                id: ROOT_ID.into(),
                children: ChildList::from_ids(root_children),
                ..Default::default()
            }
            .into(),
        );
        for component in components {
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

