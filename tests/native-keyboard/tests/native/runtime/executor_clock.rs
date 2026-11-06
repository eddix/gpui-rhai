//! A mounted view's timers follow GPUI's executor clock unless its Host sets a
//! clock: in a GPUI test the scheduler's, which moves only when the test moves
//! it. A timeout fires once the advanced time reaches its deadline, without
//! waiting on real time, and does not fire before (feedback #2).

use std::time::Duration;

use gpui::TestAppContext;

use crate::support::{Mount, last_error, text_of};

const SOURCE: &str = r#"fn state_schema(){#{fields:#{
    fired:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn fire(ctx,payload){ctx.set_state("fired",ctx.get_state("fired")+1);}
fn view(ctx){
    if ctx.get_state("fired") == 0 { timeout("second", 1000, false, Fn("fire"), ()); }
    text(`fired ${ctx.get_state("fired")}`).test_id("fired")
}"#;

fn advance(visual: &mut gpui::VisualTestContext, millis: u64) {
    visual.executor().advance_clock(Duration::from_millis(millis));
    visual.run_until_parked();
    visual.update(|window, cx| window.simulate_next_frame(cx));
    visual.run_until_parked();
}

#[gpui::test]
fn a_timeout_fires_when_the_test_clock_reaches_its_deadline(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(SOURCE).id("clock").mount(cx);
    visual.run_until_parked();
    assert_eq!(last_error(&mut visual, &view), None);
    // Real time passing moves nothing: the deadline is on the scheduler's clock.
    std::thread::sleep(Duration::from_millis(1100));
    advance(&mut visual, 0);
    assert_eq!(text_of(&mut visual, &view, "fired"), "fired 0");
    advance(&mut visual, 990);
    assert_eq!(text_of(&mut visual, &view, "fired"), "fired 0");
    advance(&mut visual, 20);
    assert_eq!(text_of(&mut visual, &view, "fired"), "fired 1");
}
