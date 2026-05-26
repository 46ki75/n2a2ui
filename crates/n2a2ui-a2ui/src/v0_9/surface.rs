use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::block_catalog::Component;
use super::common::ComponentId;

/// A surface — the renderable unit in A2UI v0.9.
///
/// Pairs the designated root component id with the flat adjacency-list of
/// components reachable from it. Mirrors the shape of a v0.9
/// `createSurface` message minus the wire envelope.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Surface {
    pub root: ComponentId,
    pub components: IndexMap<ComponentId, Component>,
}

impl Surface {
    pub fn new(root: impl Into<ComponentId>) -> Self {
        Self {
            root: root.into(),
            components: IndexMap::new(),
        }
    }

    /// Insert a component keyed by its `id`.
    pub fn insert(&mut self, component: Component) {
        let id = component.id().to_string();
        self.components.insert(id, component);
    }
}
