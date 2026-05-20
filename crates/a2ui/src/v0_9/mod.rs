mod block_catalog;
mod common;
mod surface;

pub use block_catalog::*;
pub use common::*;
pub use surface::*;

/// Stable URI identifying the Elmethis Block Catalog this crate models.
pub const BLOCK_CATALOG_ID: &str =
    "https://46ki75.github.io/elmethis/a2ui/v0_9/block_catalog.json";
