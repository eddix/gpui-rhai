//! Shared segmented geometry, bounded inset focus and unchanged group behavior.

use crate::support::{
    Mount, Step, bounds, click_center, last_error, painted_quads, settle, snapshot,
};

#[gpui::test]
fn joined_buttons_keep_outer_corners_and_the_focused_edge_visible(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for density in ["comfortable", "compact"] {
            for vertical in [false, true] {
                let script = format!(
                    r#"import "components/button" as button;
                    import "components/button_group" as group;
                    fn state_schema(){{#{{fields:#{{count:#{{schema:#{{type:"integer"}},"default":#{{type:"integer",value:0}}}}}}}}}}
                    fn changed(ctx,value){{ctx.set_state("count",ctx.get_state("count")+1);}}
                    fn view(ctx){{column([
                        group::ButtonGroup(#{{label:"Actions",orientation:"{}",buttons:[
                            button::Button(#{{text:"Launch",variant:"secondary",on_click:Fn("changed")}}),
                            button::Button(#{{text:"Review",variant:"secondary",on_click:Fn("changed")}})]}}),
                        text(`${{ctx.get_state("count")}}`).accessibility_role("status")
                    ]).with_style(style().padding(px(20)).items_start())}}"#,
                    if vertical { "vertical" } else { "horizontal" }
                );
                let overrides = ThemeTokenOverrides {
                    environment_defaults: [("density".to_owned(), density.to_owned())].into(),
                    ..Default::default()
                };
                let (mut visual, view) = Mount::components(script)
                    .theme(theme)
                    .motion(MotionPreference::None)
                    .with(move |view| view.theme_token_overrides(overrides))
                    .id("joined-button-geometry")
                    .mount(cx);
                settle(&mut visual, Step::Frame, 2);
                let first = bounds(&mut visual, &view, "button", "Launch");
                let last = bounds(&mut visual, &view, "button", "Review");
                let radius = if name.starts_with("clean") { 6.0 } else { 12.0 };
                for (target, expected) in [
                    (
                        first,
                        if vertical {
                            [radius, radius, 0.0, 0.0]
                        } else {
                            [radius, 0.0, 0.0, radius]
                        },
                    ),
                    (
                        last,
                        if vertical {
                            [0.0, 0.0, radius, radius]
                        } else {
                            [0.0, radius, radius, 0.0]
                        },
                    ),
                ] {
                    assert!(
                        painted_quads(&mut visual)
                            .iter()
                            .any(|q| (f64::from(q.bounds.x) - target.x).abs() < 0.5
                                && (f64::from(q.bounds.y) - target.y).abs() < 0.5
                                && (f64::from(q.bounds.width) - target.width).abs() < 0.5
                                && q.corner_radii == expected),
                        "{name}/{density}/{vertical}: joined outer corners"
                    );
                }
                visual.update(|window, cx| view.focus(window, cx).unwrap());
                visual.simulate_keystrokes("tab");
                settle(&mut visual, Step::Frame, 2);
                assert!(
                    painted_quads(&mut visual)
                        .iter()
                        .any(|q| q.border_widths == [2.0; 4]
                            && (f64::from(q.bounds.x) - first.x).abs() < 0.5
                            && (f64::from(q.bounds.width) - first.width).abs() < 0.5
                            && q.mask.x <= q.bounds.x
                            && q.mask.right() >= q.bounds.right()),
                    "{name}: focused joined button is not clipped"
                );
                visual.simulate_keystrokes("space");
                settle(&mut visual, Step::Frame, 2);
                bounds(&mut visual, &view, "status", "1");
                assert!(last_error(&mut visual, &view).is_none());
            }
        }
    }
}
use gpui::TestAppContext;
use gpui_rhai::{MotionPreference, ThemeTokenOverrides};

#[gpui::test]
fn toggle_groups_keep_track_geometry_stable_through_selection_and_roving(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let clean = name.starts_with("clean");
        for density in ["comfortable", "compact"] {
            for (index, size) in ["sm", "md", "lg"].into_iter().enumerate() {
                for vertical in [false, true] {
                    let option = match (clean, density) {
                        (false, "compact") => [22.0, 26.0, 32.0],
                        (true, "compact") => [16.0, 20.0, 24.0],
                        (false, _) => [26.0, 30.0, 36.0],
                        (true, _) => [20.0, 24.0, 28.0],
                    }[index];
                    let pad = if clean || density == "compact" {
                        2.0
                    } else {
                        [2.0, 3.0, 4.0][index]
                    };
                    let orientation = if vertical { "vertical" } else { "horizontal" };
                    let script = format!(
                        r#"import "components/toggle_group" as group;
                        fn state_schema(){{#{{fields:#{{values:#{{schema:#{{type:"array",items:#{{type:"string"}}}},"default":#{{type:"array",value:[]}}}}}}}}}}
                        fn changed(ctx,values){{ctx.set_state("values",values);}}
                        fn view(ctx){{let values=ctx.get_state("values");column([
                            group::ToggleGroup(#{{key:"group",label:"View",size:"{size}",orientation:"{orientation}",
                                allow_empty:false,values:values,on_change:Fn("changed"),
                                items:[#{{value:"a",label:"Alpha"}},#{{value:"b",label:"Beta"}},#{{value:"c",label:"Locked",disabled:true}}]}}),
                            text(if values.len==0{{"none"}}else{{values[0]}}).accessibility_role("status")
                        ]).with_style(style().items_start().padding(px(20)))}}"#
                    );
                    let overrides = ThemeTokenOverrides {
                        environment_defaults: [("density".to_owned(), density.to_owned())].into(),
                        ..Default::default()
                    };
                    let (mut visual, view) = Mount::components(script)
                        .theme(theme)
                        .motion(MotionPreference::None)
                        .with(move |view| view.theme_token_overrides(overrides))
                        .id("preset-toggle-group")
                        .mount(cx);
                    settle(&mut visual, Step::Frame, 2);
                    let root = bounds(&mut visual, &view, "group", "View");
                    let alpha = bounds(&mut visual, &view, "button", "Alpha");
                    let beta = bounds(&mut visual, &view, "button", "Beta");
                    let expected = option * if vertical { 3.0 } else { 1.0 } + 2.0 * pad;
                    assert!(
                        (root.height - expected).abs() < 0.5 && (alpha.height - option).abs() < 0.5,
                        "{name}/{density}/{size}/{orientation}: {root:?}, {alpha:?}"
                    );
                    assert!((alpha.y - root.y - pad).abs() < 0.5, "track inset");
                    click_center(&mut visual, beta);
                    settle(&mut visual, Step::Frame, 2);
                    bounds(&mut visual, &view, "status", "b");
                    assert_eq!(
                        snapshot(&mut visual, &view)
                            .find_by_role_and_name("button", "Beta")
                            .next()
                            .unwrap()
                            .pressed,
                        Some(true)
                    );
                    let selected = bounds(&mut visual, &view, "group", "View");
                    assert_eq!(
                        (root.width, root.height),
                        (selected.width, selected.height),
                        "selection weight must not reflow the group"
                    );
                    visual.simulate_keystrokes(if vertical { "down" } else { "right" });
                    visual.simulate_keystrokes("space");
                    settle(&mut visual, Step::Frame, 2);
                    bounds(&mut visual, &view, "status", "a");
                    let alpha = bounds(&mut visual, &view, "button", "Alpha");
                    assert!(
                        painted_quads(&mut visual)
                            .iter()
                            .any(|q| q.border_widths == [2.0; 4]
                                && (f64::from(q.bounds.x) - alpha.x).abs() < 0.5
                                && (f64::from(q.bounds.width) - alpha.width).abs() < 0.5),
                        "{name}/{size}: inset ring follows the actual roving item"
                    );
                    assert!(last_error(&mut visual, &view).is_none());
                }
            }
        }
    }
}
