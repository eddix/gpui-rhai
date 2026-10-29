//! Two windows that run the same script through the same events, so the frame
//! digest of one can be checked against the other: the A/B harness that proves
//! a faster render path draws what the plain one does.

use gpui::{Pixels, Size, TestAppContext, VisualTestContext};
use gpui_rhai::{ScriptViewFrameDigest, ScriptViewHandle};

use super::{Mount, Step, painted_quads, settle};

pub(crate) struct Lockstep {
    sides: Vec<(VisualTestContext, ScriptViewHandle)>,
}

impl Lockstep {
    /// Mounts `a` and `b` in windows of `size` and lets both settle. Both keep
    /// the view ID of their mount, so their component paths match.
    pub(crate) fn mount(cx: &mut TestAppContext, a: Mount, b: Mount, size: Size<Pixels>) -> Self {
        let sides = [a.ids("lockstep-a", "view"), b.ids("lockstep-b", "view")]
            .into_iter()
            .map(|mount| {
                let (window, view) = mount.mount_window(cx);
                let mut visual = VisualTestContext::from_window(*window, cx);
                visual.simulate_resize(size);
                settle(&mut visual, Step::Tick, 2);
                (visual, view)
            })
            .collect();
        Self { sides }
    }

    /// Runs `action` on each window, A first.
    pub(crate) fn act(
        &mut self,
        mut action: impl FnMut(&mut VisualTestContext, &ScriptViewHandle),
    ) {
        for (visual, view) in &mut self.sides {
            action(visual, view);
        }
    }

    /// The digest of window `side` (0 for A, 1 for B).
    pub(crate) fn digest(&mut self, side: usize) -> ScriptViewFrameDigest {
        let (visual, view) = &mut self.sides[side];
        visual.update(|window, cx| view.debug_frame_digest(window, cx).unwrap())
    }

    /// Every quad window `side` painted in its last frame, in paint order.
    pub(crate) fn quads(&mut self, side: usize) -> Vec<String> {
        painted_quads(&mut self.sides[side].0)
            .iter()
            .map(|quad| format!("{quad:?}"))
            .collect()
    }

    /// Panics with the first difference when the two windows' digests or
    /// painted quads differ.
    #[track_caller]
    pub(crate) fn assert_same(&mut self, label: &str) {
        let a = self.digest(0);
        let b = self.digest(1);
        if a != b {
            let difference = first_difference(&a, &b);
            panic!("{label}: the windows differ: {difference}");
        }
        let (a, b) = (self.quads(0), self.quads(1));
        if let Some(difference) = first_of("painted quads", &a, &b) {
            panic!("{label}: the windows paint differently: {difference}");
        }
    }
}

fn first_of<T: PartialEq + std::fmt::Debug>(what: &str, a: &[T], b: &[T]) -> Option<String> {
    if a == b {
        return None;
    }
    let index = a
        .iter()
        .zip(b)
        .position(|(a, b)| a != b)
        .unwrap_or(a.len().min(b.len()));
    Some(format!(
        "{what}[{index}] (of {} and {}): {:?} vs {:?}",
        a.len(),
        b.len(),
        a.get(index),
        b.get(index)
    ))
}

fn first_difference(a: &ScriptViewFrameDigest, b: &ScriptViewFrameDigest) -> String {
    if a.retained_nodes != b.retained_nodes {
        return format!(
            "retained nodes {} vs {}",
            a.retained_nodes, b.retained_nodes
        );
    }
    if a.last_error != b.last_error {
        return format!("last error {:?} vs {:?}", a.last_error, b.last_error);
    }
    first_of("geometry", &a.geometry, &b.geometry)
        .or_else(|| first_of("accessibility", &a.accessibility, &b.accessibility))
        .or_else(|| first_of("focus path", &a.focus_path, &b.focus_path))
        .or_else(|| first_of("scroll offsets", &a.scroll_offsets, &b.scroll_offsets))
        .or_else(|| {
            first_of(
                "virtual collections",
                &a.virtual_collections,
                &b.virtual_collections,
            )
        })
        .unwrap_or_else(|| "a field the comparison does not name".to_owned())
}
