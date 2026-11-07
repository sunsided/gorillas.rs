//! Runs the program in a game-toolkit window (or a browser canvas): steps the emulated PC in
//! real time, shows its CRT through [`DosScreen`] and plays its PC speaker.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use game_toolkit_dosvideo::game_toolkit::prelude::*;
use game_toolkit_dosvideo::{Adapter, BusTiming, Dos, DosScreen, MachineConfig};

use crate::gorillas::{self, Options};
use crate::qb::{Machine, Program, Qb};

/// How to run.
#[derive(Clone, Debug, Default)]
pub struct Config {
    /// Web only: the `id` of the `<canvas>` to draw into.
    pub canvas_id: Option<String>,
    pub options: Options,
    /// Initial CRT filter preset (0 = sharp pixels); `None` picks [`DEFAULT_CRT`].
    pub crt: Option<usize>,
}

/// The CRT filter the game starts with: the second preset of the Alt+C cycle, "soft glow"
/// (0 is off, 1 crisp scanlines).
pub const DEFAULT_CRT: usize = 2;

static PENDING: Mutex<Option<Config>> = Mutex::new(None);

/// The emulated PC: a VGA (DOSBox's default `svga_s3` behaves like one for QBasic), with
/// video memory that costs nothing to write: QBasic's own drawing speed is in the cost model.
pub fn machine_config() -> MachineConfig {
    MachineConfig::new(Adapter::Vga).with_bus(BusTiming::INSTANT)
}

/// A fresh machine with GORILLAS.BAS loaded and running.
pub fn boot(options: Options) -> (Rc<RefCell<Machine>>, Program) {
    let dos = Dos::new(machine_config());
    let m = Rc::new(RefCell::new(Machine::new(
        dos,
        crate::seconds_since_midnight(),
    )));
    let program = Program::new(gorillas::run(Qb::new(m.clone()), options));
    (m, program)
}

/// Open the window and run.
pub fn run(config: Config) -> Result<()> {
    let app = AppConfig {
        title: "QBasic Gorillas".into(),
        width: 1024,
        height: 768,
        canvas_id: config.canvas_id.clone(),
        ..Default::default()
    };
    *PENDING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(config);
    game_toolkit_dosvideo::game_toolkit::prelude::run::<Runner>(app)
}

/// Output rate of the PC speaker.
const AUDIO_RATE: u32 = 44_100;

/// PC speaker samples handed from the emulation to the audio thread.
///
/// The emulation runs on frame time, the sound card on its own clock. The queue keeps about
/// [`SpeakerQueue::TARGET`] of audio in flight; [`SpeakerQueue::trim`] tells the host loop to
/// run the machine a little faster or slower to hold it there, so the clocks stay locked
/// instead of the queue periodically overflowing (dropped samples) or running dry (clicks).
/// After an underrun (a long frame, a GC pause) it starts over with a full cushion.
#[derive(Clone, Default)]
struct SpeakerQueue(Arc<Mutex<QueueState>>);

#[derive(Default)]
struct QueueState {
    samples: VecDeque<f32>,
    /// Ran dry since the last push.
    starved: bool,
    /// The audio device has asked for samples at least once.
    started: bool,
    underruns: u32,
}

impl SpeakerQueue {
    /// Audio kept in flight: enough to ride out a dropped frame or two.
    const TARGET: usize = AUDIO_RATE as usize / 10;
    /// More than this is dropped (only before the audio device starts, normally).
    const MAX: usize = AUDIO_RATE as usize * 3 / 10;

    fn lock(&self) -> std::sync::MutexGuard<'_, QueueState> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn push(&self, samples: &[f32]) {
        let mut q = self.lock();
        if q.starved {
            q.starved = false;
            let pad = Self::TARGET.saturating_sub(q.samples.len());
            q.samples.extend(std::iter::repeat_n(0.0, pad));
        }
        q.samples.extend(samples);
        // Until the device plays, keep only the cushion: whatever plays first must be now.
        let max = if q.started { Self::MAX } else { Self::TARGET };
        let excess = q.samples.len().saturating_sub(max);
        q.samples.drain(..excess);
    }

    fn fill(&self, out: &mut [f32]) {
        let mut q = self.lock();
        q.started = true;
        for o in out.iter_mut() {
            match q.samples.pop_front() {
                Some(v) => *o = v,
                None => {
                    *o = 0.0;
                    if !q.starved {
                        q.starved = true;
                        q.underruns += 1;
                    }
                }
            }
        }
    }

    /// Speed factor for the emulation clock: above one when the queue runs low.
    fn trim(&self) -> f64 {
        let q = self.lock();
        if !q.started {
            return 1.0;
        }
        let error = (Self::TARGET as f64 - q.samples.len() as f64) / Self::TARGET as f64;
        (1.0 + 0.05 * error).clamp(0.95, 1.05)
    }

    /// Drop what is queued and start over with a fresh cushion: after the canvas was off the
    /// page, the queue holds audio from before, which would only play late.
    fn restart(&self) {
        let mut q = self.lock();
        q.samples.clear();
        q.starved = true;
    }

    /// Underruns since the last call.
    fn take_underruns(&self) -> u32 {
        std::mem::take(&mut self.lock().underruns)
    }
}

struct Runner {
    options: Options,
    machine: Rc<RefCell<Machine>>,
    program: Program,
    screen: DosScreen,
    deadline: f64,
    audio: Option<(SpeakerQueue, StreamHandle)>,
    audio_buf: Vec<f32>,
    /// Seconds waited for the sound before powering on, `None` once the machine runs.
    power_on_wait: Option<f32>,
}

/// How long to hold the power switch for the sound: on the web the toolkit asks the browser
/// whether audio may start without a click on the canvas, which takes up to 1.5 s. Waiting
/// that long lets the intro tune play when the player already clicked on the host page.
const POWER_ON_WAIT: f32 = 1.6;

fn start_speaker(ctx: &mut Context) -> Option<(SpeakerQueue, StreamHandle)> {
    let audio = ctx.audio.as_mut()?;
    let q = SpeakerQueue::default();
    q.push(&vec![0.0; SpeakerQueue::TARGET]);
    let feed = q.clone();
    let opts = PlayOptions {
        deferred: true,
        ..Default::default()
    };
    match audio.play_stream_with(move |out: &mut [f32]| feed.fill(out), AUDIO_RATE, opts) {
        Ok(h) => Some((q, h)),
        Err(e) => {
            log::warn!("PC speaker: no audio ({e})");
            None
        }
    }
}

impl Game for Runner {
    fn init(ctx: &mut Context) -> Result<Self> {
        let config = PENDING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
            .unwrap_or_default();
        let (machine, program) = boot(config.options);
        let mut screen = DosScreen::new();
        // Like DOSBox: no overscan border (dosvideo derives a lopsided one, see docs/UPSTREAM.md).
        screen.show_border = false;
        screen.set_crt_preset(config.crt.unwrap_or(DEFAULT_CRT));
        screen.upload(&mut ctx.gfx, &machine.borrow().dos);
        let deadline = machine.borrow().dos.now_ns();
        Ok(Self {
            options: config.options,
            machine,
            program,
            screen,
            deadline,
            audio: start_speaker(ctx),
            audio_buf: Vec::new(),
            power_on_wait: Some(0.0),
        })
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) {
        let alt = ctx.input.key_held(Key::AltLeft) || ctx.input.key_held(Key::AltRight);
        if alt && ctx.input.key_pressed(Key::KeyC) {
            self.screen.cycle_crt();
        } else if alt && ctx.input.key_pressed(Key::Enter) {
            let fs = ctx.window.fullscreen().is_some();
            ctx.window
                .set_fullscreen((!fs).then_some(winit::window::Fullscreen::Borderless(None)));
        } else {
            let size = ctx.gfx.size();
            self.screen
                .feed_input(&ctx.input, &mut self.machine.borrow_mut().dos, size);
        }

        if let Some(waited) = &mut self.power_on_wait {
            *waited += dt;
            let sound = ctx.audio.as_ref().is_none_or(|a| a.is_started());
            if !sound && *waited < POWER_ON_WAIT {
                return;
            }
            self.power_on_wait = None;
            self.deadline = self.machine.borrow().dos.now_ns();
        }

        let trim = self.audio.as_ref().map_or(1.0, |(q, _)| q.trim());
        self.deadline += f64::from(dt.clamp(0.0, 0.1)) * 1e9 * trim;
        let mut steps = 0;
        while self.machine.borrow().dos.now_ns() < self.deadline && steps < 5000 {
            steps += 1;
            if self.program.step() {
                // END: QBasic would drop back into its editor. Close the window natively;
                // on the web, start over.
                #[cfg(not(target_arch = "wasm32"))]
                {
                    ctx.quit();
                    return;
                }
                #[cfg(target_arch = "wasm32")]
                {
                    let (machine, program) = boot(self.options);
                    self.machine = machine;
                    self.program = program;
                    self.deadline = self.machine.borrow().dos.now_ns();
                    break;
                }
            }
        }
        let _ = self.options;
        let mut m = self.machine.borrow_mut();
        if let Some((q, _)) = &self.audio {
            self.audio_buf.clear();
            m.dos.speaker_render(AUDIO_RATE, &mut self.audio_buf);
            q.push(&self.audio_buf);
            let underruns = q.take_underruns();
            if underruns > 0 {
                log::debug!("PC speaker: {underruns} underrun(s)");
            }
        } else {
            m.dos.speaker_skip();
        }
        let now = m.dos.now_ns();
        if (self.deadline - now).abs() > 0.25e9 {
            self.deadline = now;
        }
    }

    /// The host page put the canvas back after taking it off: the toolkit kept the audio
    /// suspended meanwhile and the emulation got no frames, so start the speaker afresh and
    /// don't let the machine race to catch up.
    fn event(&mut self, _ctx: &mut Context, e: &GameEvent) {
        if let GameEvent::CanvasAttached(true) = e {
            if let Some((q, _)) = &self.audio {
                q.restart();
            }
            self.deadline = self.machine.borrow().dos.now_ns();
        }
    }

    fn render(&mut self, ctx: &mut Context, frame: &mut Frame) {
        let mut m = self.machine.borrow_mut();
        m.flush();
        m.dos.sync();
        self.screen.upload(&mut ctx.gfx, &m.dos);
        let size = ctx.gfx.size();
        {
            let mut p = frame.painter(&mut ctx.gfx);
            p.clear([0.0, 0.0, 0.0, 1.0]);
            self.screen.draw(&mut p, &m.dos, size);
        }
        self.screen.draw_crt(&ctx.gfx, frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn len(q: &SpeakerQueue) -> usize {
        q.lock().samples.len()
    }

    #[test]
    fn speaker_queue_keeps_a_cushion() {
        let q = SpeakerQueue::default();
        // Before the device plays, only the cushion is kept, so nothing plays late.
        q.push(&vec![0.5; SpeakerQueue::MAX * 2]);
        assert_eq!(len(&q), SpeakerQueue::TARGET);
        assert!(
            (q.trim() - 1.0).abs() < 1e-9,
            "no steering before the device runs"
        );

        // Running dry re-primes the cushion on the next push.
        let mut out = vec![0.0; SpeakerQueue::TARGET + 10];
        q.fill(&mut out);
        assert_eq!(q.take_underruns(), 1);
        q.push(&[0.5; 10]);
        assert_eq!(len(&q), SpeakerQueue::TARGET + 10);

        // A running device steers the emulation clock by the fill level.
        assert!((q.trim() - 1.0).abs() < 0.01);
        q.push(&vec![0.5; SpeakerQueue::TARGET]);
        assert!(q.trim() < 1.0, "too full: run the machine slower");

        // Back on the page: stale audio is dropped and the cushion rebuilt.
        q.restart();
        q.push(&[0.5; 10]);
        assert_eq!(len(&q), SpeakerQueue::TARGET + 10);
    }
}
