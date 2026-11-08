//! The `SCREEN 9` (EGA 640x350x16) drawing statements, on a shadow copy of the screen.
//!
//! `LINE` and `CIRCLE` follow the algorithms of Microsoft's GW-BASIC (released under the MIT
//! license), which QBasic inherited: a Bresenham line that always runs top to bottom, and a
//! half-pixel midpoint circle that plots eight octants at once, scales one axis by an 8.8
//! fixed-point aspect ratio and decides arc membership by counting points along the
//! circumference rather than by angle. Getting those details right is what makes the gorillas
//! (built from arcs) come out pixel for pixel like the original.
//!
//! The shadow copy holds one color index per pixel; the runtime copies changed rows into the
//! emulated EGA planes before time passes.

/// The 8x14 character set the VGA BIOS uses in 350-line graphics modes: fntcol16's
/// `VGA-ROM.F14` (public domain, see fonts/README.md), the same bytes DOSBox's BIOS uses. It
/// differs from `Font::ega()` in glyph shapes and baseline.
static FONT_8X14: &[u8; 3584] = include_bytes!("../../fonts/vga8x14.bin");

/// Screen width in pixels.
pub const W: i32 = 640;
/// Screen height in pixels.
pub const H: i32 = 350;

/// A `GET` array: a rectangle of pixels.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sprite {
    pub w: i32,
    pub h: i32,
    pub pix: Vec<u8>,
}

impl Sprite {
    /// A picture from rows of hex digits (one color per pixel), `.` or space for color 0.
    pub fn from_art(rows: &[&str]) -> Self {
        let h = rows.len() as i32;
        let w = rows.first().map_or(0, |r| r.len()) as i32;
        let pix = rows
            .iter()
            .flat_map(|r| r.chars())
            .map(|c| c.to_digit(16).unwrap_or(0) as u8)
            .collect();
        Self { w, h, pix }
    }

    /// Decode an EGA `GET` array stored as `LONG`s (as in GORILLAS.BAS's `DATA` lines): a
    /// header word with the width and one with the height, then per row the four planes, each
    /// padded to whole bytes, most significant bit leftmost.
    pub fn from_ega_longs(longs: &[i32]) -> Self {
        let bytes: Vec<u8> = longs.iter().flat_map(|l| l.to_le_bytes()).collect();
        let w = i32::from(u16::from_le_bytes([bytes[0], bytes[1]]));
        let h = i32::from(u16::from_le_bytes([bytes[2], bytes[3]]));
        let bpp = ((w + 7) / 8) as usize;
        let mut pix = vec![0u8; (w * h) as usize];
        for y in 0..h as usize {
            for plane in 0..4 {
                let row = 4 + (y * 4 + plane) * bpp;
                for x in 0..w as usize {
                    let byte = bytes.get(row + x / 8).copied().unwrap_or(0);
                    if byte & (0x80 >> (x % 8)) != 0 {
                        pix[y * w as usize + x] |= 1 << plane;
                    }
                }
            }
        }
        Self { w, h, pix }
    }
}

/// `PUT` actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Put {
    Pset,
    Xor,
}

/// The `SCREEN 9` frame buffer.
#[derive(Clone, Debug)]
pub struct Ega {
    pix: Vec<u8>,
    /// Changed area since the last flush, inclusive: `[x0, y0, x1, y1]`.
    dirty: Option<[i32; 4]>,
}

impl Default for Ega {
    fn default() -> Self {
        Self::new()
    }
}

/// Default aspect ratio of `CIRCLE` in `SCREEN 9`: 4/3 * 350/640.
pub const SCREEN9_ASPECT: f32 = 4.0 / 3.0 * 350.0 / 640.0;

impl Ega {
    pub fn new() -> Self {
        Self {
            pix: vec![0; (W * H) as usize],
            dirty: None,
        }
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pix
    }

    fn touch(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.dirty = Some(match self.dirty {
            None => [x0, y0, x1, y1],
            Some([a, b, c, d]) => [a.min(x0), b.min(y0), c.max(x1), d.max(y1)],
        });
    }

    /// Take the area changed since the last call.
    pub fn take_dirty(&mut self) -> Option<[i32; 4]> {
        self.dirty.take()
    }

    pub fn on_screen(x: i32, y: i32) -> bool {
        (0..W).contains(&x) && (0..H).contains(&y)
    }

    /// `PSET`; off-screen points are ignored.
    pub fn pset(&mut self, x: i32, y: i32, c: u8) {
        if Self::on_screen(x, y) {
            self.pix[(y * W + x) as usize] = c & 15;
            self.touch(x, y, x, y);
        }
    }

    /// `POINT`: the color at a pixel, `-1` off screen.
    pub fn point(&self, x: i32, y: i32) -> i32 {
        if Self::on_screen(x, y) {
            i32::from(self.pix[(y * W + x) as usize])
        } else {
            -1
        }
    }

    /// Fill rows `y0..=y1` with color `c` (`CLS`).
    pub fn clear_rows(&mut self, y0: i32, y1: i32, c: u8) {
        let (y0, y1) = (y0.max(0), y1.min(H - 1));
        if y0 > y1 {
            return;
        }
        self.pix[(y0 * W) as usize..((y1 + 1) * W) as usize].fill(c & 15);
        self.touch(0, y0, W - 1, y1);
    }

    /// `LINE (x1, y1)-(x2, y2), c`. Returns the number of points visited.
    ///
    /// QBasic draws from the first point to the second (unlike GW-BASIC, it doesn't swap the
    /// ends to run downwards) and starts the error term at `major - minor / 2`, which shows in
    /// the sun's rays.
    pub fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, c: u8) -> u32 {
        let clamp = |x: i32, y: i32| (x.clamp(0, W - 1), y.clamp(0, H - 1));
        let (ax, ay) = clamp(x1, y1);
        let (bx, by) = clamp(x2, y2);
        let (dx, dy) = ((bx - ax).abs(), (by - ay).abs());
        let (sx, sy) = ((bx - ax).signum(), (by - ay).signum());
        let (major, minor, x_major) = if dx > dy {
            (dx, dy, true)
        } else {
            (dy, dx, false)
        };
        let mut sum = major * 3 / 4;
        let (mut x, mut y) = (ax, ay);
        for _ in 0..=major {
            self.pset(x, y, c);
            sum += minor;
            if sum >= major {
                sum -= major;
                if x_major {
                    y += sy;
                } else {
                    x += sx;
                }
            }
            if x_major {
                x += sx;
            } else {
                y += sy;
            }
        }
        major as u32 + 1
    }

    /// `LINE (x1, y1)-(x2, y2), c, B`.
    pub fn rect(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, c: u8) -> u32 {
        self.line(x1, y1, x2, y1, c)
            + self.line(x1, y2, x2, y2, c)
            + self.line(x1, y1, x1, y2, c)
            + self.line(x2, y1, x2, y2, c)
    }

    /// `LINE (x1, y1)-(x2, y2), c, BF`. Returns the number of rows filled.
    pub fn fill(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, c: u8) -> u32 {
        let (x0, x1) = (x1.min(x2).max(0), x1.max(x2).min(W - 1));
        let (y0, y1) = (y1.min(y2).max(0), y1.max(y2).min(H - 1));
        if x0 > x1 || y0 > y1 {
            return 0;
        }
        for y in y0..=y1 {
            self.pix[(y * W + x0) as usize..=(y * W + x1) as usize].fill(c & 15);
        }
        self.touch(x0, y0, x1, y1);
        (y1 - y0 + 1) as u32
    }

    /// `CIRCLE (cx, cy), r, c, start, end, aspect`. Angles in radians; a negative angle also
    /// draws a line from the center to that end of the arc. Returns the number of points
    /// computed (for the cost model).
    #[allow(clippy::too_many_arguments)]
    pub fn circle(
        &mut self,
        cx: i32,
        cy: i32,
        r: i32,
        c: u8,
        start: Option<f32>,
        end: Option<f32>,
        aspect: f32,
    ) -> u32 {
        let r = r.max(0);
        let npts = (r as f32 * std::f32::consts::FRAC_1_SQRT_2).round_ties_even() as i32;
        let mut clinef = 0u8;
        let mut count = |a: Option<f32>, flag: u8, default: u16| -> u16 {
            match a {
                None => default,
                Some(mut a) => {
                    if a < 0.0 {
                        a = -a;
                        clinef |= flag;
                    }
                    let frac = a * (1.0 / std::f32::consts::TAU);
                    (frac * (npts * 8) as f32).round_ties_even() as i32 as u16
                }
            }
        };
        let mut s = count(start, 1, 0);
        let mut e = count(end, 0x80, 0xFFFF);
        let clinef_now = clinef;
        let mut plot_outside = false;
        let mut lines = clinef_now;
        if e < s {
            std::mem::swap(&mut s, &mut e);
            plot_outside = true;
            lines = ((clinef_now & 1) << 7) | ((clinef_now >> 7) & 1);
        }
        let (scale_x, a) = if aspect > 1.0 {
            (true, 1.0 / aspect)
        } else {
            (false, aspect)
        };
        let asp = (a * 256.0).round_ties_even() as i32 as u16;
        let scale = |v: i32| -> i32 {
            let lo = i32::from(asp & 0xFF);
            if lo == 0 {
                return if asp >> 8 != 0 { v } else { 0 };
            }
            (((lo * v) + 128) >> 8) & 0xFFFF
        };
        let mut st = Arc {
            cx,
            cy,
            c,
            s,
            e,
            plot_outside,
            lines,
            npts,
            cxoff: 0,
            cyoff: 0,
            cpcnt: 0,
            cpcnt8: 0,
        };
        // [HL] = X = 2 * radius (one bit of fraction for rounding), [DE] = Y = 0.
        let (mut x, mut y, mut sum) = (2 * r, 0i32, 0i32);
        let mut points = 0;
        loop {
            if y & 1 == 0 {
                self.plot8(&mut st, (x + 1) >> 1, (y + 1) >> 1, scale_x, &scale);
                points += 8;
                if y >= x {
                    break;
                }
            }
            sum += 2 * y + 1;
            if sum as i16 >= 0 {
                sum -= 2 * x - 1;
                x -= 1;
            }
            y += 1;
        }
        points
    }

    fn plot8(&mut self, st: &mut Arc, x: i32, y: i32, scale_x: bool, scale: &impl Fn(i32) -> i32) {
        // CPLSCX: (unscaled, scaled), swapped when the aspect ratio is above one.
        let sc = |v: i32| {
            if scale_x {
                (scale(v), v)
            } else {
                (v, scale(v))
            }
        };
        st.cpcnt = y;
        st.cpcnt8 = 0;
        let (a1, b1) = sc(y);
        st.cxoff = a1;
        let (a2, b2) = sc(x);
        st.cyoff = b2;
        let (mut hl, mut de) = (a2, -b1);
        self.plot4(st, &mut hl, &mut de);
        st.cpcnt8 = st.npts;
        st.cpcnt = st.npts - st.cpcnt;
        st.cxoff = -st.cxoff;
        de = -de;
        self.plot4(st, &mut hl, &mut de);
    }

    fn plot4(&mut self, st: &mut Arc, hl: &mut i32, de: &mut i32) {
        for k in 0..4 {
            let base = st.cpcnt8;
            st.cpcnt8 += 2 * st.npts;
            let pc = (st.cpcnt + base) as u16;
            enum Act {
                Plot,
                Line,
                Skip,
            }
            let between = if st.plot_outside {
                Act::Skip
            } else {
                Act::Plot
            };
            let outside = if st.plot_outside {
                Act::Plot
            } else {
                Act::Skip
            };
            let act = if st.s == pc {
                if st.lines & 1 != 0 {
                    Act::Line
                } else {
                    Act::Plot
                }
            } else if st.s > pc {
                outside
            } else if st.e == pc {
                if st.lines & 0x80 != 0 {
                    Act::Line
                } else {
                    Act::Plot
                }
            } else if st.e > pc {
                between
            } else {
                outside
            };
            let (px, py) = (st.cx + *hl, st.cy + *de);
            match act {
                Act::Plot => self.pset(px, py, st.c),
                Act::Line => {
                    self.line(px, py, st.cx, st.cy, st.c);
                }
                Act::Skip => {}
            }
            if k < 3 {
                let (ox, oy) = (*hl, *de);
                *hl = -st.cxoff;
                *de = -st.cyoff;
                st.cxoff = ox;
                st.cyoff = oy;
            }
        }
    }

    /// `PAINT (x, y), paint, border`: 4-connected flood fill up to the border color. Returns
    /// the number of pixels filled.
    pub fn paint(&mut self, x: i32, y: i32, paint: u8, border: u8) -> u32 {
        if !Self::on_screen(x, y) {
            return 0;
        }
        let (paint, border) = (paint & 15, border & 15);
        let mut seen = vec![false; (W * H) as usize];
        let fillable = |pix: &[u8], seen: &[bool], x: i32, y: i32| {
            Self::on_screen(x, y)
                && !seen[(y * W + x) as usize]
                && pix[(y * W + x) as usize] != border
        };
        if !fillable(&self.pix, &seen, x, y) {
            return 0;
        }
        let mut stack = vec![(x, y)];
        let mut n = 0;
        while let Some((x, y)) = stack.pop() {
            if !fillable(&self.pix, &seen, x, y) {
                continue;
            }
            let mut l = x;
            while fillable(&self.pix, &seen, l - 1, y) {
                l -= 1;
            }
            let mut r = x;
            while fillable(&self.pix, &seen, r + 1, y) {
                r += 1;
            }
            for xx in l..=r {
                seen[(y * W + xx) as usize] = true;
                self.pix[(y * W + xx) as usize] = paint;
                n += 1;
                for ny in [y - 1, y + 1] {
                    if fillable(&self.pix, &seen, xx, ny) {
                        stack.push((xx, ny));
                    }
                }
            }
            self.touch(l, y, r, y);
        }
        n
    }

    /// `GET (x1, y1)-(x2, y2), array`.
    pub fn get(&self, x1: i32, y1: i32, x2: i32, y2: i32) -> Sprite {
        let (x0, x1) = (x1.min(x2), x1.max(x2));
        let (y0, y1) = (y1.min(y2), y1.max(y2));
        let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
        let mut pix = Vec::with_capacity((w * h) as usize);
        for y in y0..=y1 {
            for x in x0..=x1 {
                pix.push(self.point(x, y).max(0) as u8);
            }
        }
        Sprite { w, h, pix }
    }

    /// `PUT (x, y), array, action`. Parts off the screen are clipped (QBasic would stop with
    /// "Illegal function call"; GORILLAS.BAS never gets there).
    pub fn put(&mut self, x: i32, y: i32, s: &Sprite, action: Put) {
        for yy in 0..s.h {
            for xx in 0..s.w {
                let (px, py) = (x + xx, y + yy);
                if !Self::on_screen(px, py) {
                    continue;
                }
                let v = s.pix[(yy * s.w + xx) as usize];
                let i = (py * W + px) as usize;
                self.pix[i] = match action {
                    Put::Pset => v,
                    Put::Xor => self.pix[i] ^ v,
                };
            }
        }
        self.touch(
            x.max(0),
            y.max(0),
            (x + s.w - 1).min(W - 1),
            (y + s.h - 1).min(H - 1),
        );
    }

    /// Draw one character cell of text (8x14, 1-based row and column), like `PRINT` does in
    /// `SCREEN 9`: foreground on a background-colored cell.
    pub fn glyph(&mut self, row: i32, col: i32, ch: u8, fg: u8, bg: u8) {
        let g = &FONT_8X14[ch as usize * 14..ch as usize * 14 + 14];
        let (x0, y0) = ((col - 1) * 8, (row - 1) * 14);
        for (r, bits) in g.iter().enumerate().take(14) {
            for b in 0..8 {
                let on = (bits >> (7 - b)) & 1 != 0;
                self.pset(x0 + b, y0 + r as i32, if on { fg } else { bg });
            }
        }
    }

    /// The planar bytes of plane `plane` for row `y`, bytes `bx0..=bx1`.
    pub fn plane_bytes(&self, y: i32, bx0: i32, bx1: i32, plane: u8, out: &mut Vec<u8>) {
        out.clear();
        let row = &self.pix[(y * W) as usize..((y + 1) * W) as usize];
        for bx in bx0..=bx1 {
            let mut v = 0u8;
            for (i, &p) in row[(bx * 8) as usize..(bx * 8 + 8) as usize]
                .iter()
                .enumerate()
            {
                if p & (1 << plane) != 0 {
                    v |= 0x80 >> i;
                }
            }
            out.push(v);
        }
    }
}

/// State of one `CIRCLE` while it plots.
struct Arc {
    cx: i32,
    cy: i32,
    c: u8,
    s: u16,
    e: u16,
    plot_outside: bool,
    lines: u8,
    npts: i32,
    cxoff: i32,
    cyoff: i32,
    cpcnt: i32,
    cpcnt8: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banana_decodes_to_color_14() {
        let left = [
            458_758,
            202_116_096,
            471_604_224,
            943_208_448,
            943_208_448,
            943_208_448,
            471_604_224,
            202_116_096,
            0,
        ];
        let s = Sprite::from_ega_longs(&left);
        assert_eq!((s.w, s.h), (6, 7));
        assert!(s.pix.iter().all(|&p| p == 0 || p == 14));
        assert!(s.pix.contains(&14));
    }

    #[test]
    fn full_circle_is_symmetric() {
        let mut g = Ega::new();
        g.circle(100, 100, 10, 3, None, None, SCREEN9_ASPECT);
        for y in 80..120 {
            for x in 85..115 {
                assert_eq!(g.point(x, y), g.point(200 - x, y), "x symmetry at {x},{y}");
                assert_eq!(g.point(x, y), g.point(x, 200 - y), "y symmetry at {x},{y}");
            }
        }
        // Radius 10 horizontally, 10 * 0.729 vertically.
        assert_eq!(g.point(110, 100), 3);
        assert_eq!(g.point(100, 107), 3);
        assert_eq!(g.point(100, 108), 0);
    }

    #[test]
    fn line_matches_qbasic_sun_ray() {
        // LINE (305, 15)-(335, 35) in QBasic: offsets 0, 1, 2, 2, 3 for the first five columns.
        let mut g = Ega::new();
        g.line(305, 15, 335, 35, 3);
        let ys: Vec<i32> = (305..310)
            .map(|x| (0..H).find(|&y| g.point(x, y) == 3).unwrap() - 15)
            .collect();
        assert_eq!(ys, [0, 1, 2, 2, 3]);
        assert_eq!(g.point(335, 35), 3);
    }
}
