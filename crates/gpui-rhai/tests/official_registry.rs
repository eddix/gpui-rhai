use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use gpui_rhai::{
    AssetData, ComponentInstancePath, ComponentStateSchema, EmbeddedScriptSource, ExecutionPhase,
    InMemoryAssetProvider, ModuleId, OpaqueHandle, RestrictedModuleResolver, RuntimeEngine,
    ScriptCallback, ScriptLifecycle, StateField, UiContext, UiNodeKind, UiRuntimeState, UiValue,
    ValueSchema, parse_component_header,
};

const BUTTON: &str = include_str!("../../../registry/components/button.rhai");
const LABEL: &str = include_str!("../../../registry/components/label.rhai");
const ICON: &str = include_str!("../../../registry/components/icon.rhai");
const INPUT: &str = include_str!("../../../registry/components/input.rhai");
const TEXTAREA: &str = include_str!("../../../registry/components/textarea.rhai");
const DIVIDER: &str = include_str!("../../../registry/components/divider.rhai");
const POPOVER: &str = include_str!("../../../registry/components/popover.rhai");
const DIALOG: &str = include_str!("../../../registry/components/dialog.rhai");
const COMBOBOX: &str = include_str!("../../../registry/components/combobox.rhai");
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
const ALERT: &str = include_str!("../../../registry/components/alert.rhai");
const ALERT_DIALOG: &str = include_str!("../../../registry/components/alert_dialog.rhai");
const BADGE: &str = include_str!("../../../registry/components/badge.rhai");
const BUTTON_GROUP: &str = include_str!("../../../registry/components/button_group.rhai");
const CARD: &str = include_str!("../../../registry/components/card.rhai");
const EMPTY: &str = include_str!("../../../registry/components/empty.rhai");
const GROUP_BOX: &str = include_str!("../../../registry/components/group_box.rhai");
const INPUT_GROUP: &str = include_str!("../../../registry/components/input_group.rhai");
const KBD: &str = include_str!("../../../registry/components/kbd.rhai");
const TOGGLE: &str = include_str!("../../../registry/components/toggle.rhai");
const TOGGLE_GROUP: &str = include_str!("../../../registry/components/toggle_group.rhai");
const SLIDER: &str = include_str!("../../../registry/components/slider.rhai");
const CONTEXT_MENU: &str = include_str!("../../../registry/components/context_menu.rhai");
const SHEET: &str = include_str!("../../../registry/components/sheet.rhai");
const COMMAND: &str = include_str!("../../../registry/components/command.rhai");
const COMMAND_DIALOG: &str = include_str!("../../../registry/components/command_dialog.rhai");
const SPINNER: &str = include_str!("../../../registry/components/spinner.rhai");
const SCROLL_AREA: &str = include_str!("../../../registry/components/scroll_area.rhai");
const TITLE_BAR: &str = include_str!("../../../registry/components/title_bar.rhai");
const STATUS_BAR: &str = include_str!("../../../registry/components/status_bar.rhai");
const DATE_PICKER_TEST_APP: &str = r#"
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
"#;
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
const ETHEREAL: &str = include_str!("../../../registry/themes/ethereal.rhai");
const EVERFOREST: &str = include_str!("../../../registry/themes/everforest.rhai");
const GRUVBOX: &str = include_str!("../../../registry/themes/gruvbox.rhai");
const HACKERMAN: &str = include_str!("../../../registry/themes/hackerman.rhai");
const NORD: &str = include_str!("../../../registry/themes/nord.rhai");
const RETRO_82: &str = include_str!("../../../registry/themes/retro_82.rhai");
const HERMARCHY: &str = include_str!("../../../registry/themes/hermarchy.rhai");
const FUTURISM: &str = include_str!("../../../registry/themes/futurism.rhai");
const AETHERIA: &str = include_str!("../../../registry/themes/aetheria.rhai");

const BUNDLED_THEMES: &[(&str, &str)] = &[
    ("default_light", DEFAULT_LIGHT),
    ("default_dark", DEFAULT_DARK),
    ("tokyo_night", TOKYO_NIGHT),
    ("tokyo_storm", TOKYO_STORM),
    ("catppuccin_latte", CATPPUCCIN_LATTE),
    ("catppuccin_mocha", CATPPUCCIN_MOCHA),
    ("ethereal", ETHEREAL),
    ("everforest", EVERFOREST),
    ("gruvbox", GRUVBOX),
    ("hackerman", HACKERMAN),
    ("nord", NORD),
    ("retro_82", RETRO_82),
    ("hermarchy", HERMARCHY),
    ("futurism", FUTURISM),
    ("aetheria", AETHERIA),
];

fn pagination_source() -> EmbeddedScriptSource {
    EmbeddedScriptSource::new(BTreeMap::from([
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
        (
            ModuleId::parse("components/combobox").unwrap(),
            COMBOBOX.to_owned(),
        ),
        (
            ModuleId::parse("components/input").unwrap(),
            INPUT.to_owned(),
        ),
    ]))
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
    for (id, source) in [
        ("combobox", COMBOBOX),
        ("select", SELECT),
        ("date_picker", DATE_PICKER),
        ("table", TABLE),
        ("pagination", PAGINATION),
        ("menu", MENU),
        ("tabs", TABS),
        ("toast", TOAST),
    ] {
        for constructor in &forbidden {
            assert!(
                !contains_global_call(source, constructor),
                "official source `{id}` uses privileged constructor `{constructor}`"
            );
        }
    }
}

fn combobox_source() -> EmbeddedScriptSource {
    EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/combobox").unwrap(),
            COMBOBOX.to_owned(),
        ),
        (
            ModuleId::parse("components/input").unwrap(),
            INPUT.to_owned(),
        ),
    ]))
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
        Some(gpui_rhai::Length::Pixels(333.0).into())
    );
    assert_eq!(
        root.style().base.height,
        Some(gpui_rhai::Length::Pixels(77.0).into())
    );
    assert!(matches!(root.kind(), UiNodeKind::Box { children } if children.len() == 3));

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
fn bundled_themes_share_the_dense_square_metric_contract() {
    let engine = RuntimeEngine::new();
    for &(name, source) in BUNDLED_THEMES {
        let theme =
            gpui_rhai::load_theme_source(engine.engine(), &format!("{name}.rhai"), source).unwrap();
        assert_eq!(theme.tokens.radii["sm"], gpui_rhai::Length::Pixels(0.0));
        assert_eq!(theme.tokens.radii["md"], gpui_rhai::Length::Pixels(0.0));
        assert_eq!(theme.tokens.radii["lg"], gpui_rhai::Length::Pixels(0.0));
        assert_eq!(
            theme.tokens.typography.roles["body"].size,
            gpui_rhai::Length::Pixels(12.0)
        );
        assert_eq!(
            theme.tokens.typography.roles["body"].line_height,
            gpui_rhai::Length::Pixels(16.0)
        );
        assert_ne!(
            theme.tokens.colors["surface"],
            theme.tokens.colors["surface_raised"]
        );
        assert_ne!(
            theme.tokens.colors["surface"],
            theme.tokens.colors["surface_hover"]
        );
    }
}

fn linear_channel(channel: u8) -> f64 {
    let channel = f64::from(channel) / 255.0;
    if channel <= 0.040_45 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

fn luminance(color: gpui_rhai::Rgba8) -> f64 {
    let [red, green, blue, _alpha] = color.as_rgba_hex().to_be_bytes();
    0.2126 * linear_channel(red) + 0.7152 * linear_channel(green) + 0.0722 * linear_channel(blue)
}

fn contrast(first: gpui_rhai::Rgba8, second: gpui_rhai::Rgba8) -> f64 {
    let first = luminance(first);
    let second = luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

#[test]
fn bundled_theme_text_pairs_meet_small_text_contrast() {
    let engine = RuntimeEngine::new();
    for &(name, source) in BUNDLED_THEMES {
        let theme =
            gpui_rhai::load_theme_source(engine.engine(), &format!("{name}.rhai"), source).unwrap();
        for (foreground, background) in [
            ("text_primary", "surface"),
            ("text_muted", "surface"),
            ("on_accent", "accent"),
            ("on_danger", "danger"),
            ("on_warning", "warning"),
            ("on_success", "success"),
        ] {
            let ratio = contrast(
                theme.tokens.colors[foreground],
                theme.tokens.colors[background],
            );
            assert!(
                ratio >= 4.5,
                "{name}: {foreground} on {background} contrast is {ratio:.2}:1"
            );
        }
        assert!(
            contrast(
                theme.tokens.colors["focus_ring"],
                theme.tokens.colors["surface"]
            ) >= 3.0,
            "{name}: focus ring does not reach 3:1 against the surface"
        );
    }
}

#[test]
fn official_component_sources_reject_decorative_visual_drift() {
    for (id, source) in [
        ("button", BUTTON),
        ("checkbox", CHECKBOX),
        ("radio", RADIO),
        ("switch", SWITCH),
        ("tag", TAG),
        ("input", INPUT),
        ("textarea", TEXTAREA),
        ("combobox", COMBOBOX),
        ("select", SELECT),
        ("date_picker", DATE_PICKER),
        ("table", TABLE),
        ("pagination", PAGINATION),
        ("dialog", DIALOG),
        ("popover", POPOVER),
        ("tooltip", TOOLTIP),
        ("menu", MENU),
        ("toast", TOAST),
        ("tabs", TABS),
        ("accordion", ACCORDION),
        ("alert", ALERT),
        ("alert_dialog", ALERT_DIALOG),
        ("badge", BADGE),
        ("button_group", BUTTON_GROUP),
        ("card", CARD),
        ("empty", EMPTY),
        ("group_box", GROUP_BOX),
        ("input_group", INPUT_GROUP),
        ("kbd", KBD),
        ("toggle", TOGGLE),
        ("toggle_group", TOGGLE_GROUP),
        ("slider", SLIDER),
        ("context_menu", CONTEXT_MENU),
        ("sheet", SHEET),
        ("command", COMMAND),
        ("command_dialog", COMMAND_DIALOG),
        ("spinner", SPINNER),
        ("scroll_area", SCROLL_AREA),
        ("title_bar", TITLE_BAR),
        ("status_bar", STATUS_BAR),
    ] {
        for prohibited in [
            "shadow(",
            "linear_gradient(",
            "radius(px(10",
            "radius(px(12",
            "font_size(px(",
            "line_height(px(",
        ] {
            assert!(
                !source.contains(prohibited),
                "official component {id} contains prohibited visual pattern {prohibited}"
            );
        }
        for line in source.lines().filter(|line| line.contains("rgba(0x")) {
            assert!(
                line.contains("rgba(0x00000000)"),
                "official component {id} hard-codes a palette color: {line}"
            );
        }
    }
}

#[test]
fn window_bars_are_source_owned_compositions_without_native_authority() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/title_bar").unwrap(),
            TITLE_BAR.to_owned(),
        ),
        (
            ModuleId::parse("components/status_bar").unwrap(),
            STATUS_BAR.to_owned(),
        ),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
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
#[allow(clippy::too_many_lines)]
fn public_launch_static_components_compile_and_compose() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/alert").unwrap(),
            ALERT.to_owned(),
        ),
        (
            ModuleId::parse("components/alert_dialog").unwrap(),
            ALERT_DIALOG.to_owned(),
        ),
        (
            ModuleId::parse("components/badge").unwrap(),
            BADGE.to_owned(),
        ),
        (
            ModuleId::parse("components/button_group").unwrap(),
            BUTTON_GROUP.to_owned(),
        ),
        (ModuleId::parse("components/card").unwrap(), CARD.to_owned()),
        (
            ModuleId::parse("components/empty").unwrap(),
            EMPTY.to_owned(),
        ),
        (
            ModuleId::parse("components/group_box").unwrap(),
            GROUP_BOX.to_owned(),
        ),
        (
            ModuleId::parse("components/input_group").unwrap(),
            INPUT_GROUP.to_owned(),
        ),
        (ModuleId::parse("components/kbd").unwrap(), KBD.to_owned()),
        (
            ModuleId::parse("components/button").unwrap(),
            BUTTON.to_owned(),
        ),
        (
            ModuleId::parse("components/dialog").unwrap(),
            DIALOG.to_owned(),
        ),
        (
            ModuleId::parse("components/input").unwrap(),
            INPUT.to_owned(),
        ),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
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
                            footer: text("Updated now"), elevated: true }),
                        empty::Empty(#{ title: "No projects", description: "Create one to begin.",
                            actions: [primary] }),
                        group_box::GroupBox(#{ label: "Sync", description: "Cloud settings",
                            content: text("Enabled") }),
                        input_group::InputGroup(#{ label: "Search", prefix: text("⌕"),
                            suffix: kbd::Kbd(#{ text: "⌘ K" }),
                            control: input::Input(#{ key: "search", value: "", placeholder: "Search" }) }),
                        kbd::Kbd(#{ text: "⌘ K", label: "Command K" })
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
    let UiNodeKind::Box { children } = root.kind() else {
        panic!("public launch static specimen must render a column");
    };
    assert_eq!(children.len(), 9);
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
        Rc::clone(&runtime),
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
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/divider").unwrap(),
            DIVIDER.to_owned(),
        ),
        (ModuleId::parse("components/menu").unwrap(), MENU.to_owned()),
        (
            ModuleId::parse("components/context_menu").unwrap(),
            CONTEXT_MENU.to_owned(),
        ),
        (
            ModuleId::parse("components/sheet").unwrap(),
            SHEET.to_owned(),
        ),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/context_menu_sheet.rhai",
            r#"
                import "components/context_menu" as context_menu;
                import "components/sheet" as sheet;
                fn changed(ctx, value) { () }
                fn view(ctx) {
                    column([
                        context_menu::ContextMenu(#{
                            key: "row-actions", trigger: text("Right click"), open: false,
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
        )
        .unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        runtime,
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
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
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/command").unwrap(),
            COMMAND.to_owned(),
        ),
        (
            ModuleId::parse("components/command_dialog").unwrap(),
            COMMAND_DIALOG.to_owned(),
        ),
        (
            ModuleId::parse("components/dialog").unwrap(),
            DIALOG.to_owned(),
        ),
        (
            ModuleId::parse("components/input").unwrap(),
            INPUT.to_owned(),
        ),
        (ModuleId::parse("components/kbd").unwrap(), KBD.to_owned()),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
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
        )
        .unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
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
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("command test root must render a column");
    };
    let command = &children[0];
    let collection = find_virtual_collection(command).expect("fuzzy result list");
    let UiNodeKind::VirtualCollection { spec } = collection.kind() else {
        unreachable!()
    };
    assert_eq!(spec.data.len(), 2, "one group header plus one fuzzy match");
    assert_eq!(spec.height, Some(64.0), "small command sets must shrink");
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
#[allow(clippy::too_many_lines)]
fn command_filters_native_collection_and_keeps_large_rows_out_of_rhai() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/command").unwrap(),
            COMMAND.to_owned(),
        ),
        (
            ModuleId::parse("components/input").unwrap(),
            INPUT.to_owned(),
        ),
        (ModuleId::parse("components/kbd").unwrap(), KBD.to_owned()),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
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
        )
        .unwrap();
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
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    runtime
        .borrow_mut()
        .native_collections
        .register("commands", rows)
        .unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
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
fn official_combobox_is_public_overlay_and_virtual_collection_composition() {
    let combobox_id = ModuleId::parse("components/combobox").unwrap();
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (combobox_id, COMBOBOX.to_owned()),
        (
            ModuleId::parse("components/input").unwrap(),
            INPUT.to_owned(),
        ),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/combobox_test.rhai",
            r#"
                import "components/combobox" as combobox_component;
                fn selected(ctx, values) { () }
                fn opened(ctx, open) { () }
                fn queried(ctx, query) { () }
                fn view(ctx) {
                    combobox_component::Combobox(#{
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
                        part_styles: #{ option: style().height(px(41)) },
                        on_change: Fn("selected"),
                        on_open_change: Fn("opened"),
                        on_query_change: Fn("queried")
                    })
                }
            "#,
        )
        .unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        Rc::clone(&runtime),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
    let UiNodeKind::Overlay { trigger, spec, .. } = lifecycle.root().unwrap().kind() else {
        panic!("Combobox must compose the generic Overlay node");
    };
    assert_eq!(spec.id.as_str(), "theme");
    assert!(spec.open);
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
    let source = combobox_source();
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/combobox_keyboard.rhai",
            r#"
                import "components/combobox" as combobox;
                fn view(ctx) {
                    combobox::Combobox(#{
                        key: "grouped", open: false, selected: [], query: "",
                        options: [
                            #{ value: "a", label: "Alpha", group: "Second" },
                            #{ value: "b", label: "Beta", group: "First", disabled: true },
                            #{ value: "c", label: "Charlie", group: "Second" },
                            #{ value: "d", label: "Delta" }
                        ]
                    })
                }
            "#,
        )
        .unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        Rc::clone(&runtime),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();

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
            "unknown",
            r#"[#{ value: "known", label: "Known" }]"#,
            r#"["missing"]"#,
        ),
        (
            "empty_group",
            r#"[#{ value: "known", label: "Known", group: " " }]"#,
            "[]",
        ),
    ] {
        let source = combobox_source();
        let mut engine = RuntimeEngine::new();
        engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
        let script = format!(
            r#"
                import "components/combobox" as combobox;
                fn view(ctx) {{
                    combobox::Combobox(#{{
                        key: "invalid", options: {options}, selected: {selected}, open: false, query: ""
                    }})
                }}
            "#
        );
        let script_name = format!("ui/combobox_{name}.rhai");
        let compiled = engine
            .compile_self_contained_named(&script_name, &script)
            .unwrap();
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            Rc::new(RefCell::new(UiRuntimeState::new())),
            ComponentInstancePath::root("App", "root"),
            Some("main".to_owned()),
            BTreeMap::new(),
            &ComponentStateSchema::default(),
        )
        .unwrap();
        assert!(lifecycle.start(&mut engine).is_err(), "case {name}");
    }
}

#[test]
fn official_select_uses_scalar_controlled_choice_semantics() {
    let select_id = ModuleId::parse("components/select").unwrap();
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (select_id, SELECT.to_owned()),
        (
            ModuleId::parse("components/combobox").unwrap(),
            COMBOBOX.to_owned(),
        ),
        (
            ModuleId::parse("components/input").unwrap(),
            INPUT.to_owned(),
        ),
    ]));
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
                        key: "country", value: (), open: false, query: "",
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
        )
        .unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        runtime,
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
    let UiNodeKind::Overlay { spec, .. } = lifecycle.root().unwrap().kind() else {
        panic!("Select must compose Combobox over generic Overlay");
    };
    assert_eq!(spec.id.as_str(), "country-combobox");
}

#[test]
fn official_date_picker_consumes_locale_and_strict_iso_values() {
    let date_picker_id = ModuleId::parse("components/date_picker").unwrap();
    let source =
        EmbeddedScriptSource::new(BTreeMap::from([(date_picker_id, DATE_PICKER.to_owned())]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named("ui/date_picker_test.rhai", DATE_PICKER_TEST_APP)
        .unwrap();
    let locale = gpui_rhai::load_locale_source(
        engine.engine(),
        "en.rhai",
        include_str!("../../../registry/locales/en.rhai"),
    )
    .unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    runtime.borrow_mut().locale =
        Some(gpui_rhai::LocaleManager::new([locale], "en", "en").unwrap());
    runtime.borrow_mut().calendar_clock =
        gpui_rhai::CalendarClock::fixed(gpui_rhai::GregorianDate::parse_iso("2026-08-30").unwrap());
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        Rc::clone(&runtime),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
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
    let UiNodeKind::Box { children } = content.kind() else {
        unreachable!()
    };
    let UiNodeKind::Box { children: weeks } = children[2].kind() else {
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
    let (right, payload) = target_handler(content, "key:right");
    invoke_and_render(&mut lifecycle, &mut engine, &right, payload);
    assert_eq!(
        runtime
            .borrow()
            .component_state
            .get(&component, "focused_date"),
        Some(&UiValue::String("2026-09-02".to_owned()))
    );
    let UiNodeKind::Overlay { content, .. } = lifecycle.root().unwrap().kind() else {
        unreachable!()
    };
    let (commit, payload) = target_handler(content, "key:enter");
    invoke_and_render(&mut lifecycle, &mut engine, &commit, payload);
    let events = runtime.borrow_mut().drain_batch().events;
    assert!(events.iter().any(|event| {
        event.event.name == "change"
            && event.event.payload == UiValue::String("2026-09-02".to_owned())
    }));
}

#[test]
fn official_table_is_public_data_backed_rhai_composition() {
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
                fn sorted(ctx, value) { () }
                fn selected(ctx, value) { () }
                fn clicked(ctx, value) { () }
                fn view(ctx) {
                    table::Table(#{
                        key: "users", label: "Users", row_key: "id", fill_height: true,
                        rows: [
                            #{ id: "u1", name: "Ada", score: 12.5, joined: "2026-08-30", status: "Active" },
                            #{ id: "u2", name: "Lin", score: 9, joined: "2026-08-31", status: "Away" }
                        ],
                        columns: [
                            #{ key: "name", title: "Name", width: #{ kind: "fixed", value: 120 }, sortable: true },
                            #{ key: "score", title: "Score", width: #{ kind: "flex", value: 100 } },
                            #{ key: "joined", title: "Joined", width: #{ kind: "percent", value: 30 } },
                            #{ key: "status", title: "Status", width: #{ kind: "fixed", value: 90 } }
                        ],
                        loading: false, selection_mode: "multiple",
                        selected_keys: ["u1"], striped: true,
                        on_sort_change: Fn("sorted"), on_selection_change: Fn("selected"),
                        on_row_click: Fn("clicked")
                    })
                }
            "#,
        )
        .unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        runtime,
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("Table must be a public Box composition");
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
    let UiNodeKind::Box { children: cells } = row.kind() else {
        panic!("table row must remain a public row composition");
    };
    assert!(cells.iter().all(|cell| {
        cell.style().base.white_space == Some(gpui_rhai::WhiteSpaceMode::NoWrap)
            && cell.style().base.text_ellipsis == Some(true)
    }));
}

#[test]
fn official_table_groups_array_rows_with_controlled_collapse() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([(
        ModuleId::parse("components/table").unwrap(),
        TABLE.to_owned(),
    )]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
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
                        collapsed_groups: ctx.get_state("collapsed"),
                        on_group_toggle: Fn("toggled")
                    })
                }
            "#,
        )
        .unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        Rc::clone(&runtime),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        BTreeMap::new(),
        &schema,
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();

    let UiNodeKind::VirtualCollection { spec } = find_virtual_collection(lifecycle.root().unwrap())
        .expect("grouped Table must use virtual_collection")
        .kind()
    else {
        unreachable!()
    };
    assert_eq!(spec.data.len(), 5);
    assert_eq!(spec.sticky_headers.as_ref(), &BTreeSet::from([0, 3]));
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
fn official_table_groups_native_collection_without_materializing_rows_in_rhai() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([(
        ModuleId::parse("components/table").unwrap(),
        TABLE.to_owned(),
    )]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/native_grouped_table_test.rhai",
            r#"
                import "components/table" as table;
                fn view(ctx) {
                    table::Table(#{
                        key: "clusters", label: "Clusters", row_key: "id", height: 120,
                        rows: ctx.get_native_collection("clusters"), group_by: "track",
                        columns: [
                            #{ key: "track", title: "Track",
                                width: #{ kind: "fixed", value: 120 } },
                            #{ key: "state", title: "State",
                                width: #{ kind: "fixed", value: 100 } }
                        ]
                    })
                }
            "#,
        )
        .unwrap();
    let rows = gpui_rhai::NativeCollection::new(
        "id",
        [
            BTreeMap::from([
                ("id".to_owned(), UiValue::String("a".to_owned())),
                ("track".to_owned(), UiValue::String("alpha".to_owned())),
                ("state".to_owned(), UiValue::String("Ready".to_owned())),
            ]),
            BTreeMap::from([
                ("id".to_owned(), UiValue::String("b".to_owned())),
                ("track".to_owned(), UiValue::String("beta".to_owned())),
                ("state".to_owned(), UiValue::String("Drift".to_owned())),
            ]),
            BTreeMap::from([
                ("id".to_owned(), UiValue::String("c".to_owned())),
                ("track".to_owned(), UiValue::String("alpha".to_owned())),
                ("state".to_owned(), UiValue::String("Drift".to_owned())),
            ]),
        ],
    )
    .unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    runtime
        .borrow_mut()
        .native_collections
        .register("clusters", rows)
        .unwrap();
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        runtime,
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();

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
        (
            ModuleId::parse("components/combobox").unwrap(),
            COMBOBOX.to_owned(),
        ),
        (
            ModuleId::parse("components/input").unwrap(),
            INPUT.to_owned(),
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
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    runtime.borrow_mut().locale =
        Some(gpui_rhai::LocaleManager::new([locale], "en", "en").unwrap());
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        runtime,
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
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
        (
            ModuleId::parse("components/combobox").unwrap(),
            COMBOBOX.to_owned(),
        ),
        (
            ModuleId::parse("components/input").unwrap(),
            INPUT.to_owned(),
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
        (
            ModuleId::parse("components/combobox").unwrap(),
            COMBOBOX.to_owned(),
        ),
        (
            ModuleId::parse("components/input").unwrap(),
            INPUT.to_owned(),
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
    let source = pagination_source();
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
    let collection = find_virtual_collection(lifecycle.root().unwrap())
        .expect("page-size Select virtual options");
    let UiNodeKind::VirtualCollection { spec } = collection.kind() else {
        unreachable!()
    };
    let option = spec
        .realized
        .values()
        .find(|node| {
            matches!(
                node.handler_payload("click"),
                Some(UiValue::Map(payload))
                    if matches!(payload.get("selected"),
                        Some(UiValue::Array(values))
                            if values == &[UiValue::String("25".to_owned())])
            )
        })
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
    let UiNodeKind::Box { children } = root.kind() else {
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
    let UiNodeKind::Box {
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
fn radio_group_forwards_pointer_and_keyboard_changes_to_its_caller() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/radio").unwrap(),
            RADIO.to_owned(),
        ),
        (
            ModuleId::parse("components/radio_group").unwrap(),
            RADIO_GROUP.to_owned(),
        ),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
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
        )
        .unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
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
#[allow(clippy::too_many_lines)]
fn toggle_group_routes_pointer_and_roving_keyboard_changes_to_caller_state() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/button").unwrap(),
            BUTTON.to_owned(),
        ),
        (
            ModuleId::parse("components/button_group").unwrap(),
            BUTTON_GROUP.to_owned(),
        ),
        (
            ModuleId::parse("components/toggle").unwrap(),
            TOGGLE.to_owned(),
        ),
        (
            ModuleId::parse("components/toggle_group").unwrap(),
            TOGGLE_GROUP.to_owned(),
        ),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
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
        )
        .unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
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

    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("ToggleGroup must render grouped Toggle children");
    };
    let toggle = children[0]
        .handler("click")
        .and_then(gpui_rhai::UiEventHandler::as_script)
        .cloned()
        .unwrap();
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &toggle, UiValue::Null)
        .unwrap();
    for _ in 0..2 {
        let events = runtime.borrow_mut().drain_batch().events;
        assert_eq!(events.len(), 1);
        for event in events {
            let _ = lifecycle
                .invoke_component_event_transactional(&engine, event)
                .unwrap();
        }
    }
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert_eq!(
        runtime.borrow().component_state.get(&path, "formats"),
        Some(&UiValue::Array(vec![UiValue::String("bold".to_owned())]))
    );

    let (right, payload) = target_handler(lifecycle.root().unwrap(), "key:right");
    invoke_and_render(&mut lifecycle, &mut engine, &right, payload);
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
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
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
    let source = EmbeddedScriptSource::new(BTreeMap::from([(
        ModuleId::parse("components/slider").unwrap(),
        SLIDER.to_owned(),
    )]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
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
    let source = EmbeddedScriptSource::new(BTreeMap::from([(
        ModuleId::parse("components/scroll_area").unwrap(),
        SCROLL_AREA.to_owned(),
    )]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/scroll_area.rhai",
            r#"
                import "components/scroll_area" as scroll_area;
                fn view(ctx) {
                    scroll_area::ScrollArea(#{ key: "logs", label: "Logs", width: px(240), height: px(120),
                        axis: "vertical", vertical_scrollbar: "always",
                        content: column([text("one"), text("two"), text("three")]) })
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
        (
            ModuleId::parse("components/spinner").unwrap(),
            SPINNER.to_owned(),
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
                import "components/spinner" as spinner;
                fn view(ctx) {
                    column([
                        avatar::Avatar(#{ name: "Ada", presence: "online" }),
                        progress::Progress(#{ key: "download", value: 42, max: 100 }),
                        progress::Progress(#{ key: "loading", indeterminate: true }),
                        skeleton::Skeleton(#{ key: "card", width: 240, height: 80 }),
                        spinner::Spinner(#{ key: "loading-spinner", label: "Loading" })
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
        determinate[0].animations()[0],
        gpui_rhai::AnimationSpec::Transition(_)
    ));
    let UiNodeKind::Box {
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
    assert!(matches!(
        children[4].animations()[0],
        gpui_rhai::AnimationSpec::LoopingTransition(spec)
            if spec.property == gpui_rhai::AnimationProperty::Rotate
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
    let UiNodeKind::Box { children } = root.kind() else {
        panic!("M2 composites must compose into a container");
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
        collapsible_children[1].animations()[0],
        gpui_rhai::AnimationSpec::Transition(_)
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
    assert!(!first_item[1].animations().is_empty());
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
    let manual_clock = gpui_rhai::ManualRuntimeClock::new(std::time::Instant::now());
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    runtime.borrow_mut().clock = manual_clock.clock();
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        Rc::clone(&runtime),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
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
    assert_eq!(
        header[0]
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
