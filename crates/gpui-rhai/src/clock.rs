use std::cell::Cell;
use std::fmt;
use std::rc::Rc;
use std::time::{Duration, Instant};

pub trait RuntimeClockSource {
    fn now(&self) -> Instant;

    fn advance(&self, _duration: Duration) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct RuntimeClock(Rc<dyn RuntimeClockSource>);

impl RuntimeClock {
    #[must_use]
    pub fn system() -> Self {
        Self(Rc::new(SystemRuntimeClock))
    }

    #[must_use]
    pub fn from_source(source: impl RuntimeClockSource + 'static) -> Self {
        Self(Rc::new(source))
    }

    #[must_use]
    pub fn now(&self) -> Instant {
        self.0.now()
    }

    #[must_use]
    pub fn advance(&self, duration: Duration) -> bool {
        self.0.advance(duration)
    }
}

impl Default for RuntimeClock {
    fn default() -> Self {
        Self::system()
    }
}

impl fmt::Debug for RuntimeClock {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RuntimeClock(..)")
    }
}

#[derive(Clone, Copy, Debug)]
struct SystemRuntimeClock;

impl RuntimeClockSource for SystemRuntimeClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[derive(Clone, Debug)]
pub struct ManualRuntimeClock {
    now: Rc<Cell<Instant>>,
}

impl ManualRuntimeClock {
    #[must_use]
    pub fn new(now: Instant) -> Self {
        Self {
            now: Rc::new(Cell::new(now)),
        }
    }

    #[must_use]
    pub fn clock(&self) -> RuntimeClock {
        RuntimeClock::from_source(self.clone())
    }

    pub fn advance(&self, duration: Duration) {
        self.now.set(self.now.get() + duration);
    }

    pub fn set(&self, now: Instant) {
        self.now.set(now);
    }
}

impl RuntimeClockSource for ManualRuntimeClock {
    fn now(&self) -> Instant {
        self.now.get()
    }

    fn advance(&self, duration: Duration) -> bool {
        self.advance(duration);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_clock_is_shared_and_monotonic_by_host_policy() {
        let start = Instant::now();
        let manual = ManualRuntimeClock::new(start);
        let clock = manual.clock();
        manual.advance(Duration::from_millis(16));
        assert_eq!(clock.now(), start + Duration::from_millis(16));
    }
}
