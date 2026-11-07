//! What statements cost, in microseconds of emulated time.
//!
//! Measured with a timing program in QBasic 1.1 under DOSBox-X at `cycles=fixed 3000` (the
//! DOSBox default for real-mode programs), so the drawing animations, explosions and the
//! sparkle border run at the speed people know from DOSBox. GORILLAS.BAS's own `Rest` delays
//! are scaled by the loop count `CalcDelay` measures; see [`MACH_SPEED`].

/// What `CalcDelay` counts in half a second at 3000 cycles (`DO: i! = i! + 1: LOOP UNTIL
/// TIMER - s! >= .5`).
pub const MACH_SPEED: f32 = 328.0;

/// A simple statement or expression.
pub const STMT: f64 = 25.0;
/// One pass of an empty `FOR` loop.
pub const FOR_ITERATION: f64 = 27.3;
/// One round of a `TIMER` busy-wait loop.
pub const TIMER_POLL: f64 = 1_100.0;
/// `INKEY$`.
pub const INKEY: f64 = 120.0;
/// `SCREEN`, `WIDTH`.
pub const SCREEN: f64 = 20_000.0;
/// `CLS`, per text row.
pub const CLS_PER_ROW_TEXT: f64 = 30.0;
pub const CLS_PER_ROW_EGA: f64 = 1_150.0;
/// `LOCATE`.
pub const LOCATE: f64 = 60.0;
/// `PRINT`, plus per character.
pub const PRINT: f64 = 170.0;
pub const PRINT_CHAR_TEXT: f64 = 44.0;
pub const PRINT_CHAR_EGA: f64 = 67.0;
/// `PSET`.
pub const PSET: f64 = 250.0;
/// `POINT`.
pub const POINT: f64 = 130.0;
/// `LINE`, plus per pixel.
pub const LINE: f64 = 200.0;
pub const LINE_PER_PIXEL: f64 = 4.5;
/// `LINE ..., BF` per row, plus per pixel of the row.
pub const FILL_PER_ROW: f64 = 30.0;
pub const FILL_PER_PIXEL: f64 = 0.9;
/// `CIRCLE`, plus per point computed (8 per step of the algorithm). Ellipses taller than wide
/// take a slower path.
pub const CIRCLE: f64 = 800.0;
pub const CIRCLE_PER_POINT: f64 = 14.0;
/// With an explicit aspect ratio.
pub const CIRCLE_PER_POINT_ASPECT: f64 = 37.0;
/// Extra for start and end angles.
pub const CIRCLE_ARC: f64 = 7_700.0;
/// `PAINT`, plus per pixel filled.
pub const PAINT: f64 = 300.0;
pub const PAINT_PER_PIXEL: f64 = 2.0;
/// `GET` / `PUT`, plus per row.
pub const PUT: f64 = 500.0;
pub const PUT_PER_ROW_BYTE: f64 = 55.0;
/// Double-precision arithmetic with `^` (PlotShot's trajectory step).
pub const POW: f64 = 5_200.0;
/// `RND` (through `FnRan`).
pub const RND: f64 = 900.0;
/// `LINE ..., BF` setup.
pub const FILL: f64 = 300.0;
