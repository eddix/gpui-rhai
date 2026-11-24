//! An immutable local-coordinate scene, isolated before its one composition.
use super::{PaintOperation, Primitive};
use crate::{
    Bounds, ContentMask, DevicePixels, DrawOrder, Point, ScaledPixels, Scene, Size,
    TransformationMatrix, point, size,
};
use std::{ops::Range, rc::Rc};

#[derive(Clone)]
/// Immutable local-coordinate commands. Native external surfaces are not capturable.
pub struct IsolatedGroupSource {
    scene: Rc<Scene>,
    size: Size<DevicePixels>,
    blur_sigma: f32,
}
impl IsolatedGroupSource {
    /// Capture one balanced raw paint range into an immutable local scene.
    /// This is intentionally read-only: layout/paint replay indices still
    /// refer to the original operation stream. The caller must subsequently
    /// replace the range in its *drawing* stream, not replay it twice.
    pub fn capture_paint_range(
        scene: &Scene,
        range: Range<usize>,
    ) -> Result<Option<(Self, Point<ScaledPixels>)>, &'static str> {
        Self::capture_filtered_paint_range(scene, range, 0., None, false)
    }

    pub(super) fn capture_filtered_paint_range(
        scene: &Scene,
        range: Range<usize>,
        blur_sigma: f32,
        seed_bounds: Option<Bounds<ScaledPixels>>,
        placement_only: bool,
    ) -> Result<Option<(Self, Point<ScaledPixels>)>, &'static str> {
        if !blur_sigma.is_finite() || !(0.0..=128.0).contains(&blur_sigma) {
            return Err("invalid foreground blur sigma");
        }
        let operations = scene
            .paint_operations
            .get(range)
            .ok_or("invalid paint range")?;
        let mut coverage = seed_bounds.filter(|bounds| !bounds.is_empty());
        // Layer and analytic clip boundaries must be closed in this range.
        let mut scopes = Vec::new();
        for operation in operations {
            match operation {
                PaintOperation::Isolation(_) => return Err("unresolved isolation inside a paint capture"),
                PaintOperation::Primitive(Primitive::Surface(_)) => {
                    return Err("external surface in paint range");
                }
                PaintOperation::Primitive(primitive) => {
                    let bounds = painted_bounds(primitive)
                        .intersect(&primitive.content_mask().bounds);
                    if !bounds.is_empty() {
                        // The mask limits the visible result, not the input
                        // to a placed backdrop's convolution.
                        let bounds = match primitive {
                            Primitive::BackdropBlur(read) if placement_only => read.bounds,
                            _ => bounds,
                        };
                        coverage =
                            Some(coverage.map_or(bounds, |combined| combined.union(&bounds)));
                    }
                }
                PaintOperation::StartLayer(_) => scopes.push(false),
                PaintOperation::EndLayer => {
                    if scopes.pop() != Some(false) {
                        return Err("unbalanced layer in paint range");
                    }
                }
            }
        }
        if !scopes.is_empty() {
            return Err("unclosed scope in paint range");
        }
        let Some(coverage) = coverage else {
            return Ok(None);
        };
        // Use the actual clipped paint footprint (which may include shadows),
        // not the logical layout/hit rectangle. Snap in physical pixels
        // before translating paths, clip corners, and atlas sprites.
        let padding = (4. * blur_sigma).ceil();
        let origin = point(
            ScaledPixels(coverage.origin.x.0.floor() - padding),
            ScaledPixels(coverage.origin.y.0.floor() - padding),
        );
        let far = coverage.bottom_right();
        let width = far.x.0.ceil() + padding - origin.x.0;
        let height = far.y.0.ceil() + padding - origin.y.0;
        if ![width, height]
            .iter()
            .all(|v| v.is_finite() && *v >= 1. && *v < i32::MAX as f32)
        {
            return Err("invalid isolated raster extent");
        }
        let delta = point(ScaledPixels(-origin.x.0), ScaledPixels(-origin.y.0));
        let mut child = Scene::default();
        for operation in operations {
            match operation {
                PaintOperation::Isolation(_) => unreachable!("checked the paint range above"),
                PaintOperation::Primitive(primitive) => {
                    child.insert_primitive(translate_primitive(primitive, delta));
                }
                PaintOperation::StartLayer(bounds) => {
                    let mut bounds = *bounds;
                    bounds.origin += delta;
                    child.push_layer(bounds);
                }
                PaintOperation::EndLayer => child.pop_layer(),
            }
        }
        let mut source = Self::new(
            child,
            size(DevicePixels(width as i32), DevicePixels(height as i32)),
        )?;
        source.blur_sigma = blur_sigma;
        Ok(Some((source, origin)))
    }

    /// Finish and retain a source viewport in physical pixels; rejects nonpositive sizes and external surfaces.
    pub fn new(mut scene: Scene, size: Size<DevicePixels>) -> Result<Self, &'static str> {
        if size.width.0 <= 0 || size.height.0 <= 0 {
            return Err("group source size must be positive");
        }
        scene.finish();
        if scene.has_external_group_surfaces() {
            return Err("native external surfaces cannot be captured as an immutable group source");
        }
        Ok(Self {
            scene: Rc::new(scene),
            size,
            blur_sigma: 0.,
        })
    }
}

fn painted_bounds(primitive: &Primitive) -> Bounds<ScaledPixels> {
    fn transformed(bounds: Bounds<ScaledPixels>, transform: &TransformationMatrix) -> Bounds<ScaledPixels> {
        if *transform == TransformationMatrix::unit() {
            return bounds;
        }
        let [a,b] = transform.rotation_scale[0];
        let [c,d] = transform.rotation_scale[1];
        let [tx,ty] = transform.translation;
        let far = bounds.bottom_right();
        let mut lo = point(ScaledPixels(f32::INFINITY), ScaledPixels(f32::INFINITY));
        let mut hi = point(ScaledPixels(f32::NEG_INFINITY), ScaledPixels(f32::NEG_INFINITY));
        for x in [bounds.origin.x.0, far.x.0] {
            for y in [bounds.origin.y.0, far.y.0] {
                let u = a*x+b*y+tx;
                let v = c*x+d*y+ty;
                lo.x.0 = lo.x.0.min(u);
                lo.y.0 = lo.y.0.min(v);
                hi.x.0 = hi.x.0.max(u);
                hi.y.0 = hi.y.0.max(v);
            }
        }
        Bounds::from_corners(lo,hi)
    }

    match primitive {
        Primitive::Shadow(shadow) if shadow.inset != 0 => shadow.element_bounds,
        Primitive::Shadow(shadow) => {
            // The Metal vertex shader expands a drop shadow by 3 sigma.
            let margin = ScaledPixels(3. * shadow.blur_radius.0.max(0.));
            let delta = point(margin,margin);
            Bounds::from_corners(shadow.bounds.origin-delta, shadow.bounds.bottom_right()+delta)
        }
        Primitive::MonochromeSprite(sprite) => transformed(sprite.bounds, &sprite.transformation),
        Primitive::SubpixelSprite(sprite) => transformed(sprite.bounds, &sprite.transformation),
        primitive => *primitive.bounds(),
    }
}

// Translate every device-space coordinate carried by a primitive, matching
// the fast fork's existing retained-layer translator.
fn translate_primitive(primitive: &Primitive, delta: Point<ScaledPixels>) -> Primitive {
    let mut p = primitive.clone();
    let mv = |b: &mut Bounds<ScaledPixels>| b.origin += delta;
    let to_local_transform = |t: &mut TransformationMatrix| {
        let r = t.rotation_scale;
        let (x, y) = (delta.x.0, delta.y.0);
        t.translation[0] += x - (r[0][0] * x + r[0][1] * y);
        t.translation[1] += y - (r[1][0] * x + r[1][1] * y);
    };
    match &mut p {
        Primitive::BackdropBlur(v) => {
            mv(&mut v.bounds);
            mv(&mut v.content_mask.bounds);
        }
        Primitive::Shadow(v) => {
            mv(&mut v.bounds);
            mv(&mut v.element_bounds);
            mv(&mut v.content_mask.bounds);
        }
        Primitive::Quad(v) => {
            mv(&mut v.bounds);
            mv(&mut v.content_mask.bounds);
        }
        Primitive::Path(v) => {
            mv(&mut v.bounds);
            mv(&mut v.content_mask.bounds);
            for vertex in &mut v.vertices {
                vertex.xy_position += delta;
                if !vertex.content_mask.bounds.is_empty() {
                    mv(&mut vertex.content_mask.bounds);
                }
            }
        }
        Primitive::Underline(v) => {
            mv(&mut v.bounds);
            mv(&mut v.content_mask.bounds);
        }
        Primitive::MonochromeSprite(v) => {
            mv(&mut v.bounds);
            mv(&mut v.content_mask.bounds);
            to_local_transform(&mut v.transformation);
        }
        Primitive::SubpixelSprite(v) => {
            mv(&mut v.bounds);
            mv(&mut v.content_mask.bounds);
            to_local_transform(&mut v.transformation);
        }
        Primitive::PolychromeSprite(v) => {
            mv(&mut v.bounds);
            mv(&mut v.content_mask.bounds);
        }
        Primitive::LayeredFill(v) => {
            mv(&mut v.bounds);
            mv(&mut v.content_mask.bounds);
        }
        Primitive::IsolatedGroup(v) => v.translate(delta),
        Primitive::Surface(v) => {
            mv(&mut v.bounds);
            mv(&mut v.content_mask.bounds);
        }
    }
    p
}
#[derive(Clone)]
/// One isolated source placement. Pixels are composed once after its children.
pub struct IsolatedGroup {
    /// Scene-assigned drawing order.
    pub order: DrawOrder,
    /// Paint bounds include the linear sampler footprint, not logical hit bounds.
    pub bounds: Bounds<ScaledPixels>,
    /// Rectangular clip in the destination's device-coordinate space.
    pub content_mask: ContentMask<ScaledPixels>,
    source: IsolatedGroupSource,
    transform: TransformationMatrix,
    inverse: TransformationMatrix,
    opacity: f32,
    grayscale: f32,
    alpha_ramp: [f32; 5],
    glyph_ink: [f32; 9],
    rounded_reveal: [f32; 6],
    parent_backdrop: Option<crate::BackdropBlur>,
    affine_passthrough: bool,
    renderable: bool,
}

impl std::fmt::Debug for IsolatedGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IsolatedGroup")
            .field("bounds", &self.bounds)
            .field("source_size", &self.source.size)
            .field("transform", &self.transform)
            .field("opacity", &self.opacity)
            .field("grayscale", &self.grayscale)
            .field("alpha_ramp", &self.alpha_ramp)
            .finish_non_exhaustive()
    }
}

impl IsolatedGroup {
    /// Validate a finite affine placement and opacity. Singular finite matrices are non-painting.
    pub fn new(
        source: IsolatedGroupSource,
        transform: TransformationMatrix,
        opacity: f32,
        content_mask: ContentMask<ScaledPixels>,
    ) -> Result<Self, &'static str> {
        if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
            return Err("invalid group opacity");
        }
        let [a, b] = transform.rotation_scale[0];
        let [c, d] = transform.rotation_scale[1];
        let [x, y] = transform.translation;
        if ![a, b, c, d, x, y].iter().all(|v| v.is_finite()) {
            return Err("non-finite group transform");
        }
        let det = f64::from(a) * f64::from(d) - f64::from(b) * f64::from(c);
        let renderable = det != 0.;
        if !det.is_finite() {
            return Err("invalid group determinant");
        }
        let [ia, ib, ic, id] = if renderable {
            [
                f64::from(d) / det,
                -f64::from(b) / det,
                -f64::from(c) / det,
                f64::from(a) / det,
            ]
            .map(|v| v as f32)
        } else {
            [1., 0., 0., 1.]
        };
        let inverse = TransformationMatrix {
            rotation_scale: [[ia, ib], [ic, id]],
            translation: [-(ia * x + ib * y), -(ic * x + id * y)],
        };
        if !inverse
            .rotation_scale
            .iter()
            .flatten()
            .chain(inverse.translation.iter())
            .all(|v| v.is_finite())
        {
            return Err("group inverse exceeds finite device coordinates");
        }
        // Include the linear sampler footprint in paint bounds, not hit bounds.
        let w = source.size.width.0 as f32;
        let h = source.size.height.0 as f32;
        let corners = [
            [-0.5, -0.5],
            [w + 0.5, -0.5],
            [-0.5, h + 0.5],
            [w + 0.5, h + 0.5],
        ]
        .map(|[u, v]| [a * u + b * v + x, c * u + d * v + y]);
        let left = corners.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
        let top = corners.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
        let right = corners
            .iter()
            .map(|p| p[0])
            .fold(f32::NEG_INFINITY, f32::max);
        let bottom = corners
            .iter()
            .map(|p| p[1])
            .fold(f32::NEG_INFINITY, f32::max);
        if ![left, top, right, bottom, right - left, bottom - top]
            .iter()
            .all(|v| v.is_finite())
        {
            return Err("group paint bounds exceed finite device coordinates");
        }
        let m = &content_mask.bounds;
        if ![m.origin.x.0, m.origin.y.0, m.size.width.0, m.size.height.0]
            .iter()
            .all(|v| v.is_finite())
            || m.size.width.0 < 0.0
            || m.size.height.0 < 0.0
        {
            return Err("invalid group clip");
        }
        Ok(Self {
            order: 0,
            bounds: Bounds {
                origin: point(ScaledPixels(left), ScaledPixels(top)),
                size: size(ScaledPixels(right - left), ScaledPixels(bottom - top)),
            },
            content_mask,
            source,
            transform,
            inverse,
            opacity,
            grayscale: 0.,
            alpha_ramp: [0.; 5],
            glyph_ink: [0.; 9],
            rounded_reveal: [0.; 6],
            parent_backdrop: None,
            affine_passthrough: false,
            renderable,
        })
    }
    /// Immutable source commands in their own viewport coordinates.
    pub fn source(&self) -> &Scene {
        &self.source.scene
    }

    /// Input from the parent target at this placement, in source-local
    /// device coordinates. It is not a command in the child's own scene.
    pub fn parent_backdrop(&self) -> Option<&crate::BackdropBlur> {
        self.parent_backdrop.as_ref()
    }

    pub(super) fn set_parent_backdrop(&mut self, read: crate::BackdropBlur) {
        self.parent_backdrop = Some(read);
    }

    /// A placement-only surface does not establish a new backdrop boundary.
    pub fn preserves_parent_backdrop(&self) -> bool {
        self.affine_passthrough
    }

    pub(super) fn preserve_parent_backdrop(&mut self) {
        self.affine_passthrough = true;
    }

    /// Whether painting this group must wait for the parent draw position.
    pub fn needs_live_parent(&self) -> bool {
        self.parent_backdrop.is_some()
            || self.affine_passthrough && self.source().has_backdrop_blur()
    }
    /// Share the source while changing placement without changing resource identity.
    pub fn source_handle(&self) -> IsolatedGroupSource {
        self.source.clone()
    }
    /// Physical raster viewport size.
    pub fn source_size(&self) -> Size<DevicePixels> {
        self.source.size
    }
    /// Foreground Gaussian standard deviation in source device pixels.
    pub fn source_blur_sigma(&self) -> f32 {
        self.source.blur_sigma
    }
    /// Affine mapping from source pixels to destination pixels.
    pub fn transform(&self) -> TransformationMatrix {
        self.transform
    }
    /// The inverse mapping; absent for collapsed/noninvertible placements.
    pub fn inverse(&self) -> Option<TransformationMatrix> {
        self.renderable.then_some(self.inverse)
    }
    /// Group opacity applied to the already-premultiplied subtree result.
    pub fn opacity(&self) -> f32 {
        self.opacity
    }
    /// Grayscale is a placement effect, not part of the reusable source pixels.
    pub fn grayscale(&self) -> f32 {
        self.grayscale
    }
    pub(super) fn set_grayscale(&mut self, amount: f32) {
        debug_assert!((0. ..=1.).contains(&amount));
        self.grayscale = amount;
    }
    /// Source-pixel origin and inverse width of the leading/trailing ramps,
    /// then 0 for Y or 1 for X. A zero inverse disables that edge. Placement
    /// can change its ramps without changing the source texture.
    pub fn alpha_ramps(&self) -> [f32; 5] {
        self.alpha_ramp
    }
    /// Premultiplied base RGBA, premultiplied highlight RGBA, and enabled.
    /// The source texture in ink mode stores monochrome atlas coverage.
    pub fn glyph_ink(&self) -> [f32; 9] {
        self.glyph_ink
    }
    /// Source-local left, top, width, height, radius, and enabled flag.
    pub fn rounded_reveal(&self) -> [f32; 6] {
        self.rounded_reveal
    }
    pub(crate) fn set_rounded_reveal(&mut self, [x,y,w,h,r]: [f32; 5]) -> Result<(), &'static str> {
        if [x,y,w,h,r].iter().any(|value| !value.is_finite())
            || w <= 0. || h <= 0. || !(0. ..=w.min(h)*0.5).contains(&r)
        {
            return Err("invalid rounded paint reveal");
        }
        self.rounded_reveal = [x,y,w,h,r,1.];
        Ok(())
    }
    pub(crate) fn set_glyph_ink(&mut self, base: crate::Hsla, highlight: crate::Hsla) -> Result<(), &'static str> {
        let a = base.to_rgb();
        let b = highlight.to_rgb();
        if [a.r, a.g, a.b, a.a, b.r, b.g, b.b, b.a].iter().any(|value| !value.is_finite()) {
            return Err("invalid glyph ink");
        }
        self.glyph_ink = [
            a.r * a.a, a.g * a.a, a.b * a.a, a.a,
            b.r * b.a, b.g * b.a, b.b * b.a, b.a,
            1.,
        ];
        Ok(())
    }
    pub(crate) fn set_alpha_ramps(&mut self, ramps: [f32; 5]) -> Result<(), &'static str> {
        if ramps.iter().any(|value| !value.is_finite())
            || ramps[1] < 0. || ramps[3] < 0. || ![0., 1.].contains(&ramps[4])
        {
            return Err("invalid group alpha ramp");
        }
        self.alpha_ramp = ramps;
        Ok(())
    }
    /// Whether this placement can produce pixels (opacity zero is skipped too).
    pub fn is_renderable(&self) -> bool {
        self.renderable && self.opacity > 0.
    }
    /// Identity is held strongly by every retained copy; an address cannot be
    /// recycled while a frame/cache still owns this immutable source.
    pub fn source_identity(&self) -> *const Scene {
        Rc::as_ptr(&self.source.scene)
    }
    /// Translate placement, clipping and inverse mapping together during retained replay.
    pub fn translate(&mut self, delta: crate::Point<ScaledPixels>) {
        self.bounds.origin += delta;
        self.content_mask.bounds.origin += delta;
        self.transform.translation[0] += delta.x.0;
        self.transform.translation[1] += delta.y.0;
        self.inverse.translation[0] -= self.inverse.rotation_scale[0][0] * delta.x.0
            + self.inverse.rotation_scale[0][1] * delta.y.0;
        self.inverse.translation[1] -= self.inverse.rotation_scale[1][0] * delta.x.0
            + self.inverse.rotation_scale[1][1] * delta.y.0;
    }
}

impl From<IsolatedGroup> for crate::Primitive {
    fn from(group: IsolatedGroup) -> Self {
        Self::IsolatedGroup(group)
    }
}

impl Scene {
    /// Whether this scene or a retained layer has a potentially drawable group.
    pub fn has_isolated_groups(&self) -> bool {
        if self
            .isolated_groups
            .iter()
            .any(IsolatedGroup::is_renderable)
        {
            return true;
        }
        // @HAS_LAYER_GROUPS@
        false
    }
    fn has_external_group_surfaces(&self) -> bool {
        if !self.surfaces.is_empty() {
            return true;
        }
        // @HAS_LAYER_SURFACES@
        false
    }
}
