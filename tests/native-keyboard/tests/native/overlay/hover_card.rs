//! Native input, deadline and overlay ownership across actual frames.
use gpui::{Modifiers, Pixels, Point, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{MotionPreference, ScriptViewHandle};
use std::time::{Duration, Instant};

use crate::support::{
    Mount, Step, bounds, by_test_id, center, click_center, dispatch, last_error, nodes, settle,
    text_of,
};

const PREVIEW: &str = r#"
 import "components/button" as button;
 import "components/hover_card" as hovercard;
 fn state_schema(){#{fields:#{
    hits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    anchor:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    after_hits:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
 }}}
 fn press(ctx,v){ctx.set_state("hits",ctx.get_state("hits")+1);}
 fn click_anchor(ctx,v){ctx.set_state("anchor",ctx.get_state("anchor")+1);}
 fn after(ctx,v){ctx.set_state("after_hits",ctx.get_state("after_hits")+1);}
 fn view(ctx){column([
    hovercard::HoverCard(#{key:"details",label:"Service preview",show_delay_ms:96,hide_delay_ms:48,gap:16.0,
       trigger:button::Button(#{key:"trigger",text:"Service",on_click:Fn("click_anchor")}),
       content:button::Button(#{key:"save",text:"Bookmark",on_click:Fn("press")})}),
    text(ctx.get_state("hits").to_string()).test_id("hits"),
    text(ctx.get_state("anchor").to_string()).test_id("anchor"),
    text(ctx.get_state("after_hits").to_string()).test_id("after-hits"),
    button::Button(#{key:"after",text:"After",on_click:Fn("after")}),
 ]).with_style(style().width(px(380)).items_start().padding(px(24)).gap(px(36)))}
"#;

fn native(cx: &mut TestAppContext, source: &str) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(source)
        .motion(MotionPreference::None)
        .mount(cx);
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::TickRefresh, 3);
    (visual, view)
}
fn preview_open(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> bool {
    nodes(visual, view)
        .into_iter()
        .any(|node| node.name == "Service preview" && node.geometry.is_some())
}

#[gpui::test]
fn touch_is_not_hover_even_when_it_dwells_on_the_anchor(cx: &mut TestAppContext) {
    let (mut visual, view) = native(cx, PREVIEW);
    let position = center(bounds(&mut visual, &view, "button", "Service"));
    for phase in [gpui::TouchPhase::Started, gpui::TouchPhase::Ended] {
        visual.simulate_event(gpui::TouchEvent {
            id: gpui::TouchId(15),
            phase,
            position,
            predicted_position: None,
            force: None,
        });
        settle(&mut visual, Step::TickRefresh, 5);
        assert!(!preview_open(&mut visual, &view));
    }
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn tab_from_preview_resumes_the_page_order_in_both_directions(cx: &mut TestAppContext) {
    let (mut visual, view) = native(cx, PREVIEW);
    visual
        .update(|window, app| view.focus(window, app))
        .unwrap();
    visual.simulate_keystrokes("tab");
    settle(&mut visual, Step::TickRefresh, 5);
    let anchor = visual.update(|w, a| w.focused(a)).unwrap();
    visual.simulate_keystrokes("tab");
    settle(&mut visual, Step::TickRefresh, 2);
    assert_ne!(visual.update(|w, a| w.focused(a)), Some(anchor.clone()));
    visual.simulate_keystrokes("shift-tab");
    settle(&mut visual, Step::TickRefresh, 2);
    assert_eq!(visual.update(|w, a| w.focused(a)), Some(anchor));
    assert!(preview_open(&mut visual, &view));
    visual.simulate_keystrokes("tab tab enter");
    settle(&mut visual, Step::TickRefresh, 5);
    assert_eq!(
        text_of(&mut visual, &view, "after-hits"),
        "1",
        "forward Tab resumes after the opener"
    );
    assert!(
        !preview_open(&mut visual, &view),
        "leaving both focus domains closes the preview"
    );
    assert_eq!(last_error(&mut visual, &view), None);
}
fn hover(visual: &mut VisualTestContext, p: Point<Pixels>) {
    visual.simulate_mouse_move(p, None, Modifiers::none());
    visual.run_until_parked();
}

#[gpui::test]
fn pointer_dwell_crossing_corridor_and_original_click(cx: &mut TestAppContext) {
    let (mut visual, view) = native(cx, PREVIEW);
    let trigger = bounds(&mut visual, &view, "button", "Service");
    hover(&mut visual, center(trigger));
    settle(&mut visual, Step::TickRefresh, 2);
    assert!(
        !preview_open(&mut visual, &view),
        "brief transit does not open"
    );
    hover(&mut visual, point(px(10.), px(240.)));
    settle(&mut visual, Step::TickRefresh, 5);
    assert!(
        !preview_open(&mut visual, &view),
        "leaving cancels the open task"
    );
    hover(&mut visual, center(trigger));
    settle(&mut visual, Step::TickRefresh, 5);
    assert!(preview_open(&mut visual, &view));
    let panel = bounds(&mut visual, &view, "group", "Service preview");
    assert!(panel.y > trigger.y + trigger.height + 8.);
    let between = point(
        px((trigger.x + trigger.width / 2.) as f32),
        px(((trigger.y + trigger.height + panel.y) / 2.) as f32),
    );
    hover(&mut visual, between);
    settle(&mut visual, Step::TickRefresh, 4);
    assert!(
        preview_open(&mut visual, &view),
        "the crossing corridor survives the hide delay"
    );
    hover(&mut visual, point(px(600.), px(320.)));
    settle(&mut visual, Step::TickRefresh, 4);
    assert!(
        !preview_open(&mut visual, &view),
        "sideways travel exits the crossing corridor"
    );
    hover(&mut visual, center(trigger));
    settle(&mut visual, Step::TickRefresh, 5);
    let save = bounds(&mut visual, &view, "button", "Bookmark");
    click_center(&mut visual, save);
    settle(&mut visual, Step::TickRefresh, 4);
    assert_eq!(text_of(&mut visual, &view, "hits"), "1");
    assert!(preview_open(&mut visual, &view));
    hover(&mut visual, point(px(600.), px(320.)));
    settle(&mut visual, Step::TickRefresh, 5);
    // A focused child keeps the interactive panel open, so Escape is an
    // explicit dismissal here rather than pointer exit stealing its focus.
    visual.simulate_keystrokes("escape");
    settle(&mut visual, Step::TickRefresh, 5);
    assert!(!preview_open(&mut visual, &view));
    click_center(&mut visual, trigger);
    settle(&mut visual, Step::TickRefresh, 6);
    assert_eq!(text_of(&mut visual, &view, "anchor"), "1");
    assert!(
        !preview_open(&mut visual, &view),
        "activating the anchor does not reopen"
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn keyboard_enters_the_open_preview_and_escape_restores_the_opener(cx: &mut TestAppContext) {
    let (mut visual, view) = native(cx, PREVIEW);
    visual
        .update(|window, app| view.focus(window, app))
        .unwrap();
    visual.simulate_keystrokes("tab");
    settle(&mut visual, Step::TickRefresh, 5);
    assert!(
        preview_open(&mut visual, &view),
        "keyboard focus is a real opening source"
    );
    let trigger = visual.update(|window, app| window.focused(app)).unwrap();
    visual.simulate_keystrokes("tab enter");
    settle(&mut visual, Step::TickRefresh, 4);
    assert_eq!(
        text_of(&mut visual, &view, "hits"),
        "1",
        "Tab reaches the action inside"
    );
    assert_ne!(
        visual.update(|window, app| window.focused(app)),
        Some(trigger.clone())
    );
    visual.simulate_keystrokes("escape");
    settle(&mut visual, Step::TickRefresh, 7);
    assert!(!preview_open(&mut visual, &view));
    assert_eq!(
        visual.update(|window, app| window.focused(app)),
        Some(trigger),
        "Escape returns to the actual focusable opener"
    );
    assert_eq!(
        text_of(&mut visual, &view, "anchor"),
        "0",
        "focus restoration is not activation"
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn a_declined_or_unmounted_preview_cannot_repeat_or_deliver_an_old_request(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = native(
        cx,
        r#"
      import "components/button" as button;
      import "components/hover_card" as hovercard;
      fn state_schema(){#{fields:#{
          count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
          live:#{schema:#{type:"bool"},"default":#{type:"bool",value:true}}
      }}}
      fn note(ctx,value){if value{ctx.set_state("count",ctx.get_state("count")+1);}}
      fn toggle(ctx,value){ctx.set_state("live",!ctx.get_state("live"));}
      fn view(ctx){column([
          if ctx.get_state("live"){hovercard::HoverCard(#{key:"same-key",label:"Service preview",
               show_delay_ms:96,open:false,on_open_change:Fn("note"),content:text("Preview"),
               trigger:button::Button(#{key:"show",text:"Service"})})}else{text("Gone")},
          button::Button(#{key:"toggle",text:"Toggle",on_click:Fn("toggle")}).test_id("toggle"),
          text(ctx.get_state("count").to_string()).test_id("count"),
      ]).with_style(style().width(px(380)).items_start().gap(px(30)))}
    "#,
    );
    let trigger = bounds(&mut visual, &view, "button", "Service");
    hover(&mut visual, center(trigger));
    settle(&mut visual, Step::TickRefresh, 15);
    assert_eq!(
        text_of(&mut visual, &view, "count"),
        "1",
        "rejected proposal is not retried without new input"
    );
    assert!(!preview_open(&mut visual, &view));
    hover(&mut visual, point(px(480.), px(390.)));
    hover(&mut visual, center(trigger));
    settle(&mut visual, Step::TickRefresh, 1);
    dispatch(&mut visual, &view, by_test_id("toggle"), "click").unwrap();
    settle(&mut visual, Step::TickRefresh, 1);
    hover(&mut visual, point(px(480.), px(390.)));
    dispatch(&mut visual, &view, by_test_id("toggle"), "click").unwrap();
    settle(&mut visual, Step::TickRefresh, 8);
    assert_eq!(
        text_of(&mut visual, &view, "count"),
        "1",
        "same id after remount does not inherit an old deadline"
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn a_second_hover_card_closes_its_peer_without_waiting_for_the_old_hide_delay(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = native(
        cx,
        r#"
        import "components/button" as button;
        import "components/hover_card" as hovercard;
        fn view(ctx){row([
            hovercard::HoverCard(#{key:"one",label:"Preview one",show_delay_ms:48,hide_delay_ms:5000,
                trigger:button::Button(#{key:"one",text:"Service one"}),content:text("First service")}),
            hovercard::HoverCard(#{key:"two",label:"Preview two",show_delay_ms:48,hide_delay_ms:5000,
                trigger:button::Button(#{key:"two",text:"Service two"}),content:text("Second service")}),
        ]).with_style(style().items_center().gap(px(90)).padding(px(30)))}
    "#,
    );
    let first = bounds(&mut visual, &view, "button", "Service one");
    let second = bounds(&mut visual, &view, "button", "Service two");
    let visible = |v: &mut VisualTestContext, name| {
        nodes(v, &view)
            .into_iter()
            .any(|n| n.name == name && n.geometry.is_some())
    };
    hover(&mut visual, center(first));
    settle(&mut visual, Step::TickRefresh, 4);
    assert!(visible(&mut visual, "Preview one"));
    hover(&mut visual, center(second));
    settle(&mut visual, Step::TickRefresh, 4);
    assert!(!visible(&mut visual, "Preview one"));
    assert!(visible(&mut visual, "Preview two"));
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn constrained_card_flips_using_logical_start_in_ltr_and_rtl(cx: &mut TestAppContext) {
    for rtl in [false, true] {
        let source = format!(
            r#"
            import "components/button" as button;
            import "components/hover_card" as hovercard;
            fn init(ctx){{ctx.set_locale("{}");}}
            fn view(ctx){{box([
                box([hovercard::HoverCard(#{{key:"flip",label:"Flipped preview",placement:"bottom",
                    align:"start",width:px(118),show_delay_ms:32,
                    trigger:button::Button(#{{key:"anchor",text:"Here"}}),
                    content:text("A tall preview").with_style(style().height(px(55)))}})])
                    .with_style(style().absolute().left(px(95)).top(px(220))),
            ]).with_style(style().relative().width(relative(1.0)).height(relative(1.0))) }}
        "#,
            if rtl { "ar" } else { "en" }
        );
        let (mut visual, view) = Mount::components(source)
            .motion(MotionPreference::None)
            .locale("ar.rhai", gpui_rhai_registry::AR_LOCALE)
            .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
            .mount(cx);
        visual.simulate_resize(gpui::size(px(300.), px(270.)));
        settle(&mut visual, Step::TickRefresh, 4);
        let trigger = bounds(&mut visual, &view, "button", "Here");
        hover(&mut visual, center(trigger));
        settle(&mut visual, Step::TickRefresh, 4);
        let panel = bounds(&mut visual, &view, "group", "Flipped preview");
        assert!(
            panel.y + panel.height + 6. < trigger.y,
            "must flip above: {panel:?} {trigger:?}"
        );
        assert!(panel.x >= 7.5 && panel.x + panel.width <= 292.5);
        let offset = if rtl {
            (panel.x + panel.width) - (trigger.x + trigger.width)
        } else {
            panel.x - trigger.x
        };
        assert!(
            offset.abs() < 2.0,
            "rtl={rtl}, anchor alignment {offset}, {panel:?} {trigger:?}"
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn reveal_moves_from_the_real_side_after_a_flip_without_animating_the_trigger(
    cx: &mut TestAppContext,
) {
    for (top, flipped) in [(45, false), (220, true)] {
        let source = format!(
            r#"
            import "components/button" as button;
            import "components/hover_card" as hovercard;
            fn view(ctx){{box([
                box([hovercard::HoverCard(#{{key:"animated",label:"Reveal preview",placement:"bottom",
                    align:"start",width:px(118),show_delay_ms:32,
                    trigger:button::Button(#{{key:"anchor",text:"Here"}}),
                    content:text("A tall preview").with_style(style().height(px(55)))}})])
                    .with_style(style().absolute().left(px(95)).top(px({top}))),
            ]).with_style(style().relative().width(relative(1.0)).height(relative(1.0))) }}
        "#
        );
        let clock = gpui_rhai::ManualRuntimeClock::new(Instant::now());
        let runtime = clock.clock();
        let (mut visual, view) = Mount::components(source)
            .motion(MotionPreference::Normal)
            .with(move |view| view.runtime_clock(runtime))
            .mount(cx);
        visual.simulate_resize(gpui::size(px(300.), px(270.)));
        settle(&mut visual, Step::TickRefresh, 4);
        let trigger = bounds(&mut visual, &view, "button", "Here");
        hover(&mut visual, center(trigger));
        settle(&mut visual, Step::TickRefresh, 4);
        clock.advance(Duration::from_millis(22));
        settle(&mut visual, Step::Refresh, 3);
        let entering = bounds(&mut visual, &view, "group", "Reveal preview");
        clock.advance(Duration::from_millis(240));
        settle(&mut visual, Step::Refresh, 3);
        let rested = bounds(&mut visual, &view, "group", "Reveal preview");
        if gpui_rhai::EXTENDED_RENDERING {
            assert!(
                entering.width < rested.width - 0.3,
                "scale sample absent: {entering:?} {rested:?}"
            );
        } else {
            assert!((entering.width - rested.width).abs() < 0.5);
        }
        let displacement = entering.y - rested.y;
        assert!(
            if flipped {
                displacement > 0.5
            } else {
                displacement < -0.5
            },
            "flipped={flipped}, placement motion needs the actual side: {displacement}, {entering:?} {rested:?}"
        );
        assert_eq!(bounds(&mut visual, &view, "button", "Here"), trigger);
        assert_eq!(last_error(&mut visual, &view), None);
    }
}
