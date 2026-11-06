//! `interval` fires every period on the view's clock until its declaration
//! pauses it (feedback #19). The test clock advances through every timer in
//! order, so late ticks merging is covered by the timer registry's own test.

use std::time::Duration;

use gpui::TestAppContext;

use crate::support::{Mount, last_error, text_of};

const SOURCE: &str = r#"fn state_schema(){#{fields:#{
    ticks:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn tick(ctx,payload){ctx.set_state("ticks",ctx.get_state("ticks")+1);}
fn view(ctx){
    interval("tick", 100, ctx.get_state("ticks") >= 4, Fn("tick"), ());
    text(`ticks ${ctx.get_state("ticks")}`).test_id("ticks")
}"#;

fn advance(visual: &mut gpui::VisualTestContext, millis: u64) {
    visual.executor().advance_clock(Duration::from_millis(millis));
    visual.run_until_parked();
    visual.update(|window, cx| window.simulate_next_frame(cx));
    visual.run_until_parked();
}

#[gpui::test]
fn an_interval_ticks_every_period_until_its_declaration_pauses_it(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(SOURCE).id("interval").mount(cx);
    visual.run_until_parked();
    assert_eq!(last_error(&mut visual, &view), None);
    // The runtime polls its timers every 16ms, so each check leaves a frame
    // of slack after a deadline.
    advance(&mut visual, 99);
    assert_eq!(text_of(&mut visual, &view, "ticks"), "ticks 0");
    advance(&mut visual, 21);
    assert_eq!(text_of(&mut visual, &view, "ticks"), "ticks 1");
    advance(&mut visual, 100);
    assert_eq!(text_of(&mut visual, &view, "ticks"), "ticks 2");
    advance(&mut visual, 100);
    assert_eq!(text_of(&mut visual, &view, "ticks"), "ticks 3");
    advance(&mut visual, 100);
    assert_eq!(text_of(&mut visual, &view, "ticks"), "ticks 4");
    // The fourth tick's render declares the interval paused.
    advance(&mut visual, 1000);
    assert_eq!(text_of(&mut visual, &view, "ticks"), "ticks 4");
    assert_eq!(last_error(&mut visual, &view), None);
}
