//! Shared part names must compose by cascade layer, not by nested resolution.

mod common;

use std::{cell::RefCell, rc::Rc};

use gpui_rhai::{
    BackgroundFill, ColorValue, Length, Rgba8, ThemeManager, ThemeSelection, ThemeTokenOverrides,
    UiNode, UiNodeKind, UiRuntimeState, load_component_styles, load_theme_with_layers,
    load_token_base,
};

struct Case {
    module: &'static str,
    export: &'static str,
    props: &'static str,
    parts: [&'static str; 2],
    target: Option<&'static str>,
}

const CASES: &[Case] = &[
    Case {
        module: "components/table",
        export: "Table",
        props: "label:\"Rows\",height:120,row_key:\"id\",columns:[#{key:\"name\",title:\"Name\",width:#{kind:\"flex\",value:1}}],rows:[#{id:\"r1\",name:\"First\"}],selection_mode:\"single\",selected_keys:[\"r1\"],",
        parts: ["row", "row_selected"],
        target: Some("r1"),
    },
    Case {
        module: "components/list",
        export: "List",
        props: "label:\"Rows\",height:120,items:[#{key:\"r1\",title:\"First\"}],selection_mode:\"single\",selected_keys:[\"r1\"],",
        parts: ["row", "row_selected"],
        target: Some("r1"),
    },
    Case {
        module: "components/selectable_card_group",
        export: "SelectableCardGroup",
        props: "label:\"Choices\",values:[\"one\"],items:[#{value:\"one\",label:\"One\",content:text(\"Body\")}],",
        parts: ["card", "card_selected"],
        target: Some("one"),
    },
    Case {
        module: "components/input",
        export: "Input",
        props: "value:\"sample\",label:\"Field\",",
        parts: ["root", "input"],
        target: None,
    },
    Case {
        module: "components/time_input",
        export: "TimeInput",
        props: "value:\"12:30\",label:\"Time\",",
        parts: ["root", "input"],
        target: None,
    },
    Case {
        module: "components/progress",
        export: "Progress",
        props: "value:25,label:\"Progress\",",
        parts: ["root", "track"],
        target: None,
    },
    Case {
        module: "components/command_dialog",
        export: "CommandDialog",
        props: "open:false,query:\"\",active_value:\"\",label:\"Commands\",items:[],on_open_change:Fn(\"noop\"),on_query_change:Fn(\"noop\"),on_active_change:Fn(\"noop\"),on_action:Fn(\"noop\"),",
        parts: ["root", "dialog"],
        target: None,
    },
    Case {
        module: "components/toast",
        export: "Toast",
        props: "items:[#{id:\"notice\",title:\"Notice\",variant:\"success\",paused:true}],on_dismiss:Fn(\"noop\"),",
        parts: ["toast", "toast_success"],
        target: Some("notice"),
    },
    Case {
        module: "patterns/app_shell",
        export: "AppShell",
        props: "label:\"Shell\",main:text(\"Main\"),sidebar:text(\"Navigation\"),sidebar_mode:\"overlay\",on_sidebar_dismiss:Fn(\"noop\"),",
        parts: ["drawer", "sidebar"],
        target: Some("probe-sidebar"),
    },
    Case {
        module: "components/spinner",
        export: "Spinner",
        props: "label:\"Activity\",",
        parts: ["root", "indicator"],
        target: None,
    },
    Case {
        module: "components/table",
        export: "Table",
        props: "label:\"Rows\",height:120,row_key:\"id\",columns:[#{key:\"name\",title:\"Name\",width:#{kind:\"flex\",value:1}}],rows:[#{id:\"r1\",name:\"First\"}],selection_mode:\"single\",selected_keys:[\"r1\"],",
        parts: ["cell", "cell_selected"],
        target: Some("r1/name"),
    },
    Case {
        module: "components/command",
        export: "Command",
        props: "label:\"Commands\",query:\"\",active_value:\"first\",items:[#{value:\"first\",label:\"First\"}],on_query_change:Fn(\"noop\"),on_active_change:Fn(\"noop\"),on_action:Fn(\"noop\"),",
        parts: ["item", "item_active"],
        target: Some("item:first"),
    },
    Case {
        module: "components/command",
        export: "Command",
        props: "label:\"Commands\",query:\"\",active_value:\"\",items:[],appearance:\"embedded\",",
        parts: ["root", "root_embedded"],
        target: None,
    },
    Case {
        module: "components/command",
        export: "Command",
        props: "label:\"Commands\",query:\"\",active_value:\"\",items:[],appearance:\"embedded\",",
        parts: ["card", "card_embedded"],
        target: Some("card"),
    },
    Case {
        module: "components/command",
        export: "Command",
        props: "label:\"Commands\",query:\"\",active_value:\"first\",items:[#{value:\"first\",label:\"First\"}],",
        parts: ["icon_slot", "icon_slot_active"],
        target: Some("item:first/leading"),
    },
    Case {
        module: "components/command",
        export: "Command",
        props: "label:\"Commands\",query:\"\",active_value:\"first\",items:[#{value:\"first\",label:\"First\",logo:text(\"L\")}],",
        parts: ["icon_slot", "logo_slot"],
        target: Some("item:first/leading"),
    },
];

fn find_key<'a>(node: &'a UiNode, key: &str) -> Option<&'a UiNode> {
    if node
        .key()
        .is_some_and(|candidate| candidate.as_str() == key)
    {
        return Some(node);
    }
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(|child| find_key(child, key))
        }
        UiNodeKind::Overlay {
            trigger, content, ..
        } => find_key(trigger, key).or_else(|| find_key(content, key)),
        UiNodeKind::Layer { content, .. } => find_key(content, key),
        UiNodeKind::ErrorBoundary { child, fallback } => {
            find_key(child, key).or_else(|| find_key(fallback, key))
        }
        UiNodeKind::VirtualCollection { spec } => {
            spec.realized.values().find_map(|row| find_key(row, key))
        }
        _ => None,
    }
}

fn source(case: &Case, extra: &str) -> String {
    format!(
        "import \"{}\" as widget; fn noop(ctx,payload){{()}} fn view(ctx){{widget::{}(#{{key:\"probe\",{}{extra}}})}}",
        case.module, case.export, case.props
    )
}

fn sheet(case: &Case, parts: &[&str], color: u32, radius: u32) -> String {
    let rules = parts
        .iter()
        .map(|part| format!("{part}:style().background(rgb({color})).radius(px({radius}))"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "fn component_styles(){{#{{\"{}\":#{{{rules}}}}}}}",
        case.module
    )
}

fn assert_case(case: &Case) {
    const GREEN: u32 = 0x0000_ff00;
    const RED: u32 = 0x00ff_0000;
    const BLUE: u32 = 0x0000_00ff;
    let common = case.parts[0];
    let specific = case.parts[1];
    let scenarios = [
        (sheet(case, &[common], GREEN, 41), String::new(), GREEN, 41),
        (
            sheet(case, &[specific], GREEN, 41),
            String::new(),
            GREEN,
            41,
        ),
        (
            sheet(case, &[specific], RED, 3),
            format!("part_styles:#{{{common}:style().background(rgb({GREEN})).radius(px(41))}}"),
            GREEN,
            41,
        ),
        (
            sheet(case, &[common], RED, 3),
            format!("part_styles:#{{{specific}:style().background(rgb({GREEN})).radius(px(41))}}"),
            GREEN,
            41,
        ),
        (
            sheet(case, &[], RED, 3),
            format!(
                "part_styles:#{{{common}:style().background(rgb({RED})).radius(px(3)),{specific}:style().background(rgb({BLUE})).radius(px(43))}}"
            ),
            BLUE,
            43,
        ),
    ];
    for (scenario, (application, extra, expected, radius)) in scenarios.into_iter().enumerate() {
        let (mut engine, compiled) =
            common::compile_official("ui/part_cascade.rhai", &source(case, &extra));
        // Obtain the actual definitions by evaluating the real formal export.
        let seed = common::new_runtime();
        let _initial = common::mount(&mut engine, compiled.clone(), &seed);
        let registry = engine.component_exports().unwrap();
        let preset_rules = load_component_styles(
            engine.engine(),
            "preset_styles.rhai",
            &sheet(case, &case.parts, RED, 3),
            &registry,
        )
        .unwrap();
        let application =
            load_component_styles(engine.engine(), "styles.rhai", &application, &registry).unwrap();
        let base = load_token_base(
            engine.engine(),
            "tokens.rhai",
            gpui_rhai_registry::TOKEN_BASE_SOURCE,
        )
        .unwrap();
        for &(name, preset) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
            let theme = load_theme_with_layers(
                engine.engine(),
                Some(&base),
                name,
                preset,
                &ThemeTokenOverrides {
                    component_styles: preset_rules.clone(),
                    ..Default::default()
                },
            )
            .unwrap();
            let selection = ThemeSelection::new(&theme.family, &theme.name);
            let mut state = UiRuntimeState::new();
            state.theme = Some(ThemeManager::from_variants([theme], selection).unwrap());
            state.replace_component_styles_from_host(application.clone());
            let runtime = Rc::new(RefCell::new(state));
            let lifecycle = common::mount(&mut engine, compiled.clone(), &runtime);
            let root = lifecycle.root().unwrap();
            let target = case.target.map_or(root, |path| {
                path.split('/')
                    .fold(root, |node, key| find_key(node, key).unwrap())
            });
            let actual = target.style();
            assert_eq!(
                actual
                    .base
                    .background
                    .as_deref()
                    .and_then(BackgroundFill::as_solid_value),
                Some(&ColorValue::Literal(Rgba8::from_rgb_hex(expected))),
                "{}/{name}/scenario{scenario}: background",
                case.module
            );
            assert_eq!(
                actual.base.radii.top_left,
                Some(Length::Pixels(f64::from(radius))),
                "{}/{name}/scenario{scenario}: radius",
                case.module
            );
        }
    }
}

macro_rules! cascade_case {
    ($name:ident, $index:expr) => {
        #[test]
        fn $name() {
            assert_case(&CASES[$index]);
        }
    };
}
cascade_case!(table_row_part_layers, 0);
cascade_case!(list_row_part_layers, 1);
cascade_case!(selectable_card_part_layers, 2);
cascade_case!(input_part_layers, 3);
cascade_case!(time_input_part_layers, 4);
cascade_case!(progress_part_layers, 5);
cascade_case!(command_dialog_part_layers, 6);
cascade_case!(toast_part_layers, 7);
cascade_case!(app_shell_drawer_part_layers, 8);
cascade_case!(spinner_part_layers_positive_control, 9);
cascade_case!(table_cell_part_layers, 10);
cascade_case!(command_item_part_layers, 11);
cascade_case!(command_embedded_root_part_layers, 12);
cascade_case!(command_embedded_card_part_layers, 13);
cascade_case!(command_active_icon_part_layers, 14);
cascade_case!(command_logo_part_layers, 15);

#[test]
fn time_input_keeps_the_native_input_default_surface() {
    for &(name, preset) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut engine, compiled) = common::compile_official(
            "ui/time_surface.rhai",
            r#"
            import "components/input" as input;
            import "components/time_input" as time_input;
            fn view(ctx){row([input::Input(#{key:"plain",value:"12:30",label:"Plain"}),
                time_input::TimeInput(#{key:"clock",value:"12:30",label:"Clock"})])}
        "#,
        );
        let base = load_token_base(
            engine.engine(),
            "tokens.rhai",
            gpui_rhai_registry::TOKEN_BASE_SOURCE,
        )
        .unwrap();
        let theme = load_theme_with_layers(
            engine.engine(),
            Some(&base),
            name,
            preset,
            &ThemeTokenOverrides::default(),
        )
        .unwrap();
        let selection = ThemeSelection::new(&theme.family, &theme.name);
        let mut state = UiRuntimeState::new();
        state.theme = Some(ThemeManager::from_variants([theme], selection).unwrap());
        let root = common::try_render_in(&mut engine, &compiled, state).unwrap();
        let UiNodeKind::Box { children } = root.kind() else {
            panic!("two fields")
        };
        assert_eq!(
            children[0].style(),
            children[1].style(),
            "{name}: a temporal wrapper must not replace its native Input surface"
        );
    }
}
