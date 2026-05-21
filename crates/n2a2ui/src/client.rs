use a2ui::v0_9::{
    BLOCK_CATALOG_ID, ChildList, Column, Component, ComponentId, CreateSurface, Message, Surface,
    UpdateComponents,
};
use async_stream::try_stream;
use futures::{Stream, TryStreamExt};

use crate::convert::{Converter, SiblingGroup, top_level_groups};
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
    fn converter(&self) -> Converter<'_> {
        Converter {
            notionrs: &self.notionrs_client,
            reqwest: &self.reqwest_client,
            enable_unsupported_block: self.enable_unsupported_block,
            enable_fetch_image_meta: self.enable_fetch_image_meta,
        }
    }

    /// Convert a Notion block (typically a page id) into an A2UI surface.
    ///
    /// Walks `block_id`'s direct children via `get_block_children`
    /// (paginated), recursively converts each block to its A2UI component
    /// equivalent, and wraps the top-level result in a `Column` root.
    /// Unrecognized block types either become `Unsupported` placeholders
    /// or are skipped based on `enable_unsupported_block`.
    pub async fn convert_block(&self, block_id: &str) -> Result<Surface, Error> {
        let (root_children, components) = self.converter().convert_children(block_id).await?;

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

    /// Stream the v0.9 message sequence that renders `block_id` as a
    /// surface, emitting one chunk per top-level sibling group.
    ///
    /// Wire shape:
    ///
    /// 1. `createSurface { surface_id, catalog_id }`
    /// 2. `updateComponents` with the root `Column` carrying an empty
    ///    `children` array — the surface mounts immediately and the
    ///    all-skipped case still produces a valid root.
    /// 3. One `updateComponents` per top-level sibling group (a single
    ///    Notion block, or a consecutive run of list-items collapsed into
    ///    one `List`). Each chunk carries every component synthesized for
    ///    that group plus an updated root `Column` whose `children` grows
    ///    by the new group id. `updateComponents` is upsert-by-id, so
    ///    re-sending the root is cheap and replay-safe.
    ///
    /// The eager [`Client::convert_block_to_messages`] is defined as
    /// `convert_block_stream(...).try_collect().await` — both share this
    /// chunking, so streaming and eager consumers see the same sequence.
    pub fn convert_block_stream<'a>(
        &'a self,
        block_id: &'a str,
        surface_id: impl Into<String>,
    ) -> impl Stream<Item = Result<Message, Error>> + 'a {
        let surface_id = surface_id.into();
        try_stream! {
            let converter = self.converter();
            let blocks = converter.fetch_children(block_id).await?;

            yield Message::from(CreateSurface {
                surface_id: surface_id.clone(),
                catalog_id: BLOCK_CATALOG_ID.into(),
                theme: None,
                send_data_model: None,
            });

            let mut accumulated: Vec<ComponentId> = Vec::new();
            yield Message::from(UpdateComponents {
                surface_id: surface_id.clone(),
                components: vec![root_column(&accumulated)],
            });

            for group in top_level_groups(&blocks) {
                let chunk: Option<(ComponentId, Vec<Component>)> = match group {
                    SiblingGroup::List { range, style } => Some(
                        converter
                            .convert_list_group_to_chunk(&blocks[range], style)
                            .await?,
                    ),
                    SiblingGroup::Single { index } => {
                        converter.convert_single_block_to_chunk(&blocks[index]).await?
                    }
                };

                if let Some((chunk_id, mut components)) = chunk {
                    accumulated.push(chunk_id);
                    components.push(root_column(&accumulated));
                    yield Message::from(UpdateComponents {
                        surface_id: surface_id.clone(),
                        components,
                    });
                }
            }
        }
    }

    /// Convert a Notion block and render it as the v0.9 message sequence
    /// bound to the Elmethis block catalog. Equivalent to collecting
    /// [`Client::convert_block_stream`] — the wire shape is `createSurface`
    /// followed by N+1 `updateComponents` chunks (one empty-root mount,
    /// then one per top-level sibling group).
    pub async fn convert_block_to_messages(
        &self,
        block_id: &str,
        surface_id: impl Into<String>,
    ) -> Result<Vec<Message>, Error> {
        self.convert_block_stream(block_id, surface_id)
            .try_collect()
            .await
    }
}

fn root_column(children: &[ComponentId]) -> Component {
    Column {
        id: ROOT_ID.into(),
        children: ChildList::from_ids(children.to_vec()),
        ..Default::default()
    }
    .into()
}
