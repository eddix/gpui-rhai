//! Preserve existing partial batching between ordered paint barriers.
use super::{DrawOrder, PaintOperation, PathId, Primitive, Scene};
fn order(p: &Primitive) -> DrawOrder {
    match p {
        Primitive::BackdropBlur(p) => p.order,
        Primitive::Shadow(p) => p.order,
        Primitive::Quad(p) => p.order,
        Primitive::Path(p) => p.order,
        Primitive::Underline(p) => p.order,
        Primitive::MonochromeSprite(p) => p.order,
        Primitive::SubpixelSprite(p) => p.order,
        Primitive::PolychromeSprite(p) => p.order,
        Primitive::Surface(p) => p.order,
        Primitive::IsolatedGroup(p) => p.order,
        Primitive::LayeredFill(p)=>p.order,
    }
}
fn assign(p: &mut Primitive, value: DrawOrder) {
    match p {
        Primitive::BackdropBlur(p) => p.order = value,
        Primitive::Shadow(p) => p.order = value,
        Primitive::Quad(p) => p.order = value,
        Primitive::Path(p) => p.order = value,
        Primitive::Underline(p) => p.order = value,
        Primitive::MonochromeSprite(p) => p.order = value,
        Primitive::SubpixelSprite(p) => p.order = value,
        Primitive::PolychromeSprite(p) => p.order = value,
        Primitive::Surface(p) => p.order = value,
        Primitive::IsolatedGroup(p) => p.order = value,
        Primitive::LayeredFill(p)=>p.order=value,
    }
}
fn span(operations: &mut [PaintOperation], start: usize, end: usize, base: DrawOrder) -> DrawOrder {
    let mut min = u32::MAX;
    let mut max = 0;
    let mut any = false;
    for op in &operations[start..end] {
        if let PaintOperation::Primitive(p) = op {
            let v = order(p);
            min = min.min(v);
            max = max.max(v);
            any = true;
        }
    }
    if !any {
        return base;
    }
    for op in &mut operations[start..end] {
        if let PaintOperation::Primitive(p) = op {
            assign(
                p,
                base.checked_add(order(p) - min)
                    .expect("group order overflow"),
            );
        }
    }
    base.checked_add(max - min)
        .and_then(|v| v.checked_add(1))
        .expect("group order overflow")
}
impl Scene {
    pub(crate) fn order_isolated_groups(&mut self) {
        if !self.has_isolated_groups() && !self.has_layered_fills() && !self.has_backdrop_blur() {
            return;
        }
        let compiled = self.isolated_draw_operations.is_some();
        let mut ops = if compiled {
            self.isolated_draw_operations.take().unwrap()
        } else {
            std::mem::take(&mut self.paint_operations)
        };
        let mut base = 0;
        let mut start = 0;
        for index in 0..ops.len() {
            let barrier = match &ops[index] {
                PaintOperation::Primitive(Primitive::BackdropBlur(_)) => true,
                PaintOperation::Primitive(Primitive::IsolatedGroup(group)) => group.is_renderable(),
                PaintOperation::Primitive(Primitive::PolychromeSprite(sprite)) => crate::fast::rounded_clip_scene::is_live_backdrop_barrier(self, sprite.tile.texture_id) || sprite
                    .tile
                    .texture_id
                    .index
                    .checked_sub(crate::fast::layers::scene::LAYER_TILE_TEXTURE_BASE)
                    .is_some_and(|key| {
                        self.layers
                            .frames
                            .iter()
                            .find(|frame| frame.key.0 == key)
                            .is_some_and(|frame| {
                                frame.content.scenes().any(|scene|scene.has_isolated_groups() || scene.has_layered_fills())
                            })
                    }),
                PaintOperation::Primitive(Primitive::LayeredFill(_))=>true,
                _ => false,
            };
            if barrier {
                base = span(&mut ops, start, index, base);
                if let PaintOperation::Primitive(p) = &mut ops[index] {
                    assign(p, base);
                }
                base = base.checked_add(1).expect("group order overflow");
                start = index + 1;
            }
        }
        let end = ops.len();
        span(&mut ops, start, end, base);
        self.shadows.clear();
        self.quads.clear();
        self.paths.clear();
        self.underlines.clear();
        self.monochrome_sprites.clear();
        self.subpixel_sprites.clear();
        self.polychrome_sprites.clear();
        self.surfaces.clear();
        self.isolated_groups.clear();
        self.layered_fills.clear();
        self.backdrop_blurs.clear();
        for operation in &mut ops {
            let PaintOperation::Primitive(p) = operation else {
                continue;
            };
            match p {
                Primitive::BackdropBlur(p) => self.backdrop_blurs.push(*p),
                Primitive::Shadow(p) => self.shadows.push(*p),
                Primitive::Quad(p) => self.quads.push(*p),
                Primitive::Path(p) => {
                    p.id = PathId(self.paths.len());
                    self.paths.push(p.clone());
                }
                Primitive::Underline(p) => self.underlines.push(*p),
                Primitive::MonochromeSprite(p) => self.monochrome_sprites.push(*p),
                Primitive::SubpixelSprite(p) => self.subpixel_sprites.push(*p),
                Primitive::PolychromeSprite(p) => self.polychrome_sprites.push(*p),
                Primitive::Surface(p) => self.surfaces.push(p.clone()),
                Primitive::IsolatedGroup(p) => self.isolated_groups.push(p.clone()),
                Primitive::LayeredFill(p)=>self.layered_fills.push(p.clone()),
            }
        }
        if compiled {
            self.isolated_draw_operations = Some(ops);
        } else {
            self.paint_operations = ops;
        }
    }
}
