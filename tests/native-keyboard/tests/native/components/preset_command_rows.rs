//! Rich and data-only command rows share one leading slot, baseline and key owner.

use std::collections::BTreeMap;

use gpui::TestAppContext;
use gpui_rhai::{
    MotionPreference, NativeCollection, ScriptViewExtension, ThemeTokenOverrides, UiRuntimeState,
    UiValue,
};

use crate::support::{
    Mount, Step, bounds, click_center, filled, filled_including_rounded_clips, last_error, settle,
    snapshot, text_of,
};

const MAIN: &str = r#"
    import "components/command" as command;
    fn state_schema(){#{fields:#{
        query:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        active:#{schema:#{type:"string"},"default":#{type:"string",value:"a"}},
        action:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}}
    }}}
    fn init(ctx){ctx.set_locale("__LOCALE__");}
    fn queried(ctx,value){ctx.set_state("query",value);}
    fn moved(ctx,value){ctx.set_state("active",value);}
    fn acted(ctx,value){ctx.set_state("action",value);}
    fn array_items(){[
        #{value:"a",label:"Alpha",hint:"Remote deployment",logo:svg("<svg width='24' height='24' viewBox='0 0 24 24'><path fill='currentColor' d='M4 4h16v16H4z'/></svg>"),icon:"app/icons/command_search_modern"},
        #{value:"b",label:"Beta",hint:"Local logs",icon:"app/icons/command_search_modern",shortcut:"cmd-k"},
        #{value:"c",label:"Gamma"},
        #{value:"d",label:"Delta",icon:svg("<svg width='24' height='24' viewBox='0 0 24 24'><path fill='currentColor' d='M5 5h14v14H5z'/></svg>")}
    ]}
    fn view(ctx){column([
        command::Command(#{key:"commands",label:"Commands",query:ctx.get_state("query"),active_value:ctx.get_state("active"),
            items:__ITEMS__,on_query_change:Fn("queried"),on_active_change:Fn("moved"),on_action:Fn("acted"),
            part_styles:#{icon:style().background(rgb(0xff00ff)),logo:style().background(rgb(0x00ff00))}
        }).with_style(style().width(px(560))),
        text(`${ctx.get_state("action")}/${ctx.get_state("query")}`).test_id("state")
    ]).with_style(style().width(px(600)).padding(px(20)))}
"#;

#[derive(Clone)]
struct Data;

impl ScriptViewExtension for Data {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        let rows = [
            (
                "a",
                "Alpha",
                "Remote deployment",
                "app/icons/command_search_modern",
                "app/icons/command_enter_clean",
                "",
            ),
            (
                "b",
                "Beta",
                "Local logs",
                "app/icons/command_search_modern",
                "",
                "cmd-k",
            ),
            ("c", "Gamma", "", "", "", ""),
            ("d", "Delta", "", "app/icons/command_search_modern", "", ""),
        ]
        .into_iter()
        .map(|(value, label, hint, icon, logo, shortcut)| {
            BTreeMap::from([
                ("value".into(), UiValue::String(value.into())),
                ("label".into(), UiValue::String(label.into())),
                ("hint".into(), UiValue::String(hint.into())),
                ("icon".into(), UiValue::String(icon.into())),
                ("logo".into(), UiValue::String(logo.into())),
                ("shortcut".into(), UiValue::String(shortcut.into())),
            ])
        });
        runtime
            .native_collections
            .register(
                "command-rows",
                NativeCollection::new("value", rows).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())
    }
}

fn source(native: bool, rtl: bool) -> String {
    MAIN.replace("__LOCALE__", if rtl { "ar" } else { "en" })
        .replace(
            "__ITEMS__",
            if native {
                "ctx.get_native_collection(\"command-rows\")"
            } else {
                "array_items()"
            },
        )
}

fn close(actual: f64, expected: f64, case: &str) {
    assert!(
        (actual - expected).abs() < 0.6,
        "{case}: {actual} != {expected}"
    );
}

#[gpui::test]
fn rich_and_native_command_rows_reserve_source_sized_glyph_slots_and_inline_hints(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for native in [false, true] {
            for rtl in [false, true] {
                for scale in [100, 200] {
                    let clean = name.starts_with("clean");
                    let f = f64::from(scale) / 100.0;
                    let overrides = ThemeTokenOverrides {
                        environment_defaults: [("scale".into(), scale.to_string())].into(),
                        ..Default::default()
                    };
                    let (mut visual, view) = Mount::components(source(native, rtl))
                        .theme(theme)
                        .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
                        .locale("ar.rhai", gpui_rhai_registry::AR_LOCALE)
                        .motion(MotionPreference::None)
                        .with(move |view| view.extension(Data).theme_token_overrides(overrides))
                        .mount(cx);
                    settle(&mut visual, Step::Frame, 8);
                    let case = format!("{name}/{native}/{rtl}/{scale}");
                    let slot = (if clean { 20.0 } else { 26.0 }) * f;
                    let pad = (if clean { 8.0 } else { 10.0 }) * f;
                    let gap = (if clean { 10.0 } else { 12.0 }) * f;
                    for name in ["Alpha", "Beta", "Gamma", "Delta"] {
                        let row = bounds(&mut visual, &view, "option", name);
                        let label = bounds(&mut visual, &view, "text", name);
                        let leading = if rtl {
                            row.x + row.width - label.x - label.width
                        } else {
                            label.x - row.x
                        };
                        close(leading, pad + slot + gap, &case);
                    }
                    let label = bounds(&mut visual, &view, "text", "Alpha");
                    let hint = bounds(&mut visual, &view, "text", "Remote deployment");
                    close(label.y, hint.y, &case);
                    close(label.height, hint.height, &case);
                    let between = if rtl {
                        label.x - hint.x - hint.width
                    } else {
                        hint.x - label.x - label.width
                    };
                    close(between, 8.0 * f, &case);
                    for (color, label, size) in [
                        (0x00ff00ff, "Alpha", if clean { 12.0 } else { 14.0 }),
                        (0xff00ffff, "Beta", if clean { 16.0 } else { 17.0 }),
                    ] {
                        let row = bounds(&mut visual, &view, "option", label);
                        let quads = filled_including_rounded_clips(&mut visual, color);
                        let glyph = quads.iter().copied()
                            .find(|quad| {
                                f64::from(quad.bounds.y) >= row.y
                                    && f64::from(quad.bounds.bottom()) <= row.y + row.height
                            })
                            .unwrap_or_else(||panic!("{case}/{label}: the actual glyph box must paint; row={row:?}, quads={quads:?}"));
                        close(f64::from(glyph.bounds.width), size * f, &case);
                        close(f64::from(glyph.bounds.height), size * f, &case);
                        let expected = if rtl {
                            row.x + row.width - pad - slot / 2.0
                        } else {
                            row.x + pad + slot / 2.0
                        };
                        close(
                            f64::from(glyph.bounds.x + glyph.bounds.width / 2.0),
                            expected,
                            &case,
                        );
                        close(
                            f64::from(glyph.bounds.y + glyph.bounds.height / 2.0),
                            row.y + row.height / 2.0,
                            &case,
                        );
                    }
                    let tree = snapshot(&mut visual, &view);
                    assert_eq!(
                        tree.find_by_role_and_name("option", "Alpha")
                            .next()
                            .unwrap()
                            .description,
                        "Remote deployment"
                    );
                    assert!(
                        !tree.nodes().any(|node| node.role == "image"),
                        "decorations do not become extra spoken controls"
                    );
                    assert!(last_error(&mut visual, &view).is_none(), "{case}");
                }
            }
        }
    }
}

#[gpui::test]
fn hint_search_and_activation_keep_the_same_live_editor_across_design_switches(
    cx: &mut TestAppContext,
) {
    for native in [false, true] {
        let (mut visual, view) = Mount::components(source(native, false))
            .theme(gpui_rhai_registry::MODERN_LIGHT_THEME)
            .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
            .with(|view| {
                view.extension(Data).theme_sources([(
                    "clean_dark.rhai".into(),
                    gpui_rhai_registry::CLEAN_DARK_THEME.into(),
                )])
            })
            .mount(cx);
        settle(&mut visual, Step::Frame, 8);
        let input = snapshot(&mut visual, &view)
            .find_by_role_and_name("text_field", "Commands")
            .next()
            .unwrap()
            .clone();
        click_center(&mut visual, input.geometry.unwrap().visual);
        visual.simulate_input("Remote");
        settle(&mut visual, Step::Frame, 8);
        assert_eq!(
            snapshot(&mut visual, &view)
                .find_by_role_and_name("option", "Alpha")
                .count(),
            1
        );
        assert_eq!(
            snapshot(&mut visual, &view)
                .find_by_role_and_name("option", "Beta")
                .count(),
            0
        );
        visual
            .update(|_, cx| view.select_theme("Clean", "Dark", cx))
            .unwrap();
        settle(&mut visual, Step::Frame, 8);
        assert_eq!(
            snapshot(&mut visual, &view)
                .find_by_role_and_name("text_field", "Commands")
                .next()
                .unwrap()
                .id,
            input.id
        );
        visual.simulate_keystrokes("enter");
        settle(&mut visual, Step::Frame, 4);
        assert_eq!(text_of(&mut visual, &view, "state"), "a/Remote");
        assert!(last_error(&mut visual, &view).is_none());
    }
}

#[gpui::test]
fn long_command_labels_and_hints_shrink_inside_their_row_without_losing_semantic_text(
    cx: &mut TestAppContext,
) {
    let label = "长期运行的配置检查 ".repeat(50);
    let hint = "Another remote endpoint ".repeat(50);
    let main = source(false, false)
        .replace("label:\"Beta\"", &format!("label:{label:?}"))
        .replace("hint:\"Local logs\"", &format!("hint:{hint:?}"));
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual, view) = Mount::components(main.clone())
            .theme(theme)
            .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
            .motion(MotionPreference::None)
            .mount(cx);
        settle(&mut visual, Step::Frame, 8);
        let row = bounds(&mut visual, &view, "option", &label);
        let label_box = bounds(&mut visual, &view, "text", &label);
        let hint_box = bounds(&mut visual, &view, "text", &hint);
        for field in [label_box, hint_box] {
            assert!(
                field.width > 0.0
                    && field.x >= row.x
                    && field.x + field.width <= row.x + row.width + 0.6,
                "{name}: long text leaves its actual row: {field:?}, {row:?}"
            );
            assert!(field.y >= row.y && field.y + field.height <= row.y + row.height + 0.6);
        }
        assert_eq!(
            snapshot(&mut visual, &view)
                .find_by_role_and_name("option", &label)
                .next()
                .unwrap()
                .description,
            hint
        );
        assert!(last_error(&mut visual, &view).is_none());
    }
}

#[gpui::test]
fn oversized_decorative_content_is_clipped_to_the_command_leading_slot(cx: &mut TestAppContext) {
    let main=source(false,false).replace("icon:\"app/icons/command_search_modern\"",r#"icon:box([
        box([]).with_style(style().width(px(200)).height(px(60)).flex_shrink(false).background(rgb(0xff8800)))
    ])"#);
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let clean = name.starts_with("clean");
        let (mut visual, view) = Mount::components(main.clone())
            .theme(theme)
            .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
            .motion(MotionPreference::None)
            .mount(cx);
        settle(&mut visual, Step::Frame, 8);
        let row = bounds(&mut visual, &view, "option", "Beta");
        let left = row.x + if clean { 8.0 } else { 10.0 };
        let right = left + if clean { 20.0 } else { 26.0 };
        let painted = filled(&mut visual, 0xff8800ff)
            .into_iter()
            .find(|quad| {
                f64::from(quad.bounds.y) < row.y + row.height
                    && f64::from(quad.bounds.bottom()) > row.y
            })
            .unwrap();
        assert!(
            f64::from(painted.bounds.right()) > right + 20.0,
            "the fixture really overflows before clipping"
        );
        let visible = painted.visible();
        assert!(
            f64::from(visible.x) >= left - 0.6 && f64::from(visible.right()) <= right + 0.6,
            "{name}: decoration paints over the command label: {painted:?}"
        );
        assert!(last_error(&mut visual, &view).is_none());
    }
}

#[gpui::test]
fn command_hint_updates_clear_the_description_without_replacing_the_option(
    cx: &mut TestAppContext,
) {
    let (mut visual,view)=Mount::components(r#"
        import "components/command" as command;
        fn state_schema(){#{fields:#{show:#{schema:#{type:"bool"},"default":#{type:"bool",value:true}}}}}
        fn toggle(ctx,value){ctx.set_state("show",!ctx.get_state("show"));}
        fn view(ctx){column([
            text("Toggle hint").accessibility_role("button").accessibility_label("Toggle hint").on_click(Fn("toggle")),
            command::Command(#{key:"commands",label:"Commands",query:"",active_value:"a",
                items:[#{value:"a",label:"Alpha",hint:if ctx.get_state("show"){"Remote"}else{""}}]})
        ]).with_style(style().width(px(560)).padding(px(20)))}
    "#).motion(MotionPreference::None).mount(cx);
    settle(&mut visual, Step::Frame, 8);
    let first = snapshot(&mut visual, &view)
        .find_by_role_and_name("option", "Alpha")
        .next()
        .unwrap()
        .clone();
    assert_eq!(first.description, "Remote");
    for expected in ["", "Remote"] {
        let toggle = bounds(&mut visual, &view, "button", "Toggle hint");
        click_center(&mut visual, toggle);
        settle(&mut visual, Step::Frame, 8);
        let current = snapshot(&mut visual, &view)
            .find_by_role_and_name("option", "Alpha")
            .next()
            .unwrap()
            .clone();
        assert_eq!(current.id, first.id);
        assert_eq!(
            current.description, expected,
            "a removed hint must not survive in the committed semantic frame"
        );
        assert!(last_error(&mut visual, &view).is_none());
    }
}
