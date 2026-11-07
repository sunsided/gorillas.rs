//! GORILLAS.BAS (Copyright (C) IBM Corporation 1991), ported statement by statement.
//!
//! Each `SUB` and `FUNCTION` of the original is a method here, in the same order of
//! statements, with BASIC's semantics kept where they show: `INTEGER` variables round on
//! assignment ([`cint`]), arguments passed by reference come back as return values, undeclared
//! variables are zero (`MaxHeight` in `MakeCityScape`, `BACKGROUND` there too), and the
//! `SELECT CASE` that can never reach `CASE 4` still can't. Only the EGA path is ported: the
//! machine is a VGA, so `SCREEN 9` always works and the CGA fallback never runs.

use crate::qb::gfx::{Put, Sprite};
use crate::qb::{FALSE, Mode, Qb, TRUE, cint, cost};

const SPEEDCONST: f64 = 500.0;
const HITSELF: i32 = 1;
const BACKATTR: u8 = 0;
const OBJECTCOLOR: u8 = 1;
const WINDOWCOLOR: u8 = 14;
const SUNATTR: u8 = 3;
const SUNHAPPY: i32 = FALSE;
const SUNSHOCK: i32 = TRUE;
const RIGHTUP: i32 = 1;
const LEFTUP: i32 = 2;
const ARMSDOWN: i32 = 3;

/// `EGABanana:` the four banana pictures, one hex digit per pixel (the color), `.` for 0.
/// GORILLAS.BAS stores them as EGA `GET` arrays in `DATA` lines; the test below checks the art
/// against those.
#[rustfmt::skip]
const BANANA_LEFT: &[&str] = &[
    "....EE",
    "...EEE",
    "..EEE.",
    "..EEE.",
    "..EEE.",
    "...EEE",
    "....EE",
];
#[rustfmt::skip]
const BANANA_DOWN: &[&str] = &[
    "EE.....EE",
    "EEEEEEEEE",
    ".EEEEEEE.",
    "..EEEEE..",
];
#[rustfmt::skip]
const BANANA_UP: &[&str] = &[
    "..EEEEE..",
    ".EEEEEEE.",
    "EEEEEEEEE",
    "EE.....EE",
];
#[rustfmt::skip]
const BANANA_RIGHT: &[&str] = &[
    "EE....",
    "EEE...",
    ".EEE..",
    ".EEE..",
    ".EEE..",
    "EEE...",
    "EE....",
];

/// How to run the program.
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// Use `RANDOMIZE seed` instead of `RANDOMIZE (TIMER)`, for comparing against a copy of
    /// GORILLAS.BAS patched the same way.
    pub seed: Option<f64>,
}

#[derive(Clone, Copy, Debug, Default)]
struct XyPoint {
    x: i32,
    y: i32,
}

/// The `DIM SHARED` variables.
struct Gorillas {
    qb: Qb,
    opts: Options,
    gorilla_x: [i32; 3],
    gorilla_y: [i32; 3],
    last_building: i32,
    pi: f64,
    lban: Sprite,
    rban: Sprite,
    uban: Sprite,
    dban: Sprite,
    gor_d: Sprite,
    gor_l: Sprite,
    gor_r: Sprite,
    gravity: f64,
    wind: i32,
    scr_height: i32,
    scr_width: i32,
    max_col: i32,
    explosion_color: u8,
    back_color: u8,
    sun_hit: i32,
    sun_ht: i32,
    g_height: i32,
    mach_speed: f32,
}

/// `FnRan(x) = INT(RND(1) * x) + 1`.
fn fn_ran(qb: &Qb, x: i32) -> i32 {
    (qb.rnd() * x as f32).floor() as i32 + 1
}

/// `Scl(n!)` in `SCREEN 9`: `CINT(n!)`.
fn scl(n: f32) -> i32 {
    cint(f64::from(n))
}

/// `VAL(s$)`.
fn val(s: &str) -> f64 {
    let s = s.trim_start_matches([' ', '\t']);
    let b = s.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'-' || b[i] == b'+') {
        i += 1;
    }
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
    }
    let mantissa = i;
    if i < b.len() && matches!(b[i], b'e' | b'E' | b'd' | b'D') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'-' || b[j] == b'+') {
            j += 1;
        }
        let digits = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > digits {
            let e = s[mantissa + 1..j].replace(['d', 'D'], "e");
            return format!("{}e{}", &s[..mantissa], e)
                .parse()
                .unwrap_or_else(|_| s[..mantissa].parse().unwrap_or(0.0));
        }
    }
    s[..mantissa].parse().unwrap_or(0.0)
}

/// `LTRIM$(STR$(n))`.
fn num_str(n: i32) -> String {
    n.to_string()
}

/// How `PRINT n` shows a number: sign or space, digits, space.
fn print_num(n: i32) -> String {
    if n < 0 {
        format!("{n} ")
    } else {
        format!(" {n} ")
    }
}

/// The main module: `InitVars`, `Intro`, then games until the players say no.
pub async fn run(qb: Qb, opts: Options) {
    let mut g = Gorillas {
        qb: qb.clone(),
        opts,
        gorilla_x: [0; 3],
        gorilla_y: [0; 3],
        last_building: 0,
        pi: 0.0,
        lban: Sprite::default(),
        rban: Sprite::default(),
        uban: Sprite::default(),
        dban: Sprite::default(),
        gor_d: Sprite::default(),
        gor_l: Sprite::default(),
        gor_r: Sprite::default(),
        gravity: 0.0,
        wind: 0,
        scr_height: 0,
        scr_width: 0,
        max_col: 0,
        explosion_color: 0,
        back_color: 0,
        sun_hit: 0,
        sun_ht: 0,
        g_height: 0,
        mach_speed: 0.0,
    };
    // DEF SEG = 0: POKE 1047, KeyFlags OR 32 turns NumLock on; the host keyboard has its own.
    g.init_vars().await;
    g.intro().await;
    loop {
        // spam:
        let (name1, name2, num_games) = g.get_inputs().await;
        g.gorilla_intro(&name1, &name2).await;
        g.play_game(&name1, &name2, num_games).await;

        qb.locate(11, 24).await;
        qb.color(5, None);
        qb.print_ln("Would you like to play again?").await;
        qb.color(7, None);
        let again = loop {
            let k = qb.inkey().await;
            if k == b"y" || k == b"n" {
                break k;
            }
        };
        qb.cls().await;
        if again != b"y" {
            break;
        }
    }
}

impl Gorillas {
    /// `InitVars:`
    async fn init_vars(&mut self) {
        let qb = self.qb.clone();
        self.pi = 4.0 * 1f64.atan();

        qb.screen(Mode::Ega).await;
        qb.palette(4, 0).await; // Check for 64K EGA

        self.mach_speed = self.calc_delay().await;

        self.scr_width = 640;
        self.scr_height = 350;
        self.g_height = 25;
        self.lban = Sprite::from_art(BANANA_LEFT);
        self.dban = Sprite::from_art(BANANA_DOWN);
        self.uban = Sprite::from_art(BANANA_UP);
        self.rban = Sprite::from_art(BANANA_RIGHT);
        self.sun_ht = 39;
    }

    /// `CalcDelay!`: counts loop passes for half a second. The count is what QBasic reaches
    /// in DOSBox; the half second is spent for real.
    async fn calc_delay(&self) -> f32 {
        let s = self.qb.timer();
        self.qb.wait_timer(|t| t - s >= 0.5).await;
        cost::MACH_SPEED
    }

    /// `Center (Row, Text$)`.
    async fn center(&self, row: i32, text: &str) {
        let col = self.max_col / 2;
        let at = f64::from(col) - (text.len() as f64 / 2.0 + 0.5);
        self.qb.locate(row, cint(at)).await;
        self.qb.print(text).await;
    }

    /// `DoExplosion (x#, y#)`.
    async fn do_explosion(&self, x: f64, y: f64) {
        let qb = &self.qb;
        qb.play("MBO0L32EFGEFDC").await;
        let radius = self.scr_height / 50;
        let inc = 0.5;
        let steps = cint(f64::from(radius) / inc);
        for k in 0..=steps {
            let c = f64::from(k) * inc;
            qb.circle(x, y, c, self.explosion_color, None, None).await;
        }
        for k in (0..=steps).rev() {
            let c = f64::from(k) * inc;
            qb.circle(x, y, c, BACKATTR, None, None).await;
            qb.for_loop(100).await;
            self.rest(0.005).await;
        }
    }

    /// `DoShot (PlayerNum, x, y)`. Returns the result and `PlayerNum`, which `DoShot` changes
    /// for its caller (BASIC passes arguments by reference).
    async fn do_shot(&mut self, mut player_num: i32, x: i32, y: i32) -> (i32, i32) {
        let qb = self.qb.clone();
        let locate_col = if player_num == 1 { 1 } else { 66 };

        qb.locate(2, locate_col).await;
        qb.print("Angle:").await;
        let mut angle = self.get_num(2, locate_col + 7).await;

        qb.locate(3, locate_col).await;
        qb.print("Velocity:").await;
        let velocity = cint(self.get_num(3, locate_col + 10).await);

        if player_num == 2 {
            angle = 180.0 - angle;
        }

        // Erase input
        let w = 30 / (80 / self.max_col);
        for i in 1..=4 {
            qb.locate(i, 1).await;
            qb.print(&" ".repeat(w as usize)).await;
            qb.locate(i, 50 / (80 / self.max_col)).await;
            qb.print(&" ".repeat(w as usize)).await;
        }

        self.sun_hit = FALSE;
        let player_hit = self.plot_shot(x, y, angle, velocity, player_num).await;
        if player_hit == 0 {
            (FALSE, player_num)
        } else {
            if player_hit == player_num {
                player_num = 3 - player_num;
            }
            self.victory_dance(player_num).await;
            (TRUE, player_num)
        }
    }

    /// `DoSun (Mouth)`.
    async fn do_sun(&self, mouth: i32) {
        let qb = &self.qb;
        let pi = self.pi;
        // set position of sun
        let x = f64::from(self.scr_width / 2);
        let y = f64::from(scl(25.0));
        let s = |n: f32| f64::from(scl(n));

        // clear old sun
        qb.line_bf(x - s(22.0), y - s(18.0), x + s(22.0), y + s(18.0), BACKATTR)
            .await;

        // draw new sun: body
        qb.circle(x, y, s(12.0), SUNATTR, None, None).await;
        qb.paint(x, y, SUNATTR, SUNATTR).await;

        // rays
        qb.line(x - s(20.0), y, x + s(20.0), y, SUNATTR).await;
        qb.line(x, y - s(15.0), x, y + s(15.0), SUNATTR).await;

        qb.line(x - s(15.0), y - s(10.0), x + s(15.0), y + s(10.0), SUNATTR)
            .await;
        qb.line(x - s(15.0), y + s(10.0), x + s(15.0), y - s(10.0), SUNATTR)
            .await;

        qb.line(x - s(8.0), y - s(13.0), x + s(8.0), y + s(13.0), SUNATTR)
            .await;
        qb.line(x - s(8.0), y + s(13.0), x + s(8.0), y - s(13.0), SUNATTR)
            .await;

        qb.line(x - s(18.0), y - s(5.0), x + s(18.0), y + s(5.0), SUNATTR)
            .await;
        qb.line(x - s(18.0), y + s(5.0), x + s(18.0), y - s(5.0), SUNATTR)
            .await;

        // mouth
        if mouth != 0 {
            // draw "o" mouth
            qb.circle(x, y + s(5.0), s(2.9), 0, None, None).await;
            qb.paint(x, y + s(5.0), 0, 0).await;
        } else {
            // draw smile
            let arc = (210.0 * pi / 180.0, 330.0 * pi / 180.0);
            qb.circle(x, y, s(8.0), 0, Some(arc), None).await;
        }

        // eyes
        qb.circle(x - 3.0, y - 2.0, 1.0, 0, None, None).await;
        qb.circle(x + 3.0, y - 2.0, 1.0, 0, None, None).await;
        qb.pset(x - 3.0, y - 2.0, 0).await;
        qb.pset(x + 3.0, y - 2.0, 0).await;
    }

    /// `DrawBan (xc#, yc#, r, bc)`.
    async fn draw_ban(&self, xc: f64, yc: f64, r: i32, bc: bool) {
        let sprite = match r {
            0 => &self.lban,
            1 => &self.uban,
            2 => &self.dban,
            3 => &self.rban,
            _ => return,
        };
        let action = if bc { Put::Pset } else { Put::Xor };
        self.qb.put(xc, yc, sprite, action).await;
    }

    /// `DrawGorilla (x, y, arms)`: draws the gorilla and `GET`s it into `GorR&`, `GorL&` or
    /// `GorD&`.
    async fn draw_gorilla(&mut self, x: i32, y: i32, arms: i32) {
        let qb = self.qb.clone();
        let pi = self.pi;
        let (x, y) = (f64::from(x), f64::from(y));
        let s = |n: f32| f64::from(scl(n));

        // draw head
        qb.line_bf(x - s(4.0), y, x + s(2.9), y + s(6.0), OBJECTCOLOR)
            .await;
        qb.line_bf(x - s(5.0), y + s(2.0), x + s(4.0), y + s(4.0), OBJECTCOLOR)
            .await;

        // draw eyes/brow
        qb.line(x - s(3.0), y + s(2.0), x + s(2.0), y + s(2.0), 0)
            .await;

        // draw nose if ega
        for i in [-2.0f64, -1.0] {
            qb.pset(x + i, y + 4.0, 0).await;
            qb.pset(x + i + 3.0, y + 4.0, 0).await;
        }

        // neck
        qb.line(x - s(3.0), y + s(7.0), x + s(2.0), y + s(7.0), OBJECTCOLOR)
            .await;

        // body
        qb.line_bf(x - s(8.0), y + s(8.0), x + s(6.9), y + s(14.0), OBJECTCOLOR)
            .await;
        qb.line_bf(
            x - s(6.0),
            y + s(15.0),
            x + s(4.9),
            y + s(20.0),
            OBJECTCOLOR,
        )
        .await;

        // legs
        for i in 0..=4 {
            let i = i as f32;
            qb.circle(
                x + s(i),
                y + s(25.0),
                s(10.0),
                OBJECTCOLOR,
                Some((3.0 * pi / 4.0, 9.0 * pi / 8.0)),
                None,
            )
            .await;
            qb.circle(
                x + s(-6.0) + s(i - 0.1),
                y + s(25.0),
                s(10.0),
                OBJECTCOLOR,
                Some((15.0 * pi / 8.0, pi / 4.0)),
                None,
            )
            .await;
        }

        // chest
        qb.circle(
            x - s(4.9),
            y + s(10.0),
            s(4.9),
            0,
            Some((3.0 * pi / 2.0, 0.0)),
            None,
        )
        .await;
        qb.circle(
            x + s(4.9),
            y + s(10.0),
            s(4.9),
            0,
            Some((pi, 3.0 * pi / 2.0)),
            None,
        )
        .await;

        let left = (3.0 * pi / 4.0, 5.0 * pi / 4.0);
        let right = (7.0 * pi / 4.0, pi / 4.0);
        for i in -5..=-1 {
            let i = i as f32;
            let (ly, ry) = match arms {
                // Right arm up
                RIGHTUP => (14.0, 4.0),
                // Left arm up
                LEFTUP => (4.0, 14.0),
                // Both arms down
                _ => (14.0, 14.0),
            };
            qb.circle(
                x + s(i - 0.1),
                y + s(ly),
                s(9.0),
                OBJECTCOLOR,
                Some(left),
                None,
            )
            .await;
            qb.circle(
                x + s(4.9) + s(i),
                y + s(ry),
                s(9.0),
                OBJECTCOLOR,
                Some(right),
                None,
            )
            .await;
            let got = qb
                .get(x - s(15.0), y - s(1.0), x + s(14.0), y + s(28.0))
                .await;
            match arms {
                RIGHTUP => self.gor_r = got,
                LEFTUP => self.gor_l = got,
                _ => self.gor_d = got,
            }
        }
    }

    /// `ExplodeGorilla (x#, y#)`: returns the player hit.
    async fn explode_gorilla(&self, x: f64, _y: f64) -> i32 {
        let qb = &self.qb;
        let y_adj = f64::from(scl(12.0));
        let x_adj = f64::from(scl(5.0));
        let scl_x = f64::from(self.scr_width) / 320.0;
        let scl_y = f64::from(self.scr_height) / 200.0;
        let player_hit = if x < f64::from(self.scr_width) / 2.0 {
            1
        } else {
            2
        };
        qb.play("MBO0L16EFGEFDC").await;

        let gx = f64::from(self.gorilla_x[player_hit]);
        let gy = f64::from(self.gorilla_y[player_hit]);
        let cx = gx + 3.5 * scl_x + x_adj;
        let ec = self.explosion_color;

        let mut i = 1;
        while f64::from(i) <= 8.0 * scl_x {
            qb.circle(
                cx,
                gy + 7.0 * scl_y + y_adj,
                f64::from(i),
                ec,
                None,
                Some(-1.57),
            )
            .await;
            let ly = gy + 9.0 * scl_y - f64::from(i);
            qb.line(gx + 7.0 * scl_x, ly, gx, ly, ec).await;
            i += 1;
        }

        let mut i = 1;
        while f64::from(i) <= 16.0 * scl_x {
            if f64::from(i) < 8.0 * scl_x {
                let r = (8.0 * scl_x + 1.0) - f64::from(i);
                qb.circle(cx, gy + 7.0 * scl_y + y_adj, r, BACKATTR, None, Some(-1.57))
                    .await;
            }
            let c = (i % 2 + 1) as u8;
            qb.circle(cx, gy + y_adj, f64::from(i), c, None, Some(-1.57))
                .await;
            i += 1;
        }

        let mut i = cint(24.0 * scl_x);
        while i >= 1 {
            qb.circle(cx, gy + y_adj, f64::from(i), BACKATTR, None, Some(-1.57))
                .await;
            qb.for_loop(200).await;
            i -= 1;
        }

        player_hit as i32
    }

    /// `GetInputs (Player1$, Player2$, NumGames)`.
    async fn get_inputs(&mut self) -> (String, String, i32) {
        let qb = self.qb.clone();
        qb.color(7, Some(0));
        qb.cls().await;

        qb.locate(8, 15).await;
        let mut player1 = qb
            .input("Name of Player 1 (Default = 'Player 1'): ", false)
            .await;
        if player1.is_empty() {
            player1 = "Player 1".into();
        } else {
            player1 = player1.chars().take(10).collect();
        }

        qb.locate(10, 15).await;
        let mut player2 = qb
            .input("Name of Player 2 (Default = 'Player 2'): ", false)
            .await;
        if player2.is_empty() {
            player2 = "Player 2".into();
        } else {
            player2 = player2.chars().take(10).collect();
        }

        let mut num_games;
        loop {
            qb.locate(12, 56).await;
            qb.print(&" ".repeat(25)).await;
            qb.locate(12, 13).await;
            let game = qb
                .input("Play to how many total points (Default = 3)", true)
                .await;
            let game = game.trim().to_string();
            num_games = cint(val(&game.chars().take(2).collect::<String>()));
            if (num_games > 0 && game.len() < 3) || game.is_empty() {
                break;
            }
        }
        if num_games == 0 {
            num_games = 3;
        }

        loop {
            qb.locate(14, 53).await;
            qb.print(&" ".repeat(28)).await;
            qb.locate(14, 17).await;
            let grav = qb.input("Gravity in Meters/Sec (Earth = 9.8)", true).await;
            let grav = grav.trim().to_string();
            self.gravity = val(&grav);
            if self.gravity > 0.0 || grav.is_empty() {
                break;
            }
        }
        if self.gravity == 0.0 {
            self.gravity = 9.8;
        }
        (player1, player2, num_games)
    }

    /// `GetNum# (Row, Col)`: reads a number, echoing it with a `_` cursor.
    async fn get_num(&self, row: i32, col: i32) -> f64 {
        let qb = &self.qb;
        let mut result = String::new();
        let mut done = false;
        qb.flush_keys().await; // Clear keyboard buffer

        while !done {
            qb.locate(row, col).await;
            qb.print(&format!("{result}_    ")).await;

            let kbd = qb.inkey().await;
            match kbd.as_slice() {
                [c @ b'0'..=b'9'] => result.push(*c as char),
                [b'.'] => {
                    if !result.contains('.') {
                        result.push('.');
                    }
                }
                [13] => {
                    if val(&result) > 360.0 {
                        result.clear();
                    } else {
                        done = true;
                    }
                }
                [8] => {
                    result.pop();
                }
                [] => {}
                _ => qb.beep().await,
            }
        }

        qb.locate(row, col).await;
        qb.print(&format!("{result} ")).await;

        val(&result)
    }

    /// `GorillaIntro (Player1$, Player2$)`: draws the three gorilla pictures and `GET`s them,
    /// then shows the dance if the players ask for it.
    async fn gorilla_intro(&mut self, player1: &str, player2: &str) {
        let qb = self.qb.clone();
        qb.locate(16, 34).await;
        qb.print_ln("--------------").await;
        qb.locate(18, 34).await;
        qb.print_ln("V = View Intro").await;
        qb.locate(19, 34).await;
        qb.print_ln("P = Play Game").await;
        qb.locate(21, 35).await;
        qb.print_ln("Your Choice?").await;

        let char_ = qb.wait_key().await;

        let (x, y) = (278, 175);

        qb.screen(Mode::Ega).await;
        self.set_screen().await;

        qb.view_print(9, 24);

        qb.palette(OBJECTCOLOR, self.back_color).await;

        self.draw_gorilla(x, y, ARMSDOWN).await;
        qb.cls().await;
        self.draw_gorilla(x, y, LEFTUP).await;
        qb.cls().await;
        self.draw_gorilla(x, y, RIGHTUP).await;
        qb.cls().await;

        qb.view_print(1, 25);
        qb.palette(OBJECTCOLOR, 46).await;

        if char_.eq_ignore_ascii_case(b"V") {
            self.center(2, "Q B A S I C   G O R I L L A S").await;
            self.center(5, "             STARRING:               ")
                .await;
            let p = format!("{player1} AND {player2}");
            self.center(7, &p).await;

            let (x, y) = (f64::from(x), f64::from(y));
            let (l, r) = (x - 13.0, x + 47.0);
            qb.put(l, y, &self.gor_d, Put::Pset).await;
            qb.put(r, y, &self.gor_d, Put::Pset).await;
            self.rest(1.0).await;

            qb.put(l, y, &self.gor_l, Put::Pset).await;
            qb.put(r, y, &self.gor_r, Put::Pset).await;
            qb.play("t120o1l16b9n0baan0bn0bn0baaan0b9n0baan0b").await;
            self.rest(0.3).await;

            qb.put(l, y, &self.gor_r, Put::Pset).await;
            qb.put(r, y, &self.gor_l, Put::Pset).await;
            qb.play("o2l16e-9n0e-d-d-n0e-n0e-n0e-d-d-d-n0e-9n0e-d-d-n0e-")
                .await;
            self.rest(0.3).await;

            qb.put(l, y, &self.gor_l, Put::Pset).await;
            qb.put(r, y, &self.gor_r, Put::Pset).await;
            qb.play("o2l16g-9n0g-een0g-n0g-n0g-eeen0g-9n0g-een0g-")
                .await;
            self.rest(0.3).await;

            qb.put(l, y, &self.gor_r, Put::Pset).await;
            qb.put(r, y, &self.gor_l, Put::Pset).await;
            qb.play("o2l16b9n0baan0g-n0g-n0g-eeen0o1b9n0baan0b").await;
            self.rest(0.3).await;

            for _ in 1..=4 {
                qb.put(l, y, &self.gor_l, Put::Pset).await;
                qb.put(r, y, &self.gor_r, Put::Pset).await;
                qb.play("T160O0L32EFGEFDC").await;
                self.rest(0.1).await;
                qb.put(l, y, &self.gor_r, Put::Pset).await;
                qb.put(r, y, &self.gor_l, Put::Pset).await;
                qb.play("T160O0L32EFGEFDC").await;
                self.rest(0.1).await;
            }
        }
    }

    /// `Intro`.
    async fn intro(&mut self) {
        let qb = self.qb.clone();
        qb.screen(Mode::Text).await;
        self.max_col = 80;
        qb.color(15, Some(0));
        qb.cls().await;

        self.center(4, "Q B a s i c    G O R I L L A S").await;
        qb.color(7, None);
        self.center(6, "Copyright (C) IBM Corporation 1991").await;
        self.center(8, "Your mission is to hit your opponent with the exploding")
            .await;
        self.center(
            9,
            "banana by varying the angle and power of your throw, taking",
        )
        .await;
        self.center(
            10,
            "into account wind speed, gravity, and the city skyline.",
        )
        .await;
        self.center(
            11,
            "The wind speed is shown by a directional arrow at the bottom",
        )
        .await;
        self.center(
            12,
            "of the playing field, its length relative to its strength.",
        )
        .await;
        self.center(24, "Press any key to continue").await;

        qb.play("MBT160O1L8CDEDCDL4ECC").await;
        self.sparkle_pause().await;
    }

    /// `MakeCityScape (BCoor())`.
    async fn make_city_scape(&mut self, bcoor: &mut [XyPoint]) {
        let qb = self.qb.clone();
        let mut x = 2;

        // Set the sloping trend of the city scape. NewHt is new building height
        let slope = fn_ran(&qb, 6);
        let mut new_ht = match slope {
            1 => 15,     // Upward slope
            2 => 130,    // Downward slope
            3..=5 => 15, // "V" slope - most common
            _ => 130,    // Inverted "V" slope
        };

        let bottom_line = 335; // Bottom of building
        let ht_inc = 10; // Increase value for new height
        let def_b_width = 37; // Default building height
        let random_height = 120; // Random height difference
        let w_width = 3; // Window width
        let w_height = 6; // Window height
        let w_dif_v = 15; // Counter for window spacing - vertical
        let w_dif_h = 10; // Counter for window spacing - horizontal
        let max_height = 0; // never assigned in GORILLAS.BAS

        let mut cur_building = 1;
        loop {
            match slope {
                1 => new_ht += ht_inc,
                2 => new_ht -= ht_inc,
                3..=5 => {
                    if x > self.scr_width / 2 {
                        new_ht -= 2 * ht_inc;
                    } else {
                        new_ht += 2 * ht_inc;
                    }
                }
                // CASE 4 (the inverted "V") is shadowed by CASE 3 TO 5, and slope 6 has no
                // case at all: those skylines stay level.
                _ => {}
            }

            // Set width of building and check to see if it would go off the screen
            let mut b_width = fn_ran(&qb, def_b_width) + def_b_width;
            if x + b_width > self.scr_width {
                b_width = self.scr_width - x - 2;
            }

            // Set height of building and check to see if it goes below screen
            let mut b_height = fn_ran(&qb, random_height) + new_ht;
            if b_height < ht_inc {
                b_height = ht_inc;
            }

            // Check to see if Building is too high
            if bottom_line - b_height <= max_height + self.g_height {
                b_height = max_height + self.g_height - 5;
            }

            // Set the coordinates of the building into the array
            bcoor[cur_building] = XyPoint {
                x,
                y: bottom_line - b_height,
            };

            let building_color = (fn_ran(&qb, 3) + 4) as u8;

            // Draw the building, outline first, then filled
            let (xf, bl, bh) = (f64::from(x), f64::from(bottom_line), f64::from(b_height));
            let bw = f64::from(b_width);
            qb.line_b(xf - 1.0, bl + 1.0, xf + bw + 1.0, bl - bh - 1.0, 0)
                .await;
            qb.line_bf(xf, bl, xf + bw, bl - bh, building_color).await;

            // Draw the windows
            let mut c = x + 3;
            loop {
                let mut i = b_height - 3;
                while i >= 7 {
                    let win_colr = if fn_ran(&qb, 4) == 1 { 8 } else { WINDOWCOLOR };
                    let (cf, yf) = (f64::from(c), f64::from(bottom_line - i));
                    qb.line_bf(
                        cf,
                        yf,
                        cf + f64::from(w_width),
                        yf + f64::from(w_height),
                        win_colr,
                    )
                    .await;
                    i -= w_dif_v;
                }
                c += w_dif_h;
                if c >= x + b_width - 3 {
                    break;
                }
            }

            x += b_width + 2;
            cur_building += 1;

            if x > self.scr_width - ht_inc {
                break;
            }
        }

        self.last_building = cur_building as i32 - 1;

        // Set Wind speed
        self.wind = fn_ran(&qb, 10) - 5;
        if fn_ran(&qb, 3) == 1 {
            if self.wind > 0 {
                self.wind += fn_ran(&qb, 10);
            } else {
                self.wind -= fn_ran(&qb, 10);
            }
        }

        // Draw Wind speed arrow
        if self.wind != 0 {
            let wind_line = self.wind * 3 * (self.scr_width / 320);
            let (sw, sh) = (f64::from(self.scr_width), f64::from(self.scr_height));
            let wl = f64::from(wind_line);
            let ec = self.explosion_color;
            let mid = f64::from(self.scr_width / 2);
            qb.line(mid, sh - 5.0, mid + wl, sh - 5.0, ec).await;
            let arrow_dir = if self.wind > 0 { -2.0 } else { 2.0 };
            qb.line(
                sw / 2.0 + wl,
                sh - 5.0,
                sw / 2.0 + wl + arrow_dir,
                sh - 5.0 - 2.0,
                ec,
            )
            .await;
            qb.line(
                sw / 2.0 + wl,
                sh - 5.0,
                sw / 2.0 + wl + arrow_dir,
                sh - 5.0 + 2.0,
                ec,
            )
            .await;
        }
    }

    /// `PlaceGorillas (BCoor())`.
    async fn place_gorillas(&mut self, bcoor: &[XyPoint]) {
        let qb = self.qb.clone();
        let x_adj = 14.0;
        let y_adj = 30;

        // Place gorillas on second or third building from edge
        for i in 1..=2usize {
            let b_num = if i == 1 {
                fn_ran(&qb, 2) + 1
            } else {
                self.last_building - fn_ran(&qb, 2)
            } as usize;

            let b_width = bcoor[b_num + 1].x - bcoor[b_num].x;
            self.gorilla_x[i] =
                cint(f64::from(bcoor[b_num].x) + f64::from(b_width as f32 / 2.0) - x_adj);
            self.gorilla_y[i] = bcoor[b_num].y - y_adj;
            let (gx, gy) = (f64::from(self.gorilla_x[i]), f64::from(self.gorilla_y[i]));
            qb.put(gx, gy, &self.gor_d, Put::Pset).await;
        }
    }

    /// `PlayGame (Player1$, Player2$, NumGames)`.
    async fn play_game(&mut self, player1: &str, player2: &str, num_games: i32) {
        let qb = self.qb.clone();
        let mut bcoor = [XyPoint::default(); 31];
        let mut total_wins = [0i32; 3];

        let mut j = 1;

        for _ in 1..=num_games {
            qb.cls().await;
            let seed = self.opts.seed.unwrap_or_else(|| qb.timer());
            qb.randomize(seed);
            self.make_city_scape(&mut bcoor).await;
            self.place_gorillas(&bcoor).await;
            self.do_sun(SUNHAPPY).await;
            let mut hit = FALSE;
            while hit == FALSE {
                j = 1 - j;
                qb.locate(1, 1).await;
                qb.print_ln(player1).await;
                qb.locate(1, self.max_col - 1 - player2.len() as i32).await;
                qb.print_ln(player2).await;
                let score = format!(
                    "{}>Score<{}",
                    num_str(total_wins[1]),
                    num_str(total_wins[2])
                );
                self.center(23, &score).await;
                let mut tosser = j + 1;

                // Plot the shot.  Hit is true if Gorilla gets hit.
                let (gx, gy) = (
                    self.gorilla_x[tosser as usize],
                    self.gorilla_y[tosser as usize],
                );
                (hit, tosser) = self.do_shot(tosser, gx, gy).await;

                // Reset the sun, if it got hit
                if self.sun_hit != 0 {
                    self.do_sun(SUNHAPPY).await;
                }

                if hit == TRUE {
                    update_scores(&mut total_wins, tosser, hit);
                }
            }
            qb.sleep(1.0).await;
        }

        qb.screen(Mode::Text).await;
        qb.color(7, Some(0));
        self.max_col = 80;
        qb.cls().await;

        self.center(8, "GAME OVER!").await;
        self.center(10, "Score:").await;
        qb.locate(11, 30).await;
        qb.print(player1).await;
        qb.tab(50).await;
        qb.print_ln(&print_num(total_wins[1])).await;
        qb.locate(12, 30).await;
        qb.print(player2).await;
        qb.tab(50).await;
        qb.print_ln(&print_num(total_wins[2])).await;
        self.center(24, "Press any key to continue").await;
        self.sparkle_pause().await;
        qb.color(7, Some(0));
        qb.cls().await;
    }

    /// `PlotShot (StartX, StartY, Angle#, Velocity, PlayerNum)`: flies the banana; returns
    /// the player hit, or 0.
    async fn plot_shot(
        &mut self,
        start_x: i32,
        start_y: i32,
        angle: f64,
        velocity: i32,
        player_num: i32,
    ) -> i32 {
        let qb = self.qb.clone();
        let angle = angle / 180.0 * self.pi; // Convert degree angle to radians

        let init_x_vel = angle.cos() * f64::from(velocity);
        let init_y_vel = angle.sin() * f64::from(velocity);

        let (sx, sy) = (f64::from(start_x), f64::from(start_y));
        let mut oldx = sx;
        let mut oldy = sy;

        // draw gorilla toss
        if player_num == 1 {
            qb.put(sx, sy, &self.gor_l, Put::Pset).await;
        } else {
            qb.put(sx, sy, &self.gor_r, Put::Pset).await;
        }

        // throw sound
        qb.play("MBo0L32A-L64CL16BL64A+").await;
        self.rest(0.1).await;

        // redraw gorilla
        qb.put(sx, sy, &self.gor_d, Put::Pset).await;

        let adjust = scl(4.0); // For scaling CGA

        let mut impact = false;
        let mut shot_in_sun = false;
        let mut on_screen = true;
        let mut player_hit = 0;
        let mut need_erase = false;

        let mut start_x_pos = start_x;
        let start_y_pos = start_y - adjust - 3;

        let direction = if player_num == 2 {
            start_x_pos += scl(25.0);
            scl(4.0)
        } else {
            scl(-4.0)
        };

        let mut x = 0.0f64;
        let mut y = 0.0f64;
        let mut pointval = 0;
        if velocity < 2 {
            // Shot too slow - hit self
            x = sx;
            y = sy;
            pointval = i32::from(OBJECTCOLOR);
        }

        let mut t = 0.0f64;
        let mut rot = 0;
        let mut oldrot = 0;
        let (sw, sh) = (f64::from(self.scr_width), f64::from(self.scr_height));

        while !impact && on_screen {
            self.rest(0.02).await;

            // Erase old banana, if necessary
            if need_erase {
                need_erase = false;
                self.draw_ban(oldx, oldy, oldrot, false).await;
            }

            x = f64::from(start_x_pos)
                + (init_x_vel * t)
                + (0.5 * f64::from(self.wind as f32 / 5.0) * t.powi(2));
            y = f64::from(start_y_pos)
                + ((-(init_y_vel * t)) + (0.5 * self.gravity * t.powi(2))) * (sh / 350.0);
            qb.spend(2.0 * cost::POW).await;

            if x >= sw - f64::from(scl(10.0)) || x <= 3.0 || y >= sh - 3.0 {
                on_screen = false;
            }

            if on_screen && y > 0.0 {
                // check it
                let mut look_y = 0;
                let mut look_x = scl((8 * (2 - player_num)) as f32);
                loop {
                    pointval = qb.point(x + f64::from(look_x), y + f64::from(look_y)).await;
                    if pointval == 0 {
                        impact = false;
                        if shot_in_sun
                            && ((f64::from(self.scr_width / 2) - x).abs() > f64::from(scl(20.0))
                                || y > f64::from(self.sun_ht))
                        {
                            shot_in_sun = false;
                        }
                    } else if pointval == i32::from(SUNATTR) && y < f64::from(self.sun_ht) {
                        if self.sun_hit == 0 {
                            self.do_sun(SUNSHOCK).await;
                        }
                        self.sun_hit = TRUE;
                        shot_in_sun = true;
                    } else {
                        impact = true;
                    }
                    look_x += direction;
                    look_y += scl(6.0);
                    if impact || look_x != scl(4.0) {
                        break;
                    }
                }

                if !shot_in_sun && !impact {
                    // plot it
                    rot = cint(t * 10.0).rem_euclid(4);
                    self.draw_ban(x, y, rot, true).await;
                    need_erase = true;
                }

                oldx = x;
                oldy = y;
                oldrot = rot;
            }

            t += 0.1;
        }

        if pointval != i32::from(OBJECTCOLOR) && impact {
            let a = f64::from(adjust);
            self.do_explosion(x + a, y + a).await;
        } else if pointval == i32::from(OBJECTCOLOR) {
            player_hit = self.explode_gorilla(x, y).await;
        }

        player_hit
    }

    /// `Rest (t#)`: busy-waits `MachSpeed * t# / SPEEDCONST` seconds of `TIMER`.
    async fn rest(&self, t: f64) {
        let s = self.qb.timer();
        let t2 = f64::from(self.mach_speed) * t / SPEEDCONST;
        self.qb.wait_timer(|now| now - s > t2).await;
    }

    /// `SetScreen`.
    async fn set_screen(&mut self) {
        let qb = &self.qb;
        self.explosion_color = 2;
        self.back_color = 1;
        qb.palette(0, 1).await;
        qb.palette(1, 46).await;
        qb.palette(2, 44).await;
        qb.palette(3, 54).await;
        qb.palette(5, 7).await;
        qb.palette(6, 4).await;
        qb.palette(7, 3).await;
        qb.palette(9, 63).await; // Display Color
    }

    /// `SparklePause`: the marching stars around the intro and game-over screens, until a key.
    async fn sparkle_pause(&self) {
        let qb = &self.qb;
        qb.color(4, Some(0));
        let a_s = "*    ".repeat(17);
        qb.flush_keys().await; // Clear keyboard buffer

        while qb.inkey().await.is_empty() {
            for a in 1..=5usize {
                qb.locate(1, 1).await; // print horizontal sparkles
                qb.print(&a_s[a - 1..a - 1 + 80]).await;
                qb.locate(22, 1).await;
                qb.print(&a_s[5 - a..5 - a + 80]).await;

                for b in 2..=21 {
                    // Print Vertical sparkles
                    let c = (a as i32 + b) % 5;
                    let s = if c == 1 { "*" } else { " " };
                    qb.locate(b, 80).await;
                    qb.print(s).await;
                    qb.locate(23 - b, 1).await;
                    qb.print(s).await;
                }
            }
        }
    }

    /// `VictoryDance (Player)`.
    async fn victory_dance(&self, player: i32) {
        let qb = &self.qb;
        let (x, y) = (
            f64::from(self.gorilla_x[player as usize]),
            f64::from(self.gorilla_y[player as usize]),
        );
        for _ in 1..=4 {
            qb.put(x, y, &self.gor_l, Put::Pset).await;
            qb.play("MFO0L32EFGEFDC").await;
            self.rest(0.2).await;
            qb.put(x, y, &self.gor_r, Put::Pset).await;
            qb.play("MFO0L32EFGEFDC").await;
            self.rest(0.2).await;
        }
    }
}

/// `UpdateScores (Record(), PlayerNum, Results)`.
fn update_scores(record: &mut [i32; 3], player_num: i32, results: i32) {
    if results == HITSELF {
        let p = (player_num - 3).unsigned_abs() as usize;
        record[p] += 1;
    } else {
        record[player_num as usize] += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn val_parses_like_basic() {
        assert_eq!(val("45"), 45.0);
        assert_eq!(val("  12.5xyz"), 12.5);
        assert_eq!(val("1.2.3"), 1.2);
        assert_eq!(val(""), 0.0);
        assert_eq!(val("abc"), 0.0);
        assert_eq!(val("-3"), -3.0);
    }

    #[test]
    fn banana_art_matches_the_data_lines() {
        // EGABanana: in GORILLAS.BAS.
        let banana_left: [i32; 9] = [
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
        let banana_down: [i32; 9] = [
            262_153,
            -2_134_835_200,
            -2_134_802_239,
            -2_130_771_968,
            -2_130_738_945,
            8_323_072,
            8_323_199,
            4_063_232,
            4_063_294,
        ];
        let banana_up: [i32; 9] = [
            262_153,
            4_063_232,
            4_063_294,
            8_323_072,
            8_323_199,
            -2_130_771_968,
            -2_130_738_945,
            -2_134_835_200,
            -2_134_802_239,
        ];
        let banana_right: [i32; 9] = [
            458_758,
            -1_061_109_760,
            -522_133_504,
            1_886_416_896,
            1_886_416_896,
            1_886_416_896,
            -522_133_504,
            -1_061_109_760,
            0,
        ];
        assert_eq!(
            Sprite::from_art(BANANA_LEFT),
            Sprite::from_ega_longs(&banana_left)
        );
        assert_eq!(
            Sprite::from_art(BANANA_DOWN),
            Sprite::from_ega_longs(&banana_down)
        );
        assert_eq!(
            Sprite::from_art(BANANA_UP),
            Sprite::from_ega_longs(&banana_up)
        );
        assert_eq!(
            Sprite::from_art(BANANA_RIGHT),
            Sprite::from_ega_longs(&banana_right)
        );
    }

    #[test]
    fn hits_score_for_the_thrower_or_the_other_one() {
        let mut r = [0; 3];
        // DoShot hands back the winner as the thrower, and Hit = TRUE is not HITSELF.
        update_scores(&mut r, 2, TRUE);
        assert_eq!(r, [0, 0, 1]);
    }
}
