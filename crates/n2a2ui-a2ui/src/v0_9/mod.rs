mod block_catalog;
mod common;
mod message;
mod surface;

pub use block_catalog::*;
pub use common::*;
pub use message::*;
pub use surface::*;

/// Stable URI identifying the Elmethis Notion Block Catalog this crate models.
pub const NOTION_BLOCK_CATALOG_ID: &str =
    "https://46ki75.github.io/elmethis/a2ui/v0_9/notion_block_catalog.json";
