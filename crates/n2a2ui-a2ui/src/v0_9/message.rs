//! A2UI v0.9 server-to-client message envelopes.
//!
//! On the wire each top-level message is an object with a `version` field
//! plus exactly one of four discriminator keys (`createSurface`,
//! `updateComponents`, `updateDataModel`, `deleteSurface`). A `Surface`
//! renders as the pair `[createSurface, updateComponents]` — see
//! [`Surface::to_messages`].

use serde::{Deserialize, Serialize};

use super::block_catalog::Component;
use super::surface::Surface;

/// The protocol version string emitted in every v0.9 wire message.
pub const VERSION: &str = "v0.9";

/// A single v0.9 server-to-client message envelope.
///
/// Serialized form:
/// ```json
/// { "version": "v0.9", "createSurface": { ... } }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub version: String,
    #[serde(flatten)]
    pub body: MessageBody,
}

impl Message {
    /// Wraps a body in a `version = "v0.9"` envelope.
    pub fn new(body: impl Into<MessageBody>) -> Self {
        Self {
            version: VERSION.into(),
            body: body.into(),
        }
    }
}

/// The body discriminator of a v0.9 message. Each variant flattens into
/// the envelope as `{ "<variantName>": { ... } }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageBody {
    CreateSurface(CreateSurface),
    UpdateComponents(UpdateComponents),
    UpdateDataModel(UpdateDataModel),
    DeleteSurface(DeleteSurface),
}

macro_rules! body_from {
    ($($variant:ident),* $(,)?) => {
        $(
            impl From<$variant> for MessageBody {
                fn from(value: $variant) -> Self {
                    Self::$variant(value)
                }
            }
            impl From<$variant> for Message {
                fn from(value: $variant) -> Self {
                    Message::new(value)
                }
            }
        )*
    };
}
body_from!(
    CreateSurface,
    UpdateComponents,
    UpdateDataModel,
    DeleteSurface
);

/// `createSurface` — create a surface and bind it to a catalog and theme.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSurface {
    pub surface_id: String,
    pub catalog_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_data_model: Option<bool>,
}

/// `updateComponents` — add or replace components in a surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateComponents {
    pub surface_id: String,
    pub components: Vec<Component>,
}

/// `updateDataModel` — upsert (or remove) a value at a JSON-Pointer path.
///
/// Omitting `value` removes the key. Omitting `path` (or setting it to
/// `"/"`) replaces the whole model.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDataModel {
    pub surface_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

/// `deleteSurface` — remove the surface and all its state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSurface {
    pub surface_id: String,
}

impl Surface {
    /// Render this surface as a v0.9 message sequence: a `createSurface`
    /// envelope followed by one `updateComponents` carrying every
    /// component in insertion order.
    ///
    /// The two-message split mirrors the wire protocol — `createSurface`
    /// binds the catalog, `updateComponents` delivers the tree.
    pub fn to_messages(
        &self,
        surface_id: impl Into<String>,
        catalog_id: impl Into<String>,
    ) -> Vec<Message> {
        let surface_id = surface_id.into();
        vec![
            CreateSurface {
                surface_id: surface_id.clone(),
                catalog_id: catalog_id.into(),
                theme: None,
                send_data_model: None,
            }
            .into(),
            UpdateComponents {
                surface_id,
                components: self.components.values().cloned().collect(),
            }
            .into(),
        ]
    }
}
