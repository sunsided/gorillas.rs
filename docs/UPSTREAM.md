# Candidates for game-toolkit-dosvideo

Things the Gorillas port had to build or work around that belong in
[game-toolkit-dosvideo](https://github.com/sunsided/game-toolkit-dosvideo).

## Accuracy

- **`Font::ega()` is not the IBM 8x14 ROM font.** It looks derived from the 8x16 VGA font;
  glyphs differ in shape and sit one row higher than what DOSBox (and real EGA/VGA BIOSes)
  draw in 350-line modes. fntcol16's `VGA-ROM.F14` (public domain) is that font byte for
  byte; the port uses it (`fonts/vga8x14.bin`).
- **`Font::vga()` differs from the IBM 8x16 ROM font in a few glyphs** (the apostrophe, for
  one). The exact 8x16 is in VileR's Oldschool PC Font Pack (CC-BY-SA-4.0); fntcol16's
  public-domain `VGA-ROM.F16`, which the port uses, differs from it in 19 glyphs.
- **Lopsided overscan border on VGA.** The border comes straight from the CRTC: blanking
  starts at `h_blank_start` and ends at `h_blank_end`. With the BIOS values for mode 10h
  (CR02 = 0x50 right after the 80 displayed characters, blank end at character 98 of 100)
  that is 16 dots of border on the left and none on the right (mode 03h: 18 and 0), so the
  picture sits visibly off-center. A real VGA shows about 8 dots on each side: blanking runs
  through the pipeline a character later than the counter compare (VESA's 640x480 timing has
  8-pixel borders left and right). Delaying the derived blanking by one character clock would
  match.
- **Numpad with NumLock.** `KEYMAP` always maps numpad keys to the navigation scancodes with
  ASCII 0. GORILLAS.BAS turns NumLock on and expects digits from the keypad. The keyboard
  could track NumLock (and the BIOS flag at `0040:0017`) and produce digits.

## Features

- **QBasic `PLAY` semantics** (`src/qb/play.rs`): `Melody` ignores `MB`/`MF`, resets octave,
  length and tempo per string and has no queue. QBasic keeps that state across `PLAY`
  statements, queues notes in the background and blocks when the queue is full (it holds
  about 24 notes in DOSBox, not the documented 32), and in `MF` returns once its last note
  has started. `N0` is a rest. A `Music` type with this behavior could sit next to
  `Melody`.
- **GW-BASIC/QBasic drawing primitives** (`src/qb/gfx.rs`): `LINE` (drawn from the first
  point, error term starting at `3 * major / 4`, fitted to QBasic's output), `CIRCLE` (GW-BASIC half-pixel algorithm,
  8.8 fixed-point aspect, arcs by point count, negative angles draw radii), `PAINT`, and EGA
  planar `GET`/`PUT` arrays (decoding the `DATA` format). `Bitmap::line`/`circle` don't match
  BASIC pixel for pixel; a `basic` drawing module would help any BASIC port.
- **`TIMER` since midnight**: `bios_ticks()` counts from power-on. An optional
  time-of-day offset in `MachineConfig` would give programs a real `TIMER` / `INT 1Ah`.

## Host

- **PC speaker audio crackles under load, especially on the web.** `host.rs` primes the
  speaker queue with 50 ms and never re-primes: after one underrun (a long frame, a GC pause)
  it runs with no cushion, and since the emulation follows frame time while the device follows
  its own clock, the queue drifts into overflow (dropped samples) or underrun (clicks). The
  port's runner (`src/app.rs`, `SpeakerQueue`) keeps ~100 ms in flight, re-primes after an
  underrun and nudges the emulation clock by up to ±5% from the queue level so both clocks
  stay locked. Worth taking over into `Runner`.

- **`DosConfig` has no `canvas_id`.** On the web the canvas is appended to `<body>`; embedding
  in a page (SvelteKit) needs `AppConfig::canvas_id`. The port has its own runner because of
  this, copying the private `SpeakerQueue` / `start_speaker` from `host.rs`. Either add
  `canvas_id` to `DosConfig` or make the speaker queue public.
- **Sound that starts late on the web.** A DOS program plays its intro music in the first
  second, before the toolkit's autoplay probe (up to 1.5 s) or the first click has started
  the audio, so it is lost; and before audio starts, `SpeakerQueue` fills up to its maximum,
  which then plays late. The port's runner holds the machine's power switch until the audio
  runs or the probe window has passed, keeps only the cushion until the device plays, and
  restarts the queue on `GameEvent::CanvasAttached(true)`. `Runner` could do the same.
- **Blocking programs.** The port runs the BASIC program as an `async fn` polled from the
  host loop (`src/qb/exec.rs`), so code can be written like DOS code (busy waits, `INKEY$`
  loops) without a thread. A `DosProgram` variant of `DosGame` taking an `async` closure would
  make that reusable, and work on wasm32.
- **Headless capture**: `DOSVIDEO_SHOT` writes one frame including the border; the port's
  `capture` command takes a timeline (keys, typing, shots on the emulated clock), writes the
  active area only (`FrameView` cropped to `active`), a frame sequence and the speaker as WAV.
  `FrameView::active_png()` and a timeline-driven headless runner would be useful upstream.
