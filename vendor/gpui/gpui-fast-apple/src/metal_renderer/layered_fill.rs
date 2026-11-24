//! Shared pooled-instance-buffer upload and premultiplied layered fill draw.
use super::*;
use gpui::{BACKGROUND_LAYER_CAP, BACKGROUND_STOP_CAP, LayeredFill};
#[repr(C, align(16))]
#[derive(Clone, Copy, Default)]
pub(super) struct Stop {
    color: [f32; 4],
    position: [f32; 4],
}
#[repr(C, align(16))]
#[derive(Clone, Copy, Default)]
pub(super) struct Layer {
    origin: [f32; 4],
    clip: [f32; 4],
    radii_x: [f32; 4],
    radii_y: [f32; 4],
    info: [f32; 4],
    stops: [Stop; BACKGROUND_STOP_CAP],
}
#[repr(C, align(16))]
#[derive(Clone, Copy, Default)]
pub(super) struct Packet {
    bounds: [f32; 4],
    mask: [f32; 4],
    info: [f32; 4],
    pad: [f32; 4],
    layers: [Layer; BACKGROUND_LAYER_CAP],
}
fn rect(b: Bounds<ScaledPixels>) -> [f32; 4] {
    [b.origin.x.0, b.origin.y.0, b.size.width.0, b.size.height.0]
}
fn corners(c: gpui::Corners<ScaledPixels>) -> [f32; 4] {
    [
        c.top_left.0,
        c.top_right.0,
        c.bottom_right.0,
        c.bottom_left.0,
    ]
}
impl Packet {
    pub(super) fn encode(fill: &LayeredFill) -> Result<Self> {
        anyhow::ensure!(
            fill.layers.len() <= BACKGROUND_LAYER_CAP,
            "too many background layers"
        );
        anyhow::ensure!(
            fill.opacity.is_finite() && (0. ..=1.).contains(&fill.opacity),
            "invalid background opacity"
        );
        let mut out = Self {
            bounds: rect(fill.bounds),
            mask: rect(fill.content_mask.bounds),
            info: [fill.layers.len() as f32, fill.opacity, 0., 0.],
            ..Default::default()
        };
        for (i, l) in fill.layers.iter().enumerate() {
            anyhow::ensure!(
                (2..=BACKGROUND_STOP_CAP).contains(&l.stops.len()),
                "invalid background stop count"
            );
            let origin = rect(l.origin);
            let clip = rect(l.clip);
            anyhow::ensure!(
                origin.iter().chain(&clip).all(|v| v.is_finite()),
                "nonfinite background geometry"
            );
            let span = l.direction.x.abs() * origin[2] + l.direction.y.abs() * origin[3];
            anyhow::ensure!(
                span.is_normal()
                    && span > 0.
                    && origin[2] > 0.
                    && origin[3] > 0.
                    && clip[2] > 0.
                    && clip[3] > 0.,
                "invalid background projection"
            );
            anyhow::ensure!(
                corners(l.radii_x)
                    .iter()
                    .chain(corners(l.radii_y).iter())
                    .all(|v| v.is_finite() && *v >= 0.),
                "invalid background radii"
            );
            let period = l.repeat_period.map_or(0., |period| period.0);
            anyhow::ensure!(period == 0. || (period > 0. && period.is_normal()), "invalid repeating background projection");
            let mut layer = Layer {
                origin,
                clip,
                radii_x: corners(l.radii_x),
                radii_y: corners(l.radii_y),
                info: [l.direction.x, l.direction.y, l.stops.len() as f32, period],
                ..Default::default()
            };
            for (j, s) in l.stops.iter().enumerate() {
                let c = s.color;
                anyhow::ensure!(
                    [c.r, c.g, c.b, c.a, s.offset].iter().all(|v| v.is_finite()),
                    "nonfinite background stop"
                );
                layer.stops[j] = Stop {
                    color: [c.r * c.a, c.g * c.a, c.b * c.a, c.a],
                    position: [s.offset, 0., 0., 0.],
                };
            }
            out.layers[i] = layer;
        }
        Ok(out)
    }
}
pub(super) fn build_pipeline(
    device: &ProtocolObject<dyn MTLDevice>,
    library: &ProtocolObject<dyn MTLLibrary>,
) -> Result<Retained<ProtocolObject<dyn MTLRenderPipelineState>>> {
    build_path_sprite_pipeline_state(
        device,
        library,
        "layered_background",
        "layered_background_vertex",
        "layered_background_fragment",
        MTLPixelFormat::BGRA8Unorm,
    )
}
impl MetalRenderer {
    pub(crate) fn draw_layered_fills(
        &self,
        range: Range<usize>,
        bindings: &InstanceBindings,
        viewport: Size<DevicePixels>,
        encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>,
    ) {
        if range.is_empty() {
            return;
        }
        encoder.setRenderPipelineState(&self.layered_fill_pipeline_state);
        // SAFETY: the frame writer owns initialized, correctly aligned packets
        // until its ordinary Metal completion fence. These bindings use the
        // same pooled buffer and base-instance convention as native quads.
        unsafe {
            encoder.setVertexBuffer_offset_atIndex(Some(&self.unit_vertices), 0, 0);
            encoder.setVertexBuffer_offset_atIndex(
                Some(&bindings.layered_fills.buffer),
                bindings.layered_fills.offset,
                1,
            );
            encoder.setFragmentBuffer_offset_atIndex(
                Some(&bindings.layered_fills.buffer),
                bindings.layered_fills.offset,
                1,
            );
            encoder.setVertexBytes_length_atIndex(
                ptr::NonNull::from(&viewport).cast(),
                mem::size_of_val(&viewport),
                2,
            );
            encoder.drawPrimitives_vertexStart_vertexCount_instanceCount_baseInstance(
                MTLPrimitiveType::Triangle,
                0,
                6,
                range.len(),
                range.start,
            );
        }
    }
}
