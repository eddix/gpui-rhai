//! Paint-only scopes. Raw operation indices remain the replay protocol;
//! only the drawable stream substitutes the prepared group primitive.
use std::{borrow::Cow, ops::Range};

use crate::{ContentMask, IsolatedGroup, IsolatedGroupSource, ScaledPixels, TransformationMatrix};

use super::{PaintOperation, Primitive, Scene};

#[derive(Clone)]
pub(crate) enum IsolationBoundary {
    Begin,
    /// None is an empty or rejected scope, not permission to draw its body
    /// at full opacity.
    End(Option<IsolatedGroup>),
    /// Replace this scope with an ordered stream prepared from its raw draws.
    /// Kept at the boundary so retained paint indices remain valid.
    Draws(std::rc::Rc<[PaintOperation]>),
}

pub(crate) fn drawable_operations(
    operations: &[PaintOperation],
) -> impl Iterator<Item = Cow<'_, PaintOperation>> {
    let mut depth = 0_usize;
    operations.iter().flat_map(move |operation| {
        let (single, many): (Option<Cow<'_, PaintOperation>>, &[PaintOperation]) = match operation {
            PaintOperation::Isolation(IsolationBoundary::Begin) => {
                depth += 1;
                (None, &[])
            }
            PaintOperation::Isolation(IsolationBoundary::End(group)) => {
                // Cached child ranges normally exclude both enclosing markers.
                // An unmatched closer is not a drawable group.
                let visible = depth == 1;
                depth = depth.saturating_sub(1);
                let primitive = if visible {
                    group.as_ref().map(|group| Cow::Owned(PaintOperation::Primitive(
                        Primitive::IsolatedGroup(group.clone()),
                    )))
                } else {
                    None
                };
                (primitive, &[])
            }
            PaintOperation::Isolation(IsolationBoundary::Draws(draws)) => {
                let visible = depth == 1;
                depth = depth.saturating_sub(1);
                (None, if visible { draws.as_ref() } else { &[] })
            }
            _ if depth == 0 => (Some(Cow::Borrowed(operation)), &[]),
            _ => (None, &[]),
        };
        single.into_iter().chain(many.iter().map(Cow::Borrowed))
    })
}

impl Scene {
    pub(crate) fn record_isolation_boundary(&mut self, boundary: IsolationBoundary) {
        self.isolation_boundaries = true;
        self.paint_operations.push(PaintOperation::Isolation(boundary));
    }

    pub(crate) fn begin_isolated_paint(&mut self) -> usize {
        self.record_isolation_boundary(IsolationBoundary::Begin);
        self.paint_operations.len()
    }

    pub(crate) fn end_replaced_paint(&mut self, draws: Vec<PaintOperation>) {
        self.record_isolation_boundary(IsolationBoundary::Draws(draws.into()));
    }

    pub(crate) fn end_affine_paint(
        &mut self,
        start: usize,
        opacity: f32,
        mask: ContentMask<ScaledPixels>,
        placement: TransformationMatrix,
    ) -> Result<(), &'static str> {
        let result = self.prepare_isolated_paint(
            start..self.paint_operations.len(), opacity, 0., 0., mask, placement, None, None, None, true,
        );
        self.record_isolation_boundary(IsolationBoundary::End(result.as_ref().ok().cloned().flatten()));
        result.map(|_| ())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn end_masked_paint(
        &mut self,
        start: usize,
        opacity: f32,
        blur_sigma: f32,
        grayscale: f32,
        mask: ContentMask<ScaledPixels>,
        placement: TransformationMatrix,
        alpha_ramp: Option<[f32; 5]>,
        reveal: Option<[f32; 5]>,
        parent_backdrop: Option<crate::BackdropBlur>,
    ) -> Result<(), &'static str> {
        let result =
            self.prepare_isolated_paint(start..self.paint_operations.len(), opacity, blur_sigma, grayscale, mask, placement, alpha_ramp, reveal, parent_backdrop, false);
        self.record_isolation_boundary(IsolationBoundary::End(result.as_ref().ok().cloned().flatten()));
        result.map(|_| ())
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_isolated_paint(
        &self,
        range: Range<usize>,
        opacity: f32,
        blur_sigma: f32,
        grayscale: f32,
        mask: ContentMask<ScaledPixels>,
        placement: TransformationMatrix,
        alpha_ramp: Option<[f32; 5]>,
        reveal: Option<[f32; 5]>,
        parent_backdrop: Option<crate::BackdropBlur>,
        placement_only: bool,
    ) -> Result<Option<IsolatedGroup>, &'static str> {
        if opacity == 0. || (range.is_empty() && parent_backdrop.is_none()) {
            return Ok(None);
        }
        let mut child = Scene::default();
        let operations = self.paint_operations.get(range).ok_or("invalid isolation range")?;
        for op in drawable_operations(operations) {
            child.replay_operation(op.as_ref());
        }
        let Some((source, origin)) =
            IsolatedGroupSource::capture_filtered_paint_range(
                &child, 0..child.len(), blur_sigma,
                // The visible clip must not shorten the input ROI of a
                // blur kernel. For example a scrolling edge can hide half
                // the panel without changing samples on the visible half.
                parent_backdrop.filter(|read| !read.bounds.intersect(&read.content_mask.bounds).is_empty())
                    .map(|read| read.bounds),
                placement_only,
            )?
        else {
            return Ok(None);
        };
        let mut transform = TransformationMatrix::unit();
        transform.translation = [origin.x.0, origin.y.0];
        let mut group = IsolatedGroup::new(source, placement.compose(transform), opacity, mask)?;
        group.set_grayscale(grayscale);
        if placement_only {
            group.preserve_parent_backdrop();
        }
        if let Some(mut read) = parent_backdrop {
            read.bounds.origin -= origin;
            read.content_mask.bounds.origin -= origin;
            group.set_parent_backdrop(read);
        }
        if let Some(mut ramps) = alpha_ramp {
            // Capture origin may move with the visible paint/scroll clip.
            // Keep the authored border-box line in the group's source space.
            let offset = if ramps[4] != 0. { origin.x.0 } else { origin.y.0 };
            if ramps[1] > 0. { ramps[0] -= offset; }
            if ramps[3] > 0. { ramps[2] -= offset; }
            group.set_alpha_ramps(ramps)?;
        }
        if let Some([x,y,width,height,radius]) = reveal {
            group.set_rounded_reveal([x-origin.x.0, y-origin.y.0, width, height, radius])?;
        }
        Ok(Some(group))
    }

    /// Sort the effective drawing stream, while leaving all cached view
    /// ranges pointing into the raw commands.
    pub(super) fn materialize_isolated_paint(&mut self) -> bool {
        if !self.isolation_boundaries {
            return false;
        }
        if self.isolated_draw_operations.is_some() {
            return true;
        }
        let raw = std::mem::take(&mut self.paint_operations);
        let mut drawable = Self::default();
        for op in drawable_operations(&raw) {
            drawable.replay_operation(op.as_ref());
        }
        drawable.finish();
        if drawable.isolated_draw_operations.is_none() {
            drawable.isolated_draw_operations = Some(std::mem::take(&mut drawable.paint_operations));
        }
        drawable.paint_operations = raw;
        drawable.isolation_boundaries = true;
        *self = drawable;
        true
    }
}
