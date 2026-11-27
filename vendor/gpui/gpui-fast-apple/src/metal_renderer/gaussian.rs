//! Two-pass, zero-extended convolution of premultiplied group pixels.
use super::*;

pub(super) struct Gaussian {
    horizontal: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    vertical: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
}

impl Gaussian {
    pub(super) fn new(
        device: &ProtocolObject<dyn MTLDevice>,
        library: &ProtocolObject<dyn MTLLibrary>,
    ) -> Result<Self> {
        let vertex = library
            .newFunctionWithName(&NSString::from_str("group_gaussian_vertex"))
            .context("group Gaussian vertex function")?;
        let fragment = library
            .newFunctionWithName(&NSString::from_str("group_gaussian_fragment"))
            .context("group Gaussian fragment function")?;
        let pipeline = |format| {
            let desc = MTLRenderPipelineDescriptor::new();
            desc.setVertexFunction(Some(&vertex));
            desc.setFragmentFunction(Some(&fragment));
            let color = unsafe { desc.colorAttachments().objectAtIndexedSubscript(0) };
            color.setPixelFormat(format);
            color.setBlendingEnabled(false);
            device
                .newRenderPipelineStateWithDescriptor_error(&desc)
                .context("creating group Gaussian pipeline")
        };
        Ok(Self {
            horizontal: pipeline(MTLPixelFormat::RGBA16Float)?,
            vertical: pipeline(MTLPixelFormat::BGRA8Unorm)?,
        })
    }

    pub(super) fn apply(
        &self,
        commands: &ProtocolObject<dyn MTLCommandBuffer>,
        target: &ProtocolObject<dyn MTLTexture>,
        scratch: &ProtocolObject<dyn MTLTexture>,
        sigma: f32,
    ) -> Result<()> {
        self.encode(
            commands,
            target,
            scratch,
            target,
            [target.width() as u32, target.height() as u32],
            sigma,
            false,
        )
    }

    /// The passes may use a smaller valid rectangle than the pooled texture.
    /// Foreground extends with transparent pixels; a backdrop mirrors its ROI.
    pub(super) fn encode(
        &self,
        commands: &ProtocolObject<dyn MTLCommandBuffer>,
        source: &ProtocolObject<dyn MTLTexture>,
        scratch: &ProtocolObject<dyn MTLTexture>,
        destination: &ProtocolObject<dyn MTLTexture>,
        extent: [u32; 2],
        sigma: f32,
        mirror: bool,
    ) -> Result<()> {
        let radius = (4. * sigma).ceil() as usize;
        let variance = -2. * f64::from(sigma).powi(2);
        let weights = (0..=radius)
            .map(|i| (f64::from(i as u32).powi(2) / variance).exp())
            .collect::<Vec<_>>();
        let norm = 1. + 2. * weights[1..].iter().sum::<f64>();
        let mut taps = Vec::with_capacity(radius.div_ceil(2) + 1);
        taps.push([0., (1. / norm) as f32]);
        // One linear lookup per adjacent same-side pair, preserving its exact
        // two weights and dropping only pairs whose contribution underflows.
        for i in (1..=radius).step_by(2) {
            let a = weights[i];
            let b = weights.get(i + 1).copied().unwrap_or(0.);
            if a + b > 0. {
                taps.push([(i as f64 + b / (a + b)) as f32, ((a + b) / norm) as f32]);
            }
        }

        let output_pipeline = if destination.pixelFormat() == MTLPixelFormat::RGBA16Float {
            &self.horizontal
        } else {
            &self.vertical
        };
        for (axis, pipeline, input, output) in [
            (0, &self.horizontal, source, scratch),
            (1, output_pipeline, scratch, destination),
        ] {
            let pass = MTLRenderPassDescriptor::new();
            let color = unsafe { pass.colorAttachments().objectAtIndexedSubscript(0) };
            color.setTexture(Some(output));
            color.setLoadAction(MTLLoadAction::DontCare);
            color.setStoreAction(MTLStoreAction::Store);
            let encoder = commands
                .renderCommandEncoderWithDescriptor(&pass)
                .context("encoding group Gaussian pass")?;
            encoder.setRenderPipelineState(pipeline);
            encoder.setViewport(MTLViewport {
                originX: 0.,
                originY: 0.,
                width: f64::from(extent[0]),
                height: f64::from(extent[1]),
                znear: 0.,
                zfar: 1.,
            });
            let uniforms = [
                [taps.len() as u32, axis, extent[0], extent[1]],
                [radius as u32, u32::from(mirror), 0, 0],
            ];
            // No read/write alias within a pass. Copied CPU slices have the
            // shader's pair of uint4 and array-of-float2 layouts, and the
            // bounded sigma keeps the inline tap buffer below 4 KiB.
            unsafe {
                encoder.setFragmentBytes_length_atIndex(
                    ptr::NonNull::from(&uniforms).cast(),
                    mem::size_of_val(&uniforms),
                    0,
                );
                encoder.setFragmentBytes_length_atIndex(
                    ptr::NonNull::from(taps.as_slice()).cast(),
                    mem::size_of_val(taps.as_slice()),
                    1,
                );
                encoder.setFragmentTexture_atIndex(Some(input), 0);
                encoder.drawPrimitives_vertexStart_vertexCount(MTLPrimitiveType::Triangle, 0, 3);
            }
            encoder.endEncoding();
        }
        Ok(())
    }
}
