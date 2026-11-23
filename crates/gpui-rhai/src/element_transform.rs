//! One sampled element affine for presentation and native input coordinates.
use crate::{Affine2D, StyleValueError, renderer::f64_to_f32};
use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, TransformationMatrix, Window,
};
use serde::{Deserialize, Serialize};
use std::{cell::Cell, rc::Rc};

/// Visual-only affine. Scale, skew, then rotate about a border-box origin.
/// Angles are degrees; 0.5 origins are the center and 0 is the physical left/top edge.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ElementTransform {
    pub rotate: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub skew_x: f64,
    pub skew_y: f64,
    pub origin_x: f64,
    pub origin_y: f64,
}

impl Default for ElementTransform {
    fn default() -> Self {
        Self {
            rotate: 0.,
            scale_x: 1.,
            scale_y: 1.,
            skew_x: 0.,
            skew_y: 0.,
            origin_x: 0.5,
            origin_y: 0.5,
        }
    }
}

impl ElementTransform {
    pub(crate) fn validated(self) -> Result<Self, StyleValueError> {
        if self.rotate.is_finite()
            && [self.scale_x, self.scale_y]
                .into_iter()
                .all(|x| x == 0. || (1.0e-6..=16.).contains(&x.abs()))
            && [self.skew_x, self.skew_y]
                .into_iter()
                .all(|x| (-89. ..=89.).contains(&x))
            && [self.origin_x, self.origin_y]
                .into_iter()
                .all(|x| (-16. ..=16.).contains(&x))
        {
            Ok(self)
        } else {
            Err(StyleValueError::InvalidElementTransform)
        }
    }

    pub(crate) fn is_identity(self) -> bool {
        // Only exact unit scale can bypass the shared paint/input scope.
        let unit = 1.0_f64.to_bits();
        self.rotate % 360. == 0.
            && self.scale_x.to_bits() == unit
            && self.scale_y.to_bits() == unit
            && self.skew_x == 0.
            && self.skew_y == 0.
    }

    pub(crate) fn matrix(self, bounds: Bounds<Pixels>) -> Option<TransformationMatrix> {
        if self.is_identity() {
            return Some(TransformationMatrix::unit());
        }
        let origin = (
            self.origin_x
                .mul_add(f64::from(bounds.size.width), f64::from(bounds.left())),
            self.origin_y
                .mul_add(f64::from(bounds.size.height), f64::from(bounds.top())),
        );
        let (xx, yx, xy, yy, tx, ty) = Affine2D::scale(self.scale_x, self.scale_y)
            .ok()?
            .then(Affine2D::skew_degrees(self.skew_x, self.skew_y).ok()?)
            .ok()?
            .then(Affine2D::rotation_degrees(self.rotate % 360.).ok()?)
            .ok()?
            .around(origin)
            .ok()?
            .components();
        Some(TransformationMatrix {
            rotation_scale: [
                [f64_to_f32(xx), f64_to_f32(xy)],
                [f64_to_f32(yx), f64_to_f32(yy)],
            ],
            translation: [f64_to_f32(tx), f64_to_f32(ty)],
        })
    }
}

pub(crate) fn transform(element: AnyElement, affine: Option<ElementTransform>) -> AnyElement {
    match affine {
        Some(affine) if !affine.is_identity() => AffineElement {
            child: Some(element),
            affine: AffineSource::Static(affine),
        }
        .into_any_element(),
        _ => element,
    }
}

/// Variant for a portal whose pivot is known only after placement.
pub(crate) fn transform_at_placement(
    element: AnyElement,
    affine: Rc<Cell<ElementTransform>>,
) -> AnyElement {
    AffineElement {
        child: Some(element),
        affine: AffineSource::Placed(affine),
    }
    .into_any_element()
}

enum AffineSource {
    Static(ElementTransform),
    Placed(Rc<Cell<ElementTransform>>),
}

struct AffineElement {
    child: Option<AnyElement>,
    affine: AffineSource,
}

impl IntoElement for AffineElement {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for AffineElement {
    type RequestLayoutState = AnyElement;
    type PrepaintState = Option<TransformationMatrix>;

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut child = self.child.take().expect("transform subtree requested once");
        (child.request_layout(window, cx), child)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let affine = match &self.affine {
            AffineSource::Static(spec) => *spec,
            AffineSource::Placed(spec) => spec.get(),
        };
        let matrix = affine.matrix(bounds)?;
        match window.with_composited_transform(matrix, |window| child.prepaint(window, cx)) {
            Ok(_) => Some(matrix),
            Err(e) => {
                eprintln!("gpui-rhai: element transform rejected: {e}");
                None
            }
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        child: &mut AnyElement,
        matrix: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(matrix) = *matrix
            && let Err(e) =
                window.with_composited_transform(matrix, |window| child.paint(window, cx))
        {
            eprintln!("gpui-rhai: element transform rejected: {e}");
        }
    }
}
