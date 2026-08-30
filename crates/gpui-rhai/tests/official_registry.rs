use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui_rhai::{
    AssetData, ComponentInstancePath, ComponentStateSchema, EmbeddedScriptSource, ExecutionPhase,
    InMemoryAssetProvider, ModuleId, OpaqueHandle, RestrictedModuleResolver, RuntimeEngine,
    ScriptLifecycle, StateField, UiContext, UiNodeKind, UiRuntimeState, UiValue, ValueSchema,
    parse_component_header,
};

const BUTTON: &str = include_str!("../../../registry/components/button.rhai");
const LABEL: &str = include_str!("../../../registry/components/label.rhai");
const ICON: &str = include_str!("../../../registry/components/icon.rhai");
const INPUT: &str = include_str!("../../../registry/components/input.rhai");
const TEXTAREA: &str = include_str!("../../../registry/components/textarea.rhai");
const DIVIDER: &str = include_str!("../../../registry/components/divider.rhai");
const POPOVER: &str = include_str!("../../../registry/components/popover.rhai");
const DIALOG: &str = include_str!("../../../registry/components/dialog.rhai");
const DROPDOWN: &str = include_str!("../../../registry/components/dropdown.rhai");
const SELECT: &str = include_str!("../../../registry/components/select.rhai");
const DATE_PICKER: &str = include_str!("../../../registry/components/date_picker.rhai");
const TABLE: &str = include_str!("../../../registry/components/table.rhai");
const PAGINATION: &str = include_str!("../../../registry/components/pagination.rhai");
const CHECKBOX: &str = include_str!("../../../registry/components/checkbox.rhai");
const RADIO: &str = include_str!("../../../registry/components/radio.rhai");
const RADIO_GROUP: &str = include_str!("../../../registry/components/radio_group.rhai");
const SWITCH: &str = include_str!("../../../registry/components/switch.rhai");
const TAG: &str = include_str!("../../../registry/components/tag.rhai");
const AVATAR: &str = include_str!("../../../registry/components/avatar.rhai");
const PROGRESS: &str = include_str!("../../../registry/components/progress.rhai");
const SKELETON: &str = include_str!("../../../registry/components/skeleton.rhai");
const FORM_FIELD: &str = include_str!("../../../registry/components/form_field.rhai");
const COLLAPSIBLE: &str = include_str!("../../../registry/components/collapsible.rhai");
const ACCORDION: &str = include_str!("../../../registry/components/accordion.rhai");
const TABS: &str = include_str!("../../../registry/components/tabs.rhai");
const TOOLTIP: &str = include_str!("../../../registry/components/tooltip.rhai");
const MENU: &str = include_str!("../../../registry/components/menu.rhai");
const TOAST: &str = include_str!("../../../registry/components/toast.rhai");
const M2_COMPOSITES_APP: &str = r#"
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
const DEFAULT_LIGHT: &str = include_str!("../../../registry/themes/default_light.rhai");
const DEFAULT_DARK: &str = include_str!("../../../registry/themes/default_dark.rhai");
const TOKYO_NIGHT: &str = include_str!("../../../registry/themes/tokyo_night.rhai");
const TOKYO_STORM: &str = include_str!("../../../registry/themes/tokyo_storm.rhai");
const CATPPUCCIN_LATTE: &str = include_str!("../../../registry/themes/catppuccin_latte.rhai");
const CATPPUCCIN_MOCHA: &str = include_str!("../../../registry/themes/catppuccin_mocha.rhai");

fn contains_select(node: &gpui_rhai::UiNode) -> bool {
    match node.kind() {
        UiNodeKind::Select { .. } => true,
        UiNodeKind::Container { children } => children.iter().any(contains_select),
        _ => false,
    }
}

fn find_select(node: &gpui_rhai::UiNode) -> Option<&gpui_rhai::UiNode> {
    match node.kind() {
        UiNodeKind::Select { .. } => Some(node),
        UiNodeKind::Container { children } => children.iter().find_map(find_select),
        _ => None,
    }
}

fn find_label<'a>(node: &'a gpui_rhai::UiNode, label: &str) -> Option<&'a gpui_rhai::UiNode> {
    if node.attributes().get("label") == Some(&UiValue::String(label.to_owned())) {
        return Some(node);
    }
    match node.kind() {
        UiNodeKind::Container { children } => {
            children.iter().find_map(|child| find_label(child, label))
        }
        UiNodeKind::Overlay {
            trigger, content, ..
        } => find_label(trigger, label).or_else(|| find_label(content, label)),
        UiNodeKind::ErrorBoundary { child, fallback } => {
            find_label(child, label).or_else(|| find_label(fallback, label))
        }
        _ => None,
    }
}

#[test]
fn official_m0_components_compile_export_and_render_together() {
    let button_id = ModuleId::parse("components/button").unwrap();
    let label_id = ModuleId::parse("components/label").unwrap();
    assert_eq!(parse_component_header(BUTTON).unwrap().id, button_id);
    assert_eq!(parse_component_header(LABEL).unwrap().id, label_id);

    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (button_id, BUTTON.to_owned()),
        (label_id, LABEL.to_owned()),
    ]));
    let resolver = RestrictedModuleResolver::from_source(&source).unwrap();
    let mut runtime = RuntimeEngine::new();
    runtime.set_module_resolver(resolver);
    let compiled = runtime
        .compile_self_contained_named(
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
        )
        .unwrap();
    let context = UiContext::new(
        Rc::new(RefCell::new(UiRuntimeState::new())),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        ExecutionPhase::Render,
        BTreeMap::new(),
    );
    let root = runtime.render_with_context(&compiled, context).unwrap();
    let UiNodeKind::Container { children } = root.kind() else {
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
        children[1].handlers()["click"].generation(),
        compiled.generation()
    );
    assert_eq!(runtime.component_exports().unwrap().len(), 2);
}

#[test]
fn official_components_validate_props_and_merge_standard_style_overrides() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([(
        ModuleId::parse("components/button").unwrap(),
        BUTTON.to_owned(),
    )]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
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
        )
        .unwrap();
    let root = engine.render(&compiled).unwrap();
    assert_eq!(
        root.style().base.width,
        Some(gpui_rhai::Length::Pixels(333.0))
    );
    assert_eq!(
        root.style().base.height,
        Some(gpui_rhai::Length::Pixels(77.0))
    );
    assert!(matches!(root.kind(), UiNodeKind::Container { children } if children.len() == 3));

    let mut rejecting_engine = RuntimeEngine::new();
    rejecting_engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let invalid = rejecting_engine
        .compile_self_contained_named(
            "ui/invalid_button.rhai",
            r#"
                import "components/button" as button;
                fn view() { button::Button(#{ text: "Save", unknown: true }) }
            "#,
        )
        .unwrap();
    assert!(rejecting_engine.render(&invalid).is_err());
}

#[test]
fn official_default_theme_pair_satisfies_one_semantic_contract() {
    let engine = RuntimeEngine::new();
    let light =
        gpui_rhai::load_theme_source(engine.engine(), "default_light.rhai", DEFAULT_LIGHT).unwrap();
    let dark =
        gpui_rhai::load_theme_source(engine.engine(), "default_dark.rhai", DEFAULT_DARK).unwrap();
    assert_eq!(light.family, dark.family);
    assert_eq!(light.mode, gpui_rhai::ThemeMode::Light);
    assert_eq!(dark.mode, gpui_rhai::ThemeMode::Dark);
    assert_eq!(
        light.tokens.colors.keys().collect::<Vec<_>>(),
        dark.tokens.colors.keys().collect::<Vec<_>>()
    );
}

#[test]
fn official_icon_uses_only_logical_asset_and_opaque_image_handle() {
    let icon_id = ModuleId::parse("components/icon").unwrap();
    let source = EmbeddedScriptSource::new(BTreeMap::from([(icon_id, ICON.to_owned())]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
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
        )
        .unwrap();
    let runtime_state = UiRuntimeState::new();
    runtime_state
        .assets
        .register(
            "app",
            InMemoryAssetProvider::new(BTreeMap::from([(
                "check".to_owned(),
                AssetData {
                    mime_type: "image/svg+xml".to_owned(),
                    bytes: include_bytes!("../../../registry/assets/icons/check.svg").to_vec(),
                },
            )])),
        )
        .unwrap();
    let runtime_state = Rc::new(RefCell::new(runtime_state));
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
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        runtime_state,
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        BTreeMap::new(),
        &state_schema,
    )
    .unwrap();
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
    let icon_id = ModuleId::parse("components/icon").unwrap();
    let source = EmbeddedScriptSource::new(BTreeMap::from([(icon_id, ICON.to_owned())]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
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
        )
        .unwrap();
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
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        runtime,
        ComponentInstancePath::root("App", "main"),
        Some("main".to_owned()),
        BTreeMap::new(),
        &state_schema,
    )
    .unwrap();
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
fn m1_palette_adaptations_implement_the_semantic_contract() {
    let engine = RuntimeEngine::new();
    let tokyo_night =
        gpui_rhai::load_theme_source(engine.engine(), "tokyo_night.rhai", TOKYO_NIGHT).unwrap();
    let tokyo_storm =
        gpui_rhai::load_theme_source(engine.engine(), "tokyo_storm.rhai", TOKYO_STORM).unwrap();
    assert_eq!(tokyo_night.family, "Tokyo Night");
    assert_eq!(tokyo_storm.family, "Tokyo Night");

    let latte =
        gpui_rhai::load_theme_source(engine.engine(), "catppuccin_latte.rhai", CATPPUCCIN_LATTE)
            .unwrap();
    let mocha =
        gpui_rhai::load_theme_source(engine.engine(), "catppuccin_mocha.rhai", CATPPUCCIN_MOCHA)
            .unwrap();
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
    let input_id = ModuleId::parse("components/input").unwrap();
    let source = EmbeddedScriptSource::new(BTreeMap::from([(input_id, INPUT.to_owned())]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/input_test.rhai",
            r#"
                import "components/input" as input;
                fn changed(ctx, value) { () }
                fn view(ctx) {
                    input::Input(#{
                        key: "name",
                        value: "Alice",
                        placeholder: "Name",
                        read_only: true,
                        on_change: Fn("changed")
                    })
                }
            "#,
        )
        .unwrap();
    let context = UiContext::new(
        Rc::new(RefCell::new(UiRuntimeState::new())),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        ExecutionPhase::Render,
        BTreeMap::new(),
    );
    let root = engine.render_with_context(&compiled, context).unwrap();
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
    let textarea_id = ModuleId::parse("components/textarea").unwrap();
    let source = EmbeddedScriptSource::new(BTreeMap::from([(textarea_id, TEXTAREA.to_owned())]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/textarea_test.rhai",
            r#"
                import "components/textarea" as textarea;
                fn changed(ctx, value) { () }
                fn view(ctx) {
                    textarea::Textarea(#{
                        key: "notes", value: "Line one\nLine two",
                        min_rows: 2, max_rows: 6, max_length: 100,
                        on_change: Fn("changed")
                    })
                }
            "#,
        )
        .unwrap();
    let context = UiContext::new(
        Rc::new(RefCell::new(UiRuntimeState::new())),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        ExecutionPhase::Render,
        BTreeMap::new(),
    );
    let root = engine.render_with_context(&compiled, context).unwrap();
    let UiNodeKind::Container { children } = root.kind() else {
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
    let divider_id = ModuleId::parse("components/divider").unwrap();
    let source = EmbeddedScriptSource::new(BTreeMap::from([(divider_id, DIVIDER.to_owned())]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
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
        )
        .unwrap();
    let context = UiContext::new(
        Rc::new(RefCell::new(UiRuntimeState::new())),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        ExecutionPhase::Render,
        BTreeMap::new(),
    );
    let root = engine.render_with_context(&compiled, context).unwrap();
    let UiNodeKind::Container { children } = root.kind() else {
        panic!("divider showcase must render a container");
    };
    assert_eq!(children.len(), 2);
    assert_eq!(
        children[0].attributes().get("role"),
        Some(&UiValue::String("separator".to_owned()))
    );
    assert_eq!(
        children[0].style().base.width,
        Some(gpui_rhai::Length::Relative(1.0))
    );
    assert_eq!(
        children[1].style().base.width,
        Some(gpui_rhai::Length::Pixels(2.0))
    );
}

#[test]
fn official_popover_and_dialog_use_native_overlay_nodes() {
    let popover_id = ModuleId::parse("components/popover").unwrap();
    let dialog_id = ModuleId::parse("components/dialog").unwrap();
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (popover_id, POPOVER.to_owned()),
        (dialog_id, DIALOG.to_owned()),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/overlay_test.rhai",
            r#"
                import "components/popover" as popover;
                import "components/dialog" as dialog;
                fn set_open(ctx, open) { () }
                fn view(ctx) {
                    column([
                        popover::Popover(#{
                            key: "help",
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
        )
        .unwrap();
    let context = UiContext::new(
        Rc::new(RefCell::new(UiRuntimeState::new())),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        ExecutionPhase::Render,
        BTreeMap::new(),
    );
    let root = engine.render_with_context(&compiled, context).unwrap();
    let UiNodeKind::Container { children } = root.kind() else {
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
fn official_dropdown_wraps_keyed_native_virtualized_entity() {
    let dropdown_id = ModuleId::parse("components/dropdown").unwrap();
    let source = EmbeddedScriptSource::new(BTreeMap::from([(dropdown_id, DROPDOWN.to_owned())]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/dropdown_test.rhai",
            r#"
                import "components/dropdown" as dropdown_component;
                fn selected(ctx, values) { () }
                fn opened(ctx, open) { () }
                fn queried(ctx, query) { () }
                fn view(ctx) {
                    dropdown_component::Dropdown(#{
                        key: "theme",
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
                        part_styles: #{ option: style().height(px(44)) },
                        on_change: Fn("selected"),
                        on_open_change: Fn("opened"),
                        on_query_change: Fn("queried")
                    })
                }
            "#,
        )
        .unwrap();
    let context = UiContext::new(
        Rc::new(RefCell::new(UiRuntimeState::new())),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        ExecutionPhase::Render,
        BTreeMap::new(),
    );
    let root = engine.render_with_context(&compiled, context).unwrap();
    let UiNodeKind::Dropdown { spec } = root.kind() else {
        panic!("Dropdown must render the native dropdown node");
    };
    assert_eq!(root.key().map(gpui_rhai::NodeKey::as_str), Some("theme"));
    assert_eq!(spec.mode, gpui_rhai::DropdownMode::Multiple);
    assert_eq!(
        spec.selected.as_deref(),
        Some(&["default".to_owned(), "tokyo".to_owned()][..])
    );
    assert_eq!(spec.query.as_deref(), Some("night"));
    assert!(spec.trigger_slot.is_some());
    assert!(spec.header_slot.is_some());
    assert!(spec.footer_slot.is_some());
    assert!(spec.empty_slot.is_some());
    assert!(root.handlers().contains_key("change"));
    assert!(root.handlers().contains_key("open_change"));
    assert!(root.handlers().contains_key("query_change"));
    assert_eq!(
        root.part_style("option").unwrap().base.height,
        Some(gpui_rhai::Length::Pixels(44.0))
    );
}

#[test]
fn official_select_uses_scalar_controlled_choice_semantics() {
    let select_id = ModuleId::parse("components/select").unwrap();
    let source = EmbeddedScriptSource::new(BTreeMap::from([(select_id, SELECT.to_owned())]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/select_test.rhai",
            r#"
                import "components/select" as select;
                fn changed(ctx, value) { () }
                fn view(ctx) {
                    select::Select(#{
                        key: "country", value: (), searchable: true, clearable: true,
                        empty_text: "No countries",
                        options: [
                            #{ value: "cn", label: "China", group: "Asia" },
                            #{ value: "fr", label: "France", group: "Europe" }
                        ],
                        on_change: Fn("changed")
                    })
                }
            "#,
        )
        .unwrap();
    let context = UiContext::new(
        Rc::new(RefCell::new(UiRuntimeState::new())),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        ExecutionPhase::Render,
        BTreeMap::new(),
    );
    let root = engine.render_with_context(&compiled, context).unwrap();
    let UiNodeKind::Select { spec } = root.kind() else {
        panic!("Select must use the native scalar choice adapter");
    };
    assert_eq!(spec.choice.selected, Some(Vec::new()));
    assert!(spec.choice.behavior.searchable);
    assert!(spec.choice.behavior.clearable);
    assert!(spec.choice.behavior.reset_query_on_close);
    assert_eq!(spec.choice.options[0].group.as_deref(), Some("Asia"));
    assert_eq!(spec.choice.check_asset.as_str(), "app/icons/check");
    assert_eq!(
        spec.choice.indicator_asset.as_str(),
        "app/icons/disclosure_down"
    );
}

#[test]
fn official_date_picker_consumes_locale_and_strict_iso_values() {
    let date_picker_id = ModuleId::parse("components/date_picker").unwrap();
    let source =
        EmbeddedScriptSource::new(BTreeMap::from([(date_picker_id, DATE_PICKER.to_owned())]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/date_picker_test.rhai",
            r#"
                import "components/date_picker" as date_picker;
                fn changed(ctx, value) { () }
                fn view(ctx) {
                    date_picker::DatePicker(#{
                        key: "appointment", value: "2026-09-01",
                        min_date: "2026-08-30", max_date: "2026-12-31",
                        placeholder: "Appointment", clearable: true,
                        presets: [#{ label: "Launch", value: "2026-09-01" }],
                        on_change: Fn("changed")
                    })
                }
            "#,
        )
        .unwrap();
    let locale = gpui_rhai::load_locale_source(
        engine.engine(),
        "en.rhai",
        include_str!("../../../registry/locales/en.rhai"),
    )
    .unwrap();
    let mut state = UiRuntimeState::new();
    state.locale = Some(gpui_rhai::LocaleManager::new([locale], "en", "en").unwrap());
    state.calendar_clock =
        gpui_rhai::CalendarClock::fixed(gpui_rhai::GregorianDate::parse_iso("2026-08-30").unwrap());
    let context = UiContext::new(
        Rc::new(RefCell::new(state)),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        ExecutionPhase::Render,
        BTreeMap::new(),
    );
    let root = engine.render_with_context(&compiled, context).unwrap();
    let UiNodeKind::DatePicker { spec } = root.kind() else {
        panic!("DatePicker must use its native calendar node");
    };
    assert_eq!(spec.value.unwrap().to_iso(), "2026-09-01");
    assert_eq!(spec.today.to_iso(), "2026-08-30");
    assert_eq!(spec.calendar.first_weekday, gpui_rhai::Weekday::Sunday);
    assert_eq!(spec.display_value, "09/01/2026");
    assert_eq!(spec.presets.len(), 1);
    assert_eq!(spec.clear_asset.as_str(), "app/icons/close");
    assert_eq!(
        root.attributes().get("calendar_previous_label"),
        Some(&UiValue::String("Previous month".to_owned()))
    );
    assert_eq!(
        root.attributes().get("row_count"),
        Some(&UiValue::Integer(6))
    );
}

#[test]
fn official_table_builds_data_rows_and_eager_custom_cells() {
    let table_id = ModuleId::parse("components/table").unwrap();
    let skeleton_id = ModuleId::parse("components/skeleton").unwrap();
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (table_id, TABLE.to_owned()),
        (skeleton_id, SKELETON.to_owned()),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/table_test.rhai",
            r#"
                import "components/table" as table;
                fn status_cell(cell) { text(cell.value) }
                fn sorted(ctx, value) { () }
                fn selected(ctx, value) { () }
                fn clicked(ctx, value) { () }
                fn view(ctx) {
                    table::Table(#{
                        key: "users", label: "Users", row_key: "id", height: px(240),
                        rows: [
                            #{ id: "u1", name: "Ada", score: 12.5, joined: "2026-08-30", status: "Active" },
                            #{ id: "u2", name: "Lin", score: 9, joined: "2026-08-31", status: "Away" }
                        ],
                        columns: [
                            #{ key: "name", title: "Name", width: #{ kind: "fixed", value: 120 }, sortable: true },
                            #{ key: "score", title: "Score", width: #{ kind: "flex", value: 1 }, format: #{ kind: "number", max_fraction_digits: 1 } },
                            #{ key: "joined", title: "Joined", width: #{ kind: "percent", value: 0.3 }, format: #{ kind: "date", style: "short" } },
                            #{ key: "status", title: "Status", width: #{ kind: "fixed", value: 90 }, cell_renderer: Fn("status_cell") }
                        ],
                        loading: true, selection_mode: "multiple",
                        selected_keys: ["u1"], striped: true,
                        on_sort_change: Fn("sorted"), on_selection_change: Fn("selected"),
                        on_row_click: Fn("clicked")
                    })
                }
            "#,
        )
        .unwrap();
    let locale = gpui_rhai::load_locale_source(
        engine.engine(),
        "en.rhai",
        include_str!("../../../registry/locales/en.rhai"),
    )
    .unwrap();
    let mut state = UiRuntimeState::new();
    state.locale = Some(gpui_rhai::LocaleManager::new([locale], "en", "en").unwrap());
    let context = UiContext::new(
        Rc::new(RefCell::new(state)),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        ExecutionPhase::Render,
        BTreeMap::new(),
    );
    let root = engine.render_with_context(&compiled, context).unwrap();
    let UiNodeKind::Table { spec } = root.kind() else {
        panic!("Table must build its data-driven native node");
    };
    assert_eq!(spec.rows.len(), 2);
    assert_eq!(spec.rows[0].key, "u1");
    assert_eq!(spec.display_cell(0, 1).unwrap(), "12.5");
    assert_eq!(spec.display_cell(0, 2).unwrap(), "08/30/2026");
    assert_eq!(spec.columns[3].custom_cells.as_ref().unwrap().len(), 2);
    assert_eq!(spec.loading_rows.len(), 100);
    assert_eq!(spec.check_asset.as_str(), "app/icons/check");
    assert_eq!(
        spec.sort_ascending_asset.as_str(),
        "app/icons/sort_ascending"
    );
    assert_eq!(
        root.attributes().get("row_count"),
        Some(&UiValue::Integer(2))
    );
}

#[test]
fn official_pagination_is_pure_rhai_composition() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/pagination").unwrap(),
            PAGINATION.to_owned(),
        ),
        (
            ModuleId::parse("components/button").unwrap(),
            BUTTON.to_owned(),
        ),
        (ModuleId::parse("components/icon").unwrap(), ICON.to_owned()),
        (
            ModuleId::parse("components/select").unwrap(),
            SELECT.to_owned(),
        ),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/pagination_test.rhai",
            r#"
                import "components/pagination" as pagination;
                fn changed(ctx, value) { () }
                fn view(ctx) {
                    pagination::Pagination(#{
                        key: "users-pages", total_items: 5000,
                        current_page: 250, page_size: 10,
                        page_size_options: [10, 25, 50],
                        on_change: Fn("changed")
                    })
                }
            "#,
        )
        .unwrap();
    let locale = gpui_rhai::load_locale_source(
        engine.engine(),
        "en.rhai",
        include_str!("../../../registry/locales/en.rhai"),
    )
    .unwrap();
    let mut state = UiRuntimeState::new();
    state.locale = Some(gpui_rhai::LocaleManager::new([locale], "en", "en").unwrap());
    let context = UiContext::new(
        Rc::new(RefCell::new(state)),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        ExecutionPhase::Render,
        BTreeMap::new(),
    );
    let root = engine.render_with_context(&compiled, context).unwrap();
    assert!(matches!(root.kind(), UiNodeKind::Container { .. }));
    assert!(contains_select(&root));
    let page = find_label(&root, "251").expect("page 251 button");
    assert_eq!(
        page.handler_payload("click"),
        Some(&UiValue::Map(BTreeMap::from([
            ("current_page".to_owned(), UiValue::Integer(251)),
            ("page_size".to_owned(), UiValue::Integer(10)),
        ])))
    );
}

#[test]
fn official_pagination_rejects_duplicate_page_sizes() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/pagination").unwrap(),
            PAGINATION.to_owned(),
        ),
        (
            ModuleId::parse("components/button").unwrap(),
            BUTTON.to_owned(),
        ),
        (ModuleId::parse("components/icon").unwrap(), ICON.to_owned()),
        (
            ModuleId::parse("components/select").unwrap(),
            SELECT.to_owned(),
        ),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/pagination_duplicate_test.rhai",
            r#"
                import "components/pagination" as pagination;
                fn view(ctx) {
                    pagination::Pagination(#{
                        key: "pages", total_items: 100, current_page: 1,
                        page_size: 10, page_size_options: [10, 10]
                    })
                }
            "#,
        )
        .unwrap();
    let locale = gpui_rhai::load_locale_source(
        engine.engine(),
        "en.rhai",
        include_str!("../../../registry/locales/en.rhai"),
    )
    .unwrap();
    let mut state = UiRuntimeState::new();
    state.locale = Some(gpui_rhai::LocaleManager::new([locale], "en", "en").unwrap());
    let context = UiContext::new(
        Rc::new(RefCell::new(state)),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        ExecutionPhase::Render,
        BTreeMap::new(),
    );
    let error = engine.render_with_context(&compiled, context).unwrap_err();
    assert!(error.to_string().contains("duplicate value 10"));
}

#[test]
fn official_pagination_page_window_covers_ellipsis_transitions() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/pagination").unwrap(),
            PAGINATION.to_owned(),
        ),
        (
            ModuleId::parse("components/button").unwrap(),
            BUTTON.to_owned(),
        ),
        (ModuleId::parse("components/icon").unwrap(), ICON.to_owned()),
        (
            ModuleId::parse("components/select").unwrap(),
            SELECT.to_owned(),
        ),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
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
        )
        .unwrap();
    let root = engine
        .render_with_context(
            &compiled,
            UiContext::new(
                Rc::new(RefCell::new(UiRuntimeState::new())),
                ComponentInstancePath::root("App", "root"),
                Some("main".to_owned()),
                ExecutionPhase::Render,
                BTreeMap::new(),
            ),
        )
        .unwrap();
    let UiNodeKind::Container { children: cases } = root.kind() else {
        panic!("pagination page-window probe must render a column");
    };
    let actual = cases
        .iter()
        .map(|case| {
            let UiNodeKind::Container { children } = case.kind() else {
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
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/pagination").unwrap(),
            PAGINATION.to_owned(),
        ),
        (
            ModuleId::parse("components/button").unwrap(),
            BUTTON.to_owned(),
        ),
        (ModuleId::parse("components/icon").unwrap(), ICON.to_owned()),
        (
            ModuleId::parse("components/select").unwrap(),
            SELECT.to_owned(),
        ),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
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
                        key: "pages", total_items: 100,
                        current_page: ctx.get_state("current_page"),
                        page_size: ctx.get_state("page_size"),
                        page_size_options: [10, 25], on_change: Fn("changed")
                    })
                }
            "#,
        )
        .unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let locale = gpui_rhai::load_locale_source(
        engine.engine(),
        "en.rhai",
        include_str!("../../../registry/locales/en.rhai"),
    )
    .unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    runtime.borrow_mut().locale =
        Some(gpui_rhai::LocaleManager::new([locale], "en", "en").unwrap());
    let path = ComponentInstancePath::root("App", "root");
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        Rc::clone(&runtime),
        path.clone(),
        Some("main".to_owned()),
        BTreeMap::new(),
        &schema,
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
    let callback = find_select(lifecycle.root().unwrap())
        .and_then(|select| select.handlers().get("change"))
        .cloned()
        .expect("page-size Select change callback");
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &callback, UiValue::String("25".to_owned()))
        .unwrap();
    let events = runtime.borrow_mut().drain_batch().events;
    assert_eq!(events.len(), 1);
    for event in events {
        let _ = lifecycle
            .invoke_component_event_transactional(&engine, event)
            .unwrap();
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
fn m2_control_sources_emit_typed_values_and_roving_keys() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/checkbox").unwrap(),
            CHECKBOX.to_owned(),
        ),
        (
            ModuleId::parse("components/radio").unwrap(),
            RADIO.to_owned(),
        ),
        (
            ModuleId::parse("components/radio_group").unwrap(),
            RADIO_GROUP.to_owned(),
        ),
        (
            ModuleId::parse("components/switch").unwrap(),
            SWITCH.to_owned(),
        ),
        (ModuleId::parse("components/tag").unwrap(), TAG.to_owned()),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/m2_controls.rhai",
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
        )
        .unwrap();
    let context = UiContext::new(
        Rc::new(RefCell::new(UiRuntimeState::new())),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        ExecutionPhase::Render,
        BTreeMap::new(),
    );
    let root = engine.render_with_context(&compiled, context).unwrap();
    let UiNodeKind::Container { children } = root.kind() else {
        panic!("M2 controls must compose into a container");
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
    let UiNodeKind::Container {
        children: tag_children,
    } = children[3].kind()
    else {
        panic!("Tag must render label and close affordance");
    };
    assert_eq!(
        tag_children[1].handler_payload("click"),
        Some(&UiValue::String("Rust".to_owned()))
    );
}

#[test]
fn m2_visual_primitives_export_fallbacks_and_rust_animations() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/avatar").unwrap(),
            AVATAR.to_owned(),
        ),
        (
            ModuleId::parse("components/progress").unwrap(),
            PROGRESS.to_owned(),
        ),
        (
            ModuleId::parse("components/skeleton").unwrap(),
            SKELETON.to_owned(),
        ),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/m2_visuals.rhai",
            r#"
                import "components/avatar" as avatar;
                import "components/progress" as progress;
                import "components/skeleton" as skeleton;
                fn view(ctx) {
                    column([
                        avatar::Avatar(#{ name: "Ada", presence: "online" }),
                        progress::Progress(#{ key: "download", value: 42, max: 100 }),
                        progress::Progress(#{ key: "loading", indeterminate: true }),
                        skeleton::Skeleton(#{ key: "card", width: 240, height: 80 })
                    ])
                }
            "#,
        )
        .unwrap();
    let context = UiContext::new(
        Rc::new(RefCell::new(UiRuntimeState::new())),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        ExecutionPhase::Render,
        BTreeMap::new(),
    );
    let root = engine.render_with_context(&compiled, context).unwrap();
    let UiNodeKind::Container { children } = root.kind() else {
        panic!("visual primitives must compose into a container");
    };
    assert_eq!(
        children[0].attributes().get("label"),
        Some(&UiValue::String("Ada".to_owned()))
    );
    let UiNodeKind::Container {
        children: determinate,
    } = children[1].kind()
    else {
        panic!("Progress must render a track container");
    };
    assert!(matches!(
        determinate[0].animations()[0],
        gpui_rhai::AnimationSpec::Transition(_)
    ));
    let UiNodeKind::Container {
        children: indeterminate,
    } = children[2].kind()
    else {
        panic!("Progress must render a track container");
    };
    assert!(matches!(
        indeterminate[0].animations()[0],
        gpui_rhai::AnimationSpec::LoopingTransition(_)
    ));
    assert!(matches!(
        children[3].animations()[0],
        gpui_rhai::AnimationSpec::LoopingTransition(_)
    ));
}

#[test]
fn m2_composites_export_slots_animation_and_keyboard_payloads() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/label").unwrap(),
            LABEL.to_owned(),
        ),
        (
            ModuleId::parse("components/form_field").unwrap(),
            FORM_FIELD.to_owned(),
        ),
        (
            ModuleId::parse("components/collapsible").unwrap(),
            COLLAPSIBLE.to_owned(),
        ),
        (
            ModuleId::parse("components/accordion").unwrap(),
            ACCORDION.to_owned(),
        ),
        (ModuleId::parse("components/tabs").unwrap(), TABS.to_owned()),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named("ui/m2_composites.rhai", M2_COMPOSITES_APP)
        .unwrap();
    let context = UiContext::new(
        Rc::new(RefCell::new(UiRuntimeState::new())),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        ExecutionPhase::Render,
        BTreeMap::new(),
    );
    let root = engine.render_with_context(&compiled, context).unwrap();
    let UiNodeKind::Container { children } = root.kind() else {
        panic!("M2 composites must compose into a container");
    };
    assert_eq!(
        children[0].attributes().get("invalid"),
        Some(&UiValue::Bool(true))
    );
    let UiNodeKind::Container {
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
    let UiNodeKind::Container {
        children: collapsible_children,
    } = children[1].kind()
    else {
        panic!("Collapsible must render trigger and panel");
    };
    assert!(matches!(
        collapsible_children[1].animations()[0],
        gpui_rhai::AnimationSpec::Transition(_)
    ));
    let UiNodeKind::Container {
        children: accordion_items,
    } = children[2].kind()
    else {
        panic!("Accordion must render item containers");
    };
    let UiNodeKind::Container {
        children: first_item,
    } = accordion_items[0].kind()
    else {
        panic!("Accordion item must render trigger and panel");
    };
    assert!(first_item[0].handler_payload("click").is_some());
    assert!(!first_item[1].animations().is_empty());
    let UiNodeKind::Container {
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
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/divider").unwrap(),
            DIVIDER.to_owned(),
        ),
        (
            ModuleId::parse("components/tooltip").unwrap(),
            TOOLTIP.to_owned(),
        ),
        (ModuleId::parse("components/menu").unwrap(), MENU.to_owned()),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/tooltip_menu.rhai",
            r#"
                import "components/tooltip" as tooltip;
                import "components/menu" as menu;
                fn changed(ctx, value) { () }
                fn view(ctx) {
                    column([
                        tooltip::Tooltip(#{
                            key: "help", trigger: text("?"), content: text("Help"),
                            show_delay_ms: 250, hide_delay_ms: 75
                        }),
                        menu::Menu(#{
                            key: "file", trigger: text("File"), open: true,
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
        )
        .unwrap();
    let context = UiContext::new(
        Rc::new(RefCell::new(UiRuntimeState::new())),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        ExecutionPhase::Render,
        BTreeMap::new(),
    );
    let root = engine.render_with_context(&compiled, context).unwrap();
    let UiNodeKind::Container { children } = root.kind() else {
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
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (ModuleId::parse("components/menu").unwrap(), MENU.to_owned()),
        (
            ModuleId::parse("components/divider").unwrap(),
            DIVIDER.to_owned(),
        ),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/nested_menu.rhai",
            r#"
                import "components/menu" as menu;
                fn changed(ctx, value) { () }
                fn view(ctx) {
                    let child = menu::Menu(#{
                        key: "file-more", parent_overlay: "file", trigger: text("More"),
                        open: true, active_value: "export", placement: "right",
                        items: [#{ kind: "item", value: "export", label: "Export" }],
                        on_action: Fn("changed"), on_active_change: Fn("changed"),
                        on_open_change: Fn("changed")
                    });
                    menu::Menu(#{
                        key: "file", trigger: text("File"), open: true,
                        active_value: "more",
                        items: [#{
                            kind: "submenu", value: "more", label: "More", submenu: child
                        }],
                        on_action: Fn("changed"), on_active_change: Fn("changed"),
                        on_open_change: Fn("changed")
                    })
                }
            "#,
        )
        .unwrap();
    let context = UiContext::new(
        Rc::new(RefCell::new(UiRuntimeState::new())),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        ExecutionPhase::Render,
        BTreeMap::new(),
    );
    let root = engine.render_with_context(&compiled, context).unwrap();
    let UiNodeKind::Overlay { content, spec, .. } = root.kind() else {
        panic!("parent Menu must render an overlay");
    };
    assert_eq!(spec.id.as_str(), "file");
    let UiNodeKind::Container { children } = content.kind() else {
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
fn toast_source_builds_native_timed_region_host() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([(
        ModuleId::parse("components/toast").unwrap(),
        TOAST.to_owned(),
    )]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
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
        )
        .unwrap();
    let context = UiContext::new(
        Rc::new(RefCell::new(UiRuntimeState::new())),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        ExecutionPhase::Render,
        BTreeMap::new(),
    );
    let root = engine.render_with_context(&compiled, context).unwrap();
    let UiNodeKind::ToastHost { spec } = root.kind() else {
        panic!("Toast must render the native toast host node");
    };
    assert_eq!(spec.max_visible, 2);
    assert_eq!(spec.items.len(), 2);
    assert_eq!(spec.items[0].region, gpui_rhai::ToastRegion::TopRight);
    assert!(spec.items[1].paused);
    assert!(root.handlers().contains_key("dismiss"));
    assert_eq!(
        root.part_style("title").unwrap().base.font_size,
        Some(gpui_rhai::Length::Pixels(18.0))
    );
}
