//! Application harness: window creation, event loop, and frame dispatch.
//!
//! [`run`] is the top-level entry point. It creates a [`winit`] event loop, then
//! delegates rendering and input to [`App`], which drives [`game::GameState`] and
//! [`audio::AudioScheduler`] on each frame.

use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

use crate::{
    audio::{AudioScheduler, SoundCue},
    game::{GameState, LOGICAL_HEIGHT, LOGICAL_WIDTH},
    render::{RenderOutcome, Renderer, RendererInitError},
};

/// Fatal application error returned from [`run`].
#[derive(Debug)]
pub enum AppError {
    /// The winit event loop failed to start or encountered an unrecoverable error.
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

/// Creates the winit event loop and runs the game until the window is closed or the
/// player chooses to quit. Blocks until the application exits.
pub fn run() -> Result<(), AppError> {
    let event_loop = EventLoop::new().map_err(AppError::EventLoop)?;
    let mut app = App::new();
    event_loop.run_app(&mut app).map_err(AppError::EventLoop)
}

const TARGET_FPS: u32 = 30;
const FRAME_DURATION: Duration = Duration::from_nanos(1_000_000_000 / TARGET_FPS as u64);

/// winit [`ApplicationHandler`] that ties together the window, renderer, game state, and audio.
struct App {
    /// The OS window; `None` until [`resumed`](App::resumed) fires.
    window: Option<Arc<Window>>,
    /// GPU renderer; `None` until the window is created.
    renderer: Option<Renderer>,
    /// Top-level game state machine.
    game: GameState,
    /// Timestamp of the previous frame, used to compute `dt`.
    last_update: Instant,
    /// Deadline for the next frame; used to cap at [`TARGET_FPS`].
    next_frame: Instant,
    /// Guards against writing more than one first-frame screenshot.
    first_screenshot_taken: bool,
    /// Audio scheduler; `None` if audio initialisation failed (non-fatal).
    audio: Option<AudioScheduler>,
}

impl App {
    /// Creates a new `App`, initialising the audio scheduler if possible.
    fn new() -> Self {
        let audio = AudioScheduler::new()
            .map_err(|e| eprintln!("audio init failed: {e}"))
            .ok();
        let now = Instant::now();
        Self {
            window: None,
            renderer: None,
            game: GameState::new(),
            last_update: now,
            next_frame: now + FRAME_DURATION,
            first_screenshot_taken: false,
            audio,
        }
    }

    /// Creates the OS window and GPU renderer on first call; no-ops on subsequent calls.
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

/// Sends a list of sound cues to the scheduler: the first cue plays immediately,
/// subsequent cues are enqueued to play after it.
fn dispatch_cues(audio: &mut Option<AudioScheduler>, cues: Vec<SoundCue>) {
    if let Some(audio) = audio.as_mut() {
        let mut iter = cues.into_iter();
        if let Some(first) = iter.next() {
            audio.play(first);
        }
        for cue in iter {
            audio.enqueue_after(cue);
        }
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
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.logical_key {
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    Key::Named(NamedKey::Backspace) => self.game.handle_backspace(),
                    Key::Named(NamedKey::Enter) => {
                        let cues = self.game.handle_submit();
                        dispatch_cues(&mut self.audio, cues);
                    }
                    Key::Character(ref text) => {
                        for ch in text.chars() {
                            self.game.handle_char(ch);
                        }
                    }
                    _ => {}
                }
                if self.game.exit_requested {
                    event_loop.exit();
                }
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
                    if let Some(audio) = self.audio.as_mut() {
                        audio.tick();
                    }

                    let cues = self.game.update(dt);
                    dispatch_cues(&mut self.audio, cues);

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

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            let now = Instant::now();
            if now >= self.next_frame {
                window.request_redraw();
                self.next_frame = now + FRAME_DURATION;
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
        }
    }
}

/// Error that can occur while creating the window or renderer during [`App::init_window`].
#[derive(Debug)]
enum InitError {
    /// OS-level window creation failed.
    Window(winit::error::OsError),
    /// wgpu surface or device initialisation failed.
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
