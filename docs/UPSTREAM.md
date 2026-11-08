# Candidates for game-toolkit-dosvideo

Things the Gorillas port had to build or work around that belong in
[game-toolkit-dosvideo](https://github.com/sunsided/game-toolkit-dosvideo).

Taken up in 0.2.2: the fntcol16 VGA fonts, the centered overscan border, the clock-locked
speaker queue, the power-on wait for audio, and `DosConfig::with_canvas_id`. The port still runs
its own host loop (`src/app.rs`): its program is an `async fn` that owns the machine, and the
speaker queue it needs is private in dosvideo. Either making that public or an async program
runner (below) would let it go.

## Accuracy

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

- **Blocking programs.** The port runs the BASIC program as an `async fn` polled from the
  host loop (`src/qb/exec.rs`), so code can be written like DOS code (busy waits, `INKEY$`
  loops) without a thread. A `DosProgram` variant of `DosGame` taking an `async` closure would
  make that reusable, and work on wasm32.
- **Headless capture**: `DOSVIDEO_SHOT` writes one frame including the border; the port's
  `capture` command takes a timeline (keys, typing, shots on the emulated clock), writes the
  active area only (`FrameView` cropped to `active`), a frame sequence and the speaker as WAV.
  `FrameView::active_png()` and a timeline-driven headless runner would be useful upstream.
