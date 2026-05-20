use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub type ComponentId = String;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessibilityAttributes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<DynamicString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<DynamicString>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChildList {
    Static(Vec<ComponentId>),
    Template(ChildListTemplate),
}

impl Default for ChildList {
    fn default() -> Self {
        Self::Static(Vec::new())
    }
}

impl ChildList {
    pub fn from_ids<I, S>(ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::Static(ids.into_iter().map(Into::into).collect())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildListTemplate {
    pub component_id: ComponentId,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataBinding {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCall {
    pub call: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<IndexMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DynamicString {
    Literal(String),
    Binding(DataBinding),
    Call(FunctionCall),
}

impl Default for DynamicString {
    fn default() -> Self {
        Self::Literal(String::new())
    }
}

impl DynamicString {
    pub fn literal(s: impl Into<String>) -> Self {
        Self::Literal(s.into())
    }
}

impl From<String> for DynamicString {
    fn from(s: String) -> Self {
        Self::Literal(s)
    }
}

impl From<&str> for DynamicString {
    fn from(s: &str) -> Self {
        Self::Literal(s.to_string())
    }
}
