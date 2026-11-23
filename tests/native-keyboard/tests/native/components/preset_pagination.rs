//! Pagination uses its own number/step geometry without losing controlled proposals.

use crate::support::{
    Mount, Step, bounds, click_center, filled, last_error, nodes, settle, text_of,
};
use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{AuditRules, GeometryBounds, MotionPreference, ScriptViewHandle};
use gpui_rhai_registry::{AR_LOCALE, EN_LOCALE};

#[gpui::test]
fn pagination_info_stays_together_and_wrapped_controls_have_their_own_row_gap(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for (rtl, scale) in [(false, 100_u16), (true, 100), (false, 200), (true, 200)] {
            let source = r#"
                import "components/pagination" as pagination;
                fn init(ctx){ctx.set_locale("__LOCALE__");}
                fn view(ctx){pagination::Pagination(#{key:"pages",label:"Pages",total_items:1283,
                    current_page:3,page_size:10,page_size_options:[10,20]})
                    .with_style(style().width(relative(1.0))).env(#{scale:"__SCALE__"})}
            "#
            .replace("__LOCALE__", if rtl { "ar" } else { "en" })
            .replace("__SCALE__", &scale.to_string());
            let (mut visual, view) = Mount::components(source)
                .theme(theme)
                .locale("en.rhai", EN_LOCALE)
                .locale("ar.rhai", AR_LOCALE)
                .mount(cx);
            settle(&mut visual, Step::Frame, 4);
            visual.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
            let factor = f64::from(scale) / 100.0;
            let native_factor = f32::from(scale) / 100.0;
            for width in [1100.0, 400.0, 1100.0] {
                visual
                    .simulate_resize(gpui::size(gpui::px(width * native_factor), gpui::px(600.0)));
                settle(&mut visual, Step::Frame, 4);
                let elements = nodes(&mut visual, &view);
                let summary = elements
                    .iter()
                    .find(|node| node.role == "text" && node.name.contains('–'))
                    .unwrap();
                let summary = summary.geometry.as_ref().unwrap().visual;
                let picker = elements
                    .iter()
                    .find(|node| node.role == "combobox")
                    .unwrap()
                    .geometry
                    .as_ref()
                    .unwrap()
                    .visual;
                let first = elements
                    .iter()
                    .find(|node| node.role == "button" && node.name == "1")
                    .unwrap()
                    .geometry
                    .as_ref()
                    .unwrap()
                    .visual;
                let height = if name.starts_with("clean") {
                    28.0
                } else {
                    32.0
                } * factor;
                // The size and range are one info group, before the navigation group.
                assert!(
                    (summary.y + summary.height / 2.0 - picker.y - picker.height / 2.0).abs() < 0.5
                );
                assert!(if rtl {
                    summary.x + summary.width < picker.x
                } else {
                    summary.x > picker.x + picker.width
                });
                if width < 500.0 {
                    assert!(
                        (first.y - picker.y - height - 8.0 * factor).abs() < 0.5,
                        "{name}/{rtl}/{scale}: wrap {picker:?} -> {first:?}"
                    );
                } else {
                    assert!((first.y - picker.y).abs() < 0.5);
                }
                assert_eq!(last_error(&mut visual, &view), None);
            }
            assert!(
                visual
                    .update(|_, cx| view.take_performance_snapshot(cx).unwrap())
                    .timings
                    .is_empty()
            );
        }
    }
}

const MAIN: &str = r#"
import "components/pagination" as pagination;
fn state_schema(){#{fields:#{
    page:#{schema:#{type:"integer"},"default":#{type:"integer",value:3}},
    size:#{schema:#{type:"integer"},"default":#{type:"integer",value:10}},
    accept:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
    log:#{schema:#{type:"string"},"default":#{type:"string",value:""}}
}}}
fn init(ctx){ctx.set_locale("__LOCALE__");}
fn accept(ctx,payload){ctx.set_state("accept",true);}
fn changed(ctx,payload){ctx.set_state("log",`${payload.current_page}/${payload.page_size}`);
    if ctx.get_state("accept"){ctx.set_state("page",payload.current_page);ctx.set_state("size",payload.page_size);}}
fn view(ctx){column([
    pagination::Pagination(#{key:"pager",label:"Pages",total_items:1000,current_page:ctx.get_state("page"),
        page_size:ctx.get_state("size"),show_summary:false,page_size_options:[10,20],on_change:Fn("changed")}),
    text("Accept proposals").accessibility_role("button").accessibility_label("Accept proposals").on_click(Fn("accept")),
    text(ctx.get_state("log")).test_id("log")
]).with_style(style().width(px(1200)).padding(px(20)).gap(px(12)))
    .env(#{scale:"__SCALE__",density:"__DENSITY__"})}
"#;

fn mount(
    cx: &mut TestAppContext,
    theme: &str,
    scale: i32,
    density: &str,
    rtl: bool,
    sheet: &str,
) -> (VisualTestContext, ScriptViewHandle) {
    let main = MAIN
        .replace("__LOCALE__", if rtl { "ar" } else { "en" })
        .replace("__SCALE__", &scale.to_string())
        .replace("__DENSITY__", density);
    let sheet = sheet.to_owned();
    let (mut visual, view) = Mount::components(main)
        .theme(theme)
        .motion(MotionPreference::None)
        .locale("en.rhai", EN_LOCALE)
        .locale("ar.rhai", AR_LOCALE)
        .with(move |view| view.component_styles(sheet))
        .id("preset-pages")
        .mount(cx);
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(last_error(&mut visual, &view), None);
    (visual, view)
}

fn current(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    nodes(visual, view)
        .into_iter()
        .find(|node| node.current.as_deref() == Some("page"))
        .unwrap()
        .name
}

fn number_boxes(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<GeometryBounds> {
    nodes(visual, view)
        .into_iter()
        .filter(|node| node.role == "button" && node.name.parse::<u64>().is_ok())
        .map(|node| node.geometry.unwrap().visual)
        .collect()
}

#[gpui::test]
fn pagination_numbers_reserve_longest_width_and_commit_only_accepted_proposals(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for scale in [100, 200] {
            for density in ["comfortable", "compact"] {
                for rtl in [false, true] {
                    let (mut visual, view) =
                        mount(cx, theme, scale, density, rtl, "fn component_styles(){#{}}");
                    let factor = f64::from(scale) / 100.0;
                    let nominal = if name.starts_with("clean") {
                        28.0_f64
                    } else {
                        32.0_f64
                    };
                    let height = (nominal - if density == "compact" { 4.0 } else { 0.0 }) * factor;
                    let boxes = number_boxes(&mut visual, &view);
                    assert!(boxes.len() > 3);
                    for item in &boxes {
                        assert!(
                            (item.height - height).abs() < 0.5,
                            "{name}/{scale}/{density}: {item:?}"
                        );
                        assert!(
                            (item.width - boxes[0].width).abs() < 0.5,
                            "longest-number reservation must size every page: {boxes:?}"
                        );
                    }
                    let picker = nodes(&mut visual, &view)
                        .into_iter()
                        .find(|node| node.role == "combobox")
                        .unwrap()
                        .geometry
                        .unwrap()
                        .visual;
                    assert!(
                        (picker.height - height).abs() < 0.5,
                        "{name}: picker {picker:?} vs {height}"
                    );
                    let fourth = bounds(&mut visual, &view, "button", "4");
                    click_center(&mut visual, fourth);
                    settle(&mut visual, Step::Frame, 2);
                    assert_eq!(text_of(&mut visual, &view, "log"), "4/10");
                    assert_eq!(
                        current(&mut visual, &view),
                        "3",
                        "a rejected proposal must not move selection"
                    );
                    let accept = bounds(&mut visual, &view, "button", "Accept proposals");
                    click_center(&mut visual, accept);
                    settle(&mut visual, Step::Frame, 2);
                    let fourth = bounds(&mut visual, &view, "button", "4");
                    click_center(&mut visual, fourth);
                    settle(&mut visual, Step::Frame, 2);
                    assert_eq!(current(&mut visual, &view), "4");
                    let after = number_boxes(&mut visual, &view);
                    assert!(
                        after
                            .iter()
                            .all(|item| (item.width - boxes[0].width).abs() < 0.5)
                    );
                    let rules = AuditRules::from_ids(["low-contrast-text"]).unwrap();
                    // The standalone page-size label is authored muted in Clean. Inspect
                    // the selected number itself rather than waiving that separate role.
                    let selected = nodes(&mut visual, &view)
                        .into_iter()
                        .find(|node| node.role == "text" && node.name == "4")
                        .unwrap();
                    let findings =
                        visual.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
                    assert!(
                        !findings
                            .iter()
                            .any(|finding| finding.node == Some(selected.id)),
                        "{findings:?}"
                    );
                    assert_eq!(last_error(&mut visual, &view), None);
                }
            }
        }
    }
}

#[gpui::test]
fn a_common_pagination_rule_stays_above_selected_preset_paint(cx: &mut TestAppContext) {
    for &(_, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual, view) = mount(
            cx,
            theme,
            100,
            "comfortable",
            false,
            r#"fn component_styles(){#{"components/pagination":#{page:style().background(rgba(0x123456ff))}}}"#,
        );
        let selected = bounds(&mut visual, &view, "button", "3");
        assert!(
            filled(&mut visual, 0x1234_56ff)
                .iter()
                .any(|quad| (f64::from(quad.bounds.x) - selected.x).abs() < 0.5
                    && (f64::from(quad.bounds.y) - selected.y).abs() < 0.5)
        );
    }
}

fn pick_page_size(visual: &mut VisualTestContext, view: &ScriptViewHandle, value: &str) {
    let picker = bounds(visual, view, "combobox", "Items per page");
    click_center(visual, picker);
    settle(visual, Step::Frame, 2);
    let option = bounds(visual, view, "option", value);
    click_center(visual, option);
    settle(visual, Step::Frame, 2);
    assert_eq!(last_error(visual, view), None);
}

#[gpui::test]
fn page_size_popup_commits_an_atomic_reset_and_keeps_step_boundaries(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual, view) = mount(
            cx,
            theme,
            100,
            "comfortable",
            false,
            "fn component_styles(){#{}}",
        );
        pick_page_size(&mut visual, &view, "20");
        assert_eq!(text_of(&mut visual, &view, "log"), "1/20");
        assert_eq!(
            current(&mut visual, &view),
            "3",
            "{name}: rejected size proposal"
        );
        let accept = bounds(&mut visual, &view, "button", "Accept proposals");
        click_center(&mut visual, accept);
        settle(&mut visual, Step::Frame, 2);
        pick_page_size(&mut visual, &view, "20");
        assert_eq!(current(&mut visual, &view), "1");
        let previous = nodes(&mut visual, &view)
            .into_iter()
            .find(|node| node.role == "button" && node.name == "Previous page")
            .unwrap();
        assert!(previous.disabled, "{name}: first-page boundary");
        let next = bounds(&mut visual, &view, "button", "Next page");
        click_center(&mut visual, next);
        settle(&mut visual, Step::Frame, 2);
        assert_eq!(current(&mut visual, &view), "2");
        assert_eq!(text_of(&mut visual, &view, "log"), "2/20");
        let previous = bounds(&mut visual, &view, "button", "Previous page");
        click_center(&mut visual, previous);
        settle(&mut visual, Step::Frame, 2);
        assert_eq!(current(&mut visual, &view), "1");
        assert_eq!(last_error(&mut visual, &view), None);
    }
}
