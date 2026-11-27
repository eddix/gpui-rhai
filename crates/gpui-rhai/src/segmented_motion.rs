use crate::{MotionEasing, motion::NativeProgressRequest, renderer::NativeNodeMotion};
use gpui::Window;

const CELL_COUNT: usize = 24;
const RING_LANE: u8 = 24;
const CARET_LANE: u8 = 25;
const EASE: MotionEasing = MotionEasing::CubicBezier {
    x1: 0.22,
    y1: 1.,
    x2: 0.36,
    y2: 1.,
};

#[derive(Default)]
pub(crate) struct SegmentedMotion {
    revision: u64,
    glyphs: [Option<NativeProgressRequest>; CELL_COUNT],
    retired: u32,
    ring: Ring,
    caret: Option<(usize, NativeProgressRequest)>,
}

#[derive(Default)]
enum Ring {
    #[default]
    Hidden,
    At(u8),
    Sliding {
        from: f32,
        to: u8,
        request: NativeProgressRequest,
    },
}

impl SegmentedMotion {
    pub(crate) fn accepted(&mut self, prior: Option<&str>, accepted: &str, editable: bool) {
        let Some(prior) = prior.filter(|_| editable) else {
            self.retire_glyphs();
            return;
        };
        let changed = accepted
            .bytes()
            .enumerate()
            .filter(|(i, byte)| prior.as_bytes().get(*i) != Some(byte))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        for i in accepted.len()..CELL_COUNT {
            if self.glyphs[i].take().is_some() {
                self.retired |= 1 << i;
            }
        }
        let first = changed.first().copied().unwrap_or_default();
        for index in changed {
            if let Some(slot) = self.glyphs.get_mut(index) {
                self.revision = self.revision.wrapping_add(1);
                *slot = Some(NativeProgressRequest {
                    revision: self.revision,
                    duration_ms: 280,
                    delay_ms: u64::try_from(index - first).unwrap_or_default() * 45,
                    easing: EASE,
                    looping: false,
                });
            }
        }
    }

    fn retire_glyphs(&mut self) {
        for (index, phase) in self.glyphs.iter_mut().enumerate() {
            if phase.take().is_some() {
                self.retired |= 1 << index;
            }
        }
    }

    /// A keyed text editor can leave segmented mode without unmounting.
    pub(crate) fn stop(&mut self, motion: Option<&NativeNodeMotion>, window: &Window) {
        self.retire_glyphs();
        self.clear_retired(motion, window);
        self.ring(None, motion, window);
        self.caret(None, motion, window);
    }

    /// Activity is identical to caret paint eligibility. No detached timer
    /// and no animation survives loss of focus, a filled slot, or unmount.
    pub(crate) fn caret(
        &mut self,
        target: Option<usize>,
        motion: Option<&NativeNodeMotion>,
        window: &Window,
    ) -> f32 {
        let Some(motion) = motion else {
            self.caret = None;
            return 1.;
        };
        let Some(target) = target else {
            if self.caret.take().is_some() {
                motion.sample(CARET_LANE, None, window);
            }
            return 1.;
        };
        let request = self.caret.filter(|(slot, _)| *slot == target).map_or_else(
            || {
                self.revision = self.revision.wrapping_add(1);
                NativeProgressRequest {
                    revision: self.revision,
                    duration_ms: 1100,
                    delay_ms: 0,
                    easing: MotionEasing::Linear,
                    looping: true,
                }
            },
            |(_, request)| request,
        );
        self.caret = Some((target, request));
        let phase = motion.sample(CARET_LANE, Some(request), window);
        crate::renderer::f64_to_f32(
            MotionEasing::EaseInOut.sample(f64::from(2_f32.mul_add(phase, -1.).abs())),
        )
    }

    pub(crate) fn clear_retired(&mut self, motion: Option<&NativeNodeMotion>, window: &Window) {
        let retired = std::mem::take(&mut self.retired);
        if let Some(motion) = motion {
            for i in 0..CELL_COUNT {
                if retired & (1 << i) != 0 {
                    motion.sample(u8::try_from(i).unwrap_or_default(), None, window);
                }
            }
        }
    }

    pub(crate) fn glyph(
        &mut self,
        index: usize,
        motion: Option<&NativeNodeMotion>,
        window: &Window,
    ) -> f32 {
        let Some(motion) = motion else { return 1. };
        let Some(request) = self.glyphs.get_mut(index).and_then(|slot| *slot) else {
            return 1.;
        };
        let lane = u8::try_from(index).unwrap_or_default();
        let progress = motion.sample(lane, Some(request), window);
        if progress >= 1. {
            self.glyphs[index] = None;
            motion.sample(lane, None, window);
        }
        progress
    }

    pub(crate) fn ring(
        &mut self,
        next: Option<usize>,
        motion: Option<&NativeNodeMotion>,
        window: &Window,
    ) -> Option<f32> {
        let next = next.and_then(|i| u8::try_from(i).ok());
        match (next, motion) {
            (Some(next), Some(motion)) => Some(self.slide_to(next, motion, window)),
            (next, motion) => {
                if let Some(motion) = motion
                    && matches!(self.ring, Ring::Sliding { .. })
                {
                    motion.sample(RING_LANE, None, window);
                }
                self.ring = next.map_or(Ring::Hidden, Ring::At);
                next.map(f32::from)
            }
        }
    }

    fn slide_to(&mut self, next: u8, motion: &NativeNodeMotion, window: &Window) -> f32 {
        let position = match self.ring {
            Ring::Hidden => {
                self.ring = Ring::At(next);
                return f32::from(next);
            }
            Ring::At(at) => f32::from(at),
            Ring::Sliding { from, to, request } => {
                let p = motion.sample(RING_LANE, Some(request), window);
                if p >= 1. {
                    motion.sample(RING_LANE, None, window);
                    self.ring = Ring::At(to);
                }
                let position = (f32::from(to) - from).mul_add(p, from);
                if next == to {
                    return position;
                }
                position
            }
        };
        if (position - f32::from(next)).abs() > f32::EPSILON {
            self.revision = self.revision.wrapping_add(1);
            let request = NativeProgressRequest {
                revision: self.revision,
                duration_ms: 260,
                delay_ms: 0,
                easing: EASE,
                looping: false,
            };
            if motion.sample(RING_LANE, Some(request), window) < 1. {
                self.ring = Ring::Sliding {
                    from: position,
                    to: next,
                    request,
                };
                return position;
            }
        }
        self.ring = Ring::At(next);
        f32::from(next)
    }
}
