//! Native date range interactions, presentation geometry and controlled draft ownership.
use gpui::{ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, point, px, size};
use gpui_rhai::{MotionPreference, ScriptViewHandle, UiValue};
use gpui_rhai_registry::{BUNDLED_DESIGN_PRESET_SOURCES, EN_LOCALE};

use crate::support::{
    Mount, Step, bounds, center, click_center, last_error, settle, snapshot, text_of,
};

const APP: &str = r#"
import "components/date_range_picker" as picker;
fn state_schema(){#{fields:#{
    result:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    changes:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    revision:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
}}}
fn changed(ctx,v){ctx.set_state("result",v);ctx.set_state("changes",ctx.get_state("changes")+1);}
fn revise(ctx,event){ctx.set_state("revision",ctx.get_state("revision")+1);handled()}
fn view(ctx){
    let result=ctx.get_state("result");
    let r=if result==(){#{start:"2024-02-29",end:"2024-03-02"}}else{result.range};
    let mode=if result==(){"off"}else{result.compare_mode};
    let comparison=if result==()||result.comparison==(){"off"}else{result.comparison.start+"/"+result.comparison.end};
    column([
        picker::DateRangePicker(#{key:"period",label:"Reporting period",value:r,compare_mode:mode,
            today:"2024-03-31",revision:ctx.get_state("revision"),on_change:Fn("changed")}),
        text(`${ctx.get_state("changes")}/${r.start}/${r.end}/${mode}/${comparison}`).test_id("result"),
    ]).with_style(style().items_start().gap(px(12)).padding(px(20))
        .width(relative(1.0)).height(relative(1.0))).on_key_value("f6",Fn("revise"),());
}
"#;

fn ready(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    settle(visual, Step::Tick, 5);
    assert_eq!(last_error(visual, view), None);
}

fn click(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    let target = bounds(visual, view, role, name);
    click_center(visual, target);
    ready(visual, view);
}

fn click_day(visual: &mut VisualTestContext, view: &ScriptViewHandle, date: &str) {
    let date = Some(UiValue::String(date.to_owned()));
    let target = snapshot(visual, view)
        .nodes()
        .find(|n| n.role == "gridcell" && n.value == date)
        .expect("date in visible calendar")
        .geometry
        .unwrap()
        .visual;
    click_center(visual, target);
    ready(visual, view);
}

#[gpui::test]
fn date_range_presets_and_comparison_propose_only_on_apply(cx: &mut TestAppContext) {
    for &(preset, theme) in BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual, view) = Mount::components(APP)
            .theme(theme)
            .locale("en.rhai", EN_LOCALE)
            .motion(MotionPreference::None)
            .mount(cx);
        visual.simulate_resize(size(px(980.), px(720.)));
        ready(&mut visual, &view);
        click(&mut visual, &view, "combobox", "Reporting period");
        let ax = snapshot(&mut visual, &view);
        assert_eq!(
            ax.nodes().filter(|n| n.role == "grid").count(),
            2,
            "{preset}"
        );
        click(&mut visual, &view, "button", "Previous year");
        assert_eq!(
            text_of(&mut visual, &view, "result"),
            "0/2024-02-29/2024-03-02/off/off"
        );
        click(&mut visual, &view, "button", "Apply");
        assert_eq!(
            text_of(&mut visual, &view, "result"),
            "1/2024-02-29/2024-03-02/year/2023-02-28/2023-03-02",
            "{preset}",
        );
        click(&mut visual, &view, "combobox", "Reporting period");
        click(&mut visual, &view, "option", "Last 7 days");
        click(&mut visual, &view, "button", "Previous period");
        let ax = snapshot(&mut visual, &view);
        assert!(ax.nodes().any(|n| n.role == "gridcell"
            && n.value == Some(UiValue::String("2024-03-22".into()))
            && n.description == "Comparison period"));
        assert!(text_of(&mut visual, &view, "result").starts_with("1/"));
        click(&mut visual, &view, "button", "Apply");
        assert_eq!(
            text_of(&mut visual, &view, "result"),
            "2/2024-03-25/2024-03-31/previous/2024-03-18/2024-03-24",
        );
    }
}

#[gpui::test]
fn date_range_narrow_overlay_preserves_draft_and_cancel_never_commits(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(APP)
        .locale("en.rhai", EN_LOCALE)
        .motion(MotionPreference::None)
        .mount(cx);
    visual.update(|w, _| w.activate_window());
    visual.simulate_resize(size(px(980.), px(720.)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "combobox", "Reporting period");
    click_day(&mut visual, &view, "2024-02-25");
    let ax = snapshot(&mut visual, &view);
    assert!(
        ax.nodes()
            .any(|n| n.role == "button" && n.name == "Apply" && n.disabled)
    );
    visual.simulate_resize(size(px(440.), px(650.)));
    ready(&mut visual, &view);
    let panel = bounds(&mut visual, &view, "dialog", "Reporting period");
    assert!(
        panel.width > 200.0 && panel.x >= 0.0 && panel.x + panel.width <= 440.5,
        "{panel:?}"
    );
    assert_eq!(
        snapshot(&mut visual, &view)
            .nodes()
            .filter(|n| n.role == "grid")
            .count(),
        1
    );
    click_day(&mut visual, &view, "2024-02-21");
    click(&mut visual, &view, "button", "Apply");
    assert_eq!(
        text_of(&mut visual, &view, "result"),
        "1/2024-02-21/2024-02-25/off/off"
    );

    click(&mut visual, &view, "combobox", "Reporting period");
    click_day(&mut visual, &view, "2024-02-04");
    let before_escape = visual.update(|w, cx| format!("{:?}", w.focused(cx)));
    visual.simulate_keystrokes("escape");
    ready(&mut visual, &view);
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "dialog"),
        "Escape must dismiss with focus {before_escape}",
    );
    assert_eq!(
        text_of(&mut visual, &view, "result"),
        "1/2024-02-21/2024-02-25/off/off"
    );
    click(&mut visual, &view, "combobox", "Reporting period");
    click_day(&mut visual, &view, "2024-02-04");
    visual.simulate_keystrokes("right enter");
    ready(&mut visual, &view);
    let ax = snapshot(&mut visual, &view);
    assert!(ax.nodes().any(|n| n.role == "gridcell"
        && n.value == Some(UiValue::String("2024-02-05".into()))
        && n.name.ends_with(", End")
        && n.selected == Some(true)));
    click(&mut visual, &view, "button", "Cancel");
    assert_eq!(
        text_of(&mut visual, &view, "result"),
        "1/2024-02-21/2024-02-25/off/off"
    );

    // A Host revision can withdraw a partial edit without changing the
    // committed value. Apply must then propose that value, not the old draft.
    click(&mut visual, &view, "combobox", "Reporting period");
    click_day(&mut visual, &view, "2024-02-04");
    visual.simulate_keystrokes("f6");
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Apply");
    assert_eq!(
        text_of(&mut visual, &view, "result"),
        "2/2024-02-21/2024-02-25/off/off"
    );
}

#[gpui::test]
fn date_range_short_viewport_scrolls_to_apply(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(APP)
        .locale("en.rhai", EN_LOCALE)
        .motion(MotionPreference::None)
        .mount(cx);
    visual.simulate_resize(size(px(440.), px(310.)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "combobox", "Reporting period");
    let panel = bounds(&mut visual, &view, "dialog", "Reporting period");
    let before = bounds(&mut visual, &view, "button", "Apply");
    assert!(
        panel.height > 220.0 && panel.y + panel.height <= 310.5,
        "{panel:?}"
    );
    assert!(before.y >= panel.y + panel.height, "{before:?}");
    visual.simulate_event(ScrollEvent {
        position: center(panel),
        delta: ScrollDelta::Pixels(point(px(0.), px(-600.))),
        ..Default::default()
    });
    ready(&mut visual, &view);
    let after = bounds(&mut visual, &view, "button", "Apply");
    assert!(
        after.y >= panel.y && after.y + after.height <= 310.5,
        "{after:?}"
    );
    click(&mut visual, &view, "button", "Apply");
    assert!(text_of(&mut visual, &view, "result").starts_with("1/"));
}
