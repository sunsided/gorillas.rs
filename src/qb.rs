//! A QBasic runtime on an emulated PC: just the statements GORILLAS.BAS uses, with their
//! quirks, on top of [`game_toolkit_dosvideo`]'s VGA card and PC speaker.
//!
//! - `SCREEN 0` text goes straight into the emulated text buffer at `B800:0000`.
//! - `SCREEN 9` graphics are drawn into a shadow frame buffer ([`gfx::Ega`]) and copied into
//!   the four EGA planes before emulated time passes, so the beam shows them as the program
//!   draws.
//! - Time is the emulated machine's. Statements cost what they cost QBasic 1.1 in DOSBox at
//!   3000 cycles (measured, see [`cost`]); busy-wait loops on `TIMER` wait for the 18.2 Hz
//!   BIOS tick like the real thing.
//! - `PLAY` keeps QBasic's persistent state and background queue ([`play::Music`]).

pub mod cost;
mod exec;
pub mod gfx;
pub mod play;
pub mod rnd;

use std::cell::RefCell;
use std::rc::Rc;

use game_toolkit_dosvideo::Dos;

pub use exec::Program;
use exec::Yield;
use gfx::{Ega, H, Put, SCREEN9_ASPECT, Sprite, W};
use play::{Music, Note, QUEUE_NOTES, Speaker};
use rnd::Rnd;

/// `TRUE` in BASIC.
pub const TRUE: i32 = -1;
/// `FALSE` in BASIC.
pub const FALSE: i32 = 0;

/// Emulated time the program runs before it yields to the host.
const SLICE_NS: f64 = 1_000_000.0;

/// BIOS timer ticks per second.
pub const TICK_HZ: f64 = 1_193_180.0 / 65_536.0;

/// `CINT`, and what assigning a float to an `INTEGER` does: round half to even.
pub fn cint(x: f64) -> i32 {
    x.round_ties_even() as i32
}

/// The display mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// `SCREEN 0`, 80x25 text.
    Text,
    /// `SCREEN 9`, EGA 640x350 in 16 colors.
    Ega,
}

/// Everything the running program can touch.
pub struct Machine {
    pub dos: Dos,
    pub ega: Ega,
    pub mode: Mode,
    /// Text cursor, 1-based.
    pub row: i32,
    pub col: i32,
    pub fg: u8,
    pub bg: u8,
    /// `VIEW PRINT top TO bottom`.
    pub view: (i32, i32),
    pub music: Music,
    pub rnd: Rnd,
    /// BIOS ticks since midnight at machine time zero, for `TIMER`.
    pub ticks0: f64,
    slice_end: f64,
    buf: Vec<u8>,
}

impl Machine {
    /// A machine whose clock reads `seconds_since_midnight` at power on.
    pub fn new(dos: Dos, seconds_since_midnight: f64) -> Self {
        Self {
            dos,
            ega: Ega::new(),
            mode: Mode::Text,
            row: 1,
            col: 1,
            fg: 7,
            bg: 0,
            view: (1, 25),
            music: Music::default(),
            rnd: Rnd::default(),
            ticks0: (seconds_since_midnight * TICK_HZ).floor(),
            slice_end: 0.0,
            buf: Vec::new(),
        }
    }

    /// Copy what changed in the shadow frame buffer into the EGA planes.
    pub fn flush(&mut self) {
        let Some([x0, y0, x1, y1]) = self.ega.take_dirty() else {
            return;
        };
        if self.mode != Mode::Ega {
            return;
        }
        let (bx0, bx1) = (x0 / 8, x1 / 8);
        let mut buf = std::mem::take(&mut self.buf);
        for plane in 0..4u8 {
            self.dos.map_mask(1 << plane);
            for y in y0..=y1 {
                self.ega.plane_bytes(y, bx0, bx1, plane, &mut buf);
                self.dos.poke_slice(0xA0000 + (y * 80 + bx0) as u32, &buf);
            }
        }
        self.dos.map_mask(0x0F);
        self.buf = buf;
    }

    /// Let `ns` of emulated time pass, playing music on the way.
    pub fn advance(&mut self, ns: f64) {
        self.flush();
        let target = self.dos.now_ns() + ns.max(0.0);
        while let Some(t) = self.music.next_event() {
            if t > target {
                break;
            }
            let t = t.max(self.dos.now_ns());
            self.dos.idle_until(t);
            match self.music.fire(t) {
                Some(Speaker::Tone(hz)) => self.dos.sound(hz),
                Some(Speaker::Off) => self.dos.nosound(),
                None => {}
            }
        }
        self.dos.idle_until(target);
    }

    /// BIOS ticks since midnight.
    pub fn ticks(&self) -> f64 {
        self.ticks0 + self.dos.bios_ticks() as f64
    }

    fn text_attr(&self) -> u8 {
        ((self.bg & 7) << 4) | (self.fg & 15) | if self.fg >= 16 { 0x80 } else { 0 }
    }

    fn put_char(&mut self, ch: u8) {
        if self.col > 80 {
            self.newline();
        }
        match self.mode {
            Mode::Text => {
                let attr = self.text_attr();
                self.dos
                    .text_put((self.col - 1) as u32, (self.row - 1) as u32, ch, attr);
            }
            Mode::Ega => {
                let (fg, bg) = (self.fg, self.bg);
                self.ega.glyph(self.row, self.col, ch, fg, bg);
            }
        }
        self.col += 1;
    }

    fn newline(&mut self) {
        self.col = 1;
        if self.row >= self.view.1 {
            self.scroll();
        } else {
            self.row += 1;
        }
    }

    fn scroll(&mut self) {
        let (top, bottom) = self.view;
        match self.mode {
            Mode::Text => {
                let attr = self.text_attr();
                self.dos.text_scroll_window(
                    0,
                    (top - 1) as u32,
                    80,
                    (bottom - top + 1) as u32,
                    1,
                    attr,
                );
            }
            Mode::Ega => {
                let (y0, y1) = ((top - 1) * 14, bottom * 14 - 1);
                let mut s = self.ega.get(0, y0 + 14, W - 1, y1);
                if s.h > 0 {
                    self.ega.put(0, y0, &s, Put::Pset);
                }
                s.pix.clear();
                let bg = self.bg;
                self.ega.fill(0, y1 - 13, W - 1, y1, bg);
            }
        }
    }

    fn sync_cursor(&mut self) {
        if self.mode == Mode::Text {
            self.dos
                .set_cursor((self.col.min(80) - 1) as u32, (self.row - 1) as u32);
        }
    }
}

/// A handle on the machine for the running program.
#[derive(Clone)]
pub struct Qb {
    m: Rc<RefCell<Machine>>,
}

impl Qb {
    pub fn new(m: Rc<RefCell<Machine>>) -> Self {
        Self { m }
    }

    /// Borrow the machine (never across an `.await`).
    pub fn with<R>(&self, f: impl FnOnce(&mut Machine) -> R) -> R {
        f(&mut self.m.borrow_mut())
    }

    /// Spend `us` microseconds of emulated time; yield to the host now and then.
    pub async fn spend(&self, us: f64) {
        let due = self.with(|m| {
            m.advance(us * 1e3);
            let now = m.dos.now_ns();
            if now >= m.slice_end {
                m.slice_end = now + SLICE_NS;
                true
            } else {
                false
            }
        });
        if due {
            Yield::new().await;
        }
    }

    // ---------------------------------------------------------------------------------------
    // Time

    /// `TIMER`: seconds since midnight, single precision, in BIOS ticks.
    pub fn timer(&self) -> f64 {
        self.with(|m| f64::from((m.ticks() * 65_536.0 / 1_193_180.0) as f32))
    }

    /// Wait (busily, like `DO: LOOP UNTIL TIMER - s >= t`) until `cond` holds for `TIMER`.
    pub async fn wait_timer(&self, cond: impl Fn(f64) -> bool) {
        while !cond(self.timer()) {
            self.spend(cost::TIMER_POLL).await;
        }
    }

    /// `SLEEP n`: wait `n` seconds or until a key is pressed (the key stays in the buffer).
    pub async fn sleep(&self, secs: f64) {
        let until = self.timer() + secs;
        while self.timer() < until && !self.with(|m| m.dos.kbhit()) {
            self.spend(cost::TIMER_POLL).await;
        }
    }

    // ---------------------------------------------------------------------------------------
    // Random numbers

    pub fn randomize(&self, n: f64) {
        self.with(|m| m.rnd.randomize(n));
    }

    pub fn rnd(&self) -> f32 {
        self.with(|m| {
            m.advance(cost::RND * 1e3);
            m.rnd.rnd()
        })
    }

    // ---------------------------------------------------------------------------------------
    // Keyboard

    /// `INKEY$`: `""`, one character, or `CHR$(0) + CHR$(scancode)` for extended keys.
    pub async fn inkey(&self) -> Vec<u8> {
        self.spend(cost::INKEY).await;
        self.with(|m| match m.dos.getch() {
            None => Vec::new(),
            Some(k) if k.ascii == 0 => vec![0, k.scan],
            Some(k) => vec![k.ascii],
        })
    }

    /// `WHILE INKEY$ <> "": WEND`.
    pub async fn flush_keys(&self) {
        while !self.inkey().await.is_empty() {}
    }

    /// Wait for any key and return it (`DO: k$ = INKEY$: LOOP UNTIL k$ <> ""`).
    pub async fn wait_key(&self) -> Vec<u8> {
        loop {
            let k = self.inkey().await;
            if !k.is_empty() {
                return k;
            }
        }
    }

    // ---------------------------------------------------------------------------------------
    // Screen

    /// `SCREEN mode` (0 or 9): sets the mode, clears the screen, resets the palette.
    pub async fn screen(&self, mode: Mode) {
        self.with(|m| {
            m.mode = mode;
            m.dos.set_mode(match mode {
                Mode::Text => 0x03,
                Mode::Ega => 0x10,
            });
            m.dos.hide_cursor();
            m.ega = Ega::new();
            m.row = 1;
            m.col = 1;
            m.view = (1, 25);
            (m.fg, m.bg) = match mode {
                Mode::Text => (7, 0),
                Mode::Ega => (15, 0),
            };
        });
        self.spend(cost::SCREEN).await;
    }

    pub fn mode(&self) -> Mode {
        self.with(|m| m.mode)
    }

    /// `PALETTE attribute, color`.
    pub async fn palette(&self, attribute: u8, color: u8) {
        self.with(|m| m.dos.attr(attribute & 15, color & 63));
        self.spend(cost::STMT).await;
    }

    /// `COLOR fg[, bg]`.
    pub fn color(&self, fg: u8, bg: Option<u8>) {
        self.with(|m| {
            m.fg = fg;
            if let Some(bg) = bg {
                m.bg = bg;
            }
        });
    }

    /// `VIEW PRINT top TO bottom`.
    pub fn view_print(&self, top: i32, bottom: i32) {
        self.with(|m| {
            m.view = (top, bottom);
            m.row = top;
            m.col = 1;
        });
    }

    /// `CLS` (whole text viewport, which is the whole screen here) or `CLS 2` (text
    /// viewport only); the same in GORILLAS.BAS, which has no graphics viewport.
    pub async fn cls(&self) {
        let rows = self.with(|m| {
            let (top, bottom) = m.view;
            match m.mode {
                Mode::Text => {
                    let attr = m.text_attr();
                    m.dos.text_fill(
                        0,
                        (top - 1) as u32,
                        80,
                        (bottom - top + 1) as u32,
                        b' ',
                        attr,
                    );
                }
                Mode::Ega => {
                    let bg = m.bg;
                    m.ega.clear_rows((top - 1) * 14, bottom * 14 - 1, bg);
                }
            }
            m.row = top;
            m.col = 1;
            bottom - top + 1
        });
        let per_row = match self.mode() {
            Mode::Text => cost::CLS_PER_ROW_TEXT,
            Mode::Ega => cost::CLS_PER_ROW_EGA,
        };
        self.spend(per_row * f64::from(rows)).await;
    }

    /// `LOCATE row, col`. Float columns round like `CINT`.
    pub async fn locate(&self, row: i32, col: i32) {
        self.with(|m| {
            m.row = row.clamp(1, 25);
            m.col = col.clamp(1, 80);
        });
        self.spend(cost::LOCATE).await;
    }

    /// `PRINT s;` (no newline).
    pub async fn print(&self, s: &str) {
        let n = s.len();
        self.with(|m| {
            for ch in game_toolkit_dosvideo::font::cp437(s) {
                m.put_char(ch);
            }
            m.sync_cursor();
        });
        let per_char = match self.mode() {
            Mode::Text => cost::PRINT_CHAR_TEXT,
            Mode::Ega => cost::PRINT_CHAR_EGA,
        };
        self.spend(cost::PRINT + per_char * n as f64).await;
    }

    /// `PRINT s` (with newline).
    pub async fn print_ln(&self, s: &str) {
        self.print(s).await;
        self.with(|m| {
            m.newline();
            m.sync_cursor();
        });
    }

    /// `TAB(n)` in a `PRINT` list: spaces up to column `n` (next line if already past it).
    pub async fn tab(&self, n: i32) {
        let col = self.with(|m| m.col);
        if col > n {
            self.with(Machine::newline);
        }
        let col = self.with(|m| m.col);
        self.print(&" ".repeat((n - col).max(0) as usize)).await;
    }

    /// `LINE INPUT "prompt"; v$` (or `INPUT "prompt"; v$` with `question`).
    pub async fn input(&self, prompt: &str, question: bool) -> String {
        self.print(prompt).await;
        if question {
            self.print("? ").await;
        }
        let (start_row, start_col) = self.with(|m| (m.row, m.col));
        self.with(|m| {
            m.dos.cursor_shape(13, 14);
            m.sync_cursor();
        });
        let mut text: Vec<u8> = Vec::new();
        loop {
            let k = self.inkey().await;
            match k.as_slice() {
                [] => {}
                [b'\r'] => break,
                [8] => {
                    if text.pop().is_some() {
                        self.with(|m| {
                            m.col -= 1;
                            m.put_char(b' ');
                            m.col -= 1;
                            m.sync_cursor();
                        });
                    }
                }
                [c] if *c >= 32 && text.len() < 255 => {
                    text.push(*c);
                    self.with(|m| {
                        m.put_char(*c);
                        m.sync_cursor();
                    });
                }
                _ => {}
            }
        }
        let _ = (start_row, start_col);
        self.with(|m| {
            m.dos.hide_cursor();
            m.newline();
            m.sync_cursor();
        });
        text.iter().map(|&b| b as char).collect()
    }

    // ---------------------------------------------------------------------------------------
    // Graphics

    /// `PSET (x, y), c`.
    pub async fn pset(&self, x: f64, y: f64, c: u8) {
        self.with(|m| m.ega.pset(cint(x), cint(y), c));
        self.spend(cost::PSET).await;
    }

    /// `POINT(x, y)`.
    pub async fn point(&self, x: f64, y: f64) -> i32 {
        let v = self.with(|m| m.ega.point(cint(x), cint(y)));
        self.spend(cost::POINT).await;
        v
    }

    /// `LINE (x1, y1)-(x2, y2), c`.
    pub async fn line(&self, x1: f64, y1: f64, x2: f64, y2: f64, c: u8) {
        let n = self.with(|m| m.ega.line(cint(x1), cint(y1), cint(x2), cint(y2), c));
        self.spend(cost::LINE + cost::LINE_PER_PIXEL * f64::from(n))
            .await;
    }

    /// `LINE (x1, y1)-(x2, y2), c, B`.
    pub async fn line_b(&self, x1: f64, y1: f64, x2: f64, y2: f64, c: u8) {
        let n = self.with(|m| m.ega.rect(cint(x1), cint(y1), cint(x2), cint(y2), c));
        self.spend(cost::LINE + cost::LINE_PER_PIXEL * f64::from(n))
            .await;
    }

    /// `LINE (x1, y1)-(x2, y2), c, BF`.
    pub async fn line_bf(&self, x1: f64, y1: f64, x2: f64, y2: f64, c: u8) {
        let rows = self.with(|m| m.ega.fill(cint(x1), cint(y1), cint(x2), cint(y2), c));
        let width = (cint(x1) - cint(x2)).abs() + 1;
        let per_row = cost::FILL_PER_ROW + cost::FILL_PER_PIXEL * f64::from(width);
        self.spend(cost::FILL + per_row * f64::from(rows)).await;
    }

    /// `CIRCLE (x, y), r, c[, start, end[, aspect]]`.
    pub async fn circle(
        &self,
        x: f64,
        y: f64,
        r: f64,
        c: u8,
        arc: Option<(f64, f64)>,
        aspect: Option<f64>,
    ) {
        let (start, end) = arc.map_or((None, None), |(s, e)| (Some(s as f32), Some(e as f32)));
        let explicit_aspect = aspect.is_some();
        let aspect = aspect.map_or(SCREEN9_ASPECT, |a| a as f32);
        let n = self.with(|m| {
            m.ega
                .circle(cint(x), cint(y), cint(r), c, start, end, aspect)
        });
        let per = if explicit_aspect {
            cost::CIRCLE_PER_POINT_ASPECT
        } else {
            cost::CIRCLE_PER_POINT
        };
        let arc_cost = if arc.is_some() { cost::CIRCLE_ARC } else { 0.0 };
        self.spend(cost::CIRCLE + arc_cost + per * f64::from(n))
            .await;
    }

    /// `PAINT (x, y), paint, border`.
    pub async fn paint(&self, x: f64, y: f64, paint: u8, border: u8) {
        let n = self.with(|m| m.ega.paint(cint(x), cint(y), paint, border));
        self.spend(cost::PAINT + cost::PAINT_PER_PIXEL * f64::from(n))
            .await;
    }

    /// `GET (x1, y1)-(x2, y2), array`.
    pub async fn get(&self, x1: f64, y1: f64, x2: f64, y2: f64) -> Sprite {
        let s = self.with(|m| m.ega.get(cint(x1), cint(y1), cint(x2), cint(y2)));
        self.spend(put_cost(&s)).await;
        s
    }

    /// `PUT (x, y), array, PSET|XOR`.
    pub async fn put(&self, x: f64, y: f64, s: &Sprite, action: Put) {
        self.with(|m| m.ega.put(cint(x), cint(y), s, action));
        self.spend(put_cost(s)).await;
    }

    // ---------------------------------------------------------------------------------------
    // Sound

    /// `PLAY s`.
    pub async fn play(&self, s: &str) {
        let notes = self.with(|m| m.music.parse(s));
        self.spend(cost::STMT).await;
        for n in notes {
            self.queue_note(n).await;
        }
        if !self.with(|m| m.music.background) {
            self.drain_music().await;
        }
    }

    async fn queue_note(&self, n: Note) {
        while self.with(|m| m.music.pending() >= QUEUE_NOTES) {
            self.spend(cost::TIMER_POLL).await;
        }
        self.with(|m| {
            m.music.push(n);
            m.advance(0.0);
        });
    }

    /// Foreground `PLAY`: wait until the last note has started.
    async fn drain_music(&self) {
        while self.with(|m| m.music.waiting() > 0) {
            self.spend(cost::TIMER_POLL).await;
        }
    }

    /// `BEEP`: 800 Hz for a quarter second, in the foreground.
    pub async fn beep(&self) {
        self.queue_note(Note {
            hz: Some(800.0),
            on: 0.25,
            dur: 0.25,
        })
        .await;
        self.drain_music().await;
    }

    /// A busy `FOR i = 1 TO n: NEXT` loop.
    pub async fn for_loop(&self, n: i32) {
        self.spend(cost::FOR_ITERATION * f64::from(n.max(0))).await;
    }
}

/// What `GET` or `PUT` of a sprite costs: per row, per byte of each plane.
fn put_cost(s: &Sprite) -> f64 {
    let bytes = f64::from((s.w + 7) / 8);
    cost::PUT + cost::PUT_PER_ROW_BYTE * bytes * f64::from(s.h)
}

/// Screen size in pixels in `SCREEN 9`.
pub const SCREEN9: (i32, i32) = (W, H);
