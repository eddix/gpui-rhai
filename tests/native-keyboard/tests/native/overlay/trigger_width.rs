//! An overlay panel's width against its trigger: `min_trigger_width` keeps the
//! panel at least as wide as the trigger and lets wider content widen it. An
//! `invisible()` node keeps its place in layout but leaves the accessibility tree.

use gpui::TestAppContext;
use gpui_rhai::*;

use crate::support::{Mount, last_error, node_by_test_id, snapshot};

fn source(panel: &str, policy: &str) -> String {
    format!(
        r#"fn noop(ctx,payload){{()}}
fn view(ctx){{
    column([
        overlay(row([text("T")]).test_id("trigger").accessibility_role("group")
                .accessibility_label("trigger").with_style(style().width(px(120))),
            column([text("{panel}").with_style(style().whitespace_nowrap()),
                text("probe label").with_style(style().invisible())])
                .test_id("panel").accessibility_role("group").accessibility_label("panel"),
            #{{id:"probe",kind:"popover",open:true,placement:"bottom",align:"start",{policy}}})
            .on_open_change(Fn("noop")),
    ]).with_style(style().width(px(600)).height(px(400)).items_start())
}}"#
    )
}

#[gpui::test]
fn a_min_trigger_width_panel_is_at_least_the_trigger_and_grows_with_content(
    cx: &mut TestAppContext,
) {
    for (panel, wide) in [
        ("x", false),
        ("a panel line much wider than its trigger", true),
    ] {
        let (mut visual, view) = Mount::new(source(panel, "min_trigger_width:true"))
            .motion(MotionPreference::None)
            .id("width")
            .mount(cx);
        visual.run_until_parked();
        assert_eq!(last_error(&mut visual, &view), None);
        let trigger = node_by_test_id(&mut visual, &view, "trigger")
            .geometry
            .unwrap()
            .visual;
        let panel_box = node_by_test_id(&mut visual, &view, "panel")
            .geometry
            .unwrap()
            .visual;
        if wide {
            assert!(panel_box.width > trigger.width + 100.0, "{panel_box:?}");
        } else {
            assert!(
                (panel_box.width - trigger.width).abs() < 0.5,
                "{panel_box:?}"
            );
        }
        assert!(
            snapshot(&mut visual, &view)
                .nodes()
                .all(|node| node.name != "probe label"),
            "an invisible node is not in the accessibility tree"
        );
    }
}

struct Blank;

impl gpui::Render for Blank {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::div()
    }
}

#[gpui::test]
fn match_and_min_trigger_width_exclude_each_other(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let prepared = Mount::new(source(
        "x",
        "match_trigger_width:true,min_trigger_width:true",
    ))
    .id("both")
    .prepare()
    .unwrap();
    let host = cx.update(|cx| ScriptViewHost::new("both-host", cx).unwrap());
    let mut error = None;
    cx.add_window(|window, cx| {
        error = prepared
            .mount(ScriptViewConfig::new("both"), host.clone(), window, cx)
            .err()
            .map(|error| error.to_string());
        Blank
    });
    let error = error.expect("mounting fails");
    assert!(error.contains("exclude each other"), "{error}");
}
