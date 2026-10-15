//! Foreground-only, coalesced invalidation of native virtual-row measurements.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::App;

#[derive(Clone)]
pub(crate) struct IntrinsicSizeInvalidator(Rc<Invalidation>);

struct Invalidation {
    guards: RefCell<Vec<InvalidationGuard>>,
    invalidate: Box<dyn Fn(&mut App)>,
}

type InvalidationGuard = Box<dyn Fn() -> bool>;

impl IntrinsicSizeInvalidator {
    pub(crate) fn new(invalidate: impl Fn(&mut App) + 'static) -> Self {
        Self(Rc::new(Invalidation {
            guards: RefCell::new(Vec::new()),
            invalidate: Box::new(invalidate),
        }))
    }

    pub(crate) fn invalidate(&self, cx: &mut App) {
        self.invalidate_if(cx, || true);
    }

    pub(crate) fn invalidate_if(&self, cx: &mut App, valid: impl Fn() -> bool + 'static) {
        let queued = !self.0.guards.borrow().is_empty();
        self.0.guards.borrow_mut().push(Box::new(valid));
        if queued {
            return;
        }
        let state = Rc::clone(&self.0);
        // Never reenter a parent entity from native render/update. Nested
        // invalidators likewise cross a foreground effect boundary per level.
        cx.defer(move |cx| {
            let guards = std::mem::take(&mut *state.guards.borrow_mut());
            if guards.into_iter().any(|valid| valid()) {
                (state.invalidate)(cx);
            }
        });
    }
}
