use std::{path::Path, sync::Arc, time::Instant};

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

use crate::{
    game::{Game, LOGICAL_HEIGHT, LOGICAL_WIDTH},
    render::{RenderOutcome, Renderer, RendererInitError},
};

#[derive(Debug)]
pub enum AppError {
    EventLoop(winit::error::EventLoopError),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventLoop(error) => write!(f, "event loop failed: {error}"),
        }
    }
}

impl std::error::Error for AppError {}

pub fn run() -> Result<(), AppError> {
    let event_loop = EventLoop::new().map_err(AppError::EventLoop)?;
    let mut app = App::new();
    event_loop.run_app(&mut app).map_err(AppError::EventLoop)
}

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    game: Game,
    last_update: Instant,
    first_screenshot_taken: bool,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            game: Game::new(),
            last_update: Instant::now(),
            first_screenshot_taken: false,
        }
    }

    fn init_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), InitError> {
        if self.window.is_some() {
            return Ok(());
        }

        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("GORILLAS")
                    .with_inner_size(LogicalSize::new(
                        f64::from(LOGICAL_WIDTH),
                        f64::from(LOGICAL_HEIGHT),
                    ))
                    .with_min_inner_size(LogicalSize::new(640.0, 350.0)),
            )?,
        );
        let renderer = pollster::block_on(Renderer::new(Arc::clone(&window)))?;

        self.window = Some(window);
        self.renderer = Some(renderer);
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(error) = self.init_window(event_loop) {
            eprintln!("{error}");
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. }
                if event.logical_key == Key::Named(NamedKey::Escape)
                    && event.state.is_pressed() =>
            {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size);
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(window.inner_size());
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = self.renderer.as_mut() {
                    let now = Instant::now();
                    let dt = now.duration_since(self.last_update).as_secs_f32();
                    self.last_update = now;
                    self.game.update(dt);

                    let frame = self.game.frame();
                    let (outcome, screenshot) = if self.first_screenshot_taken {
                        (renderer.render(&frame), None)
                    } else {
                        let path = Path::new("target/screenshots/first-frame.ppm");
                        let result = renderer.render_with_screenshot(&frame, path);
                        self.first_screenshot_taken = true;
                        result
                    };

                    if let Some(screenshot) = screenshot {
                        match screenshot {
                            Ok(path) => {
                                eprintln!("saved first-frame screenshot to {}", path.display())
                            }
                            Err(error) => eprintln!("first-frame screenshot failed: {error}"),
                        }
                    }

                    match outcome {
                        RenderOutcome::Presented => {}
                        RenderOutcome::NeedsReconfigure => renderer.resize(window.inner_size()),
                        RenderOutcome::Skipped => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

#[derive(Debug)]
enum InitError {
    Window(winit::error::OsError),
    Renderer(RendererInitError),
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Window(error) => write!(f, "window creation failed: {error}"),
            Self::Renderer(error) => write!(f, "renderer initialization failed: {error}"),
        }
    }
}

impl From<winit::error::OsError> for InitError {
    fn from(value: winit::error::OsError) -> Self {
        Self::Window(value)
    }
}

impl From<RendererInitError> for InitError {
    fn from(value: RendererInitError) -> Self {
        Self::Renderer(value)
    }
}
