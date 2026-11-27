//! Density is live layout, not an alias for changing design or font size.

use crate::support::{
    Mount, Step, bounds, by_role, click_center, dispatch, last_error, node_by_test_id, settle,
};
use gpui::TestAppContext;
use gpui_rhai::MotionPreference;

#[gpui::test]
fn compact_reflows_the_same_editor_without_losing_focus_selection_or_undo(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual,view)=Mount::components(r#"
            import "components/input" as input;
            fn state_schema(){#{fields:#{
                draft:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
                density:#{schema:#{type:"string"},"default":#{type:"string",value:"comfortable"}}
            }}}
            fn changed(ctx,value){ctx.set_state("draft",value);}
            fn density(ctx,value){ctx.set_state("density",value);}
            fn view(ctx){column([
                input::Input(#{key:"editor",label:"Draft",value:ctx.get_state("draft"),on_change:Fn("changed")}),
                text("Compact").accessibility_role("button").accessibility_label("Compact").on_click_value(Fn("density"),"compact"),
                text("Comfortable").accessibility_role("button").accessibility_label("Comfortable").on_click_value(Fn("density"),"comfortable"),
                text(ctx.get_state("draft")).test_id("draft")
            ]).with_style(style().width(px(320)).padding(px(20)).gap(px(12)))
                .env(#{density:ctx.get_state("density")})}
        "#).theme(theme).motion(MotionPreference::None).id("live-density").mount(cx);
        settle(&mut visual, Step::Frame, 2);
        let comfortable = bounds(&mut visual, &view, "text_field", "Draft");
        click_center(&mut visual, comfortable);
        visual.simulate_input("hello");
        settle(&mut visual, Step::Frame, 2);
        visual.simulate_keystrokes(&crate::support::primary_keys("secondary-a"));
        dispatch(&mut visual, &view, by_role("button", "Compact"), "click").unwrap();
        settle(&mut visual, Step::Frame, 2);
        let compact = bounds(&mut visual, &view, "text_field", "Draft");
        assert!(
            (comfortable.height - compact.height - 4.0).abs() < 0.5,
            "{name}: actual field height must compact: {comfortable:?}, {compact:?}"
        );
        // Test one edit, independent of the native test runner's per-character
        // input and history-grouping policy for a longer string.
        visual.simulate_input("x");
        settle(&mut visual, Step::Frame, 2);
        assert_eq!(
            node_by_test_id(&mut visual, &view, "draft").name,
            "x",
            "{name}: retained selection and focus"
        );
        visual.simulate_keystrokes(&crate::support::primary_keys("secondary-z"));
        settle(&mut visual, Step::Frame, 2);
        assert_eq!(
            node_by_test_id(&mut visual, &view, "draft").name,
            "hello",
            "{name}: retained native undo"
        );
        dispatch(
            &mut visual,
            &view,
            by_role("button", "Comfortable"),
            "click",
        )
        .unwrap();
        settle(&mut visual, Step::Frame, 2);
        assert_eq!(
            bounds(&mut visual, &view, "text_field", "Draft").height,
            comfortable.height
        );
        assert!(last_error(&mut visual, &view).is_none());
    }
}
