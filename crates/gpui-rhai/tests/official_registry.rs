mod common;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use common::{
    compile, compile_official, engine_with, environment, lifecycle_for, lifecycle_with_schema,
    mount, new_runtime, official_modules, official_source, render, root_path, try_render_in,
};
use gpui_rhai::{
    AssetData, ColorValue, ComponentStateSchema, InMemoryAssetProvider, ModuleId, OpaqueHandle,
    RuntimeEngine, ScriptCallback, ScriptLifecycle, StateField, UiNodeKind, UiRuntimeState,
    UiValue, ValueSchema, parse_component_header,
};
use gpui_rhai_registry as registry;

const DATE_PICKER_TEST_APP: &str = r#"
    import "components/date_picker" as date_picker;
    fn changed(ctx, value) { () }
    fn view(ctx) {
        date_picker::DatePicker(#{
            key: "appointment", label: "Appointment date", value: "2026-09-01",
            min_date: "2026-08-30", max_date: "2026-12-31",
            placeholder: "Appointment", clearable: true,
            presets: [#{ label: "Launch", value: "2026-09-01" }],
            on_change: Fn("changed")
        })
    }
"#;
const COMPOSITES_APP: &str = r#"
    import "components/form_field" as form_field;
    import "components/collapsible" as collapsible;
    import "components/accordion" as accordion;
    import "components/tabs" as tabs;
    fn changed(ctx, value) { () }
    fn view(ctx) {
        column([
            form_field::FormField(#{
                id: "name", label: "Name", control: text("Ada"), required: true,
                description: "Public name", error: "Required"
            }),
            collapsible::Collapsible(#{
                key: "details", open: true, trigger: text("Details"),
                content: text("Content"), content_height: 40,
                on_open_change: Fn("changed")
            }),
            accordion::Accordion(#{
                key: "faq", expanded: ["one"], mode: "multiple",
                items: [
                    #{ key: "one", title: "One", content: text("First"), content_height: 36 },
                    #{ key: "two", title: "Two", content: text("Second"), content_height: 36 }
                ],
                on_change: Fn("changed")
            }),
            tabs::Tabs(#{
                value: "general", label: "Settings",
                tabs: [
                    #{ value: "general", label: "General", content: text("General panel") },
                    #{ value: "advanced", label: "Advanced", content: text("Advanced panel") }
                ],
                on_change: Fn("changed")
            })
        ])
    }
"#;

/// The bundled English locale, as the date and pagination components read it.
fn english(engine: &RuntimeEngine) -> gpui_rhai::LocaleManager {
    let locale =
        gpui_rhai::load_locale_source(engine.engine(), "en.rhai", registry::EN_LOCALE).unwrap();
    gpui_rhai::LocaleManager::new([locale], "en", "en").unwrap()
}

fn contains_global_call(source: &str, name: &str) -> bool {
    let needle = format!("{name}(");
    source.match_indices(&needle).any(|(index, _)| {
        source[..index]
            .chars()
            .next_back()
            .is_none_or(|character| !(character.is_ascii_alphanumeric() || character == '_'))
    })
}

#[test]
fn official_registry_has_no_privileged_component_constructors() {
    let forbidden = [
        "table",
        "combobox",
        "select",
        "date_picker",
        "toast_host",
        "virtual_list",
    ];
    for (id, source) in official_modules() {
        for constructor in &forbidden {
            assert!(
                !contains_global_call(source, constructor),
                "official source `{id}` uses privileged constructor `{constructor}`"
            );
        }
    }
}

fn target_handler(node: &gpui_rhai::UiNode, event: &str) -> (ScriptCallback, UiValue) {
    (
        node.handler(event)
            .and_then(gpui_rhai::UiEventHandler::as_script)
            .cloned()
            .unwrap(),
        node.handler_payload(event).cloned().unwrap(),
    )
}

fn assert_table_columns_delegate_width_to_native_track(columns: &[gpui_rhai::UiNode]) {
    // Rhai cells remain ordinary compositions. The root native Table plan
    // owns viewport-based widths and sampling, rather than a second set of
    // per-cell relative/flex rules or width-override signal consumers.
    for column in columns {
        assert_eq!(column.style().base.width, None);
        assert_eq!(column.style().base.flex_basis, None);
        assert_eq!(column.style().base.flex_grow, None);
        assert_eq!(column.style().base.flex_grow_weight, None);
        assert_eq!(column.style().base.flex_shrink, Some(false));
        assert!(
            column.signal_bindings().all(|(property, _)| !matches!(
                property,
                gpui_rhai::SignalProperty::Width | gpui_rhai::SignalProperty::WidthOverride
            )),
            "native Table plan must be the only column-width consumer"
        );
    }
}

fn assert_resizable_table_autofit_measurements(
    headers: &[gpui_rhai::UiNode],
    cells: &[gpui_rhai::UiNode],
) {
    assert!(
        headers
            .iter()
            .all(|header| contains_primitive(header, "gpui_rhai.intrinsic_text_measure"))
    );
    assert!(
        cells
            .iter()
            .all(|cell| contains_primitive(cell, "gpui_rhai.intrinsic_text_measure"))
    );
    for (header, cell) in headers.iter().zip(cells) {
        let header_group = find_primitive(header, "gpui_rhai.intrinsic_text_measure")
            .unwrap()
            .props
            .get("group");
        let body_group = find_primitive(cell, "gpui_rhai.intrinsic_text_measure")
            .unwrap()
            .props
            .get("group");
        assert!(
            matches!(header_group, Some(gpui_rhai::PrimitiveValue::Signal(signal))
            if signal.id().kind() == gpui_rhai::SignalKind::OptionalFloat)
        );
        assert_eq!(
            header_group, body_group,
            "auto-fit must measure the same native column group"
        );
    }
}

fn assert_table_resize_source_widths(headers: &[gpui_rhai::UiNode], widths: &[(&str, i64)]) {
    let handles = headers
        .iter()
        .flat_map(|header| resize_handles(header))
        .collect::<Vec<_>>();
    assert_eq!(handles.len(), widths.len());
    for (handle, (kind, value)) in handles.iter().zip(widths) {
        let UiNodeKind::Custom { primitive } = handle.kind() else {
            unreachable!()
        };
        assert_eq!(
            primitive.props.get("source_kind"),
            Some(&gpui_rhai::PrimitiveValue::Data(UiValue::String(
                (*kind).to_owned()
            )))
        );
        assert_eq!(
            primitive.props.get("source_value"),
            Some(&gpui_rhai::PrimitiveValue::Data(UiValue::Integer(*value)))
        );
        assert!(
            matches!(primitive.props.get("signal"),Some(gpui_rhai::PrimitiveValue::Signal(signal))
            if signal.id().kind() == gpui_rhai::SignalKind::OptionalFloat)
        );
    }
}

fn resize_handles(header: &gpui_rhai::UiNode) -> Vec<&gpui_rhai::UiNode> {
    let UiNodeKind::Box { children } = header.kind() else {
        unreachable!()
    };
    children
        .iter()
        .filter(|node| {
            matches!(node.kind(), UiNodeKind::Custom { primitive }
                if primitive.primitive.as_str() == "gpui_rhai.column_resize")
        })
        .collect()
}

fn assert_table_resize_handles(headers: &[gpui_rhai::UiNode], titles: &[&str]) {
    // Inline edges resolve against the renderer direction, including RTL.
    let edge = |node: &gpui_rhai::UiNode, side: &str| {
        let style = &node.style().base;
        assert_eq!(style.position, Some(gpui_rhai::PositionMode::Absolute));
        if side == "start" {
            assert_eq!(
                style.inset_start,
                Some(gpui_rhai::LayoutLength::Signed(
                    gpui_rhai::SignedLength::Pixels(-4.0)
                ))
            );
        } else {
            assert_eq!(
                style.inset_end,
                Some(gpui_rhai::LayoutLength::Definite(
                    gpui_rhai::Length::Pixels(0.0)
                ))
            );
        }
    };
    assert_eq!(resize_handles(&headers[0]), [] as [&gpui_rhai::UiNode; 0]);
    for (index, header) in headers.iter().enumerate().skip(1) {
        let handles = resize_handles(header);
        let last = index + 1 == headers.len();
        assert_eq!(handles.len(), if last { 2 } else { 1 }, "header {index}");
        // The handle at the left edge of cell i resizes column i - 1.
        assert_eq!(
            handles[0].attributes().get("label"),
            Some(&UiValue::String(format!(
                "Resize {} column",
                titles[index - 1]
            )))
        );
        edge(handles[0], "start");
        if last {
            assert_eq!(
                handles[1].attributes().get("label"),
                Some(&UiValue::String(format!("Resize {} column", titles[index])))
            );
            edge(handles[1], "end");
        }
    }
}

fn assert_table_has_no_width_signals(root: &gpui_rhai::UiNode) {
    let UiNodeKind::Box { children } = root.kind() else {
        panic!("Table root must be a Box")
    };
    let UiNodeKind::Box {
        children: header_cells,
    } = children[0].kind()
    else {
        panic!("Table header must be a row")
    };
    assert!(
        header_cells
            .iter()
            .all(|cell| cell.signal_bindings().next().is_none()),
        "non-resizable Tables must not pay the native width-signal cost"
    );
    let UiNodeKind::VirtualCollection { spec } = children[1].kind() else {
        panic!("Table body must be a virtual collection")
    };
    let UiNodeKind::Box { children: cells } = spec.realized[&1].kind() else {
        panic!("realized Table row must be a Box")
    };
    assert!(
        cells
            .iter()
            .all(|cell| cell.signal_bindings().next().is_none())
    );
}

fn invoke_and_render(
    lifecycle: &mut ScriptLifecycle,
    engine: &mut RuntimeEngine,
    callback: &ScriptCallback,
    payload: UiValue,
) {
    let _ = lifecycle
        .invoke_callback_transactional(engine, callback, payload)
        .unwrap();
    assert!(lifecycle.render_dirty(engine).unwrap());
}

fn contains_overlay(node: &gpui_rhai::UiNode) -> bool {
    match node.kind() {
        UiNodeKind::Overlay { .. } => true,
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().any(contains_overlay)
        }
        _ => false,
    }
}

fn contains_primitive(node: &gpui_rhai::UiNode, id: &str) -> bool {
    match node.kind() {
        UiNodeKind::Custom { primitive } => primitive.primitive.as_str() == id,
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().any(|child| contains_primitive(child, id))
        }
        UiNodeKind::Overlay {
            trigger, content, ..
        } => contains_primitive(trigger, id) || contains_primitive(content, id),
        UiNodeKind::ErrorBoundary { child, fallback } => {
            contains_primitive(child, id) || contains_primitive(fallback, id)
        }
        _ => false,
    }
}

fn find_primitive<'node>(
    node: &'node gpui_rhai::UiNode,
    id: &str,
) -> Option<&'node gpui_rhai::PrimitiveNode> {
    match node.kind() {
        UiNodeKind::Custom { primitive } if primitive.primitive.as_str() == id => Some(primitive),
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(|child| find_primitive(child, id))
        }
        _ => None,
    }
}

fn find_virtual_collection(node: &gpui_rhai::UiNode) -> Option<&gpui_rhai::UiNode> {
    match node.kind() {
        UiNodeKind::VirtualCollection { .. } => Some(node),
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(find_virtual_collection)
        }
        UiNodeKind::Overlay {
            trigger, content, ..
        } => find_virtual_collection(trigger).or_else(|| find_virtual_collection(content)),
        _ => None,
    }
}

/// The container that sizes a fill-height virtual collection.
fn find_virtual_collection_viewport(node: &gpui_rhai::UiNode) -> Option<&gpui_rhai::UiNode> {
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            if children
                .iter()
                .any(|child| matches!(child.kind(), UiNodeKind::VirtualCollection { .. }))
            {
                Some(node)
            } else {
                children.iter().find_map(find_virtual_collection_viewport)
            }
        }
        UiNodeKind::Overlay {
            trigger, content, ..
        } => find_virtual_collection_viewport(trigger)
            .or_else(|| find_virtual_collection_viewport(content)),
        _ => None,
    }
}

fn find_label<'a>(node: &'a gpui_rhai::UiNode, label: &str) -> Option<&'a gpui_rhai::UiNode> {
    if node.attributes().get("label") == Some(&UiValue::String(label.to_owned())) {
        return Some(node);
    }
    match node.kind() {
        UiNodeKind::Box { children } => children.iter().find_map(|child| find_label(child, label)),
        UiNodeKind::Overlay {
            trigger, content, ..
        } => find_label(trigger, label).or_else(|| find_label(content, label)),
        UiNodeKind::ErrorBoundary { child, fallback } => {
            find_label(child, label).or_else(|| find_label(fallback, label))
        }
        _ => None,
    }
}

fn find_role<'a>(node: &'a gpui_rhai::UiNode, role: &str) -> Option<&'a gpui_rhai::UiNode> {
    if node.attributes().get("role") == Some(&UiValue::String(role.to_owned())) {
        return Some(node);
    }
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(|child| find_role(child, role))
        }
        _ => None,
    }
}

#[test]
fn schema_docs_are_read_and_exported() {
    // `doc` on props, nested object fields, events and slots is metadata for references
    // and editors; it reaches the definition and its serialized form.
    let id = ModuleId::parse("components/doc_probe").unwrap();
    let probe = r#"/* gpui-rhai
{
  "id": "components/doc_probe",
  "export": "DocProbe",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {}
}
*/
define_component(#{
    metadata: #{ id: "components/doc_probe", "export": "DocProbe", version: "0.1.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            items: #{ schema: #{ type: "array", items: #{ type: "object", fields: #{
                    id: #{ schema: #{ type: "string" }, required: true, doc: "Unique row id." },
                } } },
                required: true, sensitive: false, doc: "Rows to show." },
            on_pick: #{ schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false },
            footer: #{ schema: #{ type: "optional", value: #{ type: "node" } },
                required: false, sensitive: false },
        },
        state: #{ fields: #{} },
        events: #{ pick: #{ payload: #{ type: "string" }, doc: "A row was picked." } },
        slots: #{ footer: #{ required: false, multiple: false, doc: "Below the rows." } },
        parts: ["root"],
    },
    render: Fn("render_DocProbe")
});
fn DocProbe(props) { render_component("components/doc_probe", props) }
fn render_DocProbe(ctx, props) { text("probe") }
"#;
    let mut runtime = engine_with([(id.as_str(), probe)]);
    compile(
        &mut runtime,
        "ui/main.rhai",
        "import \"components/doc_probe\" as probe;\nfn view(ctx) { text(\"ok\") }\n",
    );
    let exports = runtime.component_exports().unwrap();
    let schema = &exports.get(&id).unwrap().schema;
    let items = &schema.props["items"];
    assert_eq!(items.doc.as_deref(), Some("Rows to show."));
    let gpui_rhai::ValueSchema::Array { items: row, .. } = &items.schema else {
        panic!("items is an array: {:?}", items.schema)
    };
    let gpui_rhai::ValueSchema::Object { fields, .. } = row.as_ref() else {
        panic!("rows are objects: {row:?}")
    };
    assert_eq!(fields["id"].doc.as_deref(), Some("Unique row id."));
    assert_eq!(schema.props["on_pick"].doc, None);
    assert_eq!(
        schema.events["pick"].doc.as_deref(),
        Some("A row was picked.")
    );
    assert_eq!(
        schema.slots["footer"].doc.as_deref(),
        Some("Below the rows.")
    );
    let json = serde_json::to_value(schema).unwrap();
    assert_eq!(json["props"]["items"]["doc"], "Rows to show.");
    assert!(json["props"]["on_pick"].get("doc").is_none(), "{json}");
}

#[test]
fn button_and_label_compile_export_and_render_together() {
    for id in ["components/button", "components/label"] {
        assert_eq!(
            parse_component_header(official_source(id)).unwrap().id,
            ModuleId::parse(id).unwrap()
        );
    }
    let (mut runtime, compiled) = compile_official(
        "ui/main.rhai",
        r#"
                import "components/button" as button;
                import "components/label" as label;

                fn save(ctx, payload) { () }

                fn view(ctx) {
                    column([
                        label::Label(#{ text: "Project", required: true }),
                        button::Button(#{
                            text: "Save",
                            variant: "primary",
                            on_click: Fn("save")
                        })
                    ])
                }
            "#,
    );
    let root = render(&mut runtime, &compiled);
    let UiNodeKind::Box { children } = root.kind() else {
        panic!("official component composition did not return a container");
    };
    assert_eq!(children.len(), 2);
    assert_eq!(
        children[0].attributes().get("role"),
        Some(&gpui_rhai::UiValue::String("label".to_owned()))
    );
    assert_eq!(
        children[1].attributes().get("role"),
        Some(&gpui_rhai::UiValue::String("button".to_owned()))
    );
    assert_eq!(
        children[1].attributes().get("label"),
        Some(&gpui_rhai::UiValue::String("Save".to_owned()))
    );
    assert!(children[1].handlers().contains_key("click"));
    assert_eq!(
        children[1]
            .handler("click")
            .unwrap()
            .as_script()
            .unwrap()
            .generation(),
        compiled.generation()
    );
    assert_eq!(runtime.component_exports().unwrap().len(), 2);
}

#[test]
fn official_components_validate_props_and_merge_standard_style_overrides() {
    let (mut engine, compiled) = compile_official(
        "ui/style_override.rhai",
        r#"
                import "components/button" as button;
                fn view() {
                    button::Button(#{
                        text: "Save",
                        prefix: text("<"), suffix: text(">"),
                        style: style().width(px(333)),
                        part_styles: #{ root: style().height(px(77)) }
                    })
                }
            "#,
    );
    let root = engine.render(&compiled).unwrap();
    assert_eq!(
        root.style().base.width,
        Some(gpui_rhai::Length::Pixels(333.0).into())
    );
    assert_eq!(
        root.style().base.height,
        Some(gpui_rhai::Length::Pixels(77.0).into())
    );
    assert!(matches!(root.kind(), UiNodeKind::Box { children } if children.len() == 3));

    let (mut rejecting_engine, invalid) = compile_official(
        "ui/invalid_button.rhai",
        r#"
                import "components/button" as button;
                fn view() { button::Button(#{ text: "Save", unknown: true }) }
            "#,
    );
    assert!(rejecting_engine.render(&invalid).is_err());
}

#[test]
fn interactive_component_contracts_require_explicit_accessible_names() {
    let (engine, _) = compile_official(
        "ui/accessible_names.rhai",
        r#"
                import "components/input" as input;
                import "components/textarea" as textarea;
                import "components/combobox" as combobox;
                import "components/select" as select;
                import "components/date_picker" as date_picker;
                import "components/pagination" as pagination;
                import "components/menu" as menu;
                import "components/context_menu" as context_menu;
                import "components/popover" as popover;
                import "components/tooltip" as tooltip;
                import "components/progress" as progress;
                fn view() { text("ok") }
            "#,
    );
    let exports = engine.component_exports().unwrap();
    for id in [
        "components/input",
        "components/textarea",
        "components/combobox",
        "components/select",
        "components/date_picker",
        "components/pagination",
        "components/menu",
        "components/context_menu",
        "components/popover",
        "components/tooltip",
        "components/progress",
    ] {
        let definition = exports
            .get(&ModuleId::parse(id).unwrap())
            .unwrap_or_else(|| panic!("missing imported component {id}"));
        let label = definition
            .schema
            .props
            .get("label")
            .unwrap_or_else(|| panic!("{id} must declare a label prop"));
        assert!(label.required, "{id} label must be required");
        assert!(
            label.default.is_none(),
            "{id} label must not have a fallback"
        );
    }
}

#[test]
fn unlabeled_icon_is_decorative_presentation() {
    let (mut engine, compiled) = compile_official(
        "ui/decorative_icon.rhai",
        r#"
                import "components/icon" as icon;
                fn view(ctx) { icon::Icon(#{ source: asset("app/check"), size: "sm" }) }
            "#,
    );
    let root = render(&mut engine, &compiled);
    assert_eq!(
        root.attributes().get("role"),
        Some(&UiValue::String("presentation".to_owned()))
    );
    assert!(!root.attributes().contains_key("label"));
}

/// A bundled palette layered over the bundled token base, as an app loads it.
fn load_bundled(engine: &RuntimeEngine, file: &str, source: &str) -> gpui_rhai::ThemeVariant {
    let base =
        gpui_rhai::load_token_base(engine.engine(), "tokens.rhai", registry::TOKEN_BASE_SOURCE)
            .unwrap();
    gpui_rhai::load_theme_with_layers(
        engine.engine(),
        Some(&base),
        file,
        source,
        &gpui_rhai::ThemeTokenOverrides::default(),
    )
    .unwrap_or_else(|error| panic!("{file}: {error}"))
}

#[test]
fn official_default_theme_pair_satisfies_one_semantic_contract() {
    let engine = RuntimeEngine::new();
    let light = load_bundled(&engine, "default_light.rhai", registry::DEFAULT_LIGHT_THEME);
    let dark = load_bundled(&engine, "default_dark.rhai", registry::DEFAULT_THEME);
    assert_eq!(light.family, dark.family);
    assert_eq!(light.mode, gpui_rhai::ThemeMode::Light);
    assert_eq!(dark.mode, gpui_rhai::ThemeMode::Dark);
    assert_eq!(
        light.tokens.colors.keys().collect::<Vec<_>>(),
        dark.tokens.colors.keys().collect::<Vec<_>>()
    );
    // Focus is the ink color in both modes.
    assert_eq!(
        light.tokens.colors["focus_ring"],
        light.tokens.colors["text_primary"]
    );
    assert_eq!(
        dark.tokens.colors["focus_ring"],
        dark.tokens.colors["text_primary"]
    );
}

#[test]
fn bundled_palettes_own_colors_only_and_share_the_token_base_metrics() {
    use gpui_rhai::{ColorResolver, Length};
    let engine = RuntimeEngine::new();
    let typography = [
        ("caption", 12.0, 18.0, 400),
        ("body_small", 13.0, 20.0, 400),
        ("body", 14.0, 22.0, 400),
        ("subtitle", 16.0, 24.0, 600),
        ("title", 18.0, 26.0, 600),
        ("heading", 20.0, 28.0, 600),
        ("display", 24.0, 32.0, 600),
        ("display_large", 32.0, 40.0, 600),
        ("label", 12.0, 16.0, 400),
        ("code", 13.0, 20.0, 400),
    ];
    for &(name, source) in registry::BUNDLED_THEME_SOURCES {
        let palette = gpui_rhai::load_theme_source(engine.engine(), name, source).unwrap();
        assert!(
            palette.tokens.spacing.is_empty()
                && palette.tokens.radii.is_empty()
                && palette.tokens.typography.roles.is_empty()
                && palette.tokens.namespaces.is_empty(),
            "{name}: palettes must contain colors only"
        );
        let theme = load_bundled(&engine, name, source);
        // Square is the default corner style and every role is zero in it, so
        // choosing no corner style leaves the design pixel-identical.
        for radius in ["xs", "sm", "md", "lg"] {
            assert_eq!(
                theme.resolve_length(Length::theme_radius(radius).unwrap()),
                Some(Length::Pixels(0.0)),
                "{name}: radius {radius}"
            );
        }
        for (corners, expected) in [
            ("subtle", [2.0, 2.0, 4.0, 4.0]),
            ("round", [4.0, 999.0, 999.0, 8.0]),
        ] {
            for (radius, pixels) in ["xs", "sm", "md", "lg"].into_iter().zip(expected) {
                assert_eq!(
                    theme.resolve_length_in(
                        Length::theme_radius(radius).unwrap(),
                        &environment(&[("corners", corners)])
                    ),
                    Some(Length::Pixels(pixels)),
                    "{name}: radius {radius} with {corners} corners"
                );
            }
        }
        for &(role, size, line_height, weight) in &typography {
            let token = theme.resolve_typography(role).unwrap();
            assert_eq!(token.size, Length::Pixels(size), "{name}:{role}");
            assert_eq!(
                token.line_height,
                Length::Pixels(line_height),
                "{name}:{role}"
            );
            assert_eq!(token.weight, weight, "{name}:{role}");
        }
        let control = Length::token("metrics.control").unwrap();
        assert_eq!(theme.resolve_length(control), Some(Length::Pixels(32.0)));
        assert_eq!(
            theme.resolve_length_in(
                control,
                &environment(&[("density", "compact"), ("size", "sm")])
            ),
            Some(Length::Pixels(24.0))
        );
        assert_eq!(
            theme
                .resolve_typography_in("control", &environment(&[("size", "xs")]))
                .map(|role| role.size),
            Some(Length::Pixels(13.0))
        );
    }
}

#[test]
fn field_text_clears_a_capsule_end_in_every_corner_style() {
    use gpui_rhai::{ColorResolver, Length};
    let engine = RuntimeEngine::new();
    let theme = load_bundled(&engine, "default_light.rhai", registry::DEFAULT_LIGHT_THEME);
    let pixels = |token: &str, pairs: &[(&str, &str)]| match theme
        .resolve_length_in(Length::token(token).unwrap(), &environment(pairs))
    {
        Some(Length::Pixels(value)) => value,
        other => panic!("{token} {pairs:?}: {other:?}"),
    };
    // Fields have a 2px frame on each side; their text starts half the control height
    // plus 1px from the outer edge, past a capsule's end, in every corner style.
    let frame = 2.0;
    // Textarea's vertical padding is kept from the earlier ratio rule.
    let multiline = [
        ("comfortable", [0.0, 1.0, 3.0, 5.0]),
        ("compact", [0.0, 0.0, 0.0, 3.0]),
    ];
    for (density, vertical) in multiline {
        for (size, expected) in ["xs", "sm", "md", "lg"].into_iter().zip(vertical) {
            let base = pixels("metrics.control", &[("density", density), ("size", size)]);
            for corners in ["square", "subtle", "round"] {
                let pairs = [("density", density), ("size", size), ("corners", corners)];
                let start = frame + pixels("metrics.field_pad", &pairs);
                assert!(
                    (start - (base / 2.0 + 1.0)).abs() < 0.01,
                    "{density} {size} {corners}: text starts at {start} in a {base}px field"
                );
            }
            let pairs = [("density", density), ("size", size)];
            let pad = pixels("metrics.multiline_pad", &pairs);
            assert!(
                (pad - expected).abs() < 0.01,
                "{density} {size}: multiline_pad {pad}, expected {expected}"
            );
        }
    }
}

fn hue(color: gpui_rhai::Rgba8) -> (f64, f64) {
    let [red, green, blue, _] = color.as_rgba_hex().to_be_bytes();
    let max = red.max(green).max(blue);
    let min = red.min(green).min(blue);
    if max == min {
        return (0.0, 0.0);
    }
    let unit = |channel: u8| f64::from(channel) / 255.0;
    let delta = unit(max) - unit(min);
    let lightness = f64::midpoint(unit(max), unit(min));
    let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
    let sector = if max == red {
        ((unit(green) - unit(blue)) / delta).rem_euclid(6.0)
    } else if max == green {
        (unit(blue) - unit(red)) / delta + 2.0
    } else {
        (unit(red) - unit(green)) / delta + 4.0
    };
    (sector * 60.0, saturation)
}

fn hue_distance(left: f64, right: f64) -> f64 {
    let distance = (left - right).abs() % 360.0;
    distance.min(360.0 - distance)
}

#[test]
fn bundled_palettes_meet_the_theme_authoring_constraints() {
    let engine = RuntimeEngine::new();
    for &(name, source) in registry::BUNDLED_THEME_SOURCES {
        let theme = load_bundled(&engine, name, source);
        let color = |token: &str| theme.tokens.color(token).unwrap();
        for foreground in ["text_primary", "text_muted"] {
            for background in ["surface", "surface_raised", "surface_hover", "selection"] {
                let ratio = color(foreground).contrast_ratio(color(background));
                assert!(
                    ratio >= 4.5,
                    "{name}: {foreground} on {background} is {ratio:.2}:1"
                );
            }
        }
        for fill in ["accent", "danger", "warning", "success"] {
            let ratio = color(&format!("on_{fill}")).contrast_ratio(color(fill));
            assert!(ratio >= 4.5, "{name}: on_{fill} is {ratio:.2}:1");
            let text = color(&format!("text.{fill}"));
            assert!(
                text.contrast_ratio(color("surface_hover")) >= 4.5,
                "{name}: text.{fill} is unreadable on tonal blocks"
            );
        }
        // Derived fills that carry text: a neutral Tag facet, selected Table and List rows.
        for (background, base) in [
            ("tag.facet", "surface_hover"),
            ("table.selection", "surface"),
        ] {
            let ratio = color("text_primary").contrast_ratio(color(background));
            assert!(
                ratio >= 4.5,
                "{name}: text_primary on {background} is {ratio:.2}:1"
            );
            assert_ne!(
                color(background),
                color(base),
                "{name}: {background} is {base}"
            );
        }
        assert!(
            color("focus_ring").contrast_ratio(color("surface")) >= 3.0,
            "{name}: focus"
        );
        assert!(
            color("accent").contrast_ratio(color("surface")) >= 3.0,
            "{name}: accent mark"
        );
        assert!(
            color("surface_hover").contrast_ratio(color("surface")) >= 1.15,
            "{name}: tonal block is invisible"
        );
        assert_ne!(
            color("selection"),
            color("surface_hover"),
            "{name}: selection is hover"
        );
        assert!(
            color("tabs.foreground").contrast_ratio(color("surface_hover")) >= 4.5,
            "{name}: enabled tab foreground is unreadable on its track"
        );
        let (accent_hue, accent_saturation) = hue(color("accent"));
        let statuses = ["danger", "warning", "success"].map(|token| (token, hue(color(token))));
        for (token, (status_hue, status_saturation)) in statuses {
            if accent_saturation > 0.15 && status_saturation > 0.15 {
                assert!(
                    hue_distance(accent_hue, status_hue) >= 29.5,
                    "{name}: accent and {token} hues are too close"
                );
            }
        }
        for (index, (left, (left_hue, _))) in statuses.iter().enumerate() {
            for (right, (right_hue, _)) in &statuses[index + 1..] {
                assert!(
                    hue_distance(*left_hue, *right_hue) >= 29.5,
                    "{name}: {left} and {right} hues are too close"
                );
            }
        }
    }
}

#[test]
fn the_token_base_materializes_complete_derived_palettes() {
    let engine = RuntimeEngine::new();
    for &(name, source) in registry::BUNDLED_THEME_SOURCES {
        let theme = load_bundled(&engine, name, source);
        for token in [
            "syntax.comment",
            "syntax.string",
            "syntax.number",
            "syntax.keyword",
            "syntax.function",
            "syntax.type",
            "syntax.variable",
            "syntax.constant",
            "syntax.operator",
            "syntax.punctuation",
            "syntax.tag",
            "syntax.attribute",
            "document.search_match",
            "document.search_current",
            "diff.left_only",
            "diff.right_only",
            "diff.modified",
            "diff.inline_left",
            "diff.inline_right",
            "diff.gutter",
            "diff.fold",
            "charts.axis",
            "charts.grid",
            "charts.tooltip_surface",
            "charts.tooltip_text",
            "charts.positive",
            "charts.negative",
            "charts.selection",
            "charts.map_missing",
            "charts.crosshair",
            "charts.palette_1",
            "charts.palette_2",
            "charts.palette_3",
            "charts.palette_4",
            "charts.palette_5",
            "charts.palette_6",
            "charts.palette_7",
            "charts.palette_8",
            "table.selection",
            "tabs.foreground",
            "text.accent",
            "control.hover",
            "tag.facet",
        ] {
            assert!(
                theme.tokens.color(token).is_some(),
                "{name}: missing {token}"
            );
        }
        let table_selection = theme.tokens.color("table.selection").unwrap();
        assert_eq!(table_selection.as_rgba_hex() & 0xff, 0xff);
        assert_ne!(table_selection, theme.tokens.colors["surface"]);
    }
}

/// Asserts that no state of `node` or its descendants sets a font weight.
fn assert_no_font_weight(node: &gpui_rhai::UiNode) {
    let style = node.style();
    for state in [
        Some(&style.base),
        style.hover.as_ref(),
        style.active.as_ref(),
        style.focus.as_ref(),
        style.group_focus.as_ref(),
        style.disabled.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        assert_eq!(state.font_weight, None, "{node:?}");
    }
    if let UiNodeKind::Box { children } = node.kind() {
        children.iter().for_each(assert_no_font_weight);
    }
}

#[test]
fn tabs_and_table_draw_state_surfaces_from_theme_tokens() {
    use gpui_rhai::{CornerLengths, EdgeLengths, Length};
    let (mut engine, compiled) = compile_official(
        "ui/state_surfaces.rhai",
        r#"
            import "components/tabs" as tabs;
            import "components/table" as table;
            fn changed(ctx, value) { () }
            fn view(ctx) {
                let items = [
                    #{ value: "general", label: "General" },
                    #{ value: "advanced", label: "Advanced" }
                ];
                column([
                    tabs::Tabs(#{ value: "general", label: "Settings", tabs: items,
                        panel: false, on_change: Fn("changed") }),
                    tabs::Tabs(#{ value: "general", label: "Sections", tabs: items,
                        orientation: "vertical", panel: false, on_change: Fn("changed") }),
                    table::Table(#{
                        key: "users", label: "Users", row_key: "id", height: 120,
                        rows: [#{ id: "a", name: "Ada" }, #{ id: "b", name: "Lin" }],
                        columns: [#{ key: "name", title: "Name",
                            width: #{ kind: "fixed", value: 120 } }],
                        selection_mode: "single", selected_keys: ["a"],
                        part_styles: #{
                            indicator_bar: style().background(theme_color("danger")),
                            row_selected: style().text_color(theme_color("accent"))
                        }
                    })
                ])
            }
        "#,
    );
    let lifecycle = mount(&mut engine, compiled, &new_runtime());
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("state surface specimen must render a column");
    };
    let token = |name: &str| Some(ColorValue::Token(name.to_owned()));
    let row = Length::token("metrics.row").unwrap();

    // Tabs: one tonal track inset by xxs, rounded by its orientation's role; a raised
    // thumb that carries the list focus; control type at one weight so widths never
    // jitter.
    for (tabs, corner) in [(&children[0], "md"), (&children[1], "lg")] {
        assert_eq!(
            tabs.attributes().get("role"),
            Some(&UiValue::String("tablist".to_owned()))
        );
        let track = &tabs.style().base;
        assert_eq!(track.background, token("surface_hover"));
        assert_eq!(
            track.padding,
            EdgeLengths::all(Length::theme_spacing("xxs").unwrap())
        );
        assert_eq!(
            track.radii,
            CornerLengths::all(Length::theme_radius(corner).unwrap())
        );
        let UiNodeKind::Box { children: slots } = tabs.kind() else {
            panic!("Tabs list must hold its tab slots");
        };
        for slot in slots {
            assert_eq!(slot.style().base.typography.as_deref(), Some("control"));
            assert_no_font_weight(slot);
        }
        let UiNodeKind::Box { children: selected } = slots[0].kind() else {
            panic!("the selected tab must hold its thumb");
        };
        let thumb = selected[0].style();
        assert_eq!(thumb.base.background, token("surface_raised"));
        assert_eq!(
            thumb.group_focus.as_ref().unwrap().border_color,
            token("focus_ring")
        );
    }

    // Table: label-voice header, metrics.row rows, the selection tint and the
    // indicator bar, each a part the caller can restyle.
    let UiNodeKind::Box { children: table } = children[2].kind() else {
        panic!("Table must be a Box composition");
    };
    assert_eq!(table[0].style().base.height, Some(row.into()));
    let UiNodeKind::Box { children: headers } = table[0].kind() else {
        panic!("Table header must be a row");
    };
    assert_eq!(headers[0].style().base.typography.as_deref(), Some("label"));
    assert_eq!(headers[0].style().base.height, Some(row.into()));
    let UiNodeKind::VirtualCollection { spec } = table[1].kind() else {
        panic!("Table body must be a virtual collection");
    };
    let (selected, unselected) = (&spec.realized[&0], &spec.realized[&1]);
    for row_node in [selected, unselected] {
        assert_eq!(row_node.style().base.height, Some(row.into()));
    }
    assert_eq!(selected.style().base.background, token("table.selection"));
    assert_eq!(selected.style().base.text_color, token("accent"));
    assert_eq!(unselected.style().base.text_color, token("text_primary"));
    let UiNodeKind::Box { children: cells } = selected.kind() else {
        panic!("Table row must be a row");
    };
    let bar = &cells[cells.len() - 2].style().base;
    assert_eq!(bar.width, Some(Length::Pixels(2.0).into()));
    assert_eq!(bar.background, token("danger"));
}

#[test]
fn window_bars_are_source_owned_compositions_without_native_authority() {
    let (mut engine, compiled) = compile_official(
        "ui/window_bars.rhai",
        r#"
            import "components/title_bar" as title_bar;
            import "components/status_bar" as status_bar;
            fn drag(ctx, payload) { handled() }
            fn view(ctx) {
                let breadcrumb = row([text("Workspace").with_style(style().font_weight(700)),
                    text("/ panel").with_style(style().text_color(theme_color("text_muted")))])
                    .with_key("breadcrumb").on("pointer_down", Fn("drag"))
                    .accessibility_role("group").accessibility_label("Workspace breadcrumb");
                column([
                    title_bar::TitleBar(#{ label: "Workbench", title: breadcrumb,
                        subtitle: text("main"), inset_start: 70,
                        center: [text("Editor")], end: [text("Run")] }),
                    text("Content"),
                    status_bar::StatusBar(#{ label: "Editor status",
                        start: [text("Ready")], center: [text("main")],
                        end: [text("UTF-8")] })
                ])
            }
        "#,
    );
    let root = render(&mut engine, &compiled);
    let UiNodeKind::Box { children } = root.kind() else {
        panic!("window bar specimen must render a column");
    };
    assert_eq!(children.len(), 3);
    assert_eq!(
        children[0].attributes().get("role"),
        Some(&UiValue::String("toolbar".to_owned()))
    );
    assert_eq!(
        children[0].attributes().get("label"),
        Some(&UiValue::String("Workbench".to_owned()))
    );
    assert!(
        find_label(&children[0], "Workspace breadcrumb")
            .unwrap()
            .handlers()
            .contains_key("pointer_down")
    );
    assert_eq!(
        children[2].attributes().get("role"),
        Some(&UiValue::String("statusbar".to_owned()))
    );
    for bar in [children[0].clone(), children[2].clone()] {
        assert!(bar.handlers().is_empty());
    }
}

#[test]
fn public_launch_static_components_compile_and_compose() {
    let (mut engine, compiled) = compile_official(
        "ui/public_launch_static_components.rhai",
        r#"
            import "components/alert" as alert;
            import "components/alert_dialog" as alert_dialog;
            import "components/badge" as badge;
            import "components/button" as button;
            import "components/button_group" as button_group;
            import "components/card" as card;
            import "components/empty" as empty;
            import "components/group_box" as group_box;
            import "components/input" as input;
            import "components/input_group" as input_group;
            import "components/kbd" as kbd;
            fn ignored(ctx, value) { () }
            fn view(ctx) {
                let primary = button::Button(#{ text: "Save", on_click: Fn("ignored") });
                let secondary = button::Button(#{ text: "Cancel", variant: "secondary", on_click: Fn("ignored") });
                column([
                    alert::Alert(#{ title: "Saved", description: "Changes are live.",
                        variant: "success", actions: [secondary] }),
                    alert_dialog::AlertDialog(#{ key: "confirm", open: false,
                        title: "Delete item?", description: "This cannot be undone.",
                        on_confirm: Fn("ignored"), on_cancel: Fn("ignored"),
                        on_open_change: Fn("ignored") }),
                    badge::Badge(#{ text: "Ready", variant: "success", dot: true }),
                    button_group::ButtonGroup(#{ label: "Actions", buttons: [primary, secondary] }),
                    card::Card(#{ header: text("Profile"), content: text("Ada"),
                        footer: text("Updated now"), variant: "block" }),
                    empty::Empty(#{ title: "No projects", description: "Create one to begin.",
                        actions: [primary] }),
                    group_box::GroupBox(#{ label: "Sync", description: "Cloud settings",
                        content: text("Enabled") }),
                    input_group::InputGroup(#{ label: "Search", prefix: text("⌕"),
                        suffix: kbd::Kbd(#{ text: "⌘ K" }),
                        control: input::Input(#{ key: "search", label: "Search", value: "", placeholder: "Search" }) }),
                    kbd::Kbd(#{ text: "⌘ K", label: "Command K" })
                ])
            }
        "#,
    );
    let root = render(&mut engine, &compiled);
    let UiNodeKind::Box { children } = root.kind() else {
        panic!("public launch static specimen must render a column");
    };
    assert_eq!(children.len(), 9);
}

#[test]
fn official_icon_uses_only_logical_asset_and_opaque_image_handle() {
    let (mut engine, compiled) = compile_official(
        "ui/icon_test.rhai",
        r#"
            import "components/icon" as icon;
            fn init(ctx) {
                ctx.set_state("image", ctx.load_image(asset("app/check")));
            }
            fn view(ctx) {
                icon::Icon(#{
                    source: ctx.get_state("image"),
                    size: "sm",
                    label: "Complete"
                })
            }
        "#,
    );
    let runtime_state = UiRuntimeState::new();
    runtime_state
        .assets
        .register(
            "app",
            InMemoryAssetProvider::new(BTreeMap::from([(
                "check".to_owned(),
                AssetData {
                    mime_type: "image/svg+xml".to_owned(),
                    bytes: registry::CHECK_SVG.as_bytes().to_vec(),
                },
            )])),
        )
        .unwrap();
    let runtime = Rc::new(RefCell::new(runtime_state));
    let state_schema = ComponentStateSchema::new(BTreeMap::from([(
        "image".to_owned(),
        StateField::new(
            ValueSchema::Handle {
                kind: "image".to_owned(),
            },
            UiValue::Handle(OpaqueHandle::new("image", 0)),
        ),
    )]))
    .unwrap();
    let mut lifecycle = lifecycle_with_schema(compiled, &runtime, &state_schema);
    let root = lifecycle.start(&mut engine).unwrap();
    assert!(matches!(
        root.kind(),
        UiNodeKind::Image {
            source: gpui_rhai::ImageSourceSpec::Handle(handle),
        } if handle.kind() == "image" && handle.id() != 0
    ));
}

#[test]
fn official_icon_selects_explicit_rtl_resource_pair() {
    let (mut engine, compiled) = compile_official(
        "ui/directional_icon_test.rhai",
        r#"
            import "components/icon" as icon;
            fn view(ctx) {
                icon::Icon(#{
                    source: ctx.get_state("ltr"),
                    rtl_source: ctx.get_state("rtl"),
                    label: "Forward"
                })
            }
        "#,
    );
    let state_schema = ComponentStateSchema::new(BTreeMap::from([
        (
            "ltr".to_owned(),
            StateField::new(
                ValueSchema::Handle {
                    kind: "image".to_owned(),
                },
                UiValue::Handle(OpaqueHandle::new("image", 1)),
            ),
        ),
        (
            "rtl".to_owned(),
            StateField::new(
                ValueSchema::Handle {
                    kind: "image".to_owned(),
                },
                UiValue::Handle(OpaqueHandle::new("image", 2)),
            ),
        ),
    ]))
    .unwrap();
    let mut lifecycle = lifecycle_with_schema(compiled, &new_runtime(), &state_schema);
    let root = lifecycle.start(&mut engine).unwrap();
    assert!(matches!(
        root.kind(),
        UiNodeKind::DirectionalImage {
            left_to_right: gpui_rhai::ImageSourceSpec::Handle(left_to_right),
            right_to_left: gpui_rhai::ImageSourceSpec::Handle(right_to_left),
        } if left_to_right.id() == 1 && right_to_left.id() == 2
    ));
}

#[test]
fn tokyo_night_and_catppuccin_palettes_form_theme_families() {
    let engine = RuntimeEngine::new();
    let load = |file: &str, source: &str| {
        gpui_rhai::load_theme_source(engine.engine(), file, source).unwrap()
    };
    let tokyo_night = load("tokyo_night.rhai", registry::TOKYO_NIGHT_THEME);
    let tokyo_storm = load("tokyo_storm.rhai", registry::TOKYO_STORM_THEME);
    assert_eq!(tokyo_night.family, "Tokyo Night");
    assert_eq!(tokyo_storm.family, "Tokyo Night");

    let latte = load("catppuccin_latte.rhai", registry::CATPPUCCIN_LATTE_THEME);
    let mocha = load("catppuccin_mocha.rhai", registry::CATPPUCCIN_MOCHA_THEME);
    gpui_rhai::ThemeFamily {
        name: "Catppuccin".to_owned(),
        variants: BTreeMap::from([("Latte".to_owned(), latte), ("Mocha".to_owned(), mocha)]),
        default_light: "Latte".to_owned(),
        default_dark: "Mocha".to_owned(),
    }
    .validate()
    .unwrap();
}

#[test]
fn official_input_wraps_keyed_native_text_input_primitive() {
    let (mut engine, compiled) = compile_official(
        "ui/input_test.rhai",
        r#"
            import "components/input" as input;
            fn changed(ctx, value) { () }
            fn view(ctx) {
                input::Input(#{
                    key: "name",
                    label: "Name",
                    value: "Alice",
                    placeholder: "Name",
                    read_only: true,
                    on_change: Fn("changed")
                })
            }
        "#,
    );
    let root = render(&mut engine, &compiled);
    let UiNodeKind::Custom { primitive } = root.kind() else {
        panic!("Input must wrap the native TextInput primitive");
    };
    assert_eq!(primitive.primitive.as_str(), "gpui_rhai.text_input");
    assert_eq!(primitive.key.as_deref(), Some("name"));
    assert!(matches!(
        primitive.props.get("read_only"),
        Some(gpui_rhai::PrimitiveValue::Data(UiValue::Bool(true)))
    ));
}

#[test]
fn official_textarea_wraps_independent_multiline_primitive() {
    let (mut engine, compiled) = compile_official(
        "ui/textarea_test.rhai",
        r#"
            import "components/textarea" as textarea;
            fn changed(ctx, value) { () }
            fn view(ctx) {
                textarea::Textarea(#{
                    key: "notes", label: "Notes", value: "Line one\nLine two",
                    min_rows: 2, max_rows: 6, max_length: 100,
                    on_change: Fn("changed")
                })
            }
        "#,
    );
    let root = render(&mut engine, &compiled);
    let UiNodeKind::Box { children } = root.kind() else {
        panic!("Textarea must render its source-owned root wrapper");
    };
    let UiNodeKind::Custom { primitive } = children[0].kind() else {
        panic!("Textarea must wrap the native multiline primitive");
    };
    assert_eq!(primitive.primitive.as_str(), "gpui_rhai.textarea");
    assert_eq!(primitive.key.as_deref(), Some("notes"));
    assert!(matches!(
        primitive.props.get("max_length"),
        Some(gpui_rhai::PrimitiveValue::Data(UiValue::Integer(100)))
    ));
    for part in [
        "placeholder_style",
        "selection_style",
        "caret_style",
        "scroll_style",
    ] {
        assert!(matches!(
            primitive.props.get(part),
            Some(gpui_rhai::PrimitiveValue::Style(_))
        ));
    }
}

#[test]
fn official_divider_is_typed_decorative_layout() {
    let (mut engine, compiled) = compile_official(
        "ui/divider_test.rhai",
        r#"
            import "components/divider" as divider;
            fn view(ctx) {
                column([
                    divider::Divider(#{}),
                    divider::Divider(#{ orientation: "vertical", thickness: 2 })
                ])
            }
        "#,
    );
    let root = render(&mut engine, &compiled);
    let UiNodeKind::Box { children } = root.kind() else {
        panic!("divider showcase must render a container");
    };
    assert_eq!(children.len(), 2);
    assert_eq!(
        children[0].attributes().get("role"),
        Some(&UiValue::String("separator".to_owned()))
    );
    assert_eq!(
        children[0].style().base.width,
        Some(gpui_rhai::Length::Relative(1.0).into())
    );
    assert_eq!(
        children[1].style().base.width,
        Some(gpui_rhai::Length::Pixels(2.0).into())
    );
}

#[test]
fn official_popover_and_dialog_use_native_overlay_nodes() {
    let (mut engine, compiled) = compile_official(
        "ui/overlay_test.rhai",
        r#"
            import "components/popover" as popover;
            import "components/dialog" as dialog;
            fn set_open(ctx, open) { () }
            fn view(ctx) {
                column([
                    popover::Popover(#{
                        key: "help",
                        label: "Help",
                        trigger: text("Help"),
                        content: text("Popover content"),
                        open: true,
                        placement: "right",
                        on_open_change: Fn("set_open")
                    }),
                    dialog::Dialog(#{
                        key: "confirm",
                        open: true,
                        title: "Confirm",
                        content: text("Continue?"),
                        actions: [text("Cancel"), text("Continue")],
                        on_open_change: Fn("set_open")
                    })
                ])
            }
        "#,
    );
    let root = render(&mut engine, &compiled);
    let UiNodeKind::Box { children } = root.kind() else {
        panic!("overlay showcase must render a container");
    };
    let UiNodeKind::Overlay { spec: popover, .. } = children[0].kind() else {
        panic!("popover component must render a native overlay node");
    };
    assert_eq!(popover.kind, gpui_rhai::OverlayKind::Popover);
    assert_eq!(popover.placement, gpui_rhai::OverlayPlacement::Right);
    assert!(children[0].handlers().contains_key("open_change"));

    let UiNodeKind::Overlay { spec: dialog, .. } = children[1].kind() else {
        panic!("dialog component must render a native overlay node");
    };
    assert_eq!(dialog.kind, gpui_rhai::OverlayKind::Dialog);
    assert_eq!(dialog.placement, gpui_rhai::OverlayPlacement::Center);
    assert!(dialog.modal);
}

#[test]
fn context_menu_and_sheet_use_generic_pointer_and_edge_overlay_policies() {
    let (mut engine, compiled) = compile_official(
        "ui/context_menu_sheet.rhai",
        r#"
            import "components/context_menu" as context_menu;
            import "components/sheet" as sheet;
            fn changed(ctx, value) { () }
            fn view(ctx) {
                column([
                    context_menu::ContextMenu(#{
                        key: "row-actions", label: "Row actions", trigger: text("Right click"), open: false,
                        active_value: "copy", items: [
                            #{ kind: "item", value: "copy", label: "Copy", shortcut: "⌘C" }
                        ], on_action: Fn("changed"), on_active_change: Fn("changed"),
                        on_open_change: Fn("changed")
                    }),
                    sheet::Sheet(#{ key: "inspector", open: true, side: "end",
                        title: "Inspector", content: text("Details"),
                        on_open_change: Fn("changed") })
                ])
            }
        "#,
    );
    let runtime = new_runtime();
    let lifecycle = mount(&mut engine, compiled, &runtime);
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("overlay policy specimen must render a column");
    };
    let UiNodeKind::Overlay { spec: menu, .. } = children[0].kind() else {
        panic!("ContextMenu must render the generic Overlay node");
    };
    assert_eq!(menu.kind, gpui_rhai::OverlayKind::Menu);
    assert!(!menu.activate_on_trigger);
    assert_eq!(menu.anchor, Some(gpui_rhai::OverlayBounds::default()));
    let UiNodeKind::Overlay { spec: sheet, .. } = children[1].kind() else {
        panic!("Sheet must render the generic Overlay node");
    };
    assert_eq!(sheet.kind, gpui_rhai::OverlayKind::Sheet);
    assert_eq!(sheet.placement, gpui_rhai::OverlayPlacement::End);
    assert!(sheet.modal);
    assert!(!sheet.activate_on_trigger);
}

#[test]
fn command_fuzzy_search_and_keyboard_action_are_composable_and_controlled() {
    let (mut engine, compiled) = compile_official(
        "ui/command.rhai",
        r#"
            import "components/command" as command;
            fn state_schema() { #{ fields: #{
                action: #{ schema: #{ type: "string" },
                    "default": #{ type: "string", value: "" } }
            } } }
            fn run(ctx, value) { ctx.set_state("action", value); }
            fn queried(ctx, value) { () }
            fn view(ctx) {
                column([
                    command::Command(#{ key: "palette", label: "Commands", query: "opn", active_value: "open",
                        items: [
                            #{ value: "new", label: "New file", group: "File", shortcut: "⌘N" },
                            #{ value: "open", label: "Open file", keywords: ["load document"], group: "File", shortcut: "⌘O" },
                            #{ value: "close", label: "Close window", group: "Window", disabled: true }
                        ], on_query_change: Fn("queried"), on_action: Fn("run") })
                ])
            }
        "#,
    );
    let runtime = new_runtime();
    let path = root_path();
    let lifecycle = mount(&mut engine, compiled, &runtime);
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("command test root must render a column");
    };
    let command = &children[0];
    let collection = find_virtual_collection(command).expect("fuzzy result list");
    let UiNodeKind::VirtualCollection { spec } = collection.kind() else {
        unreachable!()
    };
    assert_eq!(spec.data.len(), 2, "one group header plus one fuzzy match");
    assert_eq!(
        spec.height, None,
        "rows size the viewport through metrics.row"
    );
    assert_eq!(
        find_virtual_collection_viewport(command)
            .unwrap()
            .style()
            .base
            .height,
        Some(
            gpui_rhai::Length::token("metrics.row")
                .unwrap()
                .scaled(2.0)
                .unwrap()
                .into()
        ),
        "small command sets must shrink to whole rows"
    );
    assert_eq!(spec.reveal_key.as_deref(), Some("item:open"));
    let (enter, payload) = target_handler(command, "key:enter");
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &enter, payload)
        .unwrap();
    let events = runtime.borrow_mut().drain_batch().events;
    assert_eq!(events.len(), 1);
    for event in events {
        let _ = lifecycle
            .invoke_component_event_transactional(&engine, event)
            .unwrap();
    }
    assert_eq!(
        runtime.borrow().component_state.get(&path, "action"),
        Some(&UiValue::String("open".to_owned()))
    );
}

#[test]
fn command_array_items_keep_text_semantics_with_rich_visual_content() {
    let (mut engine, compiled) = compile_official(
        "ui/rich_command.rhai",
        r#"
            import "components/command" as command;
            fn view(ctx) {
                command::Command(#{
                    key: "palette", label: "Commands", query: "server",
                    active_value: "open", items: [#{
                        value: "open", label: "Open server config",
                        keywords: ["remote"],
                        content: row([
                            text("Server").with_style(style().text_color(theme_color("text_muted"))),
                            text("Open config")
                        ]).with_key("rich-command-content")
                    }]
                })
            }
        "#,
    );
    let runtime = new_runtime();
    let lifecycle = mount(&mut engine, compiled, &runtime);
    let collection = find_virtual_collection(lifecycle.root().unwrap()).unwrap();
    let UiNodeKind::VirtualCollection { spec } = collection.kind() else {
        unreachable!()
    };
    let item = spec
        .realized
        .values()
        .find(|node| {
            node.attributes().get("label")
                == Some(&UiValue::String("Open server config".to_owned()))
        })
        .expect("rich command item");
    let UiNodeKind::Box { children } = item.kind() else {
        panic!("command item must remain a composed row");
    };
    assert_eq!(
        children[0].key().map(gpui_rhai::NodeKey::as_str),
        Some("rich-command-content")
    );
}

#[test]
fn icon_button_owns_a_square_target_and_selected_icon_state() {
    let (mut engine, compiled) = compile_official(
        "ui/icon_button.rhai",
        r#"
            import "components/icon_button" as icon_button;
            fn view(ctx) {
                icon_button::IconButton(#{
                    icon: svg("<svg width='24' height='24' viewBox='0 0 24 24'><path fill='currentColor' d='M4 4L20 20M20 4L4 20'/></svg>"),
                    label: "Close", size: "md", variant: "ghost", selected: true
                })
            }
        "#,
    );
    let root = render(&mut engine, &compiled);
    let control = gpui_rhai::Length::token("metrics.control").unwrap();
    assert_eq!(root.style().base.width, Some(control.into()));
    assert_eq!(root.style().base.height, Some(control.into()));
    assert_eq!(
        root.attributes().get("environment"),
        Some(&UiValue::Map(BTreeMap::from([(
            "size".to_owned(),
            UiValue::String("md".to_owned())
        )])))
    );
    assert_eq!(
        root.attributes().get("label"),
        Some(&UiValue::String("Close".to_owned()))
    );
    assert_eq!(root.attributes().get("pressed"), Some(&UiValue::Bool(true)));
    // A selected ghost IconButton fills its square with the selection block.
    assert_eq!(
        root.style().base.background,
        Some(ColorValue::Token("selection".to_owned()))
    );
    assert_eq!(
        root.style().base.text_color,
        Some(ColorValue::Token("text_primary".to_owned()))
    );
    let UiNodeKind::Box { children } = root.kind() else {
        panic!("IconButton must render a centered row");
    };
    let icon = gpui_rhai::Length::token("metrics.icon").unwrap();
    assert_eq!(children[0].style().base.width, Some(icon.into()));
    assert_eq!(children[0].style().base.height, Some(icon.into()));
}

#[test]
fn command_filters_native_collection_and_keeps_large_rows_out_of_rhai() {
    let (mut engine, compiled) = compile_official(
        "ui/native_command.rhai",
        r#"
            import "components/command" as command;
            fn state_schema() { #{ fields: #{
                action: #{ schema: #{ type: "string" },
                    "default": #{ type: "string", value: "" } }
            } } }
            fn run(ctx, value) { ctx.set_state("action", value); }
            fn queried(ctx, value) { () }
            fn view(ctx) {
                command::Command(#{ key: "native-palette", label: "Commands", query: "opn", active_value: "open",
                    items: ctx.get_native_collection("commands"),
                    on_query_change: Fn("queried"), on_action: Fn("run") })
            }
        "#,
    );
    let rows = gpui_rhai::NativeCollection::new(
        "id",
        [
            BTreeMap::from([
                ("id".to_owned(), UiValue::String("new".to_owned())),
                ("label".to_owned(), UiValue::String("New file".to_owned())),
                ("group".to_owned(), UiValue::String("File".to_owned())),
                ("keywords".to_owned(), UiValue::Array(Vec::new())),
                ("shortcut".to_owned(), UiValue::String("⌘N".to_owned())),
                ("disabled".to_owned(), UiValue::Bool(false)),
            ]),
            BTreeMap::from([
                ("id".to_owned(), UiValue::String("open".to_owned())),
                ("label".to_owned(), UiValue::String("Open file".to_owned())),
                ("group".to_owned(), UiValue::String("File".to_owned())),
                (
                    "keywords".to_owned(),
                    UiValue::Array(vec![UiValue::String("load document".to_owned())]),
                ),
                ("shortcut".to_owned(), UiValue::String("⌘O".to_owned())),
                ("disabled".to_owned(), UiValue::Bool(false)),
            ]),
        ],
    )
    .unwrap();
    let runtime = new_runtime();
    runtime
        .borrow_mut()
        .native_collections
        .register("commands", rows)
        .unwrap();
    let path = root_path();
    let lifecycle = mount(&mut engine, compiled, &runtime);
    let collection = find_virtual_collection(lifecycle.root().unwrap()).unwrap();
    let UiNodeKind::VirtualCollection { spec } = collection.kind() else {
        unreachable!()
    };
    assert!(matches!(
        &spec.data,
        gpui_rhai::VirtualCollectionData::Native(_)
    ));
    assert_eq!(
        spec.data.len(),
        2,
        "one group header plus one native fuzzy match"
    );
    assert_eq!(spec.reveal_key.as_deref(), Some("open"));

    let (enter, payload) = target_handler(lifecycle.root().unwrap(), "key:enter");
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &enter, payload)
        .unwrap();
    let events = runtime.borrow_mut().drain_batch().events;
    assert_eq!(events.len(), 1);
    for event in events {
        let _ = lifecycle
            .invoke_component_event_transactional(&engine, event)
            .unwrap();
    }
    assert_eq!(
        runtime.borrow().component_state.get(&path, "action"),
        Some(&UiValue::String("open".to_owned()))
    );
}

#[test]
fn command_array_and_native_models_share_one_enabled_active_sequence() {
    for native in [false, true] {
        for grouped in [false, true] {
            for disabled_first in [false, true] {
                let runtime = new_runtime();
                let groups = if grouped {
                    ["G1", "G1", "G2"]
                } else {
                    ["", "", ""]
                };
                let rows = ["a", "b", "c"]
                    .into_iter()
                    .enumerate()
                    .map(|(index, value)| {
                        BTreeMap::from([
                            ("value".to_owned(), UiValue::String(value.to_owned())),
                            ("label".to_owned(), UiValue::String(value.to_uppercase())),
                            (
                                "group".to_owned(),
                                UiValue::String(groups[index].to_owned()),
                            ),
                            (
                                "disabled".to_owned(),
                                UiValue::Bool(disabled_first && index == 0),
                            ),
                            ("keywords".to_owned(), UiValue::Array(Vec::new())),
                            ("shortcut".to_owned(), UiValue::String(String::new())),
                        ])
                    })
                    .collect::<Vec<_>>();
                runtime
                    .borrow_mut()
                    .native_collections
                    .register(
                        "commands",
                        gpui_rhai::NativeCollection::new("value", rows).unwrap(),
                    )
                    .unwrap();
                let items = if native {
                    "ctx.get_native_collection(\"commands\")".to_owned()
                } else {
                    format!(
                        r#"[
                            #{{ value: "a", label: "A", group: "{}", disabled: {} }},
                            #{{ value: "b", label: "B", group: "{}" }},
                            #{{ value: "c", label: "C", group: "{}" }},
                        ]"#,
                        groups[0], disabled_first, groups[1], groups[2]
                    )
                };
                let entry = format!(
                    r#"import "components/command" as command;
                        fn view(ctx) {{
                            command::Command(#{{
                                key: "palette", label: "Palette", query: "",
                                active_value: "b", items: {items}, max_visible: 8,
                            }})
                        }}"#
                );
                let (mut engine, compiled) = compile_official("command-navigation", &entry);
                let lifecycle = mount(&mut engine, compiled, &runtime);
                let root = lifecycle.root().unwrap();
                let UiNodeKind::VirtualCollection { spec } =
                    find_virtual_collection(root).unwrap().kind()
                else {
                    unreachable!()
                };
                assert_eq!(spec.realized.len(), spec.data.len());
                assert!(
                    spec.sticky_headers.is_empty(),
                    "sticky presentation must not encode Command navigation"
                );
                let active = spec
                    .realized
                    .values()
                    .filter(|node| node.attributes().get("selected") == Some(&UiValue::Bool(true)))
                    .filter_map(|node| node.attributes().get("label"))
                    .cloned()
                    .collect::<Vec<_>>();
                assert_eq!(active, vec![UiValue::String("B".to_owned())]);

                let expected_home = if disabled_first { "b" } else { "a" };
                let expected_previous = if disabled_first { "c" } else { "a" };
                for (event, expected) in [
                    ("key:home", expected_home),
                    ("key:end", "c"),
                    ("key:up", expected_previous),
                    ("key:down", "c"),
                ] {
                    let (_, payload) = target_handler(root, event);
                    assert_eq!(payload, UiValue::String(expected.to_owned()), "{event}");
                }
            }
        }
    }
}

#[test]
fn command_empty_and_all_disabled_models_have_no_navigation_owner() {
    for native in [false, true] {
        for query in ["", "missing"] {
            let runtime = new_runtime();
            runtime
                .borrow_mut()
                .native_collections
                .register(
                    "commands",
                    gpui_rhai::NativeCollection::new(
                        "value",
                        [BTreeMap::from([
                            ("value".to_owned(), UiValue::String("a".to_owned())),
                            ("label".to_owned(), UiValue::String("A".to_owned())),
                            ("group".to_owned(), UiValue::String("G".to_owned())),
                            ("disabled".to_owned(), UiValue::Bool(true)),
                            ("keywords".to_owned(), UiValue::Array(Vec::new())),
                            ("shortcut".to_owned(), UiValue::String(String::new())),
                        ])],
                    )
                    .unwrap(),
                )
                .unwrap();
            let items = if native {
                "ctx.get_native_collection(\"commands\")"
            } else {
                r#"[#{ value: "a", label: "A", group: "G", disabled: true }]"#
            };
            let entry = format!(
                r#"import "components/command" as command;
                    fn view(ctx) {{
                        command::Command(#{{ key: "palette", label: "Palette",
                            query: "{query}", active_value: "", items: {items} }})
                    }}"#
            );
            let (mut engine, compiled) = compile_official("command-empty-navigation", &entry);
            let lifecycle = mount(&mut engine, compiled, &runtime);
            let root = lifecycle.root().unwrap();
            for event in ["key:up", "key:down", "key:home", "key:end", "key:enter"] {
                assert!(root.handler(event).is_none(), "unexpected handler {event}");
            }
            if let Some(collection) = find_virtual_collection(root) {
                let UiNodeKind::VirtualCollection { spec } = collection.kind() else {
                    unreachable!()
                };
                assert!(spec.realized.values().all(|node| {
                    node.attributes().get("selected") != Some(&UiValue::Bool(true))
                }));
            } else {
                assert_eq!(query, "missing");
            }
        }
    }
}

#[test]
fn code_and_diff_viewers_accept_inline_and_host_owned_documents() {
    let (mut engine, compiled) = compile_official(
        "ui/document_viewers.rhai",
        r#"
            import "components/code_viewer" as code_viewer;
            import "components/diff_viewer" as diff_viewer;
            fn view(ctx) {
                column([
                    code_viewer::CodeViewer(#{ key: "code", source: ctx.get_native_text_document("config"),
                        label: "Rhai", language: "rhai" }),
                    diff_viewer::DiffViewer(#{ key: "diff", mode: "split",
                        left: #{ source: "port = 80\n", label: "Server A", language: "rhai" },
                        right: #{ source: ctx.get_native_text_document("config"), label: "Server B", language: "rhai" } })
                ])
            }
        "#,
    );
    let runtime = new_runtime();
    runtime
        .borrow_mut()
        .native_documents
        .register(
            "config",
            gpui_rhai::NativeTextDocument::new("server-b", 1, "port = 443\n").unwrap(),
        )
        .unwrap();
    let lifecycle = mount(&mut engine, compiled, &runtime);
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("document viewer app must render a column");
    };
    assert_eq!(children.len(), 2);
    for (node, expected) in [
        (&children[0], "gpui_rhai.code_viewer"),
        (&children[1], "gpui_rhai.diff_viewer"),
    ] {
        let UiNodeKind::Custom { primitive } = node.kind() else {
            panic!("viewer must remain a public native primitive");
        };
        assert_eq!(primitive.primitive.as_str(), expected);
        assert!(
            primitive
                .props
                .iter()
                .any(|(_, value)| matches!(value, gpui_rhai::PrimitiveValue::Document(_)))
        );
    }
}

#[test]
fn official_combobox_is_public_overlay_and_virtual_collection_composition() {
    let (mut engine, compiled) = compile_official(
        "ui/combobox_test.rhai",
        r#"
            import "components/combobox" as combobox_component;
            fn selected(ctx, values) { () }
            fn opened(ctx, open) { () }
            fn queried(ctx, query) { () }
            fn view(ctx) {
                combobox_component::Combobox(#{
                    key: "theme",
                    label: "Theme",
                    options: [
                        #{ value: "default", label: "Default" },
                        #{ value: "tokyo", label: "Tokyo Night", keywords: ["night"] },
                        #{ value: "mocha", label: "Mocha", disabled: true }
                    ],
                    mode: "multiple",
                    selected: ["default", "tokyo"],
                    open: true,
                    searchable: true,
                    query: "night",
                    placeholder: "Theme",
                    search_placeholder: "Search themes",
                    empty_text: "No themes",
                    trigger: text("Custom theme trigger")
                        .with_style(style().background(theme_color("surface_raised"))),
                    header: text("Theme header"),
                    footer: text("Theme footer"),
                    empty: text("Custom empty state"),
                    part_styles: #{ option: style().height(px(41)) },
                    on_change: Fn("selected"),
                    on_open_change: Fn("opened"),
                    on_query_change: Fn("queried")
                })
            }
        "#,
    );
    let runtime = new_runtime();
    let lifecycle = mount(&mut engine, compiled, &runtime);
    let UiNodeKind::Overlay { trigger, spec, .. } = lifecycle.root().unwrap().kind() else {
        panic!("Combobox must compose the generic Overlay node");
    };
    assert_eq!(spec.id.as_str(), "theme");
    assert!(spec.open);
    assert_eq!(spec.width_policy, gpui_rhai::OverlayWidthPolicy::Content);
    assert!(
        matches!(trigger.kind(), UiNodeKind::Text { text } if text == "Custom theme trigger"),
        "a custom trigger must be the interactive trigger, not a child of default trigger chrome"
    );
    assert_eq!(
        trigger
            .style()
            .resolve(&gpui_rhai::InteractionState::default())
            .width,
        None,
        "default Combobox width must not wrap a custom trigger"
    );
    let collection = find_virtual_collection(lifecycle.root().unwrap())
        .expect("Combobox must use public data-backed virtualization");
    let UiNodeKind::VirtualCollection { spec } = collection.kind() else {
        unreachable!()
    };
    assert_eq!(spec.data.len(), 1, "keyword search must preserve one match");
    let option = spec.realized.values().next().unwrap();
    assert_eq!(
        option
            .style()
            .resolve(&gpui_rhai::InteractionState::default())
            .height,
        Some(gpui_rhai::Length::Pixels(41.0).into()),
        "virtual item renderers must inherit the component part-style snapshot"
    );
}

#[test]
fn official_combobox_groups_and_routes_keyboard_in_rhai() {
    let (mut engine, compiled) = compile_official(
        "ui/combobox_keyboard.rhai",
        r#"
            import "components/combobox" as combobox;
            fn view(ctx) {
                combobox::Combobox(#{
                    key: "grouped", label: "Grouped choices", open: false, selected: [], query: "",
                    options: [
                        #{ value: "a", label: "Alpha", group: "Second" },
                        #{ value: "b", label: "Beta", group: "First", disabled: true },
                        #{ value: "c", label: "Charlie", group: "Second" },
                        #{ value: "d", label: "Delta" }
                    ]
                })
            }
        "#,
    );
    let runtime = new_runtime();
    let mut lifecycle = mount(&mut engine, compiled, &runtime);

    let collection = find_virtual_collection(lifecycle.root().unwrap()).unwrap();
    let UiNodeKind::VirtualCollection { spec } = collection.kind() else {
        unreachable!()
    };
    let gpui_rhai::VirtualCollectionData::Values(data) = &spec.data else {
        panic!("Combobox fixture uses eager Rhai values");
    };
    let keys = data
        .iter()
        .map(|item| match item {
            UiValue::Map(item) => match &item["key"] {
                UiValue::String(key) => key.as_str(),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        keys,
        [
            "group:g:Second",
            "option:a",
            "option:c",
            "group:g:First",
            "option:b",
            "option:d",
        ]
    );

    let component = lifecycle.root().unwrap().component_root().unwrap().clone();
    let (callback, payload) = target_handler(lifecycle.root().unwrap(), "key:down");
    invoke_and_render(&mut lifecycle, &mut engine, &callback, payload);
    assert_eq!(
        runtime.borrow().component_state.get(&component, "active"),
        Some(&UiValue::String("a".to_owned()))
    );
    let events = runtime.borrow_mut().drain_batch().events;
    assert!(events.iter().any(|event| {
        event.event.name == "open_change" && event.event.payload == UiValue::Bool(true)
    }));

    let UiNodeKind::Overlay { content, .. } = lifecycle.root().unwrap().kind() else {
        unreachable!()
    };
    let (callback, payload) = target_handler(content, "key:down");
    invoke_and_render(&mut lifecycle, &mut engine, &callback, payload);
    assert_eq!(
        runtime.borrow().component_state.get(&component, "active"),
        Some(&UiValue::String("c".to_owned()))
    );
}

#[test]
fn official_combobox_rejects_invalid_choice_identity() {
    for (name, options, selected) in [
        (
            "duplicate",
            r#"[
                #{ value: "same", label: "One" },
                #{ value: "same", label: "Two" }
            ]"#,
            "[]",
        ),
        (
            // A selected value missing from `options` is accepted (options may load
            // later); two values in single mode are not.
            "two_in_single_mode",
            r#"[#{ value: "known", label: "Known" }, #{ value: "other", label: "Other" }]"#,
            r#"["known", "other"]"#,
        ),
        (
            "empty_group",
            r#"[#{ value: "known", label: "Known", group: " " }]"#,
            "[]",
        ),
    ] {
        let script = format!(
            r#"
                import "components/combobox" as combobox;
                fn view(ctx) {{
                    combobox::Combobox(#{{
                        key: "invalid", label: "Invalid choices", options: {options}, selected: {selected}, open: false, query: ""
                    }})
                }}
            "#
        );
        let script_name = format!("ui/combobox_{name}.rhai");
        let (mut engine, compiled) = compile_official(&script_name, &script);
        let mut lifecycle = lifecycle_for(&engine, compiled, &new_runtime());
        assert!(lifecycle.start(&mut engine).is_err(), "case {name}");
    }
}

#[test]
fn official_select_uses_scalar_controlled_choice_semantics() {
    let (mut engine, compiled) = compile_official(
        "ui/select_test.rhai",
        r#"
            import "components/select" as select;
            fn changed(ctx, value) { () }
            fn view(ctx) {
                select::Select(#{
                    key: "country", label: "Country", value: (), open: false, query: "",
                    searchable: true, clearable: true,
                    empty_text: "No countries",
                    options: [
                        #{ value: "cn", label: "China", group: "Asia" },
                        #{ value: "fr", label: "France", group: "Europe" }
                    ],
                    on_change: Fn("changed")
                })
            }
        "#,
    );
    let runtime = new_runtime();
    let lifecycle = mount(&mut engine, compiled, &runtime);
    let UiNodeKind::Overlay { spec, .. } = lifecycle.root().unwrap().kind() else {
        panic!("Select must compose Combobox over generic Overlay");
    };
    assert_eq!(spec.id.as_str(), "country-combobox");
}

#[test]
fn official_date_picker_consumes_locale_and_strict_iso_values() {
    let (mut engine, compiled) = compile_official("ui/date_picker_test.rhai", DATE_PICKER_TEST_APP);
    let runtime = new_runtime();
    runtime.borrow_mut().locale = Some(english(&engine));
    runtime.borrow_mut().calendar_clock =
        gpui_rhai::CalendarClock::fixed(gpui_rhai::GregorianDate::parse_iso("2026-08-30").unwrap());
    let mut lifecycle = mount(&mut engine, compiled, &runtime);
    let UiNodeKind::Overlay {
        trigger,
        content,
        spec,
    } = lifecycle.root().unwrap().kind()
    else {
        panic!("DatePicker must compose the public Overlay node");
    };
    assert_eq!(spec.id.as_str(), "appointment");
    let UiNodeKind::Box {
        children: trigger_children,
    } = trigger.kind()
    else {
        unreachable!()
    };
    assert!(
        matches!(&trigger_children[0].kind(), UiNodeKind::Text { text } if text == "09/01/2026")
    );
    let grid = find_role(content, "grid").expect("shared Calendar grid");
    let UiNodeKind::Box { children: weeks } = grid.kind() else {
        unreachable!()
    };
    assert_eq!(weeks.len(), 6);
    assert!(
        weeks
            .iter()
            .all(|week| matches!(week.kind(), UiNodeKind::Box { children } if children.len() == 7))
    );
    assert!(find_label(content, "Tuesday, September 1, 2026").is_some());

    let component = lifecycle.root().unwrap().component_root().unwrap().clone();
    let open = lifecycle
        .root()
        .unwrap()
        .handler("open_change")
        .and_then(gpui_rhai::UiEventHandler::as_script)
        .cloned()
        .unwrap();
    invoke_and_render(&mut lifecycle, &mut engine, &open, UiValue::Bool(true));
    assert_eq!(
        runtime.borrow().component_state.get(&component, "open"),
        Some(&UiValue::Bool(true))
    );
    let UiNodeKind::Overlay { content, .. } = lifecycle.root().unwrap().kind() else {
        unreachable!()
    };
    let calendar = find_label(content, "Appointment date").expect("shared Calendar");
    let calendar_component = calendar.component_root().unwrap().clone();
    let (right, payload) = target_handler(calendar, "key:right");
    invoke_and_render(&mut lifecycle, &mut engine, &right, payload);
    assert_eq!(
        runtime
            .borrow()
            .component_state
            .get(&calendar_component, "focused"),
        Some(&UiValue::String("2026-09-02".to_owned()))
    );
    let UiNodeKind::Overlay { content, .. } = lifecycle.root().unwrap().kind() else {
        unreachable!()
    };
    let calendar = find_label(content, "Appointment date").expect("shared Calendar");
    let (commit, payload) = target_handler(calendar, "key:enter");
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &commit, payload)
        .unwrap();
    let events = runtime.borrow_mut().drain_batch().events;
    assert!(events.iter().any(|event| {
        event.event.name == "change"
            && event.event.payload == UiValue::String("2026-09-02".to_owned())
    }));
    for event in events {
        let _ = lifecycle
            .invoke_component_event_transactional(&engine, event)
            .unwrap();
    }
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert_eq!(
        runtime.borrow().component_state.get(&component, "open"),
        Some(&UiValue::Bool(false))
    );
}

#[test]
fn official_table_is_public_data_backed_rhai_composition() {
    let (mut engine, compiled) = compile_official(
        "ui/table_test.rhai",
        r#"
            import "components/table" as table;
            fn sorted(ctx, value) { () }
            fn selected(ctx, value) { () }
            fn clicked(ctx, value) { () }
            fn view(ctx) {
                table::Table(#{
                    key: "users", label: "Users", row_key: "id", fill_height: true,
                    rows: [
                        #{ id: "u1", name: "Ada", score: 12.5, joined: "2026-08-30", status: "Active",
                            status_label: "Updated +1", status_variant: "success" },
                        #{ id: "u2", name: "Lin", score: 9, joined: "2026-08-31", status: "Away",
                            status_label: "Deploying", status_variant: "accent" }
                    ],
                    columns: [
                        #{ key: "name", title: "Name", width: #{ kind: "fixed", value: 120 }, sortable: true,
                            adornments: [#{ text_key: "status_label", variant_key: "status_variant" }] },
                        #{ key: "score", title: "Score", width: #{ kind: "flex", value: 2 } },
                        #{ key: "joined", title: "Joined", width: #{ kind: "percent", value: 30 } },
                        #{ key: "status", title: "Status", width: #{ kind: "fixed", value: 90 } }
                    ],
                    loading: false, selection_mode: "multiple", resizable_columns: true,
                    selected_keys: ["u1"], striped: true,
                    on_sort_change: Fn("sorted"), on_selection_change: Fn("selected"),
                    on_row_click: Fn("clicked")
                })
            }
        "#,
    );
    let runtime = new_runtime();
    let lifecycle = mount(&mut engine, compiled, &runtime);
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("Table must be a public Box composition");
    };
    let UiNodeKind::Box {
        children: header_cells,
    } = children[0].kind()
    else {
        panic!("Table header must remain a public row composition");
    };
    assert!(
        matches!(children[1].kind(), UiNodeKind::VirtualCollection { spec }
        if spec.data.len() == 2 && spec.height.is_none())
    );
    let UiNodeKind::VirtualCollection { spec } = children[1].kind() else {
        unreachable!()
    };
    let row = spec.realized.get(&0).expect("first table row");
    assert_eq!(
        row.style().base.overflow_x,
        Some(gpui_rhai::OverflowMode::Hidden)
    );
    let UiNodeKind::Box {
        children: row_children,
    } = row.kind()
    else {
        panic!("table row must remain a public row composition");
    };
    // A selected row carries the selection tint, then the indicator bar and the keyboard
    // cursor frame after its cells.
    assert!(matches!(
        row.style().base.background.as_ref(),
        Some(gpui_rhai::ColorValue::Token(token)) if token == "table.selection"
    ));
    assert_eq!(row_children.len(), 6);
    assert!(row_children[5].style().group_focus.is_some());
    assert_eq!(
        row_children[4].style().base.background,
        Some(gpui_rhai::ColorValue::Token("accent".to_owned()))
    );
    let cells = &row_children[..4];
    assert_eq!(header_cells.len(), 4);
    assert!(official_source("components/table").contains(".with_table_track("));
    assert_table_columns_delegate_width_to_native_track(header_cells);
    assert_table_columns_delegate_width_to_native_track(cells);
    assert_table_resize_handles(header_cells, &["Name", "Score", "Joined", "Status"]);
    assert_table_resize_source_widths(
        header_cells,
        &[("fixed", 120), ("flex", 2), ("percent", 30), ("fixed", 90)],
    );
    assert!(cells.iter().all(|cell| {
        cell.style().base.white_space == Some(gpui_rhai::WhiteSpaceMode::NoWrap)
            && cell.style().base.text_ellipsis == Some(true)
    }));
    assert_resizable_table_autofit_measurements(header_cells, cells);
    assert!(find_label(&cells[0], "Updated +1").is_some());
    assert_eq!(
        cells[0].attributes().get("label"),
        Some(&UiValue::String("Ada, Updated +1".to_owned()))
    );
}

#[test]
fn official_table_groups_array_rows_with_controlled_collapse() {
    let (mut engine, compiled) = compile_official(
        "ui/grouped_table_test.rhai",
        r#"
            import "components/table" as table;
            fn state_schema() { #{ fields: #{
                collapsed: #{ schema: #{ type: "array", max_items: 8,
                    items: #{ type: "string" } },
                    "default": #{ type: "array", value: [] } }
            } } }
            fn toggled(ctx, group) { ctx.set_state("collapsed", [group]); }
            fn view(ctx) {
                table::Table(#{
                    key: "clusters", label: "Clusters", row_key: "id", height: 120,
                    rows: [
                        #{ id: "a", track: "alpha", state: "Ready" },
                        #{ id: "b", track: "beta", state: "Drift" },
                        #{ id: "c", track: "alpha", state: "Drift" }
                    ],
                    columns: [
                        #{ key: "track", title: "Track",
                            width: #{ kind: "fixed", value: 120 } },
                        #{ key: "state", title: "State",
                            width: #{ kind: "fixed", value: 100 } }
                    ],
                    group_by: "track",
                    selection_mode: "multiple", selected_keys: ["b"],
                    collapsed_groups: ctx.get_state("collapsed"),
                    on_group_toggle: Fn("toggled")
                })
            }
        "#,
    );
    let runtime = new_runtime();
    let mut lifecycle = mount(&mut engine, compiled, &runtime);

    assert_table_has_no_width_signals(lifecycle.root().unwrap());

    let UiNodeKind::VirtualCollection { spec } = find_virtual_collection(lifecycle.root().unwrap())
        .expect("grouped Table must use virtual_collection")
        .kind()
    else {
        unreachable!()
    };
    assert_eq!(spec.data.len(), 5);
    assert_eq!(spec.sticky_headers.as_ref(), &BTreeSet::from([0, 3]));
    let selected = spec
        .realized
        .values()
        .find(|row| row.attributes().get("selected") == Some(&UiValue::Bool(true)))
        .expect("selected grouped row");
    assert!(matches!(
        selected.style().base.background.as_ref(),
        Some(gpui_rhai::ColorValue::Token(token)) if token == "table.selection"
    ));
    let first_group = spec.realized.get(&0).expect("first group header");
    assert_eq!(
        first_group.attributes().get("label"),
        Some(&UiValue::String("alpha, 2".to_owned()))
    );
    let (toggle, payload) = target_handler(first_group, "click");
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &toggle, payload)
        .unwrap();
    let events = runtime.borrow_mut().drain_batch().events;
    assert!(events.iter().any(|event| {
        event.event.name == "group_toggle"
            && event.event.payload == UiValue::String("alpha".to_owned())
    }));
    for event in events {
        let _ = lifecycle
            .invoke_component_event_transactional(&engine, event)
            .unwrap();
    }
    assert!(lifecycle.render_dirty(&mut engine).unwrap());

    let UiNodeKind::VirtualCollection { spec } = find_virtual_collection(lifecycle.root().unwrap())
        .expect("collapsed Table must retain virtual_collection")
        .kind()
    else {
        unreachable!()
    };
    assert_eq!(spec.data.len(), 3);
    assert_eq!(spec.sticky_headers.as_ref(), &BTreeSet::from([0, 1]));
}

#[test]
#[allow(clippy::too_many_lines)]
fn official_table_groups_native_collection_without_materializing_rows_in_rhai() {
    let (mut engine, compiled) = compile_official(
        "ui/native_grouped_table_test.rhai",
        r#"
            import "components/table" as table;
            fn view(ctx) {
                table::Table(#{
                    key: "clusters", label: "Clusters", row_key: "id", height: 120,
                    rows: ctx.get_native_collection("clusters"), group_by: "track",
                    resizable_columns: true,
                    columns: [
                        #{ key: "track", title: "Track",
                            width: #{ kind: "fixed", value: 120 } },
                        #{ key: "state", title: "State",
                            width: #{ kind: "flex", value: 2 },
                            adornments: [#{ text_key: "status_label", variant_key: "status_variant", dot: true }] }
                    ]
                })
            }
        "#,
    );
    let rows = gpui_rhai::NativeCollection::new(
        "id",
        [
            BTreeMap::from([
                ("id".to_owned(), UiValue::String("a".to_owned())),
                ("track".to_owned(), UiValue::String("alpha".to_owned())),
                ("state".to_owned(), UiValue::String("Ready".to_owned())),
                (
                    "status_label".to_owned(),
                    UiValue::String("Healthy".to_owned()),
                ),
                (
                    "status_variant".to_owned(),
                    UiValue::String("success".to_owned()),
                ),
            ]),
            BTreeMap::from([
                ("id".to_owned(), UiValue::String("b".to_owned())),
                ("track".to_owned(), UiValue::String("beta".to_owned())),
                ("state".to_owned(), UiValue::String("Drift".to_owned())),
                (
                    "status_label".to_owned(),
                    UiValue::String("Warning".to_owned()),
                ),
                (
                    "status_variant".to_owned(),
                    UiValue::String("warning".to_owned()),
                ),
            ]),
            BTreeMap::from([
                ("id".to_owned(), UiValue::String("c".to_owned())),
                ("track".to_owned(), UiValue::String("alpha".to_owned())),
                ("state".to_owned(), UiValue::String("Drift".to_owned())),
                (
                    "status_label".to_owned(),
                    UiValue::String("Deploying".to_owned()),
                ),
                (
                    "status_variant".to_owned(),
                    UiValue::String("accent".to_owned()),
                ),
            ]),
        ],
    )
    .unwrap();
    let runtime = new_runtime();
    runtime
        .borrow_mut()
        .native_collections
        .register("clusters", rows)
        .unwrap();
    let lifecycle = mount(&mut engine, compiled, &runtime);

    let UiNodeKind::VirtualCollection { spec } = find_virtual_collection(lifecycle.root().unwrap())
        .expect("native grouped Table must use virtual_collection")
        .kind()
    else {
        unreachable!()
    };
    assert_eq!(spec.data.len(), 5);
    assert_eq!(spec.sticky_headers.as_ref(), &BTreeSet::from([0, 3]));
    assert_eq!(
        spec.realized[&0].attributes().get("label"),
        Some(&UiValue::String("alpha, 2".to_owned()))
    );
    let UiNodeKind::Box { children: cells } = spec.realized[&1].kind() else {
        panic!("native Table row must remain a public row composition");
    };
    assert_eq!(cells.len(), 2);
    assert_table_columns_delegate_width_to_native_track(cells);
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        unreachable!()
    };
    let UiNodeKind::Box { children: headers } = children[0].kind() else {
        unreachable!()
    };
    assert_table_columns_delegate_width_to_native_track(headers);
    assert_table_resize_source_widths(headers, &[("fixed", 120), ("flex", 2)]);
    assert!(contains_primitive(
        &cells[0],
        "gpui_rhai.intrinsic_text_measure"
    ));
    // The last column is resizable too, so it measures for auto-fit like the others.
    assert!(contains_primitive(
        &cells[1],
        "gpui_rhai.intrinsic_text_measure"
    ));
    assert!(find_label(&cells[1], "Healthy").is_some());
}

#[test]
fn official_pagination_is_pure_rhai_composition() {
    let (mut engine, compiled) = compile_official(
        "ui/pagination_test.rhai",
        r#"
            import "components/pagination" as pagination;
            fn changed(ctx, value) { () }
            fn view(ctx) {
                pagination::Pagination(#{
                    key: "users-pages", label: "Users pagination", total_items: 5000,
                    current_page: 250, page_size: 10,
                    page_size_options: [10, 25, 50],
                    on_change: Fn("changed")
                })
            }
        "#,
    );
    let runtime = new_runtime();
    runtime.borrow_mut().locale = Some(english(&engine));
    let lifecycle = mount(&mut engine, compiled, &runtime);
    let root = lifecycle.root().unwrap();
    assert!(matches!(root.kind(), UiNodeKind::Box { .. }));
    assert!(contains_overlay(root));
    let page = find_label(root, "251").expect("page 251 button");
    assert_eq!(
        page.handler_payload("click"),
        Some(&UiValue::Map(BTreeMap::from([
            ("current_page".to_owned(), UiValue::Integer(251)),
            ("page_size".to_owned(), UiValue::Integer(10)),
        ])))
    );
    assert!(page.style().hover.is_some());
    let current = find_label(root, "250").expect("current page button");
    assert_eq!(
        current.attributes().get("current"),
        Some(&UiValue::String("page".to_owned()))
    );
    assert_eq!(
        current.attributes().get("disabled"),
        Some(&UiValue::Bool(false))
    );
    assert!(current.handler("click").is_none());
    assert!(current.style().hover.is_none());
}

#[test]
fn official_pagination_rejects_duplicate_page_sizes() {
    let (mut engine, compiled) = compile_official(
        "ui/pagination_duplicate_test.rhai",
        r#"
            import "components/pagination" as pagination;
            fn view(ctx) {
                pagination::Pagination(#{
                    key: "pages", label: "Pagination", total_items: 100, current_page: 1,
                    page_size: 10, page_size_options: [10, 10]
                })
            }
        "#,
    );
    let mut state = UiRuntimeState::new();
    state.locale = Some(english(&engine));
    let error = try_render_in(&mut engine, &compiled, state).unwrap_err();
    assert!(error.to_string().contains("duplicate value 10"));
}

#[test]
fn official_pagination_page_window_covers_ellipsis_transitions() {
    let (mut engine, compiled) = compile_official(
        "ui/pagination_windows.rhai",
        r#"
            import "components/pagination" as pagination;
            fn window_nodes(values) {
                let nodes = [];
                for value in values { nodes.push(text(`${value}`)); }
                row(nodes)
            }
            fn view(ctx) {
                column([
                    window_nodes(pagination::page_window(1, 1, 1, 1)),
                    window_nodes(pagination::page_window(5, 3, 1, 1)),
                    window_nodes(pagination::page_window(7, 1, 1, 1)),
                    window_nodes(pagination::page_window(7, 3, 1, 1)),
                    window_nodes(pagination::page_window(7, 4, 1, 1)),
                    window_nodes(pagination::page_window(7, 5, 1, 1)),
                    window_nodes(pagination::page_window(100, 50, 2, 2)),
                ])
            }
        "#,
    );
    let root = render(&mut engine, &compiled);
    let UiNodeKind::Box { children: cases } = root.kind() else {
        panic!("pagination page-window probe must render a column");
    };
    let actual = cases
        .iter()
        .map(|case| {
            let UiNodeKind::Box { children } = case.kind() else {
                panic!("pagination page-window case must render a row");
            };
            children
                .iter()
                .map(|node| match node.kind() {
                    UiNodeKind::Text { text } => text.parse::<i64>().unwrap(),
                    _ => panic!("pagination page-window values must be text"),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        vec![
            vec![1],
            vec![1, 2, 3, 4, 5],
            vec![1, 2, 0, 7],
            vec![1, 2, 3, 4, 0, 7],
            vec![1, 2, 3, 4, 5, 6, 7],
            vec![1, 0, 4, 5, 6, 7],
            vec![1, 2, 0, 48, 49, 50, 51, 52, 0, 99, 100],
        ]
    );
}

#[test]
fn official_pagination_page_size_emits_one_atomic_reset() {
    let (mut engine, compiled) = compile_official(
        "ui/pagination_atomic.rhai",
        r#"
            import "components/pagination" as pagination;
            fn state_schema() { #{ fields: #{
                current_page: #{ schema: #{ type: "integer", min: 1 },
                    "default": #{ type: "integer", value: 4 } },
                page_size: #{ schema: #{ type: "integer", min: 1 },
                    "default": #{ type: "integer", value: 10 } },
            } } }
            fn changed(ctx, value) {
                ctx.set_state("current_page", value.current_page);
                ctx.set_state("page_size", value.page_size);
            }
            fn view(ctx) {
                pagination::Pagination(#{
                    key: "pages", label: "Pagination", total_items: 100,
                    current_page: ctx.get_state("current_page"),
                    page_size: ctx.get_state("page_size"),
                    page_size_options: [10, 25], on_change: Fn("changed")
                })
            }
        "#,
    );
    let runtime = new_runtime();
    runtime.borrow_mut().locale = Some(english(&engine));
    let path = root_path();
    let lifecycle = mount(&mut engine, compiled, &runtime);
    let collection = find_virtual_collection(lifecycle.root().unwrap())
        .expect("page-size Select virtual options");
    let UiNodeKind::VirtualCollection { spec } = collection.kind() else {
        unreachable!()
    };
    let option = spec
        .realized
        .values()
        // An option row carries its value; the click computes the next selection.
        .find(|node| node.handler_payload("click") == Some(&UiValue::String("25".to_owned())))
        .expect("page-size option 25");
    let callback = option
        .handler("click")
        .and_then(gpui_rhai::UiEventHandler::as_script)
        .cloned()
        .unwrap();
    let payload = option.handler_payload("click").cloned().unwrap();
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &callback, payload)
        .unwrap();
    for _ in 0..3 {
        let events = runtime.borrow_mut().drain_batch().events;
        if events.is_empty() {
            break;
        }
        for event in events {
            let _ = lifecycle
                .invoke_component_event_transactional(&engine, event)
                .unwrap();
        }
    }
    assert_eq!(
        runtime.borrow().component_state.get(&path, "current_page"),
        Some(&UiValue::Integer(1))
    );
    assert_eq!(
        runtime.borrow().component_state.get(&path, "page_size"),
        Some(&UiValue::Integer(25))
    );
}

#[test]
fn checkbox_switch_radio_group_and_tag_emit_typed_payloads_and_roving_keys() {
    let (mut engine, compiled) = compile_official(
        "ui/controls.rhai",
        r#"
            import "components/checkbox" as checkbox;
            import "components/radio_group" as radio_group;
            import "components/switch" as switch_component;
            import "components/tag" as tag;
            fn changed(ctx, value) { () }
            fn view(ctx) {
                column([
                    checkbox::Checkbox(#{
                        checked: false, indeterminate: true, label: "Mixed",
                        on_change: Fn("changed")
                    }),
                    switch_component::Switch(#{
                        checked: true, label: "Sync", on_change: Fn("changed")
                    }),
                    radio_group::RadioGroup(#{
                        value: "b", label: "Choice",
                        options: [
                            #{ value: "a", label: "A" },
                            #{ value: "b", label: "B" },
                            #{ value: "c", label: "C", disabled: true }
                        ],
                        on_change: Fn("changed")
                    }),
                    tag::Tag(#{
                        text: "Rust", variant: "accent", closable: true,
                        on_close: Fn("changed")
                    })
                ])
            }
        "#,
    );
    let root = render(&mut engine, &compiled);
    let UiNodeKind::Box { children } = root.kind() else {
        panic!("controls must compose into a container");
    };
    assert!(matches!(
        children[0].handler_payload("click"),
        Some(UiValue::Map(payload))
            if payload["checked"] == UiValue::Bool(true)
                && payload["indeterminate"] == UiValue::Bool(false)
    ));
    assert_eq!(
        children[0].attributes().get("checked"),
        Some(&UiValue::String("mixed".to_owned()))
    );
    assert_eq!(
        children[1].handler_payload("click"),
        Some(&UiValue::Bool(false))
    );
    assert!(children[2].handlers().contains_key("key:left"));
    assert!(children[2].handlers().contains_key("key:right"));
    assert_eq!(
        children[2].handler_payload("key:right"),
        Some(&UiValue::String("a".to_owned()))
    );
    // Tag: [value segment [label, close]]; a facet segment would come first.
    let UiNodeKind::Box {
        children: tag_children,
    } = children[3].kind()
    else {
        panic!("Tag must render its value segment");
    };
    let UiNodeKind::Box {
        children: value_children,
    } = tag_children[0].kind()
    else {
        panic!("Tag value segment must render label and close affordance");
    };
    assert_eq!(
        value_children[1].handler_payload("click"),
        Some(&UiValue::String("Rust".to_owned()))
    );
    assert_eq!(
        value_children[1].attributes().get("label"),
        Some(&UiValue::String("Remove Rust".to_owned()))
    );
}

#[test]
fn radio_group_forwards_pointer_and_keyboard_changes_to_its_caller() {
    let (mut engine, compiled) = compile_official(
        "ui/radio_group_event_boundary.rhai",
        r#"
            import "components/radio_group" as radio_group;
            fn state_schema() { #{ fields: #{
                region: #{ schema: #{ type: "string" },
                    "default": #{ type: "string", value: "b" } }
            } } }
            fn changed(ctx, value) { ctx.set_state("region", value); }
            fn view(ctx) {
                radio_group::RadioGroup(#{
                    value: ctx.get_state("region"), label: "Region",
                    options: [
                        #{ value: "a", label: "A" },
                        #{ value: "b", label: "B" },
                        #{ value: "c", label: "C" }
                    ],
                    on_change: Fn("changed")
                })
            }
        "#,
    );
    let runtime = new_runtime();
    let path = root_path();
    let mut lifecycle = mount(&mut engine, compiled, &runtime);

    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("RadioGroup must render its Radio options in a Box");
    };
    let (select, payload) = target_handler(&children[0], "click");
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &select, payload)
        .unwrap();
    let events = runtime.borrow_mut().drain_batch().events;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event.name, "change");
    assert_eq!(events[0].event.payload, UiValue::String("a".to_owned()));
    for event in events {
        let _ = lifecycle
            .invoke_component_event_transactional(&engine, event)
            .unwrap();
    }
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert_eq!(
        runtime.borrow().component_state.get(&path, "region"),
        Some(&UiValue::String("a".to_owned()))
    );

    let (right, payload) = target_handler(lifecycle.root().unwrap(), "key:right");
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &right, payload)
        .unwrap();
    let events = runtime.borrow_mut().drain_batch().events;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event.payload, UiValue::String("b".to_owned()));
    for event in events {
        let _ = lifecycle
            .invoke_component_event_transactional(&engine, event)
            .unwrap();
    }
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert_eq!(
        runtime.borrow().component_state.get(&path, "region"),
        Some(&UiValue::String("b".to_owned()))
    );
}

#[test]
fn toggle_group_routes_pointer_and_roving_keyboard_changes_to_caller_state() {
    let (mut engine, compiled) = compile_official(
        "ui/toggle_group_controlled.rhai",
        r#"
            import "components/toggle_group" as toggle_group;
            fn state_schema() { #{ fields: #{
                formats: #{ schema: #{ type: "array", max_items: 4, items: #{ type: "string" } },
                    "default": #{ type: "array", value: [] } }
            } } }
            fn changed(ctx, values) { ctx.set_state("formats", values); }
            fn view(ctx) {
                toggle_group::ToggleGroup(#{
                    key: "format", label: "Formatting", mode: "multiple",
                    values: ctx.get_state("formats"),
                    items: [
                        #{ value: "bold", label: "Bold" },
                        #{ value: "italic", label: "Italic" },
                        #{ value: "code", label: "Code", disabled: true }
                    ],
                    on_change: Fn("changed")
                })
            }
        "#,
    );
    let runtime = new_runtime();
    let path = root_path();
    let mut lifecycle = mount(&mut engine, compiled, &runtime);

    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("ToggleGroup must render its segments");
    };
    // Roving focus: only the active segment is a tab stop.
    assert_eq!(
        children[0].attributes().get("tab_stop"),
        Some(&UiValue::Bool(true))
    );
    assert_eq!(
        children[1].attributes().get("tab_stop"),
        Some(&UiValue::Bool(false))
    );
    let press = |lifecycle: &mut ScriptLifecycle, engine: &mut RuntimeEngine, index: usize| {
        let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
            unreachable!()
        };
        let handler = children[index]
            .handler("click")
            .and_then(gpui_rhai::UiEventHandler::as_script)
            .cloned()
            .unwrap();
        let _ = lifecycle
            .invoke_callback_transactional(engine, &handler, UiValue::Null)
            .unwrap();
        let events = runtime.borrow_mut().drain_batch().events;
        assert_eq!(events.len(), 1);
        for event in events {
            let _ = lifecycle
                .invoke_component_event_transactional(engine, event)
                .unwrap();
        }
        assert!(lifecycle.render_dirty(engine).unwrap());
    };
    press(&mut lifecycle, &mut engine, 0);
    assert_eq!(
        runtime.borrow().component_state.get(&path, "formats"),
        Some(&UiValue::Array(vec![UiValue::String("bold".to_owned())]))
    );

    // Arrow keys move the roving segment; Enter/Space then press the focused segment.
    let (right, payload) = target_handler(lifecycle.root().unwrap(), "key:right");
    invoke_and_render(&mut lifecycle, &mut engine, &right, payload);
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        unreachable!()
    };
    assert_eq!(
        children[1].attributes().get("tab_stop"),
        Some(&UiValue::Bool(true))
    );
    press(&mut lifecycle, &mut engine, 1);
    assert_eq!(
        runtime.borrow().component_state.get(&path, "formats"),
        Some(&UiValue::Array(vec![
            UiValue::String("bold".to_owned()),
            UiValue::String("italic".to_owned()),
        ]))
    );
}

#[test]
fn slider_uses_the_public_native_range_input_with_range_semantics() {
    let (mut engine, compiled) = compile_official(
        "ui/slider.rhai",
        r#"
            import "components/slider" as slider;
            fn changed(ctx, value) { () }
            fn view(ctx) {
                slider::Slider(#{ key: "volume", label: "Volume",
                    value: 40, min: 0, max: 100, step: 5,
                    on_change: Fn("changed") })
            }
        "#,
    );
    let root = render(&mut engine, &compiled);
    let UiNodeKind::Box { children } = root.kind() else {
        panic!("Slider must render label/value and native control");
    };
    let UiNodeKind::Custom { primitive } = children[1].kind() else {
        panic!("Slider control must use the generic RangeInput primitive");
    };
    assert_eq!(primitive.primitive.as_str(), "gpui_rhai.range_input");
    assert_eq!(
        children[1].attributes().get("role"),
        Some(&UiValue::String("slider".to_owned()))
    );
    assert_eq!(
        children[1].attributes().get("value_min"),
        Some(&UiValue::Float(0.0))
    );
    assert_eq!(
        children[1].attributes().get("value_max"),
        Some(&UiValue::Float(100.0))
    );
}

#[test]
fn scroll_area_decorates_the_generic_retained_scroll_node() {
    let (mut engine, compiled) = compile_official(
        "ui/scroll_area.rhai",
        r#"
            import "components/scroll_area" as scroll_area;
            fn view(ctx) {
                scroll_area::ScrollArea(#{ key: "logs", label: "Logs", width: px(240), height: px(120),
                    axis: "vertical", vertical_scrollbar: "always",
                    content: column([text("one"), text("two"), text("three")]) })
            }
        "#,
    );
    let root = render(&mut engine, &compiled);
    assert_eq!(
        root.attributes().get("scrollbar_horizontal"),
        Some(&UiValue::String("hidden".to_owned()))
    );
    assert_eq!(
        root.attributes().get("scrollbar_vertical"),
        Some(&UiValue::String("always".to_owned()))
    );
    assert_eq!(
        root.style().base.overflow_y,
        Some(gpui_rhai::OverflowMode::Scroll)
    );
    assert!(root.part_style("scrollbar_thumb_hover").is_some());
}

#[test]
fn avatar_progress_skeleton_and_spinner_render_fallbacks_and_native_motion() {
    let (mut engine, compiled) = compile_official(
        "ui/visuals.rhai",
        r#"
            import "components/avatar" as avatar;
            import "components/progress" as progress;
            import "components/skeleton" as skeleton;
            import "components/spinner" as spinner;
            fn view(ctx) {
                column([
                    avatar::Avatar(#{ name: "Ada", presence: "online" }),
                    progress::Progress(#{ key: "download", label: "Download", value: 42, max: 100 }),
                    progress::Progress(#{ key: "loading", label: "Loading", indeterminate: true }),
                    skeleton::Skeleton(#{ key: "card", width: 240, height: 80 }),
                    spinner::Spinner(#{ key: "loading-spinner", label: "Loading" })
                ])
            }
        "#,
    );
    let root = render(&mut engine, &compiled);
    let UiNodeKind::Box { children } = root.kind() else {
        panic!("visual primitives must compose into a container");
    };
    assert_eq!(
        children[0].attributes().get("label"),
        Some(&UiValue::String("Ada".to_owned()))
    );
    let UiNodeKind::Box {
        children: determinate,
    } = children[1].kind()
    else {
        panic!("Progress must render a track container");
    };
    assert!(matches!(
        determinate[0].motions()[0],
        gpui_rhai::MotionSource::Transition(_)
    ));
    assert_eq!(
        children[1].attributes().get("value_min"),
        Some(&UiValue::Float(0.0))
    );
    assert_eq!(
        children[1].attributes().get("value_max"),
        Some(&UiValue::Float(100.0))
    );
    let UiNodeKind::Box {
        children: indeterminate,
    } = children[2].kind()
    else {
        panic!("Progress must render a track container");
    };
    assert!(matches!(
        indeterminate[0].motions()[0],
        gpui_rhai::MotionSource::Transition(ref spec) if spec.iterations.is_none()
    ));
    assert!(matches!(
        children[3].motions()[0],
        gpui_rhai::MotionSource::Transition(ref spec) if spec.iterations.is_none()
    ));
    assert!(matches!(
        children[4].motions()[0],
        gpui_rhai::MotionSource::Transition(ref spec)
            if spec.property == gpui_rhai::MotionProperty::Rotate && spec.iterations.is_none()
    ));
}

#[test]
fn form_field_and_disclosure_composites_wire_labels_motion_and_keys() {
    let (mut engine, compiled) = compile_official("ui/composites.rhai", COMPOSITES_APP);
    let root = render(&mut engine, &compiled);
    let UiNodeKind::Box { children } = root.kind() else {
        panic!("composites must compose into a container");
    };
    assert_eq!(
        children[0].attributes().get("invalid"),
        Some(&UiValue::Bool(true))
    );
    let UiNodeKind::Box {
        children: field_children,
    } = children[0].kind()
    else {
        panic!("FormField must render associated children");
    };
    assert_eq!(
        field_children[0].attributes().get("semantic_id"),
        Some(&UiValue::String("name-label".to_owned()))
    );
    assert_eq!(
        field_children[1].attributes().get("labelled_by"),
        Some(&UiValue::String("name-label".to_owned()))
    );
    assert_eq!(
        field_children[1].attributes().get("described_by"),
        Some(&UiValue::String("name-error".to_owned()))
    );
    assert_eq!(
        field_children[1].attributes().get("required"),
        Some(&UiValue::Bool(true))
    );
    let UiNodeKind::Box {
        children: collapsible_children,
    } = children[1].kind()
    else {
        panic!("Collapsible must render trigger and panel");
    };
    assert!(matches!(
        collapsible_children[1].motions()[0],
        gpui_rhai::MotionSource::Transition(_)
    ));
    let UiNodeKind::Box {
        children: accordion_items,
    } = children[2].kind()
    else {
        panic!("Accordion must render item containers");
    };
    let UiNodeKind::Box {
        children: first_item,
    } = accordion_items[0].kind()
    else {
        panic!("Accordion item must render trigger and panel");
    };
    assert!(first_item[0].handler_payload("click").is_some());
    assert_ne!(first_item[1].motions(), []);
    let UiNodeKind::Box {
        children: tabs_root,
    } = children[3].kind()
    else {
        panic!("Tabs must render list and panel");
    };
    assert!(tabs_root[0].handlers().contains_key("key:left"));
    assert_eq!(
        tabs_root[0].handler_payload("key:right"),
        Some(&UiValue::String("advanced".to_owned()))
    );
}

#[test]
fn tooltip_and_menu_use_window_overlay_policies() {
    let (mut engine, compiled) = compile_official(
        "ui/tooltip_menu.rhai",
        r#"
            import "components/tooltip" as tooltip;
            import "components/menu" as menu;
            fn changed(ctx, value) { () }
            fn view(ctx) {
                column([
                    tooltip::Tooltip(#{
                        key: "help", label: "Help", trigger: text("?"), content: text("Help"),
                        show_delay_ms: 250, hide_delay_ms: 75
                    }),
                    menu::Menu(#{
                        key: "file", label: "File menu", trigger: text("File"), open: true,
                        active_value: "open",
                        items: [
                            #{ kind: "item", value: "new", label: "New", shortcut: "⌘N" },
                            #{ kind: "separator" },
                            #{ kind: "item", value: "open", label: "Open", checked: true }
                        ],
                        on_action: Fn("changed"),
                        on_active_change: Fn("changed"),
                        on_open_change: Fn("changed")
                    })
                ])
            }
        "#,
    );
    let root = render(&mut engine, &compiled);
    let UiNodeKind::Box { children } = root.kind() else {
        panic!("Tooltip and Menu must compose into a container");
    };
    let UiNodeKind::Overlay { spec: tooltip, .. } = children[0].kind() else {
        panic!("Tooltip must use an overlay node");
    };
    assert_eq!(tooltip.kind, gpui_rhai::OverlayKind::Tooltip);
    assert_eq!(tooltip.tooltip_delays.unwrap().show_ms, 250);
    let UiNodeKind::Overlay {
        content,
        spec: menu,
        ..
    } = children[1].kind()
    else {
        panic!("Menu must use an overlay node");
    };
    assert_eq!(menu.kind, gpui_rhai::OverlayKind::Menu);
    assert!(content.handlers().contains_key("key:up"));
    assert!(content.handlers().contains_key("key:enter"));
    assert!(content.handlers().contains_key("key:n"));
    assert_eq!(
        content.handler_payload("key:n"),
        Some(&UiValue::String("new".to_owned()))
    );
}

#[test]
fn nested_menu_preserves_parent_overlay_identity() {
    let (mut engine, compiled) = compile_official(
        "ui/nested_menu.rhai",
        r#"
            import "components/menu" as menu;
            fn changed(ctx, value) { () }
            fn view(ctx) {
                let child = menu::Menu(#{
                    key: "file-more", label: "More actions", parent_overlay: "file", trigger: text("More"),
                    open: true, active_value: "export", placement: "right",
                    items: [#{ kind: "item", value: "export", label: "Export" }],
                    on_action: Fn("changed"), on_active_change: Fn("changed"),
                    on_open_change: Fn("changed")
                });
                menu::Menu(#{
                    key: "file", label: "File menu", trigger: text("File"), open: true,
                    active_value: "more",
                    items: [#{
                        kind: "submenu", value: "more", label: "More", submenu: child
                    }],
                    on_action: Fn("changed"), on_active_change: Fn("changed"),
                    on_open_change: Fn("changed")
                })
            }
        "#,
    );
    let root = render(&mut engine, &compiled);
    let UiNodeKind::Overlay { content, spec, .. } = root.kind() else {
        panic!("parent Menu must render an overlay");
    };
    assert_eq!(spec.id.as_str(), "file");
    let UiNodeKind::Box { children } = content.kind() else {
        panic!("parent Menu content must contain submenu nodes");
    };
    let UiNodeKind::Overlay { spec: child, .. } = children[0].kind() else {
        panic!("submenu must remain an overlay node");
    };
    assert_eq!(child.id.as_str(), "file-more");
    assert_eq!(
        child.parent.as_ref().map(gpui_rhai::OverlayId::as_str),
        Some("file")
    );
    assert_eq!(child.kind, gpui_rhai::OverlayKind::Menu);
}

#[test]
fn toast_source_builds_public_layers_and_declarative_timers() {
    let (mut engine, compiled) = compile_official(
        "ui/toast_test.rhai",
        r#"
            import "components/toast" as toast;
            fn dismissed(ctx, id) { () }
            fn view(ctx) {
                toast::Toast(#{
                    key: "notifications",
                    max_visible: 2,
                    items: [
                        #{ id: "saved", title: "Saved", variant: "success", duration_ms: 1200 },
                        #{ id: "warning", title: "Warning", message: "Check input", variant: "warning", paused: true }
                    ],
                    part_styles: #{ title: style().font_size(px(18)) },
                    on_dismiss: Fn("dismissed")
                })
            }
        "#,
    );
    let manual_clock = gpui_rhai::ManualRuntimeClock::new(std::time::Instant::now());
    let runtime = new_runtime();
    runtime.borrow_mut().clock = manual_clock.clock();
    let lifecycle = mount(&mut engine, compiled, &runtime);
    let UiNodeKind::Box { children: layers } = lifecycle.root().unwrap().kind() else {
        panic!("Toast must be a public Box/Layer composition");
    };
    assert_eq!(layers.len(), 1);
    let UiNodeKind::Layer { content, spec } = layers[0].kind() else {
        panic!("Toast region must use the generic Layer node");
    };
    assert_eq!(spec.placement, gpui_rhai::LayerPlacement::TopRight);
    let UiNodeKind::Box { children: toasts } = content.kind() else {
        unreachable!()
    };
    assert_eq!(toasts.len(), 2);
    assert!(toasts[0].handlers().contains_key("hover_change"));
    let UiNodeKind::Box { children } = toasts[0].kind() else {
        unreachable!()
    };
    let UiNodeKind::Box { children: header } = children[0].kind() else {
        unreachable!()
    };
    // A status toast leads with its square lamp, then the styled title.
    assert_eq!(
        header[0].style().base.background,
        Some(ColorValue::Token("success".to_owned()))
    );
    assert_eq!(
        header[1]
            .style()
            .resolve(&gpui_rhai::InteractionState::default())
            .font_size,
        Some(gpui_rhai::Length::Pixels(18.0))
    );
    assert_eq!(runtime.borrow().timers.active_count(), 2);
    manual_clock.advance(std::time::Duration::from_secs(2));
    let now = runtime.borrow().clock.now();
    let deliveries = runtime
        .borrow_mut()
        .timers
        .drain(now, lifecycle.generation());
    assert_eq!(deliveries.len(), 1, "paused toast must retain its deadline");
    assert_eq!(deliveries[0].payload, UiValue::String("saved".to_owned()));
    let _ = lifecycle
        .invoke_async_delivery_transactional(&engine, deliveries.into_iter().next().unwrap())
        .unwrap();
    let events = runtime.borrow_mut().drain_batch().events;
    assert!(events.iter().any(|event| {
        event.event.name == "dismiss" && event.event.payload == UiValue::String("saved".to_owned())
    }));
}
