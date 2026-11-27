//! Controlled number focus, compact presentation and disabled popup lifetime.

use crate::support::{
    Mount, Step, bounds, by_role, click_center, dispatch, last_error, nodes, painted_quads, settle,
    text_of,
};
use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{MotionPreference, ScriptViewHandle};
use gpui_rhai_registry::{AR_LOCALE, EN_LOCALE};

#[gpui::test]
fn pagination_automatically_collapses_one_number_cohort_and_keeps_step_identity(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for (scale, rtl) in [(100_u16, false), (200, false), (100, true), (200, true)] {
            for density in ["comfortable", "compact"] {
                let source = r#"
                    import "components/pagination" as pagination;
                    fn init(ctx){ctx.set_locale("__LOCALE__");}
                    fn state_schema(){#{fields:#{page:#{schema:#{type:"integer"},"default":#{type:"integer",value:3}}}}}
                    fn change(ctx,value){ctx.set_state("page",value.current_page);}
                    fn view(ctx){column([
                        pagination::Pagination(#{key:"pages",label:"Pages",current_page:ctx.get_state("page"),
                            page_size:10,total_items:1000,show_summary:false,on_change:Fn("change")})
                            .with_style(style().width(relative(1.0))),
                        text(`${ctx.get_state("page")}`).test_id("page")
                    ]).with_style(style().width(relative(1.0))).env(#{scale:"__SCALE__",density:"__DENSITY__"})}
                "#.replace("__SCALE__", &scale.to_string()).replace("__DENSITY__", density)
                    .replace("__LOCALE__",if rtl{"ar"}else{"en"});
                let (mut visual, view) = Mount::components(source)
                    .theme(theme)
                    .locale("en.rhai", EN_LOCALE)
                    .locale("ar.rhai", AR_LOCALE)
                    .motion(MotionPreference::None)
                    .config(|config| config.host_sets_width(true))
                    .mount(cx);
                visual.update(|window, _| window.activate_window());
                let factor = f32::from(scale) / 100.0;
                for (width, numbers) in [
                    (1000.0, true),
                    (120.0, false),
                    (1000.0, true),
                    (120.0, false),
                ] {
                    visual.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
                    visual.simulate_resize(gpui::size(gpui::px(width * factor), gpui::px(300.0)));
                    settle(&mut visual, Step::Frame, 4);
                    let targets = nodes(&mut visual, &view);
                    assert_eq!(
                        targets
                            .iter()
                            .any(|node| node.role == "button" && node.name.parse::<i64>().is_ok()),
                        numbers,
                        "{name}/{scale}/{density}/{rtl}/{width}"
                    );
                    assert_eq!(
                        targets
                            .iter()
                            .filter(
                                |node| node.role == "button" && node.name.parse::<i64>().is_err()
                            )
                            .count(),
                        2
                    );
                    assert!(
                        visual
                            .update(|_, cx| view.take_performance_snapshot(cx).unwrap())
                            .timings
                            .is_empty()
                    );
                    assert_eq!(last_error(&mut visual, &view), None);
                }
                click(
                    &mut visual,
                    &view,
                    "button",
                    if rtl {
                        "الصفحة التالية"
                    } else {
                        "Next page"
                    },
                );
                let focus = visual.update(|window, cx| window.focused(cx)).unwrap();
                assert_eq!(text_of(&mut visual, &view, "page"), "4");
                visual.simulate_resize(gpui::size(gpui::px(1000.0 * factor), gpui::px(300.0)));
                settle(&mut visual, Step::Frame, 4);
                assert_eq!(visual.update(|window, cx| window.focused(cx)), Some(focus));
                visual.simulate_keystrokes("enter");
                settle(&mut visual, Step::Frame, 4);
                assert_eq!(text_of(&mut visual, &view, "page"), "5");
                assert_eq!(last_error(&mut visual, &view), None);
            }
        }
    }
}

const MAIN: &str = r#"
import "components/pagination" as pagination;
fn state_schema(){#{fields:#{
    page:#{schema:#{type:"integer"},"default":#{type:"integer",value:3}},
    size:#{schema:#{type:"integer"},"default":#{type:"integer",value:10}},
    disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
    compact:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
    sizes:#{schema:#{type:"bool"},"default":#{type:"bool",value:true}},
    accept:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
    proposed:#{schema:#{type:"integer"},"default":#{type:"integer",value:3}},
    calls:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
}}}
fn change(ctx,value){ctx.set_state("proposed",value.current_page);ctx.set_state("calls",ctx.get_state("calls")+1);
    if ctx.get_state("accept"){ctx.set_state("page",value.current_page);ctx.set_state("size",value.page_size);}}
fn disabled(ctx,event){ctx.set_state("disabled",!ctx.get_state("disabled"));}
fn compact(ctx,event){ctx.set_state("compact",!ctx.get_state("compact"));}
fn sizes(ctx,event){ctx.set_state("sizes",!ctx.get_state("sizes"));}
fn accept(ctx,event){ctx.set_state("accept",true);}
fn confirm(ctx,event){ctx.set_state("page",ctx.get_state("proposed"));}
fn noop(ctx,event){}
fn control(name,handler){text(name).accessibility_role("button").accessibility_label(name).on_click(handler)}
fn view(ctx){column([
    pagination::Pagination(#{key:"pager",label:"Pages",total_items:1000,page_size:ctx.get_state("size"),
        current_page:ctx.get_state("page"),compact:ctx.get_state("compact"),disabled:ctx.get_state("disabled"),
        page_size_options:if ctx.get_state("sizes"){[10,20]}else{[]},on_change:Fn("change")}),
    row([control("Toggle disabled",Fn("disabled")),control("Toggle compact",Fn("compact")),
        control("Toggle sizes",Fn("sizes")),control("Accept",Fn("accept")),control("Confirm",Fn("confirm")),
        control("Outside",Fn("noop"))]).with_style(style().gap(px(20))),
    text(`${ctx.get_state("page")}/${ctx.get_state("size")}/${ctx.get_state("calls")}`).test_id("status")
]).with_style(style().width(px(1100)).padding(px(20)).gap(px(20)))}
"#;

fn mount(cx: &mut TestAppContext, theme: &str) -> (VisualTestContext, ScriptViewHandle) {
    mount_with(cx, Mount::components(MAIN).theme(theme))
}

fn mount_with(cx: &mut TestAppContext, mount: Mount) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = mount
        .locale("en.rhai", EN_LOCALE)
        .motion(MotionPreference::None)
        .mount(cx);
    // Real focus-in/out paths belong to an active window; test windows start inactive.
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(last_error(&mut visual, &view), None);
    (visual, view)
}

fn colored_alphas(visual: &mut VisualTestContext, color: u32) -> Vec<f32> {
    let expected: gpui::Hsla = gpui::rgba(color).into();
    painted_quads(visual)
        .into_iter()
        .filter_map(|quad| quad.background.as_solid())
        .filter(|color| {
            (color.h - expected.h).abs() < 0.0001
                && (color.s - expected.s).abs() < 0.0001
                && (color.l - expected.l).abs() < 0.0001
        })
        .map(|color| color.a)
        .collect()
}

#[gpui::test]
fn disabled_controls_dim_once_but_do_not_dim_the_range(cx: &mut TestAppContext) {
    for &(_, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual, view) = mount_with(cx, Mount::components(MAIN).theme(theme).with(|view|
            view.component_styles(r#"fn component_styles(){#{"components/pagination":#{
                page:style().background(rgba(0xff00ffff)),summary:style().background(rgba(0xffff00ff))}}}"#)));
        for disabled in [false, true, false] {
            let pages = colored_alphas(&mut visual, 0xff00_ffff);
            let range = colored_alphas(&mut visual, 0xffff_00ff);
            assert!(pages.len() > 3);
            assert_eq!(range.len(), 1);
            assert!(
                pages
                    .iter()
                    .all(|alpha| (*alpha - if disabled { 0.5 } else { 1.0 }).abs() < 0.0001),
                "{pages:?}"
            );
            assert!((range[0] - 1.0).abs() < 0.0001);
            host_click(&mut visual, &view, "Toggle disabled");
        }
    }
}

#[gpui::test]
fn live_design_switches_preserve_number_focus_and_the_open_size_picker(cx: &mut TestAppContext) {
    let (mut visual, view) = mount_with(
        cx,
        Mount::components(MAIN)
            .theme(gpui_rhai_registry::MODERN_LIGHT_THEME)
            .with(|view| {
                view.theme_sources(
                    gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES
                        .iter()
                        .filter(|(name, _)| *name != "modern_light.rhai")
                        .map(|(name, source)| ((*name).to_owned(), (*source).to_owned())),
                )
            }),
    );
    click(&mut visual, &view, "button", "4");
    let focus = visual.update(|window, cx| window.focused(cx)).unwrap();
    for (family, name) in [
        ("Clean", "Dark"),
        ("Modern", "Dark"),
        ("Clean", "Light"),
        ("Modern", "Light"),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, name, cx))
            .unwrap();
        settle(&mut visual, Step::Frame, 4);
        assert_eq!(text_of(&mut visual, &view, "status"), "3/10/1");
        assert_eq!(
            visual.update(|window, cx| window.focused(cx)),
            Some(focus.clone())
        );
    }
    host_click(&mut visual, &view, "Confirm");
    assert_eq!(text_of(&mut visual, &view, "status"), "4/10/1");
    assert_eq!(visual.update(|window, cx| window.focused(cx)), Some(focus));
    visual.simulate_keystrokes("shift-tab");
    visual.simulate_keystrokes("enter");
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(text_of(&mut visual, &view, "status"), "4/10/2");
    click(&mut visual, &view, "combobox", "Items per page");
    for (family, name) in [
        ("Clean", "Dark"),
        ("Modern", "Dark"),
        ("Clean", "Light"),
        ("Modern", "Light"),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, name, cx))
            .unwrap();
        settle(&mut visual, Step::Frame, 4);
        assert!(
            nodes(&mut visual, &view)
                .iter()
                .any(|node| node.role == "option" && node.name == "20")
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
    host_click(&mut visual, &view, "Accept");
    click(&mut visual, &view, "option", "20");
    assert_eq!(text_of(&mut visual, &view, "status"), "1/20/3");
}

fn host_click(visual: &mut VisualTestContext, view: &ScriptViewHandle, name: &str) {
    dispatch(visual, view, by_role("button", name), "click").unwrap();
    settle(visual, Step::Frame, 4);
    assert_eq!(last_error(visual, view), None);
}

fn click(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    let target = bounds(visual, view, role, name);
    click_center(visual, target);
    settle(visual, Step::Frame, 4);
    assert_eq!(last_error(visual, view), None);
}

#[gpui::test]
fn accepted_numbers_keep_focus_in_the_new_window_and_steps_do_not_steal_it(
    cx: &mut TestAppContext,
) {
    for &(_, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual, view) = mount(cx, theme);
        host_click(&mut visual, &view, "Accept");
        click(&mut visual, &view, "button", "5");
        assert_eq!(text_of(&mut visual, &view, "status"), "5/10/1");
        visual.simulate_keystrokes("enter");
        settle(&mut visual, Step::Frame, 4);
        assert_eq!(text_of(&mut visual, &view, "status"), "5/10/1");
        visual.simulate_keystrokes("shift-tab");
        visual.simulate_keystrokes("enter");
        settle(&mut visual, Step::Frame, 4);
        assert_eq!(text_of(&mut visual, &view, "status"), "4/10/2");
        click(&mut visual, &view, "button", "Next page");
        let focus = visual.update(|window, cx| window.focused(cx)).unwrap();
        assert_eq!(text_of(&mut visual, &view, "status"), "5/10/3");
        settle(&mut visual, Step::Frame, 4);
        assert_eq!(visual.update(|window, cx| window.focused(cx)), Some(focus));
        visual.simulate_keystrokes("enter");
        settle(&mut visual, Step::Frame, 4);
        assert_eq!(text_of(&mut visual, &view, "status"), "6/10/4");
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn steps_and_current_page_presses_do_not_redirect_a_later_confirmation(cx: &mut TestAppContext) {
    for &(_, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual, view) = mount(cx, theme);
        click(&mut visual, &view, "button", "4");
        host_click(&mut visual, &view, "Accept");
        click(&mut visual, &view, "button", "Next page");
        assert_eq!(text_of(&mut visual, &view, "status"), "4/10/2");
        visual.simulate_keystrokes("enter");
        settle(&mut visual, Step::Frame, 4);
        assert_eq!(text_of(&mut visual, &view, "status"), "5/10/3");

        let (mut visual, view) = mount(cx, theme);
        click(&mut visual, &view, "button", "4");
        click(&mut visual, &view, "button", "3");
        assert_eq!(text_of(&mut visual, &view, "status"), "3/10/1");
        let focus = visual.update(|window, cx| window.focused(cx)).unwrap();
        host_click(&mut visual, &view, "Confirm");
        assert_eq!(text_of(&mut visual, &view, "status"), "4/10/1");
        assert_eq!(visual.update(|window, cx| window.focused(cx)), Some(focus));
    }
}

#[gpui::test]
fn delayed_acceptance_does_not_steal_focus_after_leaving_the_pager(cx: &mut TestAppContext) {
    for &(_, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual, view) = mount(cx, theme);
        click(&mut visual, &view, "button", "4");
        assert_eq!(text_of(&mut visual, &view, "status"), "3/10/1");
        click(&mut visual, &view, "button", "Outside");
        let outside = visual.update(|window, cx| window.focused(cx)).unwrap();
        host_click(&mut visual, &view, "Confirm");
        assert_eq!(text_of(&mut visual, &view, "status"), "4/10/1");
        assert_eq!(
            visual.update(|window, cx| window.focused(cx)),
            Some(outside)
        );
    }
}

#[gpui::test]
fn disabling_or_removing_sizes_closes_the_popup_without_reopening_old_state(
    cx: &mut TestAppContext,
) {
    for &(_, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for toggle in ["Toggle disabled", "Toggle sizes"] {
            let (mut visual, view) = mount(cx, theme);
            click(&mut visual, &view, "combobox", "Items per page");
            assert!(
                nodes(&mut visual, &view)
                    .iter()
                    .any(|node| node.role == "option")
            );
            host_click(&mut visual, &view, toggle);
            assert!(
                !nodes(&mut visual, &view)
                    .iter()
                    .any(|node| node.role == "option")
            );
            if toggle == "Toggle disabled" {
                let controls = nodes(&mut visual, &view);
                for control in controls.iter().filter(|node| {
                    node.role == "combobox"
                        || node.role == "button"
                            && (node.name.parse::<i64>().is_ok() || node.name.ends_with("page"))
                }) {
                    assert!(control.disabled, "{control:?}");
                }
                assert_eq!(text_of(&mut visual, &view, "status"), "3/10/0");
            }
            host_click(&mut visual, &view, toggle);
            assert!(
                !nodes(&mut visual, &view)
                    .iter()
                    .any(|node| node.role == "option")
            );
            click(&mut visual, &view, "combobox", "Items per page");
            assert!(
                nodes(&mut visual, &view)
                    .iter()
                    .any(|node| node.role == "option")
            );
        }
    }
}

#[gpui::test]
fn compact_pagination_keeps_range_size_and_step_proposals_without_number_targets(
    cx: &mut TestAppContext,
) {
    for &(_, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual, view) = mount(cx, theme);
        host_click(&mut visual, &view, "Accept");
        host_click(&mut visual, &view, "Toggle compact");
        let controls = nodes(&mut visual, &view);
        assert!(
            !controls
                .iter()
                .any(|node| node.role == "button" && node.name.parse::<i64>().is_ok())
        );
        assert!(
            controls
                .iter()
                .any(|node| node.role == "text" && node.name == "21–30 of 1,000")
        );
        click(&mut visual, &view, "button", "Next page");
        assert_eq!(text_of(&mut visual, &view, "status"), "4/10/1");
        host_click(&mut visual, &view, "Toggle disabled");
        click(&mut visual, &view, "button", "Next page");
        assert_eq!(text_of(&mut visual, &view, "status"), "4/10/1");
        host_click(&mut visual, &view, "Toggle disabled");
        host_click(&mut visual, &view, "Toggle compact");
        assert!(
            nodes(&mut visual, &view)
                .iter()
                .any(|node| node.current.as_deref() == Some("page") && node.name == "4")
        );
    }
}
