//! Sanity tests for the vendored A2UI v0.9 block catalog.

use a2ui::v0_9::*;

const BLOCK_CATALOG_JSON: &str = include_str!("../schemas/v0_9/block_catalog.json");

#[test]
fn vendored_catalog_is_valid_json_and_has_expected_id() {
    let value: serde_json::Value = serde_json::from_str(BLOCK_CATALOG_JSON)
        .expect("vendored block_catalog.json must be valid JSON");
    let id = value
        .get("$id")
        .and_then(|v| v.as_str())
        .expect("catalog must declare $id");
    assert_eq!(id, BLOCK_CATALOG_ID);
}

#[test]
fn component_round_trip() {
    let para: Component = Paragraph {
        id: "p1".into(),
        children: ChildList::from_ids(["t1"]),
        ..Default::default()
    }
    .into();
    let rich: Component = RichText {
        id: "t1".into(),
        text: "hello".into(),
        decoration: Some(vec![Decoration::Bold]),
        ..Default::default()
    }
    .into();

    for component in [para, rich] {
        let json = serde_json::to_string(&component).unwrap();
        let parsed: Component = serde_json::from_str(&json).unwrap();
        assert_eq!(component, parsed);
    }
}

#[test]
fn surface_round_trip_preserves_order() {
    let mut surface = Surface::new("root");
    surface.insert(
        Column {
            id: "root".into(),
            children: ChildList::from_ids(["p1"]),
            ..Default::default()
        }
        .into(),
    );
    surface.insert(
        Paragraph {
            id: "p1".into(),
            children: ChildList::from_ids(["t1"]),
            ..Default::default()
        }
        .into(),
    );
    surface.insert(
        RichText {
            id: "t1".into(),
            text: "hello".into(),
            ..Default::default()
        }
        .into(),
    );

    let json = serde_json::to_string(&surface).unwrap();
    let parsed: Surface = serde_json::from_str(&json).unwrap();
    assert_eq!(surface, parsed);
    assert_eq!(parsed.components.len(), 3);
    assert_eq!(parsed.components.get_index(0).unwrap().0, "root");
    assert_eq!(parsed.components.get_index(2).unwrap().0, "t1");
}

#[test]
fn callout_type_field_serializes_as_type() {
    let callout: Component = Callout {
        id: "c1".into(),
        children: ChildList::from_ids(["p1"]),
        callout_type: Some(CalloutType::Warning),
        ..Default::default()
    }
    .into();
    let json = serde_json::to_string(&callout).unwrap();
    assert!(json.contains("\"type\":\"warning\""));
    assert!(!json.contains("calloutType"));
    let parsed: Component = serde_json::from_str(&json).unwrap();
    assert_eq!(callout, parsed);
}

#[test]
fn heading_level_serializes_as_number() {
    let h2: Component = Heading {
        id: "h2".into(),
        level: HeadingLevel::H2,
        children: ChildList::from_ids(["t1"]),
        ..Default::default()
    }
    .into();
    let json = serde_json::to_string(&h2).unwrap();
    assert!(json.contains("\"level\":2"));
}

#[test]
fn dynamic_string_round_trips_literal_binding_and_call() {
    let literal: DynamicString = "hello".into();
    let binding = DynamicString::Binding(DataBinding {
        path: "/user/name".into(),
    });
    let call = DynamicString::Call(FunctionCall {
        call: "trim".into(),
        args: None,
        return_type: Some("string".into()),
    });

    for ds in [literal, binding, call] {
        let json = serde_json::to_string(&ds).unwrap();
        let parsed: DynamicString = serde_json::from_str(&json).unwrap();
        assert_eq!(ds, parsed);
    }
}
