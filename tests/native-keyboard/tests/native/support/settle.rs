//! Letting a mounted view settle: pending work, the clock and frames.

use std::time::Duration;

use gpui::VisualTestContext;

/// How one settle step lets the window draw. Tests differ here on purpose: a step that
/// advances the clock fires timers and motion, one that does not leaves them pending.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Step {
    /// Run until parked, advance the clock 32ms, simulate the next frame, run until parked.
    Tick,
    /// Simulate the next frame, run until parked; the clock does not move.
    Frame,
    /// Refresh the window, run until parked; the clock does not move.
    Refresh,
    /// Advance the clock 32ms, run until parked, refresh the window; the redraw this asks
    /// for runs at the next park.
    TickRefresh,
}

/// Runs `step` `times` times.
pub(crate) fn settle(visual: &mut VisualTestContext, step: Step, times: usize) {
    for _ in 0..times {
        match step {
            Step::Tick => {
                visual.run_until_parked();
                visual.executor().advance_clock(Duration::from_millis(32));
                visual.update(|window, cx| window.simulate_next_frame(cx));
                visual.run_until_parked();
            }
            Step::Frame => {
                visual.update(|window, cx| window.simulate_next_frame(cx));
                visual.run_until_parked();
            }
            Step::Refresh => {
                visual.update(|window, _| window.refresh());
                visual.run_until_parked();
            }
            Step::TickRefresh => {
                visual.executor().advance_clock(Duration::from_millis(32));
                visual.run_until_parked();
                visual.update(|window, _| window.refresh());
            }
        }
    }
}
