//! Destination reads on the current command buffer. Scratch leases share the
//! group render budget/fence and are reused by all reads in a surface.
use super::*;
use gpui::{BackdropBlur, Corners};
use objc2_metal::{MTLBlitCommandEncoder, MTLOrigin, MTLSize};

type Texture = Retained<ProtocolObject<dyn MTLTexture>>;

/// Earlier drawing behind a placement-only surface. Valid only while
/// encoding the current draw position on the same command buffer.
pub(crate) struct BackdropAncestry<'a> {
    pub(super) target: &'a ProtocolObject<dyn MTLTexture>,
    pub(super) loss: Option<&'a ProtocolObject<dyn MTLTexture>>,
    pub(super) placement: gpui::TransformationMatrix,
    pub(super) outer: Option<&'a BackdropAncestry<'a>>,
}

impl BackdropAncestry<'_> {
    fn projection_count(&self) -> u64 {
        let mut count = 1; // the current surface, before ancestors
        let mut ancestor = Some(self);
        while let Some(layer) = ancestor {
            count += 1;
            ancestor = layer.outer;
        }
        count
    }
}

struct ProjectedInput<'a> {
    texture: &'a ProtocolObject<dyn MTLTexture>,
    loss: Option<&'a ProtocolObject<dyn MTLTexture>>,
    matrix: [f32; 6],
}

#[repr(C)]
pub(crate) struct BackdropUniforms {
    bounds: Bounds<ScaledPixels>,
    copy_bounds: Bounds<ScaledPixels>,
    clip_bounds: Bounds<ScaledPixels>,
    corner_radii: Corners<ScaledPixels>,
    viewport: Size<DevicePixels>,
    strength: f32,
    pad: f32,
}

pub(super) struct Program {
    composite: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    copy: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    project_parent: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    seed: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
}

pub(super) struct Domain {
    original: Texture,
    horizontal: Texture,
    filtered: Texture,
    /// A CAMetalDrawable may forbid reads even though render passes can
    /// write it. Offscreen/group textures can be read directly.
    pub(super) staging: Option<Texture>,
}

#[derive(Default)]
struct Plan {
    width: usize,
    height: usize,
    samples: u64,
    passes: usize,
}

#[derive(Clone, Copy)]
struct Region {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Region {
    fn extent(self) -> Size<DevicePixels> {
        size(DevicePixels(self.width as i32), DevicePixels(self.height as i32))
    }

    fn projected_inputs<'a>(
        self,
        mut input: &'a ProtocolObject<dyn MTLTexture>,
        mut loss: Option<&'a ProtocolObject<dyn MTLTexture>>,
        mut pose: gpui::TransformationMatrix,
        mut ancestry: Option<&'a BackdropAncestry<'a>>,
    ) -> Result<Vec<ProjectedInput<'a>>> {
        let offset = [self.x as f32, self.y as f32];
        let mut projected = Vec::new();
        loop {
            anyhow::ensure!(!input.isFramebufferOnly(), "parent read requires a readable color target");
            let [[a, b], [c, d]] = pose.rotation_scale;
            let matrix = [
                a, b, pose.translation[0] + a * offset[0] + b * offset[1],
                c, d, pose.translation[1] + c * offset[0] + d * offset[1],
            ];
            anyhow::ensure!(matrix.iter().all(|value| value.is_finite()), "non-finite backdrop projection");
            projected.push(ProjectedInput { texture: input, loss, matrix });
            let Some(outer) = ancestry else { return Ok(projected) };
            pose = outer.placement.compose(pose);
            input = outer.target;
            loss = outer.loss;
            ancestry = outer.outer;
        }
    }

    fn of(blur: &BackdropBlur, viewport: Size<DevicePixels>) -> Result<Option<Self>> {
        anyhow::ensure!(
            (0.0..=128.0).contains(&blur.sigma),
            "invalid backdrop sigma"
        );
        anyhow::ensure!(
            (0.0..=1.0).contains(&blur.strength),
            "invalid backdrop strength"
        );
        let bounds = blur.bounds;
        let mask = blur.content_mask.bounds;
        anyhow::ensure!(
            [
                bounds.origin.x.0,
                bounds.origin.y.0,
                bounds.right().0,
                bounds.bottom().0,
                mask.origin.x.0,
                mask.origin.y.0,
                mask.right().0,
                mask.bottom().0,
            ]
            .into_iter()
            .all(f32::is_finite),
            "invalid backdrop bounds"
        );
        let radii = blur.corner_radii;
        anyhow::ensure!(
            [
                radii.top_left.0,
                radii.top_right.0,
                radii.bottom_right.0,
                radii.bottom_left.0,
            ]
            .into_iter()
            .all(|r| r.is_finite() && r >= 0.),
            "invalid backdrop radii"
        );
        if blur.sigma == 0. || blur.strength == 0. || bounds.intersect(&mask).is_empty() {
            return Ok(None);
        }
        let w = viewport.width.0.max(0) as f32;
        let h = viewport.height.0.max(0) as f32;
        let x = bounds.origin.x.0.floor().clamp(0., w) as usize;
        let y = bounds.origin.y.0.floor().clamp(0., h) as usize;
        let right = bounds.right().0.ceil().clamp(0., w) as usize;
        let bottom = bounds.bottom().0.ceil().clamp(0., h) as usize;
        Ok((right > x && bottom > y).then(|| Self {
            x,
            y,
            width: right - x,
            height: bottom - y,
        }))
    }
}

impl Plan {
    fn include(&mut self, scene: &Scene, viewport: Size<DevicePixels>, projections: u64) -> Result<()> {
        for blur in &scene.backdrop_blurs {
            let Some(r) = Region::of(blur, viewport)? else {
                continue;
            };
            self.width = self.width.max(r.width);
            self.height = self.height.max(r.height);
            let pairs = ((4. * blur.sigma).ceil() as u64).div_ceil(2);
            self.samples = self.samples.saturating_add(
                (r.width as u64)
                    .saturating_mul(r.height as u64)
                    .saturating_mul(projections.saturating_add(2 * (1 + 2 * pairs))),
            );
            self.passes += 1;
        }
        for frame in &scene.layers.frames {
            for child in frame.content.scenes() {
                if frame.tile_size == 0 {
                    self.include(child, viewport, projections)?;
                } else {
                    anyhow::ensure!(
                        !child.has_backdrop_blur(),
                        "backdrop reads cannot be encoded in independent scroll tiles"
                    );
                }
            }
        }
        Ok(())
    }
}

impl Program {
    pub(super) fn new(
        device: &ProtocolObject<dyn MTLDevice>,
        library: &ProtocolObject<dyn MTLLibrary>,
    ) -> Result<Self> {
        let pipeline = |vertex, fragment, premultiplied_blend| {
            let vertex = library
                .newFunctionWithName(&NSString::from_str(vertex))
                .context("backdrop vertex function")?;
            let fragment = library
                .newFunctionWithName(&NSString::from_str(fragment))
                .context("backdrop fragment function")?;
            let desc = MTLRenderPipelineDescriptor::new();
            desc.setVertexFunction(Some(&vertex));
            desc.setFragmentFunction(Some(&fragment));
            // Projections accumulate oldest to newest. A placed read draws
            // its filtered overlay once; a direct read replaces its target.
            let color = unsafe { desc.colorAttachments().objectAtIndexedSubscript(0) };
            color.setPixelFormat(MTLPixelFormat::BGRA8Unorm);
            color.setBlendingEnabled(premultiplied_blend);
            if premultiplied_blend {
                color.setSourceRGBBlendFactor(MTLBlendFactor::One);
                color.setSourceAlphaBlendFactor(MTLBlendFactor::One);
                color.setDestinationRGBBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
                color.setDestinationAlphaBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
            }
            device
                .newRenderPipelineStateWithDescriptor_error(&desc)
                .context("creating backdrop pipeline")
        };
        Ok(Self {
            composite: pipeline("backdrop_rect_vertex", "backdrop_composite_fragment", false)?,
            copy: pipeline("group_gaussian_vertex", "backdrop_copy_fragment", false)?,
            project_parent: pipeline("group_gaussian_vertex", "backdrop_project_parent_fragment", true)?,
            seed: pipeline("backdrop_rect_vertex", "backdrop_seed_fragment", false)?,
        })
    }

    fn project(
        &self,
        commands: &ProtocolObject<dyn MTLCommandBuffer>,
        destination: &ProtocolObject<dyn MTLTexture>,
        region: Region,
        projected: &[ProjectedInput<'_>],
        exact: Option<&transmission::Program>,
    ) -> Result<()> {
        anyhow::ensure!(
            exact.is_some() || projected.iter().all(|input| input.loss.is_none()),
            "missing projection for affine transmission"
        );
        let clear = MTLClearColor { red: 0., green: 0., blue: 0., alpha: 0. };
        let encoder = new_command_encoder_for_texture(commands, destination, region.extent(), Some(clear))?;
        for input in projected.iter().rev() {
            if let (Some(loss), Some(exact)) = (input.loss, exact) {
                encoder.setRenderPipelineState(&exact.project);
                unsafe { encoder.setFragmentTexture_atIndex(Some(loss), 1); }
            } else {
                encoder.setRenderPipelineState(&self.project_parent);
            }
            // The affine packet is copied into the retained command stream.
            unsafe {
                encoder.setFragmentBytes_length_atIndex(ptr::NonNull::from(&input.matrix).cast(), mem::size_of_val(&input.matrix), 0);
                encoder.setFragmentTexture_atIndex(Some(input.texture), 0);
                encoder.drawPrimitives_vertexStart_vertexCount(MTLPrimitiveType::Triangle, 0, 3);
            }
        }
        encoder.endEncoding();
        Ok(())
    }

    pub(super) fn copy(
        &self,
        commands: &ProtocolObject<dyn MTLCommandBuffer>,
        source: &ProtocolObject<dyn MTLTexture>,
        destination: &ProtocolObject<dyn MTLTexture>,
        viewport: Size<DevicePixels>,
    ) -> Result<()> {
        let encoder = new_command_encoder_for_texture(
            commands,
            destination,
            viewport,
            Some(MTLClearColor {
                red: 0.,
                green: 0.,
                blue: 0.,
                alpha: 0.,
            }),
        )?;
        encoder.setRenderPipelineState(&self.copy);
        unsafe {
            encoder.setFragmentTexture_atIndex(Some(source), 0);
            encoder.drawPrimitives_vertexStart_vertexCount(MTLPrimitiveType::Triangle, 0, 3);
        }
        encoder.endEncoding();
        Ok(())
    }
}

impl MetalRenderer {
    pub(super) fn prepare_backdrop_domain(
        &mut self,
        scene: &Scene,
        destination: &ProtocolObject<dyn MTLTexture>,
        viewport: Size<DevicePixels>,
        ancestry: Option<&BackdropAncestry<'_>>,
    ) -> Result<Option<Domain>> {
        if !scene.has_backdrop_blur() {
            return Ok(None);
        }
        let mut plan = Plan::default();
        plan.include(scene, viewport, ancestry.map_or(0, BackdropAncestry::projection_count))?;
        if plan.passes == 0 && !destination.isFramebufferOnly() {
            return Ok(None);
        }
        self.reserve_backdrop_work(plan.passes, plan.samples)?;
        let size = size(
            DevicePixels(plan.width.max(1) as i32),
            DevicePixels(plan.height.max(1) as i32),
        );
        Ok(Some(Domain {
            original: self.group_texture(size, MTLPixelFormat::BGRA8Unorm)?,
            horizontal: self.group_texture(size, MTLPixelFormat::RGBA16Float)?,
            filtered: self.group_texture(size, MTLPixelFormat::RGBA16Float)?,
            staging: if destination.isFramebufferOnly() {
                Some(self.group_texture(viewport, MTLPixelFormat::BGRA8Unorm)?)
            } else {
                None
            },
        }))
    }

    /// Sample the current parent before this draw, and clear/seed the
    /// group surface in its own pixel domain. No encoder may be active.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn seed_parent_backdrop(
        &mut self,
        commands: &ProtocolObject<dyn MTLCommandBuffer>,
        parent: &ProtocolObject<dyn MTLTexture>,
        loss: Option<&ProtocolObject<dyn MTLTexture>>,
        target: &ProtocolObject<dyn MTLTexture>,
        viewport: Size<DevicePixels>,
        group: &gpui::IsolatedGroup,
        read: &BackdropBlur,
        ancestry: Option<&BackdropAncestry<'_>>,
    ) -> Result<bool> {
        let Some(region) = Region::of(read, viewport)? else { return Ok(false) };
        let offset = [region.x as f32, region.y as f32];
        let projected = region.projected_inputs(parent, loss, group.transform(), ancestry)?;
        let pairs = ((4. * read.sigma).ceil() as u64).div_ceil(2);
        let samples = (region.width as u64).saturating_mul(region.height as u64)
            .saturating_mul((projected.len() as u64).saturating_add(2 * (1 + 2 * pairs)));
        self.reserve_backdrop_work(1, samples)?;
        let sample_extent = region.extent();
        let original = self.group_texture(sample_extent, MTLPixelFormat::BGRA8Unorm)?;
        let horizontal = self.group_texture(sample_extent, MTLPixelFormat::RGBA16Float)?;
        let filtered = self.group_texture(sample_extent, MTLPixelFormat::RGBA16Float)?;
        let clear = MTLClearColor { red: 0., green: 0., blue: 0., alpha: 0. };
        self.backdrop.project(commands, &original, region, &projected, self.transmission_program.as_ref().ok())?;
        self.gaussian.encode(commands, &original, &horizontal, &filtered,
            [region.width as u32, region.height as u32], read.sigma, true)?;
        let uniforms = BackdropUniforms {
            bounds: read.bounds,
            copy_bounds: Bounds::new(
                point(ScaledPixels(offset[0]), ScaledPixels(offset[1])),
                size(ScaledPixels(region.width as f32), ScaledPixels(region.height as f32)),
            ),
            clip_bounds: read.content_mask.bounds,
            corner_radii: read.corner_radii,
            viewport,
            strength: read.strength,
            pad: 0.,
        };
        let encoder = new_command_encoder_for_texture(commands, target, viewport, Some(clear))?;
        encoder.setRenderPipelineState(&self.backdrop.seed);
        // The rounded seed only changes the clipped source region.
        unsafe {
            encoder.setVertexBytes_length_atIndex(ptr::NonNull::from(&uniforms).cast(), mem::size_of_val(&uniforms), 0);
            encoder.setFragmentBytes_length_atIndex(ptr::NonNull::from(&uniforms).cast(), mem::size_of_val(&uniforms), 0);
            encoder.setFragmentTexture_atIndex(Some(&filtered), 0);
            encoder.drawPrimitives_vertexStart_vertexCount(MTLPrimitiveType::Triangle, 0, 6);
        }
        encoder.endEncoding();
        Ok(true)
    }

    pub(crate) fn draw_backdrop_blur(
        &mut self,
        commands: &ProtocolObject<dyn MTLCommandBuffer>,
        target: &ProtocolObject<dyn MTLTexture>,
        viewport: Size<DevicePixels>,
        blur: &BackdropBlur,
        ancestry: Option<&BackdropAncestry<'_>>,
        bind_mask: impl FnOnce(&Self, &ProtocolObject<dyn MTLRenderCommandEncoder>),
    ) -> Result<()> {
        let Some(r) = Region::of(blur, viewport)? else {
            return Ok(());
        };
        anyhow::ensure!(
            ancestry.is_some() == self.transmission.is_some(),
            "placed backdrop lost its affine transmission scope"
        );
        if ancestry.is_some() {
            self.finish_transmission_chunk(commands, target, true)?;
            self.reserve_transmission_pass(viewport)?;
        }
        let domain = self
            .backdrop_domain
            .as_ref()
            .context("missing backdrop scratch lease")?;
        if let Some(transport) = &self.transmission {
            let head = &transport.front;
            let projected = r.projected_inputs(
                &head.color, Some(&head.loss), gpui::TransformationMatrix::unit(), ancestry,
            )?;
            self.backdrop.project(commands, &domain.original, r, &projected, Some(self.exact_transmission()?))?;
        } else {
            let copy = commands
                .blitCommandEncoder()
                .context("copying current backdrop")?;
            // Source coordinates are inside the destination. Domain planning
            // allocated enough scratch for all reads, in the same pixel format.
            unsafe {
                copy.copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(
                    target, 0, 0, MTLOrigin {x:r.x, y:r.y, z:0}, MTLSize {width:r.width, height:r.height, depth:1},
                    &domain.original, 0, 0, MTLOrigin {x:0, y:0, z:0}
                );
            }
            copy.endEncoding();
        }
        self.gaussian.encode(
            commands,
            &domain.original,
            &domain.horizontal,
            &domain.filtered,
            [r.width as u32, r.height as u32],
            blur.sigma,
            true,
        )?;
        let uniforms = BackdropUniforms {
            bounds: blur.bounds,
            copy_bounds: Bounds {
                origin: point(ScaledPixels(r.x as f32), ScaledPixels(r.y as f32)),
                size: size(ScaledPixels(r.width as f32), ScaledPixels(r.height as f32)),
            },
            clip_bounds: blur.content_mask.bounds,
            corner_radii: blur.corner_radii,
            viewport,
            strength: blur.strength,
            pad: 0.,
        };
        if let Some(mut transport) = self.transmission.take() {
            let result = (|| {
                transport.absorb(commands, &self.exact_transmission()?.replace_backdrop, |encoder| {
                    bind_mask(self, encoder);
                    transmission::fragment_value(encoder, &uniforms, 0);
                    unsafe { encoder.setFragmentTexture_atIndex(Some(&domain.filtered), 2); }
                })
            })();
            self.transmission = Some(transport);
            return result;
        }
        let encoder = new_command_encoder_for_texture(commands, target, viewport, None)?;
        encoder.setRenderPipelineState(&self.backdrop.composite);
        bind_mask(self, &encoder);
        unsafe {
            encoder.setVertexBytes_length_atIndex(
                ptr::NonNull::from(&uniforms).cast(),
                mem::size_of_val(&uniforms),
                0,
            );
            encoder.setFragmentBytes_length_atIndex(
                ptr::NonNull::from(&uniforms).cast(),
                mem::size_of_val(&uniforms),
                0,
            );
            encoder.setFragmentTexture_atIndex(Some(&domain.original), 0);
            encoder.setFragmentTexture_atIndex(Some(&domain.filtered), 1);
            encoder.drawPrimitives_vertexStart_vertexCount(MTLPrimitiveType::Triangle, 0, 6);
        }
        encoder.endEncoding();
        Ok(())
    }
}
