//! Backdrop blur: gpui's `BackdropBlur` primitives, CSS's
//! `backdrop-filter: blur()`.
//!
//! The main pass is interrupted at each blur's draw order (the scene's
//! batches never span one). Then:
//!
//! 1. The region, widened by the blur's reach, is copied out of the frame
//!    into `source` (the frame's texture must allow `COPY_SRC`).
//! 2. It is scaled down by `factor` (a power of two keeping the blur within
//!    a few dozen taps) into `a`, averaging the pixels each covers.
//! 3. A separable Gaussian blurs `a` into `b` horizontally and `b` back into
//!    `a` vertically.
//! 4. `a` is drawn back into the frame, scaled up, clipped to the region's
//!    rounded rectangle, and the main pass resumes over it.
use bytemuck::{Pod, Zeroable};
use gpui::{BackdropBlur, Bounds, DevicePixels, ScaledPixels, Size};
use wgpu::util::DeviceExt as _;

const SHADER: &str = include_str!("backdrop_blur.wgsl");

/// The most taps either side a blur pass takes, in the scaled-down texture.
const MAX_SIGMA_SCALED: f32 = 8.0;
/// The largest scale-down.
const MAX_FACTOR: u32 = 16;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
struct BlurParams {
    source_size: [f32; 2],
    valid_min: [f32; 2],
    valid_max: [f32; 2],
    direction: [f32; 2],
    sigma: f32,
    factor: f32,
    bounds_origin: [f32; 2],
    bounds_size: [f32; 2],
    _pad: [f32; 2],
    corner_radii: [f32; 4],
}

/// A texture and its view.
struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

/// The textures for one frame size: the frame copy, and the scaled-down
/// pair for each factor used.
struct Textures {
    width: u32,
    height: u32,
    source: Target,
    scaled: Vec<(u32, Target, Target)>,
}

/// Pipelines and textures, made when a frame first has a blur.
pub(crate) struct BackdropBlurRenderer {
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    downsample: wgpu::RenderPipeline,
    blur: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    format: wgpu::TextureFormat,
    textures: Option<Textures>,
}

/// A rectangle in whole pixels: x, y, width, height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl Rect {
    fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// What one blur draws, worked out from the primitive and the frame size.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Plan {
    /// Copied from the frame (the region widened by the blur's reach),
    /// aligned to `factor`.
    copy: Rect,
    /// Drawn into: the region clipped to its content mask and the frame.
    draw: Rect,
    factor: u32,
    /// The blur's standard deviation in the scaled-down texture's pixels.
    sigma: f32,
}

fn plan(blur: &BackdropBlur, frame: Size<DevicePixels>) -> Option<Plan> {
    let width = frame.width.0.max(0) as f32;
    let height = frame.height.0.max(0) as f32;
    let sigma = blur.blur_radius.0;
    if !(sigma > 0.0) || width < 1.0 || height < 1.0 {
        return None;
    }
    let frame_bounds = Bounds {
        origin: gpui::point(ScaledPixels(0.0), ScaledPixels(0.0)),
        size: gpui::size(ScaledPixels(width), ScaledPixels(height)),
    };
    let visible = blur
        .bounds
        .intersect(&blur.content_mask.bounds)
        .intersect(&frame_bounds);
    let draw = outer_rect(visible, width as u32, height as u32, 1)?;
    let mut factor = 1;
    while factor < MAX_FACTOR && sigma / factor as f32 > MAX_SIGMA_SCALED {
        factor *= 2;
    }
    // What the Gaussian reaches beyond the region, plus a scaled-down pixel.
    let reach = ScaledPixels(sigma * 3.0 + factor as f32);
    let copy = outer_rect(visible.dilate(reach), width as u32, height as u32, factor)?;
    Some(Plan {
        copy,
        draw,
        factor,
        sigma: sigma / factor as f32,
    })
}

/// The whole pixels covering `bounds`, within a `width`×`height` frame,
/// widened to multiples of `align`.
fn outer_rect(bounds: Bounds<ScaledPixels>, width: u32, height: u32, align: u32) -> Option<Rect> {
    let floor_to = |v: f32| ((v.max(0.0).floor() as u32) / align) * align;
    let ceil_to = |v: f32, limit: u32| ((v.max(0.0).ceil() as u32).div_ceil(align) * align).min(limit);
    let x0 = floor_to(bounds.origin.x.0).min(width);
    let y0 = floor_to(bounds.origin.y.0).min(height);
    let x1 = ceil_to(bounds.origin.x.0 + bounds.size.width.0, width);
    let y1 = ceil_to(bounds.origin.y.0 + bounds.size.height.0, height);
    let rect = Rect {
        x: x0,
        y: y0,
        width: x1.saturating_sub(x0),
        height: y1.saturating_sub(y0),
    };
    (!rect.is_empty()).then_some(rect)
}

impl BackdropBlurRenderer {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("backdrop_blur_shaders"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("backdrop_blur_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<BlurParams>() as u64,
                        ),
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("backdrop_blur_pipeline_layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |name: &str, entry: &str, blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(name),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_fullscreen"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(entry),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let downsample = pipeline("backdrop_blur_downsample", "fs_downsample", None);
        let blur = pipeline("backdrop_blur_blur", "fs_blur", None);
        let composite = pipeline(
            "backdrop_blur_composite",
            "fs_composite",
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("backdrop_blur_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            layout,
            sampler,
            downsample,
            blur,
            composite,
            format,
            textures: None,
        }
    }

    /// Frees the textures (the frame size changed, or the device is being
    /// recovered).
    pub(crate) fn release_textures(&mut self) {
        self.textures = None;
    }

    fn target(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        usage: wgpu::TextureUsages,
    ) -> Target {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("backdrop_blur_texture"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Target { texture, view }
    }

    /// The textures for a `width`×`height` frame and `factor`, made as needed.
    fn textures(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        factor: u32,
    ) -> (&Target, &Target, &Target) {
        let format = self.format;
        if self
            .textures
            .as_ref()
            .is_none_or(|t| t.width != width || t.height != height)
        {
            self.textures = Some(Textures {
                width,
                height,
                source: Self::target(
                    device,
                    format,
                    width,
                    height,
                    wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                ),
                scaled: Vec::new(),
            });
        }
        let textures = self.textures.as_mut().expect("made above");
        if !textures.scaled.iter().any(|(f, _, _)| *f == factor) {
            let usage =
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
            let (w, h) = (width.div_ceil(factor), height.div_ceil(factor));
            textures.scaled.push((
                factor,
                Self::target(device, format, w, h, usage),
                Self::target(device, format, w, h, usage),
            ));
        }
        let textures = self.textures.as_ref().expect("made above");
        let (_, a, b) = textures
            .scaled
            .iter()
            .find(|(f, _, _)| *f == factor)
            .expect("made above");
        (&textures.source, a, b)
    }

    /// Records `blur` into `encoder`, between render passes on `frame`
    /// (whose texture is `frame_texture`, `size` pixels).
    pub(crate) fn record(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        frame_texture: &wgpu::Texture,
        frame_view: &wgpu::TextureView,
        size: Size<DevicePixels>,
        blur: &BackdropBlur,
    ) {
        let Some(plan) = plan(blur, size) else {
            return;
        };
        if frame_texture.format() != self.format {
            return;
        }
        let (width, height) = (size.width.0 as u32, size.height.0 as u32);
        let factor = plan.factor;
        let layout = self.layout.clone();
        let sampler = self.sampler.clone();
        let downsample = self.downsample.clone();
        let blur_pipeline = self.blur.clone();
        let composite = self.composite.clone();
        let (source, a, b) = self.textures(device, width, height, factor);

        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: frame_texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: plan.copy.x,
                    y: plan.copy.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &source.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: plan.copy.x,
                    y: plan.copy.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: plan.copy.width,
                height: plan.copy.height,
                depth_or_array_layers: 1,
            },
        );

        let scaled = Rect {
            x: plan.copy.x / factor,
            y: plan.copy.y / factor,
            width: plan.copy.width.div_ceil(factor),
            height: plan.copy.height.div_ceil(factor),
        };
        let scaled_size = [
            width.div_ceil(factor) as f32,
            height.div_ceil(factor) as f32,
        ];
        let scaled_min = [scaled.x as f32, scaled.y as f32];
        let scaled_max = [
            (scaled.x + scaled.width) as f32,
            (scaled.y + scaled.height) as f32,
        ];
        let corners = &blur.corner_radii;
        let base = BlurParams {
            factor: factor as f32,
            sigma: plan.sigma,
            bounds_origin: [blur.bounds.origin.x.0, blur.bounds.origin.y.0],
            bounds_size: [blur.bounds.size.width.0, blur.bounds.size.height.0],
            corner_radii: [
                corners.top_left.0,
                corners.top_right.0,
                corners.bottom_right.0,
                corners.bottom_left.0,
            ],
            ..Default::default()
        };
        let passes = [
            (
                &downsample,
                &source.view,
                &a.view,
                scaled,
                BlurParams {
                    source_size: [width as f32, height as f32],
                    valid_min: [plan.copy.x as f32, plan.copy.y as f32],
                    valid_max: [
                        (plan.copy.x + plan.copy.width) as f32,
                        (plan.copy.y + plan.copy.height) as f32,
                    ],
                    ..base
                },
            ),
            (
                &blur_pipeline,
                &a.view,
                &b.view,
                scaled,
                BlurParams {
                    source_size: scaled_size,
                    valid_min: scaled_min,
                    valid_max: scaled_max,
                    direction: [1.0, 0.0],
                    ..base
                },
            ),
            (
                &blur_pipeline,
                &b.view,
                &a.view,
                scaled,
                BlurParams {
                    source_size: scaled_size,
                    valid_min: scaled_min,
                    valid_max: scaled_max,
                    direction: [0.0, 1.0],
                    ..base
                },
            ),
            (
                &composite,
                &a.view,
                frame_view,
                plan.draw,
                BlurParams {
                    source_size: scaled_size,
                    valid_min: scaled_min,
                    valid_max: scaled_max,
                    ..base
                },
            ),
        ];
        for (index, (pipeline, input, output, scissor, params)) in passes.into_iter().enumerate() {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("backdrop_blur_params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("backdrop_blur_bind_group"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(input),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: buffer.as_entire_binding(),
                    },
                ],
            });
            // The composite draws over the frame; the others replace what
            // their texture held.
            let load = if index == 3 {
                wgpu::LoadOp::Load
            } else {
                wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("backdrop_blur_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_scissor_rect(scissor.x, scissor.y, scissor.width, scissor.height);
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{ContentMask, Corners, DevicePixels, point, size};

    fn blur(x: f32, y: f32, w: f32, h: f32, sigma: f32) -> BackdropBlur {
        let bounds = Bounds {
            origin: point(ScaledPixels(x), ScaledPixels(y)),
            size: size(ScaledPixels(w), ScaledPixels(h)),
        };
        BackdropBlur {
            order: 0,
            blur_radius: ScaledPixels(sigma),
            bounds,
            corner_radii: Corners::default(),
            content_mask: ContentMask { bounds },
        }
    }

    fn frame(w: i32, h: i32) -> Size<DevicePixels> {
        size(DevicePixels(w), DevicePixels(h))
    }

    #[test]
    fn plans_scale_down_large_blurs_and_stay_in_the_frame() {
        let p = plan(&blur(10.0, 10.0, 100.0, 50.0, 4.0), frame(200, 100)).unwrap();
        assert_eq!(p.factor, 1);
        assert_eq!(p.sigma, 4.0);
        assert_eq!(p.draw, Rect { x: 10, y: 10, width: 100, height: 50 });
        // Widened by 3σ + 1, clipped to the frame.
        assert_eq!(p.copy, Rect { x: 0, y: 0, width: 123, height: 73 });

        let p = plan(&blur(0.0, 0.0, 1000.0, 800.0, 40.0), frame(1000, 800)).unwrap();
        assert_eq!(p.factor, 8);
        assert_eq!(p.sigma, 5.0);
        assert_eq!(p.copy, Rect { x: 0, y: 0, width: 1000, height: 800 });
        assert_eq!(p.copy.x % 8, 0);

        let p = plan(&blur(0.0, 0.0, 100.0, 100.0, 100_000.0), frame(100, 100)).unwrap();
        assert_eq!(p.factor, MAX_FACTOR);
    }

    #[test]
    fn nothing_is_planned_for_no_blur_or_nothing_visible() {
        assert!(plan(&blur(0.0, 0.0, 10.0, 10.0, 0.0), frame(100, 100)).is_none());
        assert!(plan(&blur(200.0, 0.0, 10.0, 10.0, 2.0), frame(100, 100)).is_none());
        assert!(plan(&blur(0.0, 0.0, 10.0, 10.0, 2.0), frame(0, 0)).is_none());
    }
}
