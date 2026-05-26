use serde::{Deserialize, Serialize};

use super::common::{AccessibilityAttributes, ChildList, ComponentId, DynamicString};

/// Tagged union of every component in the Elmethis Block Catalog.
///
/// The discriminator field is `component` (per the v0.9 protocol). The
/// `id` field is required and lives in each variant's struct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "component")]
pub enum Component {
    RichText(RichText),
    LinkText(LinkText),
    Icon(Icon),
    Row(Row),
    Column(Column),
    ColumnList(ColumnList),
    Heading(Heading),
    Paragraph(Paragraph),
    List(List),
    ListItem(ListItem),
    BlockQuote(BlockQuote),
    Callout(Callout),
    Divider(Divider),
    Toggle(Toggle),
    Bookmark(Bookmark),
    File(File),
    BlockImage(BlockImage),
    CodeBlock(CodeBlock),
    Katex(Katex),
    Mermaid(Mermaid),
    ContentTab(ContentTab),
    ContentTabs(ContentTabs),
    Table(Table),
    TableRow(TableRow),
    TableCell(TableCell),
    Unsupported(Unsupported),
}

macro_rules! component_impls {
    ($($variant:ident),* $(,)?) => {
        impl Component {
            pub fn id(&self) -> &str {
                match self {
                    $( Self::$variant(c) => &c.id, )*
                }
            }
        }
        $(
            impl From<$variant> for Component {
                fn from(value: $variant) -> Self {
                    Self::$variant(value)
                }
            }
        )*
    };
}

component_impls!(
    RichText,
    LinkText,
    Icon,
    Row,
    Column,
    ColumnList,
    Heading,
    Paragraph,
    List,
    ListItem,
    BlockQuote,
    Callout,
    Divider,
    Toggle,
    Bookmark,
    File,
    BlockImage,
    CodeBlock,
    Katex,
    Mermaid,
    ContentTab,
    ContentTabs,
    Table,
    TableRow,
    TableCell,
    Unsupported,
);

// --- shared enums -----------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decoration {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Code,
    Katex,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Justify {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    Start,
    End,
    Center,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListStyle {
    Unordered,
    Ordered,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CalloutType {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(into = "u8", try_from = "u8")]
pub enum HeadingLevel {
    #[default]
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
}

impl From<HeadingLevel> for u8 {
    fn from(level: HeadingLevel) -> Self {
        match level {
            HeadingLevel::H1 => 1,
            HeadingLevel::H2 => 2,
            HeadingLevel::H3 => 3,
            HeadingLevel::H4 => 4,
            HeadingLevel::H5 => 5,
            HeadingLevel::H6 => 6,
        }
    }
}

impl TryFrom<u8> for HeadingLevel {
    type Error = String;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            1 => Self::H1,
            2 => Self::H2,
            3 => Self::H3,
            4 => Self::H4,
            5 => Self::H5,
            6 => Self::H6,
            _ => return Err(format!("invalid heading level {value} (expected 1..=6)")),
        })
    }
}

// --- inline components ------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RichText {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub text: DynamicString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoration: Option<Vec<Decoration>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ruby: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkText {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub text: DynamicString,
    pub href: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Icon {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub src: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
}

// --- layout -----------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub children: ChildList,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justify: Option<Justify>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub children: ChildList,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justify: Option<Justify>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_ratio: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnList {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub children: Vec<ComponentId>,
}

// --- block typography -------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Heading {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub level: HeadingLevel,
    pub children: ChildList,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paragraph {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub children: ChildList,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct List {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub children: Vec<ComponentId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<ListStyle>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListItem {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub children: Vec<ComponentId>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockQuote {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub children: ChildList,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cite: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Callout {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub children: ChildList,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub callout_type: Option<CalloutType>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Divider {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Toggle {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub summary: ChildList,
    pub children: ChildList,
}

// --- media ------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bookmark {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub src: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockImage {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub src: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srcset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sizes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

// --- code / math / diagram --------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeBlock {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Katex {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub expression: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mermaid {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub code: String,
}

// --- tabs -------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentTab {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    /// Inline content that renders the tab's button. Typically a list of
    /// `RichText` / `LinkText` / `Icon` ids.
    pub label: ChildList,
    /// Block content shown when this tab is active. Use a static list of
    /// component ids, or a template binding for data-driven panels.
    pub content: ChildList,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentTabs {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub children: Vec<ComponentId>,
}

// --- table ------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub body: Vec<ComponentId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<Vec<ComponentId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_column_header: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_row_header: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRow {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub children: Vec<ComponentId>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableCell {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub children: Vec<ComponentId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_header: Option<bool>,
}

// --- fallback ---------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unsupported {
    pub id: ComponentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<AccessibilityAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}
