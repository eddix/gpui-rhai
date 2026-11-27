//! Same-frame affine operators P + (1 - loss) B. A normal color pass
//! cannot encode an independent background loss while keeping true alpha.
use super::*;
use isolated_group::GroupUniforms;

type Texture = Retained<ProtocolObject<dyn MTLTexture>>;
type Pipeline = Retained<ProtocolObject<dyn MTLRenderPipelineState>>;

const CLEAR: MTLClearColor = MTLClearColor {
    red: 0.,
    green: 0.,
    blue: 0.,
    alpha: 0.,
};

#[derive(Clone)]
pub(super) struct Fields {
    pub(super) color: Texture,
    pub(super) loss: Texture,
}

pub(super) struct Transport {
    pub(super) front: Fields,
    spare: Fields,
    extent: Size<DevicePixels>,
}

pub(super) struct Program {
    absorb_chunk: Pipeline,
    pub(super) replace_backdrop: Pipeline,
    place_in_transport: Pipeline,
    place_in_surface: Pipeline,
    pub(super) project: Pipeline,
}

impl Program {
    pub(super) fn new(
        device: &ProtocolObject<dyn MTLDevice>,
        library: &ProtocolObject<dyn MTLLibrary>,
    ) -> Result<Self> {
        let make = |vertex: &str, fragment: &str, fields: bool| -> Result<Pipeline> {
            let vertex = library
                .newFunctionWithName(&NSString::from_str(vertex))
                .context("transmission vertex")?;
            let fragment = library
                .newFunctionWithName(&NSString::from_str(fragment))
                .context("transmission fragment")?;
            let desc = MTLRenderPipelineDescriptor::new();
            desc.setVertexFunction(Some(&vertex));
            desc.setFragmentFunction(Some(&fragment));
            let attachments = desc.colorAttachments();
            let color = unsafe { attachments.objectAtIndexedSubscript(0) };
            color.setPixelFormat(if fields {
                MTLPixelFormat::RGBA16Float
            } else {
                MTLPixelFormat::BGRA8Unorm
            });
            if fields {
                let loss = unsafe { attachments.objectAtIndexedSubscript(1) };
                loss.setPixelFormat(MTLPixelFormat::R16Float);
            } else {
                color.setBlendingEnabled(true);
                color.setSourceRGBBlendFactor(MTLBlendFactor::One);
                color.setSourceAlphaBlendFactor(MTLBlendFactor::One);
                color.setDestinationRGBBlendFactor(MTLBlendFactor::Source1Alpha);
                color.setDestinationAlphaBlendFactor(MTLBlendFactor::Source1Alpha);
            }
            device
                .newRenderPipelineStateWithDescriptor_error(&desc)
                .context("creating exact transmission pipeline")
        };
        Ok(Self {
            absorb_chunk: make("group_gaussian_vertex", "transport_chunk", true)?,
            replace_backdrop: make("group_gaussian_vertex", "transport_replace", true)?,
            place_in_transport: make("group_gaussian_vertex", "transport_merge", true)?,
            place_in_surface: make("isolated_group_vertex", "transport_place", false)?,
            project: make("group_gaussian_vertex", "transport_project", false)?,
        })
    }
}

impl Fields {
    fn allocate(renderer: &mut MetalRenderer, extent: Size<DevicePixels>) -> Result<Self> {
        Ok(Self {
            color: renderer.group_texture(extent, MTLPixelFormat::RGBA16Float)?,
            loss: renderer.group_texture(extent, MTLPixelFormat::R16Float)?,
        })
    }

    fn encoder(
        &self,
        commands: &ProtocolObject<dyn MTLCommandBuffer>,
        clear: bool,
    ) -> Result<Retained<ProtocolObject<dyn MTLRenderCommandEncoder>>> {
        let pass = MTLRenderPassDescriptor::new();
        for (index, texture) in [&self.color, &self.loss].into_iter().enumerate() {
            let attachment = unsafe { pass.colorAttachments().objectAtIndexedSubscript(index) };
            attachment.setTexture(Some(texture));
            attachment.setStoreAction(MTLStoreAction::Store);
            attachment.setLoadAction(if clear {
                MTLLoadAction::Clear
            } else {
                MTLLoadAction::DontCare
            });
            if clear {
                attachment.setClearColor(CLEAR);
            }
        }
        commands
            .renderCommandEncoderWithDescriptor(&pass)
            .context("creating affine transmission encoder")
    }

    fn bind(&self, encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>, at: usize) {
        unsafe {
            encoder.setFragmentTexture_atIndex(Some(&self.color), at);
            encoder.setFragmentTexture_atIndex(Some(&self.loss), at + 1);
        }
    }
}

impl Transport {
    pub(super) fn allocate(
        renderer: &mut MetalRenderer,
        commands: &ProtocolObject<dyn MTLCommandBuffer>,
        extent: Size<DevicePixels>,
    ) -> Result<Self> {
        renderer.exact_transmission()?;
        renderer.reserve_transmission_pass(extent)?;
        let front = Fields::allocate(renderer, extent)?;
        let spare = Fields::allocate(renderer, extent)?;
        front.encoder(commands, true)?.endEncoding();
        Ok(Self {
            front,
            spare,
            extent,
        })
    }

    pub(super) fn absorb(
        &mut self,
        commands: &ProtocolObject<dyn MTLCommandBuffer>,
        pipeline: &ProtocolObject<dyn MTLRenderPipelineState>,
        bind: impl FnOnce(&ProtocolObject<dyn MTLRenderCommandEncoder>),
    ) -> Result<()> {
        let encoder = self.spare.encoder(commands, false)?;
        encoder.setRenderPipelineState(pipeline);
        self.front.bind(&encoder, 0);
        bind(&encoder);
        unsafe {
            encoder.drawPrimitives_vertexStart_vertexCount(MTLPrimitiveType::Triangle, 0, 3);
        }
        encoder.endEncoding();
        mem::swap(&mut self.front, &mut self.spare);
        Ok(())
    }
}

impl MetalRenderer {
    pub(super) fn exact_transmission(&self) -> Result<&Program> {
        self.transmission_program.as_ref().map_err(|error| {
            anyhow::anyhow!("Metal dual-source transmission is unavailable: {error:#}")
        })
    }

    pub(super) fn finish_transmission_chunk(
        &mut self,
        commands: &ProtocolObject<dyn MTLCommandBuffer>,
        texture: &ProtocolObject<dyn MTLTexture>,
        reopen: bool,
    ) -> Result<()> {
        let Some(mut transport) = self.transmission.take() else {
            return Ok(());
        };
        let result = (|| {
            self.reserve_transmission_pass(transport.extent)?;
            if reopen {
                self.reserve_transmission_pass(transport.extent)?;
            }
            transport.absorb(
                commands,
                &self.exact_transmission()?.absorb_chunk,
                |encoder| unsafe {
                    encoder.setFragmentTexture_atIndex(Some(texture), 2);
                },
            )?;
            if reopen {
                new_command_encoder_for_texture(commands, texture, transport.extent, Some(CLEAR))?
                    .endEncoding();
            }
            Ok(())
        })();
        self.transmission = Some(transport);
        result
    }

    pub(super) fn reserve_transmission_pass(&mut self, viewport: Size<DevicePixels>) -> Result<()> {
        self.reserve_backdrop_work(
            0,
            (viewport.width.0 as u64).saturating_mul(viewport.height.0 as u64),
        )
    }

    pub(super) fn merge_transmission_group(
        &mut self,
        commands: &ProtocolObject<dyn MTLCommandBuffer>,
        group: &gpui::IsolatedGroup,
        child: &Fields,
        bind_clip: impl FnOnce(&Self, &ProtocolObject<dyn MTLRenderCommandEncoder>),
    ) -> Result<()> {
        let mut transport = self
            .transmission
            .take()
            .context("missing transmission target")?;
        let result = (|| {
            self.reserve_transmission_pass(transport.extent)?;
            let uniforms = GroupUniforms::placement(group)?;
            transport.absorb(
                commands,
                &self.exact_transmission()?.place_in_transport,
                |encoder| {
                    child.bind(encoder, 2);
                    bind_clip(self, encoder);
                    fragment_value(encoder, &uniforms, 1);
                },
            )
        })();
        self.transmission = Some(transport);
        result
    }

    pub(super) fn draw_transmission_group(
        &self,
        group: &gpui::IsolatedGroup,
        viewport: Size<DevicePixels>,
        encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>,
        fields: &Fields,
    ) -> Result<()> {
        let uniforms = GroupUniforms::placement(group)?;
        encoder.setRenderPipelineState(&self.exact_transmission()?.place_in_surface);
        fields.bind(encoder, 0);
        fragment_value(encoder, &uniforms, 1);
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
            encoder.drawPrimitives_vertexStart_vertexCount(MTLPrimitiveType::Triangle, 0, 6);
        }
        Ok(())
    }
}

pub(super) fn fragment_value<T>(
    encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>,
    value: &T,
    index: usize,
) {
    unsafe {
        encoder.setFragmentBytes_length_atIndex(
            ptr::NonNull::from(value).cast(),
            mem::size_of_val(value),
            index,
        );
    }
}
