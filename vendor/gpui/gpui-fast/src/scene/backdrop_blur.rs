//! Same-frame destination read in the current compositing surface.
use crate::{Bounds, ContentMask, Corners, DrawOrder, Primitive, ScaledPixels, Scene};

/// Blur a rectangle of the current destination, without sampling later paint.
#[derive(Clone, Copy, Debug, Default)]
pub struct BackdropBlur {
    /// Scene-assigned draw order.
    pub order: DrawOrder,
    /// The border box, in device pixels. Independent of its visible clip.
    pub bounds: Bounds<ScaledPixels>,
    /// The rectangular output clip in the destination coordinate space.
    pub content_mask: ContentMask<ScaledPixels>,
    /// Rounded border-box geometry in device pixels.
    pub corner_radii: Corners<ScaledPixels>,
    /// Gaussian standard deviation in device pixels.
    pub sigma: f32,
    /// Replacement fraction between original and blurred destination pixels.
    pub strength: f32,
}

impl From<BackdropBlur> for Primitive {
    fn from(value: BackdropBlur) -> Self {
        Self::BackdropBlur(value)
    }
}

impl Scene {
    /// Ordinary replacements cross placement-only wrappers, not explicit
    /// opacity/filter isolation. Unlike a read/seed, replacement may lower alpha.
    pub fn has_backdrop_replacements(&self) -> bool {
        !self.backdrop_blurs.is_empty()
            || self.isolated_groups.iter().any(|group| {
                group.preserves_parent_backdrop() && group.source().has_backdrop_replacements()
            })
            || self.layers.frames.iter()
                .filter(|frame| frame.tile_size == 0)
                .flat_map(|frame| frame.content.scenes())
                .any(Scene::has_backdrop_replacements)
    }

    /// Whether an encoder for this surface must support live destination reads.
    /// Independently prepared opacity/foreground sources have their own surface.
    pub fn has_backdrop_blur(&self) -> bool {
        !self.backdrop_blurs.is_empty()
            || self.isolated_groups.iter().any(crate::IsolatedGroup::needs_live_parent)
            || self.layers.frames.iter()
            .filter(|frame| frame.tile_size == 0)
            .flat_map(|frame| frame.content.scenes())
            .any(Scene::has_backdrop_blur)
    }
}
