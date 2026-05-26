//! Sanity tests for the vendored A2UI v0.9 block catalog.

use n2a2ui_a2ui::v0_9::*;

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
        let json = serde_json::to_string(&component).expect("serialize component");
        let parsed: Component = serde_json::from_str(&json).expect("deserialize component");
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

    let json = serde_json::to_string(&surface).expect("serialize surface");
    let parsed: Surface = serde_json::from_str(&json).expect("deserialize surface");
    assert_eq!(surface, parsed);
    assert_eq!(parsed.components.len(), 3);
    assert_eq!(
        parsed
            .components
            .get_index(0)
            .expect("index 0 must exist")
            .0,
        "root"
    );
    assert_eq!(
        parsed
            .components
            .get_index(2)
            .expect("index 2 must exist")
            .0,
        "t1"
    );
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
    let json = serde_json::to_string(&callout).expect("serialize callout");
    assert!(json.contains("\"type\":\"warning\""));
    assert!(!json.contains("calloutType"));
    let parsed: Component = serde_json::from_str(&json).expect("deserialize callout");
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
    let json = serde_json::to_string(&h2).expect("serialize heading");
    assert!(json.contains("\"level\":2"));
}

#[test]
fn surface_to_messages_emits_create_then_update() {
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
            ..Default::default()
        }
        .into(),
    );

    let messages = surface.to_messages("my-surface", BLOCK_CATALOG_ID);
    assert_eq!(messages.len(), 2);

    match &messages[0].body {
        MessageBody::CreateSurface(cs) => {
            assert_eq!(cs.surface_id, "my-surface");
            assert_eq!(cs.catalog_id, BLOCK_CATALOG_ID);
        }
        other => panic!("expected CreateSurface, got {other:?}"),
    }
    match &messages[1].body {
        MessageBody::UpdateComponents(uc) => {
            assert_eq!(uc.surface_id, "my-surface");
            assert_eq!(uc.components.len(), 2);
            assert_eq!(uc.components[0].id(), "root");
            assert_eq!(uc.components[1].id(), "p1");
        }
        other => panic!("expected UpdateComponents, got {other:?}"),
    }
}

#[test]
fn message_envelope_wire_shape_round_trips() {
    let msg = Message::new(CreateSurface {
        surface_id: "s1".into(),
        catalog_id: BLOCK_CATALOG_ID.into(),
        theme: None,
        send_data_model: Some(true),
    });
    let json = serde_json::to_value(&msg).expect("serialize message");
    assert_eq!(json["version"], "v0.9");
    assert_eq!(json["createSurface"]["surfaceId"], "s1");
    assert_eq!(json["createSurface"]["sendDataModel"], true);

    let parsed: Message = serde_json::from_value(json).expect("deserialize message");
    assert_eq!(parsed, msg);
}

#[test]
fn content_tab_uses_child_list_label_and_content() {
    let tab: Component = ContentTab {
        id: "tab1".into(),
        label: ChildList::from_ids(["tab1_label_rt"]),
        content: ChildList::from_ids(["tab1_para"]),
        ..Default::default()
    }
    .into();
    let json = serde_json::to_value(&tab).expect("serialize tab");
    assert_eq!(json["component"], "ContentTab");
    assert_eq!(json["label"][0], "tab1_label_rt");
    assert_eq!(json["content"][0], "tab1_para");
    assert!(
        json.get("labels").is_none() && json.get("contents").is_none(),
        "old labels/contents fields must be gone"
    );

    let parsed: Component = serde_json::from_value(json).expect("deserialize tab");
    assert_eq!(parsed, tab);
}

#[test]
fn data_binding_rejects_unknown_fields() {
    // Schema: $defs/DataBinding has `additionalProperties: false`
    // (block_catalog.json:1048). Today this passes because the Rust struct
    // lacks `#[serde(deny_unknown_fields)]`.
    let json = serde_json::json!({ "path": "/x", "extra": "junk" });
    let result: Result<DataBinding, _> = serde_json::from_value(json);
    assert!(
        result.is_err(),
        "DataBinding must reject unknown fields per schema additionalProperties: false; \
         got Ok({:?})",
        result.expect("checked is_err above")
    );
}

#[test]
fn child_list_template_rejects_unknown_fields() {
    // Schema: $defs/ChildList → Template variant has `additionalProperties: false`
    // (block_catalog.json:1033). Today this passes because the Rust struct
    // lacks `#[serde(deny_unknown_fields)]`.
    let json = serde_json::json!({ "componentId": "x", "path": "/y", "extra": "junk" });
    let result: Result<ChildListTemplate, _> = serde_json::from_value(json);
    assert!(
        result.is_err(),
        "ChildListTemplate must reject unknown fields per schema additionalProperties: false; \
         got Ok({:?})",
        result.expect("checked is_err above")
    );
}

#[test]
fn dynamic_string_with_call_field_must_parse_as_call_not_binding() {
    // Per the v0.9 schema, DataBinding has `additionalProperties: false`, so
    // {"path": "/x", "call": "trim"} is NOT a valid DataBinding — it must
    // parse as the FunctionCall arm of DynamicString (FunctionCall allows
    // extra props per the schema).
    //
    // Today this misparses as `Binding` because the untagged enum tries
    // `Binding` first and DataBinding silently accepts (and drops) the
    // `call` field — real data loss on the wire.
    let json = serde_json::json!({ "path": "/x", "call": "trim" });
    let parsed: DynamicString = serde_json::from_value(json).expect("deserialize dynamic string");
    match parsed {
        DynamicString::Call(fc) => {
            assert_eq!(fc.call, "trim");
        }
        DynamicString::Binding(b) => {
            panic!("expected Call but got Binding({b:?}); the `call` field was silently dropped")
        }
        other => panic!("expected Call, got {other:?}"),
    }
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
        let json = serde_json::to_string(&ds).expect("serialize dynamic string");
        let parsed: DynamicString =
            serde_json::from_str(&json).expect("deserialize dynamic string");
        assert_eq!(ds, parsed);
    }
}
