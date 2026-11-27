//! Same-frame isolated scene preparation. No CPU pixel readback in this path.
use super::*;
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicU8, AtomicUsize, Ordering},
};

#[derive(Default)]
struct FrameFence {
    phase: AtomicU8,
    preparations: AtomicUsize,
}
impl FrameFence {
    fn ready(&self) -> bool {
        self.phase.load(Ordering::Acquire) >= 2 && self.preparations.load(Ordering::Acquire) == 0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GroupBudget {
    pub max_pool_bytes: u64,
    pub max_groups: usize,
    pub max_depth: usize,
    pub max_scene_visits: usize,
    /// Upper bound on estimated Gaussian sampler operations per frame.
    pub max_blur_samples: u64,
}
impl Default for GroupBudget {
    fn default() -> Self {
        Self {
            max_pool_bytes: 64 * 1024 * 1024,
            max_groups: 512,
            max_depth: 32,
            max_scene_visits: 4096,
            max_blur_samples: 256 * 1024 * 1024,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct GroupStats {
    pub pool_bytes: u64,
    pub pool_slots: usize,
    pub allocations: u64,
    pub reuses: u64,
    pub prepared: usize,
    pub path_allocations: u64,
}
struct Slot {
    texture: Retained<ProtocolObject<dyn MTLTexture>>,
    fence: Arc<FrameFence>,
    size: Size<DevicePixels>,
    format: MTLPixelFormat,
    bytes: u64,
}
#[derive(Default)]
pub(super) struct State {
    budget: GroupBudget,
    pipeline: Option<Retained<ProtocolObject<dyn MTLRenderPipelineState>>>,
    pool: Vec<Slot>,
    prepared: BTreeMap<usize, Retained<ProtocolObject<dyn MTLTexture>>>,
    fence: Option<Arc<FrameFence>>,
    allocations: u64,
    reuses: u64,
    group_visits: usize,
    scene_visits: usize,
    blur_samples: u64,
    backdrop_reads: usize,
    pub(super) path_allocations: u64,
}

#[repr(C)]
pub struct GroupUniforms {
    bounds: Bounds<ScaledPixels>,
    content_mask: ContentMask<ScaledPixels>,
    inverse: gpui::TransformationMatrix,
    source_size: Size<DevicePixels>,
    opacity: f32,
    grayscale: f32,
    alpha_ramp: [f32; 5],
    glyph_ink: [f32; 9],
    rounded_reveal: [f32; 6],
}

impl GroupUniforms {
    pub(super) fn placement(group: &gpui::IsolatedGroup) -> Result<Self> {
        Ok(Self {
            bounds: group.bounds,
            content_mask: group.content_mask,
            inverse: group.inverse().context("non-invertible group reached shader")?,
            source_size: group.source_size(),
            opacity: group.opacity(),
            grayscale: group.grayscale(),
            alpha_ramp: group.alpha_ramps(),
            glyph_ink: group.glyph_ink(),
            rounded_reveal: group.rounded_reveal(),
        })
    }
}

impl MetalRenderer {
    pub fn set_group_budget(&mut self, budget: GroupBudget) -> Result<()> {
        anyhow::ensure!(
            budget.max_depth <= 64,
            "group depth cap exceeds supported recursion bound"
        );
        self.isolated_groups.budget = budget;
        self.trim_idle_group_textures();
        Ok(())
    }
    pub fn trim_idle_group_textures(&mut self) {
        self.isolated_groups.pool.retain(|slot| !slot.fence.ready());
    }
    pub fn group_stats(&self) -> GroupStats {
        let state = &self.isolated_groups;
        GroupStats {
            pool_bytes: state.pool.iter().map(|slot| slot.bytes).sum(),
            pool_slots: state.pool.len(),
            allocations: state.allocations,
            reuses: state.reuses,
            prepared: state.prepared.len(),
            path_allocations: state.path_allocations,
        }
    }
    pub fn group_busy_slots(&self) -> usize {
        self.isolated_groups
            .pool
            .iter()
            .filter(|slot| !slot.fence.ready())
            .count()
    }
    pub(super) fn render_frame(
        &mut self,
        scene: &Scene,
        texture: &ProtocolObject<dyn MTLTexture>,
        viewport: Size<DevicePixels>,
    ) -> Result<Retained<ProtocolObject<dyn MTLCommandBuffer>>> {
        self.isolated_groups.prepared.clear();
        self.isolated_groups.fence = None;
        self.isolated_groups.group_visits = 0;
        self.isolated_groups.scene_visits = 0;
        self.isolated_groups.blur_samples = 0;
        self.isolated_groups.backdrop_reads = 0;
        if !scene.has_isolated_groups() && !scene.has_backdrop_blur() {
            return self.render_frame_raw(scene, texture, viewport);
        }
        let fence = Arc::new(FrameFence::default());
        self.isolated_groups.fence = Some(fence.clone());
        let result = (|| {
            self.prepare_isolated_scene(scene, 0)?;
            // Child passes may have different path intermediate dimensions.
            if scene_needs_paths(scene) {
                self.update_path_intermediate_textures(viewport)?;
            }
            self.render_frame_raw(scene, texture, viewport)
        })();
        let keepalive = self
            .isolated_groups
            .prepared
            .values()
            .cloned()
            .collect::<Vec<_>>();
        if let Ok(commands) = &result {
            fence.phase.store(1, Ordering::Release);
            let held = Cell::new(Some(keepalive));
            let callback = RcBlock::new(
                move |_: ptr::NonNull<ProtocolObject<dyn MTLCommandBuffer>>| {
                    let _textures = held.take();
                    fence.phase.store(2, Ordering::Release);
                },
            );
            // The copied completion owns every group texture until the last
            // reader finishes; only atomic readiness crosses the callback.
            unsafe {
                commands.addCompletedHandler(RcBlock::as_ptr(&callback));
            }
        } else {
            fence.phase.store(3, Ordering::Release);
        }
        self.isolated_groups.fence = None;
        result
    }

    pub(super) fn reserve_backdrop_work(&mut self, passes: usize, samples: u64) -> Result<()> {
        let state = &mut self.isolated_groups;
        state.backdrop_reads = state.backdrop_reads.saturating_add(passes);
        state.blur_samples = state.blur_samples.saturating_add(samples);
        anyhow::ensure!(state.backdrop_reads <= state.budget.max_groups,
            "backdrop read budget exceeded");
        anyhow::ensure!(state.blur_samples <= state.budget.max_blur_samples,
            "shared blur sample budget exceeded");
        Ok(())
    }

    pub(super) fn group_texture(
        &mut self,
        size: Size<DevicePixels>,
        format: MTLPixelFormat,
    ) -> Result<Retained<ProtocolObject<dyn MTLTexture>>> {
        let state = &mut self.isolated_groups;
        let bytes = (size.width.0 as u64)
            .checked_mul(size.height.0 as u64)
            .and_then(|v| v.checked_mul(match format {
                MTLPixelFormat::RGBA16Float => 8,
                MTLPixelFormat::R16Float => 2,
                _ => 4,
            }))
            .context("group byte size overflow")?;
        anyhow::ensure!(
            bytes <= state.budget.max_pool_bytes,
            "isolated group source exceeds texture budget"
        );
        let mut held = state.pool.iter().map(|slot| slot.bytes).sum::<u64>();
        state.pool.retain(|slot| {
            if held > state.budget.max_pool_bytes && slot.fence.ready() {
                held -= slot.bytes;
                false
            } else {
                true
            }
        });
        anyhow::ensure!(
            held <= state.budget.max_pool_bytes,
            "group budget held by in-flight frames"
        );
        if let Some(slot) = state
            .pool
            .iter_mut()
            .find(|slot| slot.size == size && slot.format == format && slot.fence.ready())
        {
            slot.fence = state
                .fence
                .as_ref()
                .context("group frame fence missing")?
                .clone();
            state.reuses += 1;
            return Ok(slot.texture.clone());
        }
        // Evict idle mismatched slots, never a texture an earlier frame reads.
        let mut total = state.pool.iter().map(|slot| slot.bytes).sum::<u64>();
        state.pool.retain(|slot| {
            if total.saturating_add(bytes) > state.budget.max_pool_bytes && slot.fence.ready() {
                total -= slot.bytes;
                false
            } else {
                true
            }
        });
        anyhow::ensure!(
            total
                .checked_add(bytes)
                .is_some_and(|v| v <= state.budget.max_pool_bytes),
            "isolated group texture budget exceeded"
        );
        let desc = MTLTextureDescriptor::new();
        unsafe {
            desc.setWidth(size.width.0 as usize);
            desc.setHeight(size.height.0 as usize);
        }
        desc.setPixelFormat(format);
        desc.setStorageMode(MTLStorageMode::Private);
        desc.setUsage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
        let texture = self
            .device
            .newTextureWithDescriptor(&desc)
            .context("allocating isolated group texture")?;
        let fence = state
            .fence
            .as_ref()
            .context("group frame fence missing")?
            .clone();
        state.pool.push(Slot {
            texture: texture.clone(),
            fence,
            size,
            format,
            bytes,
        });
        state.allocations += 1;
        Ok(texture)
    }

    fn foreground_scratch(
        &mut self, extent: Size<DevicePixels>, sigma: f32,
    ) -> Result<Option<Retained<ProtocolObject<dyn MTLTexture>>>> {
        if sigma <= 0. {
            return Ok(None);
        }
        let paired_taps = ((4. * sigma).ceil() as u64).div_ceil(2);
        let samples = u64::from(extent.width.0 as u32)
            .saturating_mul(u64::from(extent.height.0 as u32))
            .saturating_mul(2 * (1 + 2 * paired_taps));
        let state = &mut self.isolated_groups;
        state.blur_samples = state.blur_samples.saturating_add(samples);
        anyhow::ensure!(state.blur_samples <= state.budget.max_blur_samples,
            "foreground blur sample budget exceeded");
        self.group_texture(extent, MTLPixelFormat::RGBA16Float).map(Some)
    }

    fn prepare_isolated_scene(&mut self, scene: &Scene, depth: usize) -> Result<()> {
        self.isolated_groups.scene_visits += 1;
        anyhow::ensure!(
            self.isolated_groups.scene_visits <= self.isolated_groups.budget.max_scene_visits,
            "group scene visit budget exceeded"
        );
        for group in &scene.isolated_groups {
            if !group.is_renderable() {
                continue;
            }
            self.isolated_groups.group_visits += 1;
            anyhow::ensure!(
                self.isolated_groups.group_visits <= self.isolated_groups.budget.max_groups,
                "group instance budget exceeded"
            );
            anyhow::ensure!(
                depth < self.isolated_groups.budget.max_depth,
                "group depth budget exceeded"
            );
            let key = group.source_identity() as usize;
            let live = group.needs_live_parent();
            if !live && self.isolated_groups.prepared.contains_key(&key) {
                continue;
            }
            self.prepare_isolated_scene(group.source(), depth + 1)?;
            if live {
                // Resolve after the parent's preceding draws. The same
                // immutable source can appear at different placements.
                continue;
            }
            let extent = group.source_size();
            let target = self.group_texture(extent, MTLPixelFormat::BGRA8Unorm)?;
            let sigma = group.source_blur_sigma();
            let scratch = self.foreground_scratch(extent, sigma)?;
            let prior = self.opaque;
            self.opaque = false;
            let needs_paths = scene_needs_paths(group.source());
            let saved = needs_paths.then(|| {
                (
                    self.path_intermediate_texture.take(),
                    self.path_intermediate_msaa_texture.take(),
                )
            });
            let result = (|| {
                if needs_paths {
                    self.update_path_intermediate_textures(group.source_size())?;
                }
                self.render_frame_raw(group.source(), &target, group.source_size())
            })();
            if let Some((texture, msaa)) = saved {
                self.path_intermediate_texture = texture;
                self.path_intermediate_msaa_texture = msaa;
            }
            self.opaque = prior;
            let commands = result?;
            let filtered = match &scratch {
                Some(scratch) => self.gaussian.apply(&commands, &target, scratch, sigma),
                None => Ok(()),
            };
            let fence = self
                .isolated_groups
                .fence
                .as_ref()
                .context("group frame fence missing")?
                .clone();
            fence.preparations.fetch_add(1, Ordering::AcqRel);
            let completed = RcBlock::new(
                move |_: ptr::NonNull<ProtocolObject<dyn MTLCommandBuffer>>| {
                    fence.preparations.fetch_sub(1, Ordering::AcqRel);
                },
            );
            // Completion handlers must be attached before commit, including
            // when preparation succeeds but the parent later rejects a budget.
            unsafe {
                commands.addCompletedHandler(RcBlock::as_ptr(&completed));
            }
            commands.commit();
            // Even an encoding failure retires previously registered writer
            // resources and fences; no partial filtered source is published.
            filtered?;
            self.isolated_groups.prepared.insert(key, target);
        }
        // @LAYER_SCENES@
        Ok(())
    }

    pub(crate) fn draw_isolated_groups(
        &self,
        groups: &[gpui::IsolatedGroup],
        viewport: Size<DevicePixels>,
        encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>,
    ) -> Result<()> {
        for group in groups {
            if !group.is_renderable() {
                continue;
            }
            let texture = self
                .isolated_groups
                .prepared
                .get(&(group.source_identity() as usize))
                .context("group source was not prepared")?;
            self.draw_group_texture(group, viewport, encoder, texture)?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_isolated_groups_live(
        &mut self,
        groups: &[gpui::IsolatedGroup],
        viewport: Size<DevicePixels>,
        target: &ProtocolObject<dyn MTLTexture>,
        commands: &ProtocolObject<dyn MTLCommandBuffer>,
        mut encoder: Retained<ProtocolObject<dyn MTLRenderCommandEncoder>>,
        ancestry: Option<&BackdropAncestry<'_>>,
        bind_clip: impl Fn(&Self, &ProtocolObject<dyn MTLRenderCommandEncoder>),
    ) -> Result<Retained<ProtocolObject<dyn MTLRenderCommandEncoder>>> {
        for group in groups.iter().filter(|group| group.is_renderable()) {
            if group.needs_live_parent() {
                encoder.endEncoding();
                self.finish_transmission_chunk(commands, target, true)?;
                let (texture, fields) = self.encode_group_with_parent(group, commands, target, ancestry)?;
                let merged = fields.is_some() && self.transmission.is_some();
                if let (Some(fields), true) = (&fields, merged) {
                    self.merge_transmission_group(commands, group, fields, |renderer, encoder| bind_clip(renderer, encoder))?;
                }
                encoder = new_command_encoder_for_texture(commands, target, viewport, None)?;
                bind_clip(self, &encoder);
                match fields.as_ref() {
                    Some(fields) if !merged => self.draw_transmission_group(group, viewport, &encoder, fields)?,
                    Some(_) => (),
                    None => self.draw_group_texture(group, viewport, &encoder, &texture)?,
                }
            } else {
                self.draw_isolated_groups(std::slice::from_ref(group), viewport, &encoder)?;
            }
        }
        Ok(encoder)
    }

    fn encode_group_with_parent(
        &mut self,
        group: &gpui::IsolatedGroup,
        commands: &ProtocolObject<dyn MTLCommandBuffer>,
        parent: &ProtocolObject<dyn MTLTexture>,
        ancestry: Option<&BackdropAncestry<'_>>,
    ) -> Result<(Retained<ProtocolObject<dyn MTLTexture>>, Option<transmission::Fields>)> {
        let extent = group.source_size();
        let target = self.group_texture(extent, MTLPixelFormat::BGRA8Unorm)?;
        let sigma = group.source_blur_sigma();
        let scratch = self.foreground_scratch(extent, sigma)?;
        let prior = self.transmission.as_ref().map(|transport| transport.front.clone());
        let (parent, loss) = prior.as_ref().map_or((parent, None), |fields| (&*fields.color, Some(&*fields.loss)));
        let seeded = match group.parent_backdrop() {
            Some(read) => self.seed_parent_backdrop(commands, parent, loss, &target, extent, group, read, ancestry)?,
            None => false,
        };
        let tap = group.preserves_parent_backdrop().then_some(BackdropAncestry {
            target: parent,
            loss,
            placement: group.transform(),
            outer: ancestry,
        });
        let transport = if tap.is_some() && group.source().has_backdrop_replacements() {
            Some(transmission::Transport::allocate(self, commands, extent)?)
        } else {
            None
        };
        let clear = (!seeded).then_some(MTLClearColor { red: 0., green: 0., blue: 0., alpha: 0. });
        let needs_paths = scene_needs_paths(group.source());
        let prior_paths = needs_paths.then(|| (
            self.path_intermediate_texture.take(),
            self.path_intermediate_msaa_texture.take(),
        ));
        let result = (|| {
            if needs_paths {
                self.update_path_intermediate_textures(extent)?;
            }
            self.encode_surface_on(group.source(), &target, extent, commands, clear, tap.as_ref(), transport)
        })();
        if let Some((texture, msaa)) = prior_paths {
            self.path_intermediate_texture = texture;
            self.path_intermediate_msaa_texture = msaa;
        }
        let fields = result?.map(|transport| transport.front);
        if let Some(scratch) = &scratch {
            self.gaussian.apply(commands, &target, scratch, sigma)?;
        }
        Ok((target, fields))
    }

    fn draw_group_texture(
        &self,
        group: &gpui::IsolatedGroup,
        viewport: Size<DevicePixels>,
        encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>,
        texture: &ProtocolObject<dyn MTLTexture>,
    ) -> Result<()> {
            let uniforms = GroupUniforms::placement(group)?;
            encoder.setRenderPipelineState(
                self.isolated_groups
                    .pipeline
                    .as_ref()
                    .context("group pipeline missing")?,
            );
            // These copied uniforms and the retained six-vertex buffer match
            // the shader ABI generated from GroupUniforms.
            unsafe {
                encoder.setVertexBuffer_offset_atIndex(Some(&self.unit_vertices), 0, 0);
                encoder.setVertexBytes_length_atIndex(
                    ptr::NonNull::from(&uniforms).cast(),
                    mem::size_of_val(&uniforms),
                    1,
                );
                encoder.setVertexBytes_length_atIndex(
                    ptr::NonNull::from(&viewport).cast(),
                    mem::size_of_val(&viewport),
                    2,
                );
                encoder.setFragmentBytes_length_atIndex(
                    ptr::NonNull::from(&uniforms).cast(),
                    mem::size_of_val(&uniforms),
                    1,
                );
                encoder.setFragmentTexture_atIndex(Some(texture), 0);
                encoder.drawPrimitives_vertexStart_vertexCount(MTLPrimitiveType::Triangle, 0, 6);
            }
        Ok(())
    }
}

impl State {
    pub(super) fn new(
        device: &ProtocolObject<dyn MTLDevice>,
        library: &ProtocolObject<dyn MTLLibrary>,
    ) -> Result<Self> {
        Ok(Self {
            pipeline: Some(build_path_sprite_pipeline_state(
                device,
                library,
                "isolated_group",
                "isolated_group_vertex",
                "isolated_group_fragment",
                MTLPixelFormat::BGRA8Unorm,
            )?),
            ..Self::default()
        })
    }
}

pub(super) fn scene_needs_paths(scene: &Scene) -> bool {
    if !scene.paths.is_empty() {
        return true;
    }
    // @LAYER_PATHS@
    false
}
