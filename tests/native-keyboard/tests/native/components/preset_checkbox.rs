//! Controlled proposals stay business-owned; the shared clock owns mark frames.

use std::time::{Duration, Instant};

use gpui::TestAppContext;
use gpui_rhai::{ManualRuntimeClock, MotionPreference, MotionSource, UiNode, UiNodeKind, UiValue};

use crate::support::{
    Mount, Step, bounds, by_test_id, click_center, dispatch, last_error, settle, snapshot, text_of,
};

fn mark(node: &UiNode) -> Option<&UiNode> {
    if node.key().is_some_and(|key| key.as_str() == "mark") {
        return Some(node);
    }
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(mark)
        }
        _ => None,
    }
}

#[gpui::test]
fn checkbox_initial_checked_and_mixed_states_are_settled_and_idle(cx: &mut TestAppContext) {
    for &(_, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for mixed in [false, true] {
            let manual = ManualRuntimeClock::new(Instant::now());
            let clock = manual.clock();
            let script = format!(
                r#"import "components/checkbox" as checkbox;
                fn view(ctx){{checkbox::Checkbox(#{{key:"mounted",label:"Mounted",checked:true,indeterminate:{mixed}}})}}"#
            );
            let (mut visual, view) = Mount::components(script)
                .theme(theme)
                .motion(MotionPreference::Normal)
                .with(move |v| v.runtime_clock(clock))
                .mount(cx);
            settle(&mut visual, Step::Frame, 4);
            let mut scheduled = Vec::new();
            for _ in 0..3 {
                scheduled.push(visual.update(|window, cx| window.simulate_next_frame(cx)));
                visual.run_until_parked();
            }
            assert_eq!(
                scheduled.last(),
                Some(&0),
                "first mount must not draw an entry: {scheduled:?}"
            );
            assert_eq!(last_error(&mut visual, &view), None);
        }
    }
}

#[gpui::test]
fn checkbox_proposals_do_not_animate_until_accepted_and_native_sampling_keeps_focus(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let manual = ManualRuntimeClock::new(Instant::now());
        let clock = manual.clock();
        let (mut visual,view)=Mount::components(r#"
            import "components/checkbox" as checkbox;
            fn state_schema(){#{fields:#{
                checked:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
                accept:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
                proposals:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
            }}}
            fn changed(ctx,value){ctx.set_state("proposals",ctx.get_state("proposals")+1);
                if ctx.get_state("accept"){ctx.set_state("checked",value.checked);}}
            fn accept(ctx,p){ctx.set_state("accept",true);}
            fn view(ctx){column([
                checkbox::Checkbox(#{key:"choice",label:"Choice",checked:ctx.get_state("checked"),on_change:Fn("changed")}),
                text("Accept").test_id("accept").on_click(Fn("accept")),
                text(`${ctx.get_state("proposals")}`).test_id("proposals")
            ]).with_style(style().padding(px(20)).gap(px(12)))}
        "#).theme(theme).motion(MotionPreference::Normal).with(move|v|v.runtime_clock(clock)).mount(cx);
        visual.update(|window, _| window.activate_window());
        settle(&mut visual, Step::Frame, 4);
        let area = bounds(&mut visual, &view, "checkbox", "Choice");
        let id = snapshot(&mut visual, &view)
            .find_by_role_and_name("checkbox", "Choice")
            .next()
            .unwrap()
            .id;
        click_center(&mut visual, area);
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(text_of(&mut visual, &view, "proposals"), "1");
        assert_eq!(
            snapshot(&mut visual, &view)
                .find_by_role_and_name("checkbox", "Choice")
                .next()
                .unwrap()
                .checked,
            Some(UiValue::Bool(false))
        );
        dispatch(&mut visual, &view, by_test_id("accept"), "click").unwrap();
        settle(&mut visual, Step::Frame, 3);
        click_center(&mut visual, area);
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(text_of(&mut visual, &view, "proposals"), "2");
        let field = snapshot(&mut visual, &view)
            .find_by_role_and_name("checkbox", "Choice")
            .next()
            .unwrap()
            .clone();
        assert_eq!(field.id, id);
        assert_eq!(field.checked, Some(UiValue::Bool(true)));
        let focus = visual.update(|window, cx| window.focused(cx));
        assert!(focus.is_some());
        let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
        let glyph = mark(&root).unwrap();
        let MotionSource::Transition(spec) = &glyph.motions()[0] else {
            panic!("mark transition")
        };
        assert_eq!((spec.duration_ms, spec.delay_ms), (260, 40));
        let revision = visual.update(|_, cx| view.committed_revision(cx).unwrap());
        visual.update(|_, cx| view.take_performance_snapshot(cx).unwrap());
        manual.advance(Duration::from_millis(60));
        settle(&mut visual, Step::Refresh, 4);
        assert_eq!(
            visual.update(|_, cx| view.committed_revision(cx).unwrap()),
            revision
        );
        assert!(
            visual
                .update(|_, cx| view.take_performance_snapshot(cx).unwrap())
                .timings
                .is_empty(),
            "{name}: native frame ran Rhai"
        );
        assert_eq!(visual.update(|window, cx| window.focused(cx)), focus);
        assert_eq!(bounds(&mut visual, &view, "checkbox", "Choice"), area);
        visual
            .update(|_, cx| view.set_motion_preference(MotionPreference::None, cx))
            .unwrap();
        settle(&mut visual, Step::Frame, 4);
        let mut scheduled = Vec::new();
        for _ in 0..3 {
            scheduled.push(visual.update(|window, cx| window.simulate_next_frame(cx)));
            visual.run_until_parked();
        }
        assert_eq!(
            scheduled.last(),
            Some(&0),
            "{name}: Host None did not settle: {scheduled:?}"
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
}
