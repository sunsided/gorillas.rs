//! wgpu-based renderer. Consumes a [`Frame`] produced by the game logic and presents it
//! to the OS window, with letterboxing/pillarboxing to preserve the original 640×350 aspect ratio.
//!
//! The only primitive is a colored triangle; everything the game draws goes through
//! [`PrimitiveBatch`] which converts axis-aligned rectangles into triangle pairs.

use std::{
    borrow::Cow,
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
};

use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, window::Window};

/// One rendered frame: a clear color plus a list of colored triangle vertices.
/// Produced by [`game::GameState::frame`] and consumed by [`Renderer::render`].
#[derive(Debug)]
pub struct Frame {
    /// Logical pixel width of the game canvas (640).
    pub logical_width: u32,
    /// Logical pixel height of the game canvas (350).
    pub logical_height: u32,
    /// Color used to clear the surface before drawing vertices.
    pub clear_color: [f32; 4],
    /// Flat triangle list; every three entries form one triangle.
    pub vertices: Vec<Vertex>,
}

/// A single GPU vertex: clip-space position and linear sRGB color.
/// `repr(C)` + [`bytemuck::Pod`] so it can be cast directly into a wgpu vertex buffer.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    /// NDC position: `(-1, -1)` = bottom-left, `(1, 1)` = top-right.
    position: [f32; 2],
    /// RGBA color in the game's logical color space (sRGB values, not pre-linearised).
    color: [f32; 4],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];

    /// Returns the wgpu vertex buffer layout descriptor for this type.
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }

    /// Returns a copy of this vertex with the color replaced.
    fn with_color(self, color: [f32; 4]) -> Self {
        Self { color, ..self }
    }
}

/// Error returned from [`Renderer::new`] when the GPU setup fails.
#[derive(Debug)]
pub enum RendererInitError {
    /// wgpu surface creation failed (e.g. the window handle is unsupported).
    Surface(wgpu::CreateSurfaceError),
    /// No suitable GPU adapter was found.
    Adapter(wgpu::RequestAdapterError),
    /// GPU device/queue creation failed.
    Device(wgpu::RequestDeviceError),
    /// The surface exposes no texture formats the renderer can use.
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

/// Error returned from [`Renderer::render_with_screenshot`] when the frame capture fails.
#[derive(Debug)]
pub enum ScreenshotError {
    /// The surface was not created with `COPY_SRC` usage, so readback is impossible.
    SurfaceCopyUnsupported,
    /// The surface format is not RGBA8 or BGRA8; pixel swizzling is undefined.
    UnsupportedTextureFormat(wgpu::TextureFormat),
    /// Mapping the readback buffer failed.
    BufferMap(wgpu::BufferAsyncError),
    /// GPU polling after the buffer map failed.
    Poll(wgpu::PollError),
    /// Writing the PPM file to disk failed.
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

/// wgpu renderer. Owns the surface, device, queue, and pipeline.
pub struct Renderer {
    /// The window surface frames are presented to.
    surface: wgpu::Surface<'static>,
    /// GPU logical device used to create resources and encode commands.
    device: wgpu::Device,
    /// Command queue; submits encoded command buffers.
    queue: wgpu::Queue,
    /// Current surface configuration (format, size, present mode).
    config: wgpu::SurfaceConfiguration,
    /// The single render pipeline: colored triangles with no depth test.
    render_pipeline: wgpu::RenderPipeline,
}

/// Outcome of a [`Renderer::render`] call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderOutcome {
    /// Frame was submitted and presented successfully.
    Presented,
    /// The surface is out of date; call [`Renderer::resize`] before the next frame.
    NeedsReconfigure,
    /// Frame was skipped (e.g. occluded or validation error); no action needed.
    Skipped,
}

impl Renderer {
    /// Initialises the wgpu surface, picks a suitable adapter and device,
    /// and creates the render pipeline. Prefers low-power adapters and sRGB surface formats.
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

    /// Updates the surface configuration when the window is resized.
    /// Zero-sized surfaces are ignored (window is minimised).
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Renders `frame` to the surface without capturing a screenshot.
    pub fn render(&mut self, frame: &Frame) -> RenderOutcome {
        self.render_inner(frame, None).0
    }

    /// Renders `frame` and additionally saves a PPM screenshot to `path`.
    /// Screenshot capture is best-effort; failure is reported in the returned `Option`.
    pub fn render_with_screenshot(
        &mut self,
        frame: &Frame,
        path: impl Into<PathBuf>,
    ) -> (RenderOutcome, Option<Result<PathBuf, ScreenshotError>>) {
        let path = path.into();
        self.render_inner(frame, Some(&path))
    }

    /// Core render path shared by [`render`](Renderer::render) and
    /// [`render_with_screenshot`](Renderer::render_with_screenshot).
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

    /// Encodes a texture-to-buffer copy command and returns the readback descriptor.
    /// Fails immediately if the surface does not have `COPY_SRC` usage.
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

    /// Maps the readback buffer, converts pixels to RGB, and writes a binary PPM file.
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

/// Converts a game color to the correct space for the clear-color API call.
/// wgpu clear colors are always linear on sRGB surfaces.
fn surface_color(color: [f32; 4], surface_is_srgb: bool) -> [f32; 4] {
    if surface_is_srgb {
        linearize_rgba(color)
    } else {
        color
    }
}

/// Linearises vertex colors for sRGB surfaces, or borrows them unchanged for non-sRGB.
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

/// Converts an sRGB RGBA color to linear light, preserving alpha.
fn linearize_rgba(color: [f32; 4]) -> [f32; 4] {
    [
        srgb_channel_to_linear(color[0]),
        srgb_channel_to_linear(color[1]),
        srgb_channel_to_linear(color[2]),
        color[3],
    ]
}

/// Applies the IEC 61966-2-1 sRGB transfer function inverse to one channel.
fn srgb_channel_to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// Carries the GPU buffer and layout information needed to retrieve and decode a screenshot.
struct ScreenshotReadback {
    /// GPU buffer mapped for CPU read after the copy completes.
    buffer: wgpu::Buffer,
    /// Image width in pixels.
    width: u32,
    /// Image height in pixels.
    height: u32,
    /// Row stride including wgpu alignment padding.
    padded_bytes_per_row: u32,
    /// Actual pixel data bytes per row (width × bytes-per-pixel).
    unpadded_bytes_per_row: u32,
    /// Surface texture format, used to determine byte swizzle order.
    format: wgpu::TextureFormat,
}

/// Builds a [`wgpu::SurfaceConfiguration`] from the given parameters.
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

/// Requests `RENDER_ATTACHMENT | COPY_SRC` if supported, otherwise falls back to
/// `RENDER_ATTACHMENT` only (which disables screenshot capture).
fn surface_usage(supported: wgpu::TextureUsages) -> wgpu::TextureUsages {
    let desired = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC;
    if supported.contains(desired) {
        desired
    } else {
        wgpu::TextureUsages::RENDER_ATTACHMENT
    }
}

/// Rounds `value` up to the next multiple of `alignment`.
fn align_to(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}

/// A letterboxed/pillarboxed viewport rectangle within the physical surface.
#[derive(Clone, Copy, Debug, PartialEq)]
struct RenderViewport {
    /// Left edge in physical pixels.
    x: f32,
    /// Top edge in physical pixels.
    y: f32,
    /// Viewport width in physical pixels.
    width: f32,
    /// Viewport height in physical pixels.
    height: f32,
}

/// Computes a centered viewport that fits the logical dimensions inside the physical surface,
/// preserving aspect ratio with pillarboxing (wide surface) or letterboxing (tall surface).
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

/// Strips the row padding from the mapped buffer and swizzles pixels to packed RGB bytes,
/// handling both RGBA8 and BGRA8 surface formats.
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

/// Compiles the WGSL shader and assembles the render pipeline for colored triangles.
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

/// Accumulates axis-aligned colored rectangles and converts them to triangle-list vertices.
///
/// Coordinates are in logical pixels; [`into_vertices`](PrimitiveBatch::into_vertices) maps
/// them to NDC using the canvas `width` and `height`.
pub struct PrimitiveBatch {
    /// Logical canvas width, used for NDC mapping.
    width: f32,
    /// Logical canvas height, used for NDC mapping.
    height: f32,
    /// Accumulated triangle vertices.
    vertices: Vec<Vertex>,
}

impl PrimitiveBatch {
    /// Creates an empty batch for a canvas of the given logical size.
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            vertices: Vec::new(),
        }
    }

    /// Consumes the batch and returns the accumulated vertex list.
    pub fn into_vertices(self) -> Vec<Vertex> {
        self.vertices
    }

    /// Appends two triangles that together form a filled axis-aligned rectangle.
    pub fn rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        self.triangle((x, y), (x + width, y), (x + width, y + height), color);
        self.triangle((x, y), (x + width, y + height), (x, y + height), color);
    }

    /// Appends three vertices for one triangle.
    fn triangle(&mut self, a: (f32, f32), b: (f32, f32), c: (f32, f32), color: [f32; 4]) {
        self.vertices.push(self.vertex(a, color));
        self.vertices.push(self.vertex(b, color));
        self.vertices.push(self.vertex(c, color));
    }

    /// Converts a logical-pixel point to an NDC [`Vertex`].
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
