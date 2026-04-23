use std::{
    borrow::Cow,
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
};

use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, window::Window};

#[derive(Debug)]
pub struct Frame {
    pub logical_width: u32,
    pub logical_height: u32,
    pub clear_color: [f32; 4],
    pub vertices: Vec<Vertex>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }

    fn with_color(self, color: [f32; 4]) -> Self {
        Self { color, ..self }
    }
}

#[derive(Debug)]
pub enum RendererInitError {
    Surface(wgpu::CreateSurfaceError),
    Adapter(wgpu::RequestAdapterError),
    Device(wgpu::RequestDeviceError),
    SurfaceFormatUnavailable,
}

impl std::fmt::Display for RendererInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Surface(error) => write!(f, "surface creation failed: {error}"),
            Self::Adapter(error) => write!(f, "adapter request failed: {error}"),
            Self::Device(error) => write!(f, "device creation failed: {error}"),
            Self::SurfaceFormatUnavailable => write!(f, "no supported surface format found"),
        }
    }
}

impl std::error::Error for RendererInitError {}

#[derive(Debug)]
pub enum ScreenshotError {
    SurfaceCopyUnsupported,
    UnsupportedTextureFormat(wgpu::TextureFormat),
    BufferMap(wgpu::BufferAsyncError),
    Poll(wgpu::PollError),
    Io(io::Error),
}

impl std::fmt::Display for ScreenshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SurfaceCopyUnsupported => write!(f, "surface does not support texture readback"),
            Self::UnsupportedTextureFormat(format) => {
                write!(f, "unsupported screenshot texture format: {format:?}")
            }
            Self::BufferMap(error) => write!(f, "screenshot buffer mapping failed: {error}"),
            Self::Poll(error) => write!(f, "screenshot GPU polling failed: {error}"),
            Self::Io(error) => write!(f, "screenshot write failed: {error}"),
        }
    }
}

impl std::error::Error for ScreenshotError {}

impl From<io::Error> for ScreenshotError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderOutcome {
    Presented,
    NeedsReconfigure,
    Skipped,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Result<Self, RendererInitError> {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window)
            .map_err(RendererInitError::Surface)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(RendererInitError::Adapter)?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(RendererInitError::Device)?;
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .or_else(|| capabilities.formats.first().copied())
            .ok_or(RendererInitError::SurfaceFormatUnavailable)?;
        let present_mode = capabilities
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::PresentMode::Fifo)
            .unwrap_or(capabilities.present_modes[0]);
        let alpha_mode = capabilities.alpha_modes[0];
        let usage = surface_usage(capabilities.usages);
        let config = surface_config(size, format, present_mode, alpha_mode, usage);
        surface.configure(&device, &config);
        let render_pipeline = create_pipeline(&device, format);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
        })
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn render(&mut self, frame: &Frame) -> RenderOutcome {
        self.render_inner(frame, None).0
    }

    pub fn render_with_screenshot(
        &mut self,
        frame: &Frame,
        path: impl Into<PathBuf>,
    ) -> (RenderOutcome, Option<Result<PathBuf, ScreenshotError>>) {
        let path = path.into();
        self.render_inner(frame, Some(&path))
    }

    fn render_inner(
        &mut self,
        frame: &Frame,
        screenshot_path: Option<&Path>,
    ) -> (RenderOutcome, Option<Result<PathBuf, ScreenshotError>>) {
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(output) => output,
            wgpu::CurrentSurfaceTexture::Suboptimal(output) => output,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                return (RenderOutcome::NeedsReconfigure, None);
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => return (RenderOutcome::Skipped, None),
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let surface_is_srgb = self.config.format.is_srgb();
        let clear_color = surface_color(frame.clear_color, surface_is_srgb);
        let vertices = surface_vertices(&frame.vertices, surface_is_srgb);
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("frame vertices"),
                contents: bytemuck::cast_slice(vertices.as_ref()),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render encoder"),
            });

        {
            let viewport = aspect_viewport(
                self.config.width,
                self.config.height,
                frame.logical_width,
                frame.logical_height,
            );
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_color[0] as f64,
                            g: clear_color[1] as f64,
                            b: clear_color[2] as f64,
                            a: clear_color[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.render_pipeline);
            pass.set_viewport(
                viewport.x,
                viewport.y,
                viewport.width,
                viewport.height,
                0.0,
                1.0,
            );
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.draw(0..frame.vertices.len() as u32, 0..1);
        }

        let screenshot = screenshot_path.map(|path| {
            self.encode_screenshot_copy(&mut encoder, &output.texture, path)
                .map(|readback| (path.to_path_buf(), readback))
        });

        self.queue.submit(Some(encoder.finish()));
        let screenshot = screenshot.map(|result| {
            result.and_then(|(path, readback)| {
                self.write_screenshot(readback, &path)?;
                Ok(path)
            })
        });
        output.present();
        (RenderOutcome::Presented, screenshot)
    }

    fn encode_screenshot_copy(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        path: &Path,
    ) -> Result<ScreenshotReadback, ScreenshotError> {
        if !self.config.usage.contains(wgpu::TextureUsages::COPY_SRC) {
            return Err(ScreenshotError::SurfaceCopyUnsupported);
        }

        let width = self.config.width;
        let height = self.config.height;
        let bytes_per_pixel = 4;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let padded_bytes_per_row =
            align_to(unpadded_bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer_size = u64::from(padded_bytes_per_row) * u64::from(height);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screenshot readback"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        Ok(ScreenshotReadback {
            buffer,
            width,
            height,
            padded_bytes_per_row,
            unpadded_bytes_per_row,
            format: self.config.format,
        })
    }

    fn write_screenshot(
        &self,
        readback: ScreenshotReadback,
        path: &Path,
    ) -> Result<(), ScreenshotError> {
        let (sender, receiver) = mpsc::channel();
        let buffer_slice = readback.buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(ScreenshotError::Poll)?;
        receiver
            .recv()
            .expect("screenshot map callback channel should stay open")
            .map_err(ScreenshotError::BufferMap)?;

        {
            let data = buffer_slice.get_mapped_range();
            let rgb = screenshot_rgb_bytes(&readback, &data)?;
            let mut ppm = format!("P6\n{} {}\n255\n", readback.width, readback.height).into_bytes();
            ppm.extend_from_slice(&rgb);
            fs::write(path, ppm)?;
        }

        readback.buffer.unmap();
        Ok(())
    }
}

fn surface_color(color: [f32; 4], surface_is_srgb: bool) -> [f32; 4] {
    if surface_is_srgb {
        linearize_rgba(color)
    } else {
        color
    }
}

fn surface_vertices<'a>(vertices: &'a [Vertex], surface_is_srgb: bool) -> Cow<'a, [Vertex]> {
    if !surface_is_srgb {
        return Cow::Borrowed(vertices);
    }

    Cow::Owned(
        vertices
            .iter()
            .copied()
            .map(|vertex| vertex.with_color(linearize_rgba(vertex.color)))
            .collect(),
    )
}

fn linearize_rgba(color: [f32; 4]) -> [f32; 4] {
    [
        srgb_channel_to_linear(color[0]),
        srgb_channel_to_linear(color[1]),
        srgb_channel_to_linear(color[2]),
        color[3],
    ]
}

fn srgb_channel_to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

struct ScreenshotReadback {
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
    unpadded_bytes_per_row: u32,
    format: wgpu::TextureFormat,
}

fn surface_config(
    size: PhysicalSize<u32>,
    format: wgpu::TextureFormat,
    present_mode: wgpu::PresentMode,
    alpha_mode: wgpu::CompositeAlphaMode,
    usage: wgpu::TextureUsages,
) -> wgpu::SurfaceConfiguration {
    wgpu::SurfaceConfiguration {
        usage,
        format,
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode,
        alpha_mode,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    }
}

fn surface_usage(supported: wgpu::TextureUsages) -> wgpu::TextureUsages {
    let desired = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC;
    if supported.contains(desired) {
        desired
    } else {
        wgpu::TextureUsages::RENDER_ATTACHMENT
    }
}

fn align_to(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RenderViewport {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

fn aspect_viewport(
    surface_width: u32,
    surface_height: u32,
    logical_width: u32,
    logical_height: u32,
) -> RenderViewport {
    let surface_width = surface_width.max(1);
    let surface_height = surface_height.max(1);
    let logical_width = logical_width.max(1) as f64;
    let logical_height = logical_height.max(1) as f64;
    let logical_aspect = logical_width / logical_height;
    let surface_aspect = surface_width as f64 / surface_height as f64;

    let (viewport_width, viewport_height) = if surface_aspect > logical_aspect {
        (
            (surface_height as f64 * logical_aspect).round() as u32,
            surface_height,
        )
    } else {
        (
            surface_width,
            (surface_width as f64 / logical_aspect).round() as u32,
        )
    };

    let viewport_width = viewport_width.clamp(1, surface_width);
    let viewport_height = viewport_height.clamp(1, surface_height);

    RenderViewport {
        x: ((surface_width - viewport_width) / 2) as f32,
        y: ((surface_height - viewport_height) / 2) as f32,
        width: viewport_width as f32,
        height: viewport_height as f32,
    }
}

fn screenshot_rgb_bytes(
    readback: &ScreenshotReadback,
    data: &[u8],
) -> Result<Vec<u8>, ScreenshotError> {
    let mut rgb = Vec::with_capacity((readback.width * readback.height * 3) as usize);

    for row in 0..readback.height as usize {
        let row_start = row * readback.padded_bytes_per_row as usize;
        let row_data = &data[row_start..row_start + readback.unpadded_bytes_per_row as usize];

        for pixel in row_data.chunks_exact(4) {
            match readback.format {
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
                    rgb.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
                }
                wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => {
                    rgb.extend_from_slice(&[pixel[0], pixel[1], pixel[2]]);
                }
                format => return Err(ScreenshotError::UnsupportedTextureFormat(format)),
            }
        }
    }

    Ok(rgb)
}

fn create_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("primitive shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("render/primitive.wgsl"))),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("primitive pipeline layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("primitive pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Vertex::layout()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
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
}

pub struct PrimitiveBatch {
    width: f32,
    height: f32,
    vertices: Vec<Vertex>,
}

impl PrimitiveBatch {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            vertices: Vec::new(),
        }
    }

    pub fn into_vertices(self) -> Vec<Vertex> {
        self.vertices
    }

    pub fn rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        self.triangle((x, y), (x + width, y), (x + width, y + height), color);
        self.triangle((x, y), (x + width, y + height), (x, y + height), color);
    }

    fn triangle(&mut self, a: (f32, f32), b: (f32, f32), c: (f32, f32), color: [f32; 4]) {
        self.vertices.push(self.vertex(a, color));
        self.vertices.push(self.vertex(b, color));
        self.vertices.push(self.vertex(c, color));
    }

    fn vertex(&self, point: (f32, f32), color: [f32; 4]) -> Vertex {
        Vertex {
            position: [
                point.0 / self.width * 2.0 - 1.0,
                1.0 - point.1 / self.height * 2.0,
            ],
            color,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RenderViewport, aspect_viewport};

    #[test]
    fn viewport_uses_full_surface_when_aspect_matches() {
        assert_eq!(
            aspect_viewport(640, 350, 640, 350),
            RenderViewport {
                x: 0.0,
                y: 0.0,
                width: 640.0,
                height: 350.0,
            }
        );
    }

    #[test]
    fn viewport_pillarboxes_wide_surfaces() {
        assert_eq!(
            aspect_viewport(1000, 350, 640, 350),
            RenderViewport {
                x: 180.0,
                y: 0.0,
                width: 640.0,
                height: 350.0,
            }
        );
    }

    #[test]
    fn viewport_letterboxes_tall_surfaces() {
        assert_eq!(
            aspect_viewport(640, 700, 640, 350),
            RenderViewport {
                x: 0.0,
                y: 175.0,
                width: 640.0,
                height: 350.0,
            }
        );
    }
}
