//! Full designs keep segmented, chip, underline and document-tab geometries separate.

use gpui::TestAppContext;
use gpui_rhai::MotionPreference;

use crate::support::{Mount, Step, bounds, click_center, last_error, settle, snapshot};

#[gpui::test]
fn tab_variants_keep_their_geometry_addons_and_controlled_callbacks(cx: &mut TestAppContext) {
    for (name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let clean = name.starts_with("clean");
        for variant in ["track", "chip", "underline", "bar"] {
            for size in ["sm", "md", "lg"] {
                let index = match size {
                    "sm" => 0,
                    "md" => 1,
                    _ => 2,
                };
                let option = match (variant, clean) {
                    ("track", false) => [26.0, 30.0, 36.0],
                    ("track", true) => [20.0, 24.0, 28.0],
                    ("chip", false) => [26.0, 30.0, 36.0],
                    ("chip", true) => [24.0, 28.0, 32.0],
                    ("underline", false) => [32.0, 36.0, 42.0],
                    ("underline", true) => [30.0, 34.0, 38.0],
                    (_, false) => [28.0, 32.0, 36.0],
                    (_, true) => [24.0, 28.0, 32.0],
                }[index];
                let inset = if variant == "track" {
                    if clean { 2.0 } else { [2.0, 3.0, 4.0][index] }
                } else {
                    0.0
                };
                let total = option + inset * 2.0;
                let script = format!(
                    r#"import "components/tabs" as tabs;
                    fn state_schema(){{#{{fields:#{{value:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:"a"}}}}}}}}}}
                    fn changed(ctx,value){{ctx.set_state("value",value);}}
                    fn add(ctx,value){{}}
                    fn view(ctx){{tabs::Tabs(#{{key:"tabs",label:"Sections",value:ctx.get_state("value"),variant:"{variant}",size:"{size}",
                        panel:false,overflow_menu:true,menu_label:"All sections",on_add:Fn("add"),on_change:Fn("changed"),
                        tabs:[#{{value:"a",label:"Alpha"}},#{{value:"b",label:"Beta"}},#{{value:"c",label:"Locked",disabled:true}}]
                    }})}}"#
                );
                let (mut visual, view) = Mount::components(script)
                    .theme(*theme)
                    .motion(MotionPreference::None)
                    .id("preset-tabs")
                    .mount(cx);
                settle(&mut visual, Step::Frame, 3);
                let list = bounds(&mut visual, &view, "tablist", "Sections");
                let alpha = bounds(&mut visual, &view, "tab", "Alpha");
                let menu = bounds(&mut visual, &view, "button", "All sections");
                let add = bounds(&mut visual, &view, "button", "Add tab");
                for (actual, expected, kind) in [
                    (list.height, total, "list"),
                    (alpha.height, option, "tab"),
                    (menu.height, total, "menu"),
                    (add.height, total, "add"),
                ] {
                    assert!(
                        (actual - expected).abs() < 0.5,
                        "{name}/{variant}/{size} {kind}: {actual}, expected {expected}"
                    );
                }
                assert!(
                    (alpha.y - list.y - inset).abs() < 0.5,
                    "{name}/{variant}/{size}: track inset"
                );
                assert!(
                    (menu.width - menu.height).abs() < 0.5 && (add.width - add.height).abs() < 0.5,
                    "square icon actions"
                );
                let beta = bounds(&mut visual, &view, "tab", "Beta");
                click_center(&mut visual, beta);
                settle(&mut visual, Step::Frame, 2);
                let tree = snapshot(&mut visual, &view);
                assert_eq!(
                    tree.find_by_role_and_name("tab", "Beta")
                        .next()
                        .unwrap()
                        .selected,
                    Some(true)
                );
                click_center(&mut visual, menu);
                settle(&mut visual, Step::Frame, 2);
                assert!(
                    snapshot(&mut visual, &view)
                        .nodes()
                        .any(|node| node.role == "menu"),
                    "{name}/{variant}: actual overflow menu"
                );
                assert!(last_error(&mut visual, &view).is_none());
            }
        }
    }
}

#[gpui::test]
fn selected_tab_state_does_not_clobber_common_app_and_instance_styles(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual,view)=Mount::components(r#"import "components/tabs" as tabs;
            fn view(ctx){tabs::Tabs(#{value:"a",label:"Custom",panel:false,
                part_styles:#{tab:style().border(px(4)).border_color(rgba(0xff007fff))},
                tabs:[#{value:"a",label:"Alpha"},#{value:"b",label:"Beta"}]})}"#)
            .theme(theme).motion(MotionPreference::None).with(|view|view.component_styles(r#"fn component_styles(){#{
                "components/tabs":#{root:style().padding(px(9)),tab:style().border(px(3)).border_color(rgba(0xff007fff))}
            }}"#)).id("tab-cascade").mount(cx);
        settle(&mut visual, Step::Frame, 2);
        let selected = bounds(&mut visual, &view, "tab", "Alpha");
        assert!(
            selected.x >= 9.0,
            "{name}: panel-less root rules must apply"
        );
        // Borders are not filled quads; inspect their native widths directly.
        let quads = crate::support::painted_quads(&mut visual);
        assert!(
            quads.iter().any(|quad| quad.border_widths == [4.0; 4]
                && (f64::from(quad.bounds.x) - selected.x).abs() < 0.5),
            "{name}: selected preset overrode instance common part"
        );
        assert!(last_error(&mut visual, &view).is_none());
    }
}
