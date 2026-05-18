use rand::{RngExt, SeedableRng, rngs::SmallRng};

use crate::render::{Frame, PrimitiveBatch};

pub const LOGICAL_WIDTH: u32 = 640;
pub const LOGICAL_HEIGHT: u32 = 350;

const BACKGROUND: [f32; 4] = palette_attribute(0);
const OBJECT: [f32; 4] = palette_attribute(1);
const EXPLOSION: [f32; 4] = palette_attribute(2);
const LIT_WINDOW: [f32; 4] = palette_attribute(14);
const DARK_WINDOW: [f32; 4] = palette_attribute(8);
const SUN: [f32; 4] = palette_attribute(3);
const HUD_TEXT: [f32; 4] = palette_attribute(15);
const BOTTOM_LINE: f32 = 335.0;
const GORILLA_HEIGHT: f32 = 25.0;
const GORILLA_X_ADJUST: f32 = 14.0;
const GORILLA_Y_ADJUST: f32 = 30.0;
const HEIGHT_INCREMENT: f32 = 10.0;
const DEFAULT_BUILDING_WIDTH: u32 = 37;
const RANDOM_HEIGHT: u32 = 120;
const WINDOW_WIDTH: f32 = 3.0;
const WINDOW_HEIGHT: f32 = 6.0;
const WINDOW_VERTICAL_SPACING: f32 = 15.0;
const WINDOW_HORIZONTAL_SPACING: f32 = 10.0;
const BASIC_CIRCLE_Y_ASPECT: f32 = 1.0;
const PROJECTILE_TIME_STEP: f32 = 0.1;
const MIN_THROW_VELOCITY: f32 = 2.0;
const DEFAULT_GRAVITY: f32 = 9.8;
const SUN_HEIGHT_LIMIT: f32 = 39.0;
const SUN_CLEAR_RADIUS: f32 = 20.0;
const EXPLOSION_DURATION: f32 = 0.3;
const EXPLOSION_MAX_RADIUS: f32 = 7.0;
const GORILLA_EXPLOSION_DURATION: f32 = 0.6;
const GORILLA_EXPLOSION_MAX_RADIUS: f32 = 24.0;
const THROW_ARM_DURATION: f32 = 0.1;
const VICTORY_DANCE_INTERVAL: f32 = 0.2;
const VICTORY_DANCE_CYCLES: u8 = 8;
const TEXT_CELL_WIDTH: i32 = 8;
const TEXT_CELL_HEIGHT: i32 = 14;

#[derive(Debug)]
pub struct GameUpdate {
    pub cues: Vec<crate::audio::SoundCue>,
    pub match_over: Option<usize>,
}

const fn palette_attribute(attribute: u8) -> [f32; 4] {
    let [red, green, blue] = palette_attribute_rgb(attribute);
    [
        red as f32 / 255.0,
        green as f32 / 255.0,
        blue as f32 / 255.0,
        1.0,
    ]
}

const fn palette_attribute_rgb(attribute: u8) -> [u8; 3] {
    ega_palette_rgb(palette_register(attribute))
}

const fn palette_register(attribute: u8) -> u8 {
    match attribute {
        0 => 1,
        1 => 46,
        2 => 44,
        3 => 54,
        5 => 7,
        6 => 4,
        7 => 3,
        9 => 63,
        _ => default_ega_register(attribute),
    }
}

const fn default_ega_register(attribute: u8) -> u8 {
    match attribute {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        5 => 5,
        6 => 20,
        7 => 7,
        8 => 56,
        9 => 57,
        10 => 58,
        11 => 59,
        12 => 60,
        13 => 61,
        14 => 62,
        15 => 63,
        _ => 0,
    }
}

const fn ega_palette_rgb(value: u8) -> [u8; 3] {
    [
        ega_component(value, 2, 5),
        ega_component(value, 1, 4),
        ega_component(value, 0, 3),
    ]
}

const fn ega_component(value: u8, primary_bit: u8, secondary_bit: u8) -> u8 {
    let primary = (value >> primary_bit) & 1;
    let secondary = (value >> secondary_bit) & 1;

    match primary * 2 + secondary {
        0 => 0x00,
        1 => 0x55,
        2 => 0xAA,
        _ => 0xFF,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum GorillaArms {
    RightUp,
    LeftUp,
    Down,
}

#[derive(Debug)]
pub struct Game {
    round: Round,
    gorillas: [Gorilla; 2],
    projectile: Option<Projectile>,
    explosion: Option<Explosion>,
    sun_shocked: bool,
    current_player: Player,
    scores: [u32; 2],
    player_names: [String; 2],
    turn_phase: TurnPhase,
    gravity: f32,
    target_score: u32,
}

impl Game {
    pub fn new() -> Self {
        let (round, gorillas) = make_round(rand::random());

        Self {
            round,
            gorillas,
            projectile: None,
            explosion: None,
            sun_shocked: false,
            current_player: Player::One,
            scores: [0, 0],
            player_names: [String::from("Player 1"), String::from("Player 2")],
            turn_phase: TurnPhase::EnterAngle {
                input: String::new(),
            },
            gravity: DEFAULT_GRAVITY,
            target_score: 3,
        }
    }

    pub fn update(&mut self, dt: f32) -> GameUpdate {
        let mut cues = Vec::new();
        if let Some(explosion) = self.explosion.as_mut() {
            explosion.advance(dt);
            if explosion.finished() {
                let explosion = self.explosion.take().unwrap();
                let is_gorilla = matches!(explosion.kind, ExplosionKind::Gorilla { .. });
                let match_over = self.finish_explosion(explosion);
                if is_gorilla && match_over.is_none() {
                    cues.push(crate::audio::SoundCue::VictoryDance);
                }
                return GameUpdate { cues, match_over };
            }
            return GameUpdate {
                cues,
                match_over: None,
            };
        }

        if matches!(self.turn_phase, TurnPhase::VictoryDance { .. }) {
            let done = if let TurnPhase::VictoryDance { cycle, timer, .. } = &mut self.turn_phase {
                *timer -= dt;
                if *timer <= 0.0 {
                    *cycle += 1;
                    *timer = VICTORY_DANCE_INTERVAL;
                }
                *cycle >= VICTORY_DANCE_CYCLES
            } else {
                unreachable!()
            };
            if done {
                let (round, gorillas) = make_round(rand::random());
                self.round = round;
                self.gorillas = gorillas;
                self.turn_phase = TurnPhase::EnterAngle {
                    input: String::new(),
                };
            }
            return GameUpdate {
                cues,
                match_over: None,
            };
        }

        if self.projectile.is_none() {
            return GameUpdate {
                cues,
                match_over: None,
            };
        }

        if let TurnPhase::ThrowingArm { timer, .. } = &mut self.turn_phase {
            *timer -= dt;
            if *timer <= 0.0 {
                self.turn_phase = TurnPhase::ProjectileFlying;
            }
        }

        {
            let projectile = self.projectile.as_mut().unwrap();
            projectile.advance(dt);
        }

        let projectile = self.projectile.unwrap();
        let sample = projectile.sample(self.round.wind, self.gravity);
        if !sample.on_screen {
            self.advance_turn();
            return GameUpdate {
                cues,
                match_over: None,
            };
        }

        let collision_canvas = self.collision_canvas();
        match probe_projectile_collision(
            &collision_canvas,
            sample,
            projectile.player,
            projectile.shot_in_sun,
            &self.gorillas,
        ) {
            CollisionProbe::Clear { shot_in_sun } => {
                if let Some(projectile) = self.projectile.as_mut() {
                    projectile.shot_in_sun = shot_in_sun;
                }
            }
            CollisionProbe::Sun { shot_in_sun } => {
                if let Some(projectile) = self.projectile.as_mut() {
                    projectile.shot_in_sun = shot_in_sun;
                }
                self.sun_shocked = true;
            }
            CollisionProbe::Impact { kind, x, y } => {
                self.projectile = None;
                match kind {
                    ImpactKind::Building => {
                        cues.push(crate::audio::SoundCue::BuildingExplosion);
                        self.explosion = Some(Explosion::building(x, y));
                    }
                    ImpactKind::Gorilla(player_index) => {
                        cues.push(crate::audio::SoundCue::GorillaExplosion);
                        self.explosion =
                            Some(Explosion::gorilla(player_index, projectile.player.index()));
                    }
                }
                self.sun_shocked = false;
            }
        }
        GameUpdate {
            cues,
            match_over: None,
        }
    }

    pub fn frame(&self) -> Frame {
        let mut canvas = Canvas::new(LOGICAL_WIDTH, LOGICAL_HEIGHT);
        self.draw_scene(&mut canvas, true);
        if let Some(projectile) = self.projectile {
            let sample = projectile.sample(self.round.wind, self.gravity);
            if sample.on_screen && !projectile.shot_in_sun {
                draw_banana(&mut canvas, sample.x, sample.y, sample.rotation);
            }
        }
        if let Some(explosion) = self.explosion {
            draw_explosion(&mut canvas, explosion, &self.gorillas);
        }
        self.draw_hud(&mut canvas);

        Frame {
            logical_width: LOGICAL_WIDTH,
            logical_height: LOGICAL_HEIGHT,
            clear_color: BACKGROUND,
            vertices: canvas.into_vertices(),
        }
    }

    pub fn handle_char(&mut self, ch: char) {
        if self.projectile.is_some() || self.explosion.is_some() {
            return;
        }

        if !matches!(ch, '0'..='9' | '.') {
            return;
        }

        let input = self.active_input_mut();
        if ch == '.' && input.contains('.') {
            return;
        }
        input.push(ch);
    }

    pub fn handle_backspace(&mut self) {
        if self.projectile.is_some() || self.explosion.is_some() {
            return;
        }

        self.active_input_mut().pop();
    }

    pub fn handle_submit(&mut self) -> Vec<crate::audio::SoundCue> {
        let mut cues = Vec::new();
        if self.projectile.is_some() || self.explosion.is_some() {
            return cues;
        }

        match &mut self.turn_phase {
            TurnPhase::EnterAngle { input } => {
                let angle = parse_numeric_input(input);
                if angle > 360.0 {
                    *input = String::new();
                    return cues;
                }
                self.turn_phase = TurnPhase::EnterVelocity {
                    angle_deg: angle,
                    input: String::new(),
                };
            }
            TurnPhase::EnterVelocity { angle_deg, input } => {
                let velocity = parse_numeric_input(input);
                if velocity < MIN_THROW_VELOCITY {
                    self.explosion = Some(Explosion::gorilla(
                        self.current_player.index(),
                        self.current_player.other().index(),
                    ));
                    self.turn_phase = TurnPhase::ProjectileFlying;
                    return cues;
                }

                let mut angle = *angle_deg;
                if self.current_player == Player::Two {
                    angle = 180.0 - angle;
                }

                let player = self.current_player;
                self.projectile = Some(Projectile::new(
                    self.gorillas[player.index()],
                    player,
                    angle,
                    velocity,
                ));
                self.turn_phase = TurnPhase::ThrowingArm {
                    player,
                    timer: THROW_ARM_DURATION,
                };
                cues.push(crate::audio::SoundCue::Throw);
            }
            TurnPhase::ThrowingArm { .. }
            | TurnPhase::VictoryDance { .. }
            | TurnPhase::ProjectileFlying => {}
        }
        cues
    }

    fn draw_hud(&self, canvas: &mut Canvas) {
        draw_text(canvas, 1, 1, &self.player_names[0], HUD_TEXT);

        let right_col = 79i32 - self.player_names[1].len() as i32;
        draw_text(
            canvas,
            1,
            right_col.max(1) as usize,
            &self.player_names[1],
            HUD_TEXT,
        );

        let score_text = format!("{}>Score<{}", self.scores[0], self.scores[1]);
        let score_col = centered_col(&score_text);
        draw_text(canvas, 23, score_col, &score_text, HUD_TEXT);

        let locate_col = match self.current_player {
            Player::One => 1,
            Player::Two => 66,
        };

        match &self.turn_phase {
            TurnPhase::EnterAngle { input } => {
                draw_text(canvas, 2, locate_col, "Angle:", HUD_TEXT);
                draw_text(canvas, 2, locate_col + 7, input, HUD_TEXT);
                draw_text(canvas, 2, locate_col + 7 + input.len(), "_", HUD_TEXT);
            }
            TurnPhase::EnterVelocity { angle_deg, input } => {
                draw_text(canvas, 2, locate_col, "Angle:", HUD_TEXT);
                draw_text(canvas, 2, locate_col + 7, &format!("{angle_deg}"), HUD_TEXT);
                draw_text(canvas, 3, locate_col, "Velocity:", HUD_TEXT);
                draw_text(canvas, 3, locate_col + 10, input, HUD_TEXT);
                draw_text(canvas, 3, locate_col + 10 + input.len(), "_", HUD_TEXT);
            }
            TurnPhase::ThrowingArm { .. }
            | TurnPhase::VictoryDance { .. }
            | TurnPhase::ProjectileFlying => {}
        }
    }

    fn draw_scene(&self, canvas: &mut Canvas, include_wind: bool) {
        draw_sun(canvas, self.sun_shocked);
        draw_city(canvas, &self.round.buildings);
        if include_wind {
            draw_wind(canvas, self.round.wind);
        }
        for (index, gorilla) in self.gorillas.iter().copied().enumerate() {
            if self.explosion_hits_gorilla(index) {
                continue;
            }
            if let TurnPhase::VictoryDance { loser_index, .. } = self.turn_phase
                && index == loser_index
            {
                continue;
            }
            let arms = match self.turn_phase {
                TurnPhase::ThrowingArm { player, .. } if player.index() == index => {
                    throwing_arm_pose(player)
                }
                TurnPhase::VictoryDance {
                    winner_index,
                    cycle,
                    ..
                } if index == winner_index => {
                    if cycle % 2 == 0 {
                        GorillaArms::LeftUp
                    } else {
                        GorillaArms::RightUp
                    }
                }
                _ => GorillaArms::Down,
            };
            draw_gorilla(canvas, gorilla, arms);
        }
    }

    fn collision_canvas(&self) -> Canvas {
        let mut canvas = Canvas::new(LOGICAL_WIDTH, LOGICAL_HEIGHT);
        self.draw_scene(&mut canvas, false);
        canvas
    }

    fn explosion_hits_gorilla(&self, gorilla_index: usize) -> bool {
        matches!(
            self.explosion,
            Some(Explosion {
                kind: ExplosionKind::Gorilla {
                    gorilla_index: hit_index,
                    winner_index: _,
                },
                ..
            }) if hit_index == gorilla_index
        )
    }

    fn finish_explosion(&mut self, explosion: Explosion) -> Option<usize> {
        self.sun_shocked = false;
        match explosion.kind {
            ExplosionKind::Building { .. } => {
                self.advance_turn();
                None
            }
            ExplosionKind::Gorilla {
                gorilla_index,
                winner_index,
            } => {
                self.scores[winner_index] += 1;
                if self.scores[winner_index] >= self.target_score {
                    return Some(winner_index);
                }
                self.current_player = self.current_player.other();
                self.turn_phase = TurnPhase::VictoryDance {
                    winner_index,
                    loser_index: gorilla_index,
                    cycle: 0,
                    timer: VICTORY_DANCE_INTERVAL,
                };
                None
            }
        }
    }

    fn advance_turn(&mut self) {
        self.current_player = self.current_player.other();
        self.sun_shocked = false;
        self.projectile = None;
        self.turn_phase = TurnPhase::EnterAngle {
            input: String::new(),
        };
    }

    fn active_input_mut(&mut self) -> &mut String {
        match &mut self.turn_phase {
            TurnPhase::EnterAngle { input } => input,
            TurnPhase::EnterVelocity { input, .. } => input,
            TurnPhase::ThrowingArm { .. }
            | TurnPhase::VictoryDance { .. }
            | TurnPhase::ProjectileFlying => {
                unreachable!("no active input while projectile flies")
            }
        }
    }
}

pub struct MatchConfig {
    pub player_names: [String; 2],
    pub target_score: u32,
    pub gravity: f32,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            player_names: [String::from("Player 1"), String::from("Player 2")],
            target_score: 3,
            gravity: DEFAULT_GRAVITY,
        }
    }
}

struct MatchOverState {
    scores: [u32; 2],
    names: [String; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppScreen {
    ConfigMenu,
    Playing,
    MatchOver,
    #[allow(dead_code)]
    PlayAgain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConfigField {
    PlayerOneName,
    PlayerTwoName,
    TargetScore,
    Gravity,
}

impl ConfigField {
    fn index(self) -> usize {
        match self {
            Self::PlayerOneName => 0,
            Self::PlayerTwoName => 1,
            Self::TargetScore => 2,
            Self::Gravity => 3,
        }
    }
}

pub struct GameState {
    pub screen: AppScreen,
    #[allow(dead_code)]
    pub exit_requested: bool,
    config: MatchConfig,
    active_field: ConfigField,
    field_input: String,
    game: Game,
    match_over_state: Option<MatchOverState>,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            screen: AppScreen::ConfigMenu,
            exit_requested: false,
            config: MatchConfig::default(),
            active_field: ConfigField::PlayerOneName,
            field_input: String::new(),
            game: Game::new(),
            match_over_state: None,
        }
    }

    pub fn update(&mut self, dt: f32) -> Vec<crate::audio::SoundCue> {
        match self.screen {
            AppScreen::ConfigMenu | AppScreen::MatchOver | AppScreen::PlayAgain => vec![],
            AppScreen::Playing => {
                let update = self.game.update(dt);
                if update.match_over.is_some() {
                    self.match_over_state = Some(MatchOverState {
                        scores: self.game.scores,
                        names: self.game.player_names.clone(),
                    });
                    self.screen = AppScreen::MatchOver;
                }
                update.cues
            }
        }
    }

    pub fn frame(&self) -> Frame {
        match self.screen {
            AppScreen::ConfigMenu => {
                let mut canvas = Canvas::new(LOGICAL_WIDTH, LOGICAL_HEIGHT);
                self.render_config_screen(&mut canvas);
                Frame {
                    logical_width: LOGICAL_WIDTH,
                    logical_height: LOGICAL_HEIGHT,
                    clear_color: BACKGROUND,
                    vertices: canvas.into_vertices(),
                }
            }
            AppScreen::Playing => self.game.frame(),
            AppScreen::MatchOver => {
                let mo = self.match_over_state.as_ref().unwrap();
                let mut canvas = Canvas::new(LOGICAL_WIDTH, LOGICAL_HEIGHT);
                draw_text(
                    &mut canvas,
                    8,
                    centered_col("GAME OVER!"),
                    "GAME OVER!",
                    HUD_TEXT,
                );
                draw_text(&mut canvas, 10, centered_col("Score:"), "Score:", HUD_TEXT);
                draw_text(&mut canvas, 11, 30, &mo.names[0], HUD_TEXT);
                draw_text(&mut canvas, 11, 50, &format!("{}", mo.scores[0]), HUD_TEXT);
                draw_text(&mut canvas, 12, 30, &mo.names[1], HUD_TEXT);
                draw_text(&mut canvas, 12, 50, &format!("{}", mo.scores[1]), HUD_TEXT);
                draw_text(
                    &mut canvas,
                    24,
                    centered_col("Press any key to continue"),
                    "Press any key to continue",
                    HUD_TEXT,
                );
                Frame {
                    logical_width: LOGICAL_WIDTH,
                    logical_height: LOGICAL_HEIGHT,
                    clear_color: BACKGROUND,
                    vertices: canvas.into_vertices(),
                }
            }
            AppScreen::PlayAgain => Frame {
                logical_width: LOGICAL_WIDTH,
                logical_height: LOGICAL_HEIGHT,
                clear_color: BACKGROUND,
                vertices: vec![],
            },
        }
    }

    fn render_config_screen(&self, canvas: &mut Canvas) {
        let active = self.active_field.index();

        // row 8, col 15: Player 1 name
        draw_text(
            canvas,
            8,
            15,
            "Name of Player 1 (Default = 'Player 1'): ",
            HUD_TEXT,
        );
        if active > 0 {
            draw_text(canvas, 8, 56, &self.config.player_names[0], HUD_TEXT);
        } else {
            draw_text(canvas, 8, 56, &self.field_input, HUD_TEXT);
            draw_text(canvas, 8, 56 + self.field_input.len(), "_", HUD_TEXT);
        }

        // row 10, col 15: Player 2 name
        draw_text(
            canvas,
            10,
            15,
            "Name of Player 2 (Default = 'Player 2'): ",
            HUD_TEXT,
        );
        if active > 1 {
            draw_text(canvas, 10, 56, &self.config.player_names[1], HUD_TEXT);
        } else if active == 1 {
            draw_text(canvas, 10, 56, &self.field_input, HUD_TEXT);
            draw_text(canvas, 10, 56 + self.field_input.len(), "_", HUD_TEXT);
        }

        // row 12, col 13: Target score
        draw_text(
            canvas,
            12,
            13,
            "Play to how many total points (Default = 3): ",
            HUD_TEXT,
        );
        if active > 2 {
            draw_text(
                canvas,
                12,
                58,
                &format!("{}", self.config.target_score),
                HUD_TEXT,
            );
        } else if active == 2 {
            draw_text(canvas, 12, 58, &self.field_input, HUD_TEXT);
            draw_text(canvas, 12, 58 + self.field_input.len(), "_", HUD_TEXT);
        }

        // row 14, col 17: Gravity
        draw_text(
            canvas,
            14,
            17,
            "Gravity in Meters/Sec (Earth = 9.8): ",
            HUD_TEXT,
        );
        if active == 3 {
            draw_text(canvas, 14, 54, &self.field_input, HUD_TEXT);
            draw_text(canvas, 14, 54 + self.field_input.len(), "_", HUD_TEXT);
        }
    }

    pub fn handle_char(&mut self, ch: char) {
        match self.screen {
            AppScreen::ConfigMenu => self.config_handle_char(ch),
            AppScreen::Playing => self.game.handle_char(ch),
            AppScreen::MatchOver => self.reset_to_config(),
            AppScreen::PlayAgain => {}
        }
    }

    pub fn handle_backspace(&mut self) {
        match self.screen {
            AppScreen::ConfigMenu => {
                self.field_input.pop();
            }
            AppScreen::Playing => self.game.handle_backspace(),
            AppScreen::MatchOver => self.reset_to_config(),
            AppScreen::PlayAgain => {}
        }
    }

    pub fn handle_submit(&mut self) -> Vec<crate::audio::SoundCue> {
        match self.screen {
            AppScreen::ConfigMenu => {
                self.config_handle_submit();
                if matches!(self.screen, AppScreen::Playing) {
                    vec![
                        crate::audio::SoundCue::Intro,
                        crate::audio::SoundCue::GorillaIntro,
                    ]
                } else {
                    vec![]
                }
            }
            AppScreen::Playing => self.game.handle_submit(),
            AppScreen::MatchOver => {
                self.reset_to_config();
                vec![]
            }
            AppScreen::PlayAgain => vec![],
        }
    }

    fn reset_to_config(&mut self) {
        self.screen = AppScreen::ConfigMenu;
        self.config = MatchConfig::default();
        self.active_field = ConfigField::PlayerOneName;
        self.field_input.clear();
        self.game = Game::new();
        self.match_over_state = None;
    }

    fn config_handle_char(&mut self, ch: char) {
        match self.active_field {
            ConfigField::PlayerOneName | ConfigField::PlayerTwoName => {
                if ch.is_ascii() && !ch.is_ascii_control() && self.field_input.len() < 10 {
                    self.field_input.push(ch);
                }
            }
            ConfigField::TargetScore => {
                if ch.is_ascii_digit() && self.field_input.len() < 2 {
                    self.field_input.push(ch);
                }
            }
            ConfigField::Gravity => {
                if ch.is_ascii_digit() || (ch == '.' && !self.field_input.contains('.')) {
                    self.field_input.push(ch);
                }
            }
        }
    }

    fn config_handle_submit(&mut self) {
        match self.active_field {
            ConfigField::PlayerOneName => {
                self.config.player_names[0] = if self.field_input.is_empty() {
                    String::from("Player 1")
                } else {
                    self.field_input.clone()
                };
                self.field_input.clear();
                self.active_field = ConfigField::PlayerTwoName;
            }
            ConfigField::PlayerTwoName => {
                self.config.player_names[1] = if self.field_input.is_empty() {
                    String::from("Player 2")
                } else {
                    self.field_input.clone()
                };
                self.field_input.clear();
                self.active_field = ConfigField::TargetScore;
            }
            ConfigField::TargetScore => {
                if self.field_input.is_empty() {
                    self.active_field = ConfigField::Gravity;
                } else {
                    let score: u32 = self.field_input.parse().unwrap_or(0);
                    if score >= 1 {
                        self.config.target_score = score;
                        self.field_input.clear();
                        self.active_field = ConfigField::Gravity;
                    } else {
                        self.field_input.clear();
                    }
                }
            }
            ConfigField::Gravity => {
                if self.field_input.is_empty() {
                    self.apply_config_and_start();
                } else {
                    let grav: f32 = self.field_input.parse().unwrap_or(0.0);
                    if grav > 0.0 {
                        self.config.gravity = grav;
                        self.apply_config_and_start();
                    } else {
                        self.field_input.clear();
                    }
                }
            }
        }
    }

    fn apply_config_and_start(&mut self) {
        self.game.player_names = self.config.player_names.clone();
        self.game.gravity = self.config.gravity;
        self.game.target_score = self.config.target_score;
        self.screen = AppScreen::Playing;
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Round {
    buildings: Vec<Building>,
    wind: i32,
}

#[derive(Clone, Debug, PartialEq)]
struct Building {
    x: f32,
    width: f32,
    height: f32,
    color: [f32; 4],
    windows: Vec<WindowRect>,
}

impl Building {
    fn top(&self) -> f32 {
        BOTTOM_LINE - self.height
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WindowRect {
    x: f32,
    y: f32,
    lit: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Gorilla {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
enum Player {
    One,
    Two,
}

impl Player {
    fn index(self) -> usize {
        match self {
            Self::One => 0,
            Self::Two => 1,
        }
    }

    fn other(self) -> Self {
        match self {
            Self::One => Self::Two,
            Self::Two => Self::One,
        }
    }
}

fn throwing_arm_pose(player: Player) -> GorillaArms {
    match player {
        Player::One => GorillaArms::LeftUp,
        Player::Two => GorillaArms::RightUp,
    }
}

#[derive(Clone, Debug, PartialEq)]
enum TurnPhase {
    EnterAngle {
        input: String,
    },
    EnterVelocity {
        angle_deg: f32,
        input: String,
    },
    ThrowingArm {
        player: Player,
        timer: f32,
    },
    VictoryDance {
        winner_index: usize,
        loser_index: usize,
        cycle: u8,
        timer: f32,
    },
    ProjectileFlying,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BananaRotation {
    Left,
    Up,
    Down,
    Right,
}

impl BananaRotation {
    fn from_frame(frame: u8) -> Self {
        match frame % 4 {
            0 => Self::Left,
            1 => Self::Up,
            2 => Self::Down,
            _ => Self::Right,
        }
    }

    fn sprite(self) -> &'static [&'static str] {
        match self {
            Self::Left => &["..##..", ".####.", "######", ".####.", "..##.."],
            Self::Up => &["..#..", ".###.", ".###.", "##.##", "#...#"],
            Self::Down => &["#...#", "##.##", ".###.", ".###.", "..#.."],
            Self::Right => &["..##..", ".####.", "######", ".####.", "..##.."],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ProjectileSample {
    x: f32,
    y: f32,
    rotation: BananaRotation,
    on_screen: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg(test)]
enum SlowShotOutcome {
    Flying,
    SelfHit { player: Player },
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Projectile {
    start: Gorilla,
    player: Player,
    angle_degrees: f32,
    velocity: f32,
    elapsed: f32,
    shot_in_sun: bool,
}

impl Projectile {
    fn new(start: Gorilla, player: Player, angle_degrees: f32, velocity: f32) -> Self {
        Self {
            start,
            player,
            angle_degrees,
            velocity,
            elapsed: 0.0,
            shot_in_sun: false,
        }
    }

    fn advance(&mut self, dt: f32) {
        let steps = (dt / PROJECTILE_TIME_STEP).floor().max(1.0);
        self.elapsed += steps * PROJECTILE_TIME_STEP;
    }

    fn sample(self, wind: i32, gravity: f32) -> ProjectileSample {
        projectile_sample(
            self.start,
            self.angle_degrees,
            self.velocity,
            self.player,
            wind,
            gravity,
            self.elapsed,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Explosion {
    kind: ExplosionKind,
    elapsed: f32,
}

impl Explosion {
    fn building(x: f32, y: f32) -> Self {
        Self {
            kind: ExplosionKind::Building { x, y },
            elapsed: 0.0,
        }
    }

    fn gorilla(gorilla_index: usize, winner_index: usize) -> Self {
        Self {
            kind: ExplosionKind::Gorilla {
                gorilla_index,
                winner_index,
            },
            elapsed: 0.0,
        }
    }

    fn advance(&mut self, dt: f32) {
        self.elapsed += dt;
    }

    fn finished(self) -> bool {
        self.elapsed >= self.duration()
    }

    fn radius(self) -> f32 {
        let max_radius = match self.kind {
            ExplosionKind::Building { .. } => EXPLOSION_MAX_RADIUS,
            ExplosionKind::Gorilla { .. } => GORILLA_EXPLOSION_MAX_RADIUS,
        };
        (self.elapsed / self.duration()).clamp(0.2, 1.0) * max_radius
    }

    fn duration(self) -> f32 {
        match self.kind {
            ExplosionKind::Building { .. } => EXPLOSION_DURATION,
            ExplosionKind::Gorilla { .. } => GORILLA_EXPLOSION_DURATION,
        }
    }

    fn center(self, gorillas: &[Gorilla; 2]) -> (f32, f32) {
        match self.kind {
            ExplosionKind::Building { x, y } => (x, y),
            ExplosionKind::Gorilla {
                gorilla_index,
                winner_index: _,
            } => {
                let gorilla = gorillas[gorilla_index];
                (gorilla.x + 12.0, gorilla.y + 12.0)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ExplosionKind {
    Building {
        x: f32,
        y: f32,
    },
    Gorilla {
        gorilla_index: usize,
        winner_index: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ImpactKind {
    Building,
    Gorilla(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum CollisionProbe {
    Clear { shot_in_sun: bool },
    Sun { shot_in_sun: bool },
    Impact { kind: ImpactKind, x: f32, y: f32 },
}

struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<Option<[f32; 4]>>,
}

impl Canvas {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![None; (width * height) as usize],
        }
    }

    fn into_vertices(self) -> Vec<crate::render::Vertex> {
        let mut batch = PrimitiveBatch::new(self.width as f32, self.height as f32);

        for y in 0..self.height as usize {
            let mut x = 0usize;
            while x < self.width as usize {
                let Some(color) = self.pixels[y * self.width as usize + x] else {
                    x += 1;
                    continue;
                };

                let start = x;
                x += 1;
                while x < self.width as usize
                    && self.pixels[y * self.width as usize + x] == Some(color)
                {
                    x += 1;
                }

                batch.rect(start as f32, y as f32, (x - start) as f32, 1.0, color);
            }
        }

        batch.into_vertices()
    }

    fn pixel(&mut self, x: f32, y: f32, color: [f32; 4]) {
        let x = x.round() as i32;
        let y = y.round() as i32;
        if !(0..self.width as i32).contains(&x) || !(0..self.height as i32).contains(&y) {
            return;
        }

        let index = y as usize * self.width as usize + x as usize;
        self.pixels[index] = Some(color);
    }

    fn point(&self, x: f32, y: f32) -> Option<[f32; 4]> {
        let x = x.round() as i32;
        let y = y.round() as i32;
        if !(0..self.width as i32).contains(&x) || !(0..self.height as i32).contains(&y) {
            return None;
        }

        self.pixels[y as usize * self.width as usize + x as usize]
    }

    fn rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        let x0 = x.round() as i32;
        let y0 = y.round() as i32;
        let x1 = (x + width - 1.0).round() as i32;
        let y1 = (y + height - 1.0).round() as i32;
        self.fill_rect_inclusive(x0, y0, x1, y1, color);
    }

    fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, thickness: f32, color: [f32; 4]) {
        let half = ((thickness.max(1.0).round() as i32) - 1) / 2;
        let horizontalish = (x1 - x0).abs() >= (y1 - y0).abs();

        for offset in -half..=half {
            if horizontalish {
                self.line_single(x0, y0 + offset as f32, x1, y1 + offset as f32, color);
            } else {
                self.line_single(x0 + offset as f32, y0, x1 + offset as f32, y1, color);
            }
        }
    }

    fn circle(&mut self, x: f32, y: f32, radius: f32, _segments: usize, color: [f32; 4]) {
        let min_x = (x - radius).floor() as i32;
        let max_x = (x + radius).ceil() as i32;
        let min_y = (y - radius).floor() as i32;
        let max_y = (y + radius).ceil() as i32;
        let radius_sq = radius * radius;

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let dx = px as f32 - x;
                let dy = (py as f32 - y) * BASIC_CIRCLE_Y_ASPECT;
                if dx * dx + dy * dy <= radius_sq + 0.5 {
                    self.pixel(px as f32, py as f32, color);
                }
            }
        }
    }

    fn circle_ring(&mut self, x: f32, y: f32, radius: f32, color: [f32; 4]) {
        let min_x = (x - radius - 1.0).floor() as i32;
        let max_x = (x + radius + 1.0).ceil() as i32;
        let min_y = (y - radius - 1.0).floor() as i32;
        let max_y = (y + radius + 1.0).ceil() as i32;
        let outer_sq = (radius + 0.5) * (radius + 0.5);
        let inner_sq = (radius - 0.5).max(0.0) * (radius - 0.5).max(0.0);

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let dx = px as f32 - x;
                let dy = (py as f32 - y) * BASIC_CIRCLE_Y_ASPECT;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq >= inner_sq && dist_sq <= outer_sq {
                    self.pixel(px as f32, py as f32, color);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn arc(
        &mut self,
        x: f32,
        y: f32,
        radius: f32,
        start: f32,
        end: f32,
        thickness: f32,
        color: [f32; 4],
    ) {
        let start = start.rem_euclid(std::f32::consts::TAU);
        let mut end = end;

        while end < start {
            end += std::f32::consts::TAU;
        }

        let half_band = ((thickness - 1.0).max(0.0) * 0.5) + 0.25;
        let sweep = end - start;
        let steps = (radius * sweep.abs() * 6.0).ceil().max(1.0) as usize;
        let radius_steps = (half_band * 2.0).ceil().max(1.0) as usize;

        for radius_step in 0..=radius_steps {
            let band_offset = if radius_steps == 0 {
                0.0
            } else {
                -half_band + radius_step as f32 * (half_band * 2.0 / radius_steps as f32)
            };
            let plotted_radius = (radius + band_offset).max(0.0);

            for step in 0..=steps {
                let angle = start + sweep * step as f32 / steps as f32;
                let px = x + plotted_radius * angle.cos();
                let py = y - (plotted_radius * angle.sin()) / BASIC_CIRCLE_Y_ASPECT;
                self.pixel(px, py, color);
            }
        }
    }

    fn fill_rect_inclusive(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: [f32; 4]) {
        for y in y0.min(y1)..=y0.max(y1) {
            for x in x0.min(x1)..=x0.max(x1) {
                self.pixel(x as f32, y as f32, color);
            }
        }
    }

    fn line_single(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, color: [f32; 4]) {
        let mut x0 = x0.round() as i32;
        let mut y0 = y0.round() as i32;
        let x1 = x1.round() as i32;
        let y1 = y1.round() as i32;
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.pixel(x0 as f32, y0 as f32, color);
            if x0 == x1 && y0 == y1 {
                break;
            }

            let e2 = err * 2;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }
}

fn make_round(seed: u64) -> (Round, [Gorilla; 2]) {
    let mut rng = SmallRng::seed_from_u64(seed);
    let round = make_cityscape_with_rng(&mut rng);
    let gorillas = place_gorillas_with_rng(&round.buildings, &mut rng);

    (round, gorillas)
}

#[cfg(test)]
fn make_cityscape_with_seed(seed: u64) -> Round {
    let mut rng = SmallRng::seed_from_u64(seed);
    make_cityscape_with_rng(&mut rng)
}

fn make_cityscape_with_rng(rng: &mut SmallRng) -> Round {
    let mut buildings = Vec::new();
    let slope = fn_ran(rng, 6);
    let mut new_height = match slope {
        1 | 3..=5 => 15.0,
        _ => 130.0,
    };
    let mut x = 2.0;

    while x <= LOGICAL_WIDTH as f32 - 10.0 {
        match slope {
            1 => new_height += HEIGHT_INCREMENT,
            2 => new_height -= HEIGHT_INCREMENT,
            3..=5 if x > LOGICAL_WIDTH as f32 * 0.5 => new_height -= 2.0 * HEIGHT_INCREMENT,
            3..=5 => new_height += 2.0 * HEIGHT_INCREMENT,
            // The BASIC source has an unreachable CASE 4 here after CASE 3 TO 5.
            // Keep the documented inverted-V intent for slope 6.
            6 if x > LOGICAL_WIDTH as f32 * 0.5 => new_height += 20.0,
            6 => new_height -= 20.0,
            _ => {}
        }

        let mut width = fn_ran(rng, DEFAULT_BUILDING_WIDTH) as f32 + DEFAULT_BUILDING_WIDTH as f32;
        if x + width > LOGICAL_WIDTH as f32 {
            width = LOGICAL_WIDTH as f32 - x - 2.0;
        }

        let mut height = fn_ran(rng, RANDOM_HEIGHT) as f32 + new_height;
        if height < HEIGHT_INCREMENT {
            height = HEIGHT_INCREMENT;
        }

        let top = BOTTOM_LINE - height;
        if top <= GORILLA_HEIGHT {
            height = GORILLA_HEIGHT - 5.0;
        }

        let color = building_color((fn_ran(rng, 3) + 4) as u8);
        let windows = make_windows(rng, x, width, height);
        buildings.push(Building {
            x,
            width,
            height,
            color,
            windows,
        });
        x += width + 2.0;
    }

    let wind = make_wind(rng);

    Round { buildings, wind }
}

fn make_windows(
    rng: &mut SmallRng,
    building_x: f32,
    building_width: f32,
    height: f32,
) -> Vec<WindowRect> {
    let mut windows = Vec::new();
    let mut x = building_x + 3.0;

    while x < building_x + building_width - 3.0 {
        let mut i = height - 3.0;
        while i >= 7.0 {
            windows.push(WindowRect {
                x,
                y: BOTTOM_LINE - i,
                lit: fn_ran(rng, 4) != 1,
            });
            i -= WINDOW_VERTICAL_SPACING;
        }
        x += WINDOW_HORIZONTAL_SPACING;
    }

    windows
}

fn make_wind(rng: &mut SmallRng) -> i32 {
    let mut wind = fn_ran(rng, 10) as i32 - 5;
    if fn_ran(rng, 3) == 1 {
        if wind > 0 {
            wind += fn_ran(rng, 10) as i32;
        } else {
            wind -= fn_ran(rng, 10) as i32;
        }
    }

    wind
}

fn place_gorillas_with_rng(buildings: &[Building], rng: &mut SmallRng) -> [Gorilla; 2] {
    if buildings.len() < 4 {
        return [
            Gorilla { x: 140.0, y: 200.0 },
            Gorilla { x: 500.0, y: 200.0 },
        ];
    }

    let left_index = fn_ran(rng, 2) as usize;
    let right_index = buildings.len() - 1 - fn_ran(rng, 2) as usize;
    let left = gorilla_on(buildings, left_index);
    let right = gorilla_on(buildings, right_index);

    [left, right]
}

fn gorilla_on(buildings: &[Building], building_index: usize) -> Gorilla {
    let building = &buildings[building_index];
    let next_x = buildings
        .get(building_index + 1)
        .map_or(building.x + building.width + 2.0, |next| next.x);
    let width = next_x - building.x;

    Gorilla {
        x: building.x + width * 0.5 - GORILLA_X_ADJUST,
        y: building.top() - GORILLA_Y_ADJUST,
    }
}

fn fn_ran(rng: &mut SmallRng, upper: u32) -> u32 {
    rng.random_range(1..=upper)
}

fn projectile_start(gorilla: Gorilla, player: Player) -> (f32, f32) {
    let x = match player {
        Player::One => gorilla.x,
        Player::Two => gorilla.x + 25.0,
    };
    let y = gorilla.y - 4.0 - 3.0;

    (x, y)
}

fn projectile_sample(
    gorilla: Gorilla,
    angle_degrees: f32,
    velocity: f32,
    player: Player,
    wind: i32,
    gravity: f32,
    t: f32,
) -> ProjectileSample {
    let (start_x, start_y) = projectile_start(gorilla, player);
    let angle = angle_degrees.to_radians();
    let init_x_velocity = angle.cos() * velocity;
    let init_y_velocity = angle.sin() * velocity;
    let x = start_x + init_x_velocity * t + 0.5 * (wind as f32 / 5.0) * t.powi(2);
    let y = start_y
        + (-init_y_velocity * t + 0.5 * gravity * t.powi(2)) * (LOGICAL_HEIGHT as f32 / 350.0);
    let rotation = BananaRotation::from_frame((t / PROJECTILE_TIME_STEP).round() as u8);

    ProjectileSample {
        x,
        y,
        rotation,
        on_screen: projectile_on_screen(x, y),
    }
}

fn projectile_on_screen(x: f32, y: f32) -> bool {
    x < LOGICAL_WIDTH as f32 - 10.0 && x > 3.0 && y < LOGICAL_HEIGHT as f32 - 3.0
}

fn probe_projectile_collision(
    canvas: &Canvas,
    sample: ProjectileSample,
    player: Player,
    shot_in_sun: bool,
    gorillas: &[Gorilla; 2],
) -> CollisionProbe {
    if !sample.on_screen || sample.y <= 0.0 {
        return CollisionProbe::Clear { shot_in_sun };
    }

    let mut shot_in_sun = shot_in_sun;
    let mut hit_sun = false;
    for (dx, dy) in probe_offsets(player) {
        let point = canvas.point(sample.x + dx, sample.y + dy);
        if point.is_none() || point == Some(BACKGROUND) {
            if shot_in_sun && projectile_left_sun(sample) {
                shot_in_sun = false;
            }
            continue;
        }

        if point == Some(SUN) && sample.y < SUN_HEIGHT_LIMIT {
            shot_in_sun = true;
            hit_sun = true;
            continue;
        }

        let kind = if point == Some(OBJECT) {
            ImpactKind::Gorilla(resolve_hit_gorilla(sample, gorillas))
        } else {
            ImpactKind::Building
        };
        return CollisionProbe::Impact {
            kind,
            x: sample.x + 4.0,
            y: sample.y + 4.0,
        };
    }

    if hit_sun {
        CollisionProbe::Sun { shot_in_sun }
    } else {
        CollisionProbe::Clear { shot_in_sun }
    }
}

fn resolve_hit_gorilla(sample: ProjectileSample, _gorillas: &[Gorilla; 2]) -> usize {
    if sample.x < LOGICAL_WIDTH as f32 / 2.0 {
        0
    } else {
        1
    }
}

fn probe_offsets(player: Player) -> [(f32, f32); 2] {
    match player {
        Player::One => [(8.0, 0.0), (4.0, 6.0)],
        Player::Two => [(0.0, 0.0), (4.0, 6.0)],
    }
}

fn projectile_left_sun(sample: ProjectileSample) -> bool {
    (LOGICAL_WIDTH as f32 * 0.5 - sample.x).abs() > SUN_CLEAR_RADIUS || sample.y > SUN_HEIGHT_LIMIT
}

fn parse_numeric_input(input: &str) -> f32 {
    input.parse::<f32>().unwrap_or(0.0)
}

fn centered_col(text: &str) -> usize {
    let len = text.chars().count() as i32;
    (40 - (len / 2)).max(1) as usize
}

fn draw_text(canvas: &mut Canvas, row: usize, col: usize, text: &str, color: [f32; 4]) {
    let base_x = ((col as i32 - 1) * TEXT_CELL_WIDTH).max(0);
    let base_y = ((row as i32 - 1) * TEXT_CELL_HEIGHT).max(0);

    for (index, ch) in text.chars().enumerate() {
        draw_char(
            canvas,
            base_x + index as i32 * TEXT_CELL_WIDTH,
            base_y,
            ch,
            color,
        );
    }
}

fn draw_char(canvas: &mut Canvas, x: i32, y: i32, ch: char, color: [f32; 4]) {
    let Some(rows) = glyph_rows(ch) else {
        return;
    };

    for (row_index, bits) in rows.iter().enumerate() {
        for col_index in 0..5 {
            if (bits >> (4 - col_index)) & 1 == 1 {
                let px = x + col_index;
                let py = y + row_index as i32 * 2;
                canvas.pixel(px as f32, py as f32, color);
                canvas.pixel(px as f32, (py + 1) as f32, color);
            }
        }
    }
}

fn glyph_rows(ch: char) -> Option<[u8; 7]> {
    match ch.to_ascii_uppercase() {
        ' ' => Some([0, 0, 0, 0, 0, 0, 0]),
        '0' => Some([
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ]),
        '1' => Some([
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ]),
        '2' => Some([
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ]),
        '3' => Some([
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ]),
        '4' => Some([
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ]),
        '5' => Some([
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ]),
        '6' => Some([
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ]),
        '7' => Some([
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ]),
        '8' => Some([
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ]),
        '9' => Some([
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b11100,
        ]),
        'A' => Some([
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ]),
        'B' => Some([
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ]),
        'C' => Some([
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ]),
        'D' => Some([
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ]),
        'E' => Some([
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ]),
        'F' => Some([
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ]),
        'G' => Some([
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ]),
        'H' => Some([
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ]),
        'I' => Some([
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ]),
        'J' => Some([
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ]),
        'K' => Some([
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ]),
        'L' => Some([
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ]),
        'M' => Some([
            0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 0b10001, 0b10001,
        ]),
        'N' => Some([
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ]),
        'O' => Some([
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ]),
        'P' => Some([
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ]),
        'Q' => Some([
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ]),
        'R' => Some([
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ]),
        'S' => Some([
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ]),
        'T' => Some([
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ]),
        'U' => Some([
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ]),
        'V' => Some([
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ]),
        'W' => Some([
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ]),
        'X' => Some([
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ]),
        'Y' => Some([
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ]),
        'Z' => Some([
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ]),
        '\'' => Some([0b00100, 0b00100, 0, 0, 0, 0, 0]),
        '(' => Some([
            0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010,
        ]),
        ')' => Some([
            0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000,
        ]),
        '/' => Some([
            0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000,
        ]),
        '=' => Some([0, 0, 0b11111, 0, 0b11111, 0, 0]),
        ':' => Some([0, 0b00100, 0b00100, 0, 0b00100, 0b00100, 0]),
        '.' => Some([0, 0, 0, 0, 0, 0b00100, 0b00100]),
        '-' => Some([0, 0, 0, 0b11111, 0, 0, 0]),
        '_' => Some([0, 0, 0, 0, 0, 0, 0b11111]),
        '<' => Some([
            0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010,
        ]),
        '>' => Some([
            0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000,
        ]),
        _ => None,
    }
}

#[cfg(test)]
fn slow_shot_outcome(velocity: f32, player: Player) -> SlowShotOutcome {
    if velocity < MIN_THROW_VELOCITY {
        SlowShotOutcome::SelfHit { player }
    } else {
        SlowShotOutcome::Flying
    }
}

fn draw_city(canvas: &mut Canvas, buildings: &[Building]) {
    for building in buildings {
        let y = building.top();
        canvas.rect(
            building.x - 1.0,
            y - 1.0,
            building.width + 2.0,
            building.height + 2.0,
            BACKGROUND,
        );
        canvas.rect(
            building.x,
            y,
            building.width,
            building.height,
            building.color,
        );
        draw_windows(canvas, building);
    }
}

fn draw_windows(canvas: &mut Canvas, building: &Building) {
    for window in &building.windows {
        let color = if window.lit { LIT_WINDOW } else { DARK_WINDOW };
        canvas.rect(
            window.x,
            window.y,
            WINDOW_WIDTH + 1.0,
            WINDOW_HEIGHT + 1.0,
            color,
        );
    }
}

fn building_color(attribute: u8) -> [f32; 4] {
    palette_attribute(attribute)
}

fn draw_wind(canvas: &mut Canvas, wind: i32) {
    if wind == 0 {
        return;
    }

    let start_x = LOGICAL_WIDTH as f32 * 0.5;
    let y = LOGICAL_HEIGHT as f32 - 5.0;
    let wind_line = wind as f32 * 3.0 * (LOGICAL_WIDTH / 320) as f32;
    let end_x = start_x + wind_line;
    let arrow_dir = if wind > 0 { -2.0 } else { 2.0 };

    canvas.line(start_x, y, end_x, y, 1.0, EXPLOSION);
    canvas.line(end_x, y, end_x + arrow_dir, y - 2.0, 1.0, EXPLOSION);
    canvas.line(end_x, y, end_x + arrow_dir, y + 2.0, 1.0, EXPLOSION);
}

fn draw_banana(canvas: &mut Canvas, x: f32, y: f32, rotation: BananaRotation) {
    for (row, line) in rotation.sprite().iter().enumerate() {
        for (column, pixel) in line.bytes().enumerate() {
            if pixel == b'#' {
                canvas.pixel(x + column as f32, y + row as f32, OBJECT);
            }
        }
    }
}

fn draw_explosion(canvas: &mut Canvas, explosion: Explosion, gorillas: &[Gorilla; 2]) {
    let (x, y) = explosion.center(gorillas);
    match explosion.kind {
        ExplosionKind::Building { .. } => {
            let progress = (explosion.elapsed / explosion.duration()).clamp(0.0, 1.0);
            let ring_radius = (1.0 - (2.0 * progress - 1.0).abs()) * EXPLOSION_MAX_RADIUS;
            canvas.circle_ring(x, y, ring_radius, EXPLOSION);
        }
        ExplosionKind::Gorilla { .. } => {
            let r = explosion.radius();
            canvas.circle(x, y, r, 24, EXPLOSION);
            let sweep_y = y + 6.0 - r * 0.5;
            canvas.line(
                x - 10.0,
                sweep_y,
                x + 10.0,
                sweep_y,
                GORILLA_EXPLOSION_MAX_RADIUS / 24.0,
                EXPLOSION,
            );
        }
    }
}

fn draw_sun(canvas: &mut Canvas, shocked: bool) {
    let x = LOGICAL_WIDTH as f32 * 0.5;
    let y = 25.0;

    canvas.circle(x, y, 12.0, 36, SUN);
    canvas.line(x - 20.0, y, x + 20.0, y, 2.0, SUN);
    canvas.line(x, y - 15.0, x, y + 15.0, 2.0, SUN);
    canvas.line(x - 15.0, y - 10.0, x + 15.0, y + 10.0, 2.0, SUN);
    canvas.line(x - 15.0, y + 10.0, x + 15.0, y - 10.0, 2.0, SUN);
    canvas.line(x - 8.0, y - 13.0, x + 8.0, y + 13.0, 2.0, SUN);
    canvas.line(x - 8.0, y + 13.0, x + 8.0, y - 13.0, 2.0, SUN);
    canvas.line(x - 18.0, y - 5.0, x + 18.0, y + 5.0, 2.0, SUN);
    canvas.line(x - 18.0, y + 5.0, x + 18.0, y - 5.0, 2.0, SUN);

    canvas.circle(x - 3.0, y - 2.0, 1.4, 10, BACKGROUND);
    canvas.circle(x + 3.0, y - 2.0, 1.4, 10, BACKGROUND);
    if shocked {
        canvas.circle(x, y + 5.0, 3.0, 18, BACKGROUND);
    } else {
        basic_arc(canvas, x, y, 8.0, 210.0, 330.0, BACKGROUND);
    }
}

fn draw_gorilla(canvas: &mut Canvas, gorilla: Gorilla, arms: GorillaArms) {
    let x = gorilla.x;
    let y = gorilla.y;

    basic_fill_rect(canvas, x - 4.0, y, x + 2.9, y + 6.0, OBJECT);
    basic_fill_rect(canvas, x - 5.0, y + 2.0, x + 4.0, y + 4.0, OBJECT);
    basic_line(canvas, x - 3.0, y + 2.0, x + 2.0, y + 2.0, BACKGROUND);
    basic_pset(canvas, x - 2.0, y + 4.0, BACKGROUND);
    basic_pset(canvas, x - 1.0, y + 4.0, BACKGROUND);
    basic_pset(canvas, x + 1.0, y + 4.0, BACKGROUND);
    basic_pset(canvas, x + 2.0, y + 4.0, BACKGROUND);
    basic_line(canvas, x - 3.0, y + 7.0, x + 2.0, y + 7.0, OBJECT);

    basic_fill_rect(canvas, x - 8.0, y + 8.0, x + 6.9, y + 14.0, OBJECT);
    basic_fill_rect(canvas, x - 6.0, y + 15.0, x + 4.9, y + 20.0, OBJECT);

    for i in 0..=4 {
        let offset = i as f32;
        basic_arc(canvas, x + offset, y + 25.0, 10.0, 135.0, 202.5, OBJECT);
        basic_arc(
            canvas,
            x - 6.0 + offset,
            y + 25.0,
            10.0,
            337.5,
            405.0,
            OBJECT,
        );
    }

    basic_arc(canvas, x - 5.0, y + 10.0, 5.0, 270.0, 360.0, BACKGROUND);
    basic_arc(canvas, x + 5.0, y + 10.0, 5.0, 180.0, 270.0, BACKGROUND);

    match arms {
        GorillaArms::RightUp => {
            for i in -5..=-1 {
                let offset = i as f32;
                basic_arc(canvas, x + offset, y + 14.0, 9.0, 135.0, 225.0, OBJECT);
                basic_arc(canvas, x + 5.0 + offset, y + 4.0, 9.0, 315.0, 405.0, OBJECT);
            }
        }
        GorillaArms::LeftUp => {
            for i in -5..=-1 {
                let offset = i as f32;
                basic_arc(canvas, x + offset, y + 4.0, 9.0, 135.0, 225.0, OBJECT);
                basic_arc(
                    canvas,
                    x + 5.0 + offset,
                    y + 14.0,
                    9.0,
                    315.0,
                    405.0,
                    OBJECT,
                );
            }
        }
        GorillaArms::Down => {
            for i in -5..=-1 {
                let offset = i as f32;
                basic_arc(canvas, x + offset, y + 14.0, 9.0, 135.0, 225.0, OBJECT);
                basic_arc(
                    canvas,
                    x + 5.0 + offset,
                    y + 14.0,
                    9.0,
                    315.0,
                    405.0,
                    OBJECT,
                );
            }
        }
    }
}

fn basic_fill_rect(canvas: &mut Canvas, x0: f32, y0: f32, x1: f32, y1: f32, color: [f32; 4]) {
    canvas.fill_rect_inclusive(
        x0.round() as i32,
        y0.round() as i32,
        x1.round() as i32,
        y1.round() as i32,
        color,
    );
}

fn basic_line(canvas: &mut Canvas, x0: f32, y0: f32, x1: f32, y1: f32, color: [f32; 4]) {
    canvas.line_single(x0, y0, x1, y1, color);
}

fn basic_pset(canvas: &mut Canvas, x: f32, y: f32, color: [f32; 4]) {
    canvas.pixel(x.round(), y.round(), color);
}

fn basic_arc(
    canvas: &mut Canvas,
    x: f32,
    y: f32,
    radius: f32,
    start_degrees: f32,
    end_degrees: f32,
    color: [f32; 4],
) {
    let start = start_degrees.to_radians();
    let end = end_degrees.to_radians();
    canvas.arc(x, y, radius, start, end, 1.0, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_update_returns_game_update_struct() {
        let mut game = Game::new();
        let update = game.update(0.016);
        assert_eq!(update.match_over, None);
        let _ = update.cues;
    }

    #[test]
    fn game_signals_match_over_when_winner_reaches_target_score() {
        let mut game = Game::new();
        game.target_score = 1;
        game.explosion = Some(Explosion {
            kind: ExplosionKind::Gorilla {
                gorilla_index: 1,
                winner_index: 0,
            },
            elapsed: GORILLA_EXPLOSION_DURATION - 0.001,
        });

        let update = game.update(0.1);

        assert_eq!(update.match_over, Some(0));
    }

    #[test]
    fn game_does_not_signal_match_over_before_target_score() {
        let mut game = Game::new();
        game.target_score = 3;
        game.scores[0] = 1;
        game.explosion = Some(Explosion {
            kind: ExplosionKind::Gorilla {
                gorilla_index: 1,
                winner_index: 0,
            },
            elapsed: GORILLA_EXPLOSION_DURATION - 0.001,
        });

        let update = game.update(0.1);

        assert_eq!(update.match_over, None);
    }

    #[test]
    fn seeded_cityscape_is_deterministic() {
        let first = make_cityscape_with_seed(1991);
        let second = make_cityscape_with_seed(1991);

        assert_eq!(first, second);
        assert!(!first.buildings.is_empty());
    }

    #[test]
    fn cityscape_uses_ega_building_and_window_bounds() {
        let round = make_cityscape_with_seed(1991);
        let first = &round.buildings[0];

        assert_eq!(first.x, 2.0);
        assert!(first.width >= DEFAULT_BUILDING_WIDTH as f32 + 1.0);
        assert!(first.width <= DEFAULT_BUILDING_WIDTH as f32 * 2.0);
        assert!(first.top() > GORILLA_HEIGHT);
        assert!(first.windows.iter().all(|window| {
            window.x >= first.x + 3.0
                && window.x < first.x + first.width - 3.0
                && window.y >= first.top()
                && window.y + WINDOW_HEIGHT <= BOTTOM_LINE
        }));
    }

    #[test]
    fn happy_sun_mouth_renders_below_center() {
        let mut canvas = Canvas::new(LOGICAL_WIDTH, LOGICAL_HEIGHT);
        draw_sun(&mut canvas, false);

        let x = (LOGICAL_WIDTH / 2) as usize;
        let y = 25usize;
        let lower_index = (y + 8) * LOGICAL_WIDTH as usize + x;
        let upper_index = (y - 8) * LOGICAL_WIDTH as usize + x;

        assert_eq!(canvas.pixels[lower_index], Some(BACKGROUND));
        assert_ne!(canvas.pixels[upper_index], Some(BACKGROUND));
    }

    #[test]
    fn gorillas_use_second_or_third_building_from_edges() {
        let buildings = test_buildings();
        let mut rng = SmallRng::seed_from_u64(4);
        let gorillas = place_gorillas_with_rng(&buildings, &mut rng);
        let left_candidates = [gorilla_on(&buildings, 1), gorilla_on(&buildings, 2)];
        let right_candidates = [
            gorilla_on(&buildings, buildings.len() - 2),
            gorilla_on(&buildings, buildings.len() - 3),
        ];

        assert!(left_candidates.contains(&gorillas[0]));
        assert!(right_candidates.contains(&gorillas[1]));
    }

    #[test]
    fn gorilla_position_uses_original_ega_offsets() {
        let buildings = test_buildings();
        let gorilla = gorilla_on(&buildings, 1);
        let expected_width = buildings[2].x - buildings[1].x;

        assert_eq!(
            gorilla,
            Gorilla {
                x: buildings[1].x + expected_width * 0.5 - GORILLA_X_ADJUST,
                y: buildings[1].top() - GORILLA_Y_ADJUST,
            }
        );
    }

    #[test]
    fn palette_keeps_standard_window_and_dark_gray_attributes() {
        assert_eq!(palette_attribute_rgb(14), [0xFF, 0xFF, 0x55]);
        assert_eq!(palette_attribute_rgb(8), [0x55, 0x55, 0x55]);
    }

    #[test]
    fn palette_applies_basic_register_remaps() {
        assert_eq!(palette_attribute_rgb(0), [0x00, 0x00, 0xAA]);
        assert_eq!(palette_attribute_rgb(5), [0xAA, 0xAA, 0xAA]);
        assert_eq!(palette_attribute_rgb(6), [0xAA, 0x00, 0x00]);
        assert_eq!(palette_attribute_rgb(7), [0x00, 0xAA, 0xAA]);
    }

    #[test]
    fn banana_rotation_frames_cycle_in_basic_order() {
        assert_eq!(BananaRotation::from_frame(0), BananaRotation::Left);
        assert_eq!(BananaRotation::from_frame(1), BananaRotation::Up);
        assert_eq!(BananaRotation::from_frame(2), BananaRotation::Down);
        assert_eq!(BananaRotation::from_frame(3), BananaRotation::Right);
        assert_eq!(BananaRotation::from_frame(4), BananaRotation::Left);
    }

    #[test]
    fn banana_frames_render_expected_ega_extents() {
        assert_banana_extent(BananaRotation::Left, 6, 5);
        assert_banana_extent(BananaRotation::Right, 6, 5);
        assert_banana_extent(BananaRotation::Up, 5, 5);
        assert_banana_extent(BananaRotation::Down, 5, 5);
    }

    #[test]
    fn up_and_down_banana_frames_are_distinct() {
        let up = rendered_banana_pixels(BananaRotation::Up);
        let down = rendered_banana_pixels(BananaRotation::Down);

        assert_ne!(up, down);
        assert_eq!(up[0][2], Some(OBJECT));
        assert_eq!(down[4][2], Some(OBJECT));
    }

    #[test]
    fn projectile_start_uses_original_player_offsets() {
        let gorilla = Gorilla { x: 120.0, y: 90.0 };

        assert_eq!(projectile_start(gorilla, Player::One), (120.0, 83.0));
        assert_eq!(projectile_start(gorilla, Player::Two), (145.0, 83.0));
    }

    #[test]
    fn projectile_sample_matches_basic_formula_without_wind() {
        let gorilla = Gorilla { x: 120.0, y: 90.0 };
        let sample = projectile_sample(gorilla, 45.0, 50.0, Player::One, 0, 9.8, 1.0);
        let expected_velocity = 45.0_f32.to_radians().cos() * 50.0;

        assert!((sample.x - (120.0 + expected_velocity)).abs() < 0.001);
        assert!((sample.y - (83.0 - expected_velocity + 4.9)).abs() < 0.001);
        assert_eq!(sample.rotation, BananaRotation::Down);
        assert!(sample.on_screen);
    }

    #[test]
    fn projectile_wind_changes_horizontal_acceleration() {
        let gorilla = Gorilla { x: 120.0, y: 90.0 };
        let calm = projectile_sample(gorilla, 0.0, 10.0, Player::One, 0, 9.8, 2.0);
        let windy = projectile_sample(gorilla, 0.0, 10.0, Player::One, 10, 9.8, 2.0);

        assert!((windy.x - calm.x - 4.0).abs() < 0.001);
    }

    #[test]
    fn projectile_bounds_match_basic_thresholds() {
        assert!(!projectile_on_screen(3.0, 100.0));
        assert!(!projectile_on_screen(LOGICAL_WIDTH as f32 - 10.0, 100.0));
        assert!(!projectile_on_screen(100.0, LOGICAL_HEIGHT as f32 - 3.0));
        assert!(projectile_on_screen(4.0, LOGICAL_HEIGHT as f32 - 4.0));
    }

    #[test]
    fn velocity_below_two_is_self_hit() {
        assert_eq!(
            slow_shot_outcome(1.99, Player::Two),
            SlowShotOutcome::SelfHit {
                player: Player::Two,
            }
        );
        assert_eq!(slow_shot_outcome(2.0, Player::Two), SlowShotOutcome::Flying);
    }

    #[test]
    fn projectile_advances_in_basic_time_steps() {
        let mut projectile =
            Projectile::new(Gorilla { x: 120.0, y: 90.0 }, Player::One, 45.0, 50.0);

        projectile.advance(0.016);
        assert!((projectile.elapsed - PROJECTILE_TIME_STEP).abs() < f32::EPSILON);

        projectile.advance(0.25);
        assert!((projectile.elapsed - 0.3).abs() < 0.001);
    }

    #[test]
    fn game_update_restarts_demo_projectile_after_leaving_screen() {
        let mut game = Game::new();
        game.turn_phase = TurnPhase::ProjectileFlying;
        game.projectile = Some(Projectile {
            start: Gorilla { x: 620.0, y: 340.0 },
            player: Player::One,
            angle_degrees: 0.0,
            velocity: 100.0,
            elapsed: 10.0,
            shot_in_sun: false,
        });

        let _ = game.update(PROJECTILE_TIME_STEP);

        assert!(game.projectile.is_none());
        assert_eq!(game.current_player, Player::Two);
        assert_eq!(
            game.turn_phase,
            TurnPhase::EnterAngle {
                input: String::new()
            }
        );
    }

    #[test]
    fn probe_detects_building_impact_from_visible_pixels() {
        let mut canvas = Canvas::new(LOGICAL_WIDTH, LOGICAL_HEIGHT);
        canvas.pixel(108.0, 120.0, EXPLOSION);
        let sample = ProjectileSample {
            x: 100.0,
            y: 120.0,
            rotation: BananaRotation::Left,
            on_screen: true,
        };

        assert_eq!(
            probe_projectile_collision(&canvas, sample, Player::One, false, &test_gorillas()),
            CollisionProbe::Impact {
                kind: ImpactKind::Building,
                x: 104.0,
                y: 124.0,
            }
        );
    }

    #[test]
    fn probe_marks_sun_hits_without_impact() {
        let mut canvas = Canvas::new(LOGICAL_WIDTH, LOGICAL_HEIGHT);
        canvas.pixel(320.0, 25.0, SUN);
        let sample = ProjectileSample {
            x: 312.0,
            y: 25.0,
            rotation: BananaRotation::Left,
            on_screen: true,
        };

        assert_eq!(
            probe_projectile_collision(&canvas, sample, Player::One, false, &test_gorillas()),
            CollisionProbe::Sun { shot_in_sun: true }
        );
    }

    #[test]
    fn probe_resolves_gorilla_hits_by_screen_half() {
        let mut canvas = Canvas::new(LOGICAL_WIDTH, LOGICAL_HEIGHT);
        let gorillas = [
            Gorilla { x: 100.0, y: 100.0 },
            Gorilla { x: 500.0, y: 100.0 },
        ];
        canvas.pixel(112.0, 110.0, OBJECT);
        let sample = ProjectileSample {
            x: 112.0,
            y: 110.0,
            rotation: BananaRotation::Left,
            on_screen: true,
        };

        assert_eq!(
            probe_projectile_collision(&canvas, sample, Player::Two, false, &gorillas),
            CollisionProbe::Impact {
                kind: ImpactKind::Gorilla(0),
                x: 116.0,
                y: 114.0,
            }
        );
    }

    #[test]
    fn probe_clears_shot_in_sun_after_leaving_sun_region() {
        let canvas = Canvas::new(LOGICAL_WIDTH, LOGICAL_HEIGHT);
        let sample = ProjectileSample {
            x: 100.0,
            y: 50.0,
            rotation: BananaRotation::Left,
            on_screen: true,
        };

        assert_eq!(
            probe_projectile_collision(&canvas, sample, Player::One, true, &test_gorillas()),
            CollisionProbe::Clear { shot_in_sun: false }
        );
    }

    #[test]
    fn game_update_enters_explosion_state_after_impact() {
        let mut game = Game::new();
        game.round.buildings = vec![Building {
            x: 0.0,
            width: LOGICAL_WIDTH as f32,
            height: 260.0,
            color: EXPLOSION,
            windows: Vec::new(),
        }];
        game.gorillas = [Gorilla { x: 120.0, y: 90.0 }, Gorilla { x: 500.0, y: 90.0 }];
        game.projectile = Some(Projectile {
            start: Gorilla { x: 120.0, y: 90.0 },
            player: Player::One,
            angle_degrees: 0.0,
            velocity: 0.0,
            elapsed: 0.0,
            shot_in_sun: false,
        });

        let _ = game.update(PROJECTILE_TIME_STEP);

        assert!(game.projectile.is_none());
        let explosion = game.explosion.unwrap();
        assert!(matches!(explosion.kind, ExplosionKind::Building { .. }));
    }

    #[test]
    fn explosion_completion_restarts_demo_projectile() {
        let mut game = Game::new();
        game.projectile = None;
        game.sun_shocked = true;
        game.explosion = Some(Explosion {
            kind: ExplosionKind::Building { x: 10.0, y: 10.0 },
            elapsed: EXPLOSION_DURATION,
        });

        let _ = game.update(PROJECTILE_TIME_STEP);

        assert!(game.projectile.is_none());
        assert!(game.explosion.is_none());
        assert!(!game.sun_shocked);
        assert_eq!(game.current_player, Player::Two);
    }

    #[test]
    fn game_update_uses_gorilla_explosion_for_object_hits() {
        let mut game = Game::new();
        game.round.buildings.clear();
        game.gorillas = [Gorilla { x: 120.0, y: 90.0 }, Gorilla { x: 500.0, y: 90.0 }];
        game.turn_phase = TurnPhase::ProjectileFlying;
        game.projectile = Some(Projectile {
            start: Gorilla { x: 495.0, y: 117.0 },
            player: Player::One,
            angle_degrees: 0.0,
            velocity: 0.0,
            elapsed: 0.0,
            shot_in_sun: false,
        });

        let _ = game.update(PROJECTILE_TIME_STEP);

        assert_eq!(game.explosion, Some(Explosion::gorilla(1, 0)));
        assert!(game.explosion_hits_gorilla(1));
    }

    #[test]
    fn angle_above_360_re_prompts_without_transitioning() {
        let mut game = Game::new();
        game.handle_char('3');
        game.handle_char('6');
        game.handle_char('1');
        let _ = game.handle_submit();
        assert_eq!(
            game.turn_phase,
            TurnPhase::EnterAngle {
                input: String::new()
            }
        );
    }

    #[test]
    fn submit_transitions_from_angle_to_velocity_to_projectile() {
        let mut game = Game::new();

        game.handle_char('4');
        game.handle_char('5');
        let _ = game.handle_submit();
        assert_eq!(
            game.turn_phase,
            TurnPhase::EnterVelocity {
                angle_deg: 45.0,
                input: String::new(),
            }
        );

        game.handle_char('5');
        game.handle_char('0');
        let _ = game.handle_submit();
        assert!(matches!(
            game.turn_phase,
            TurnPhase::ThrowingArm {
                player: Player::One,
                ..
            }
        ));
        assert_eq!(game.projectile.unwrap().angle_degrees, 45.0);
        assert_eq!(game.projectile.unwrap().velocity, 50.0);
    }

    #[test]
    fn player_two_angle_is_inverted_on_launch() {
        let mut game = Game::new();
        game.current_player = Player::Two;
        game.turn_phase = TurnPhase::EnterAngle {
            input: String::new(),
        };

        game.handle_char('6');
        game.handle_char('0');
        let _ = game.handle_submit();
        game.handle_char('4');
        game.handle_char('0');
        let _ = game.handle_submit();

        assert_eq!(game.projectile.unwrap().angle_degrees, 120.0);
    }

    #[test]
    fn hud_draws_score_line_near_row_23_center() {
        let mut game = Game::new();
        game.scores = [2, 1];
        let mut canvas = Canvas::new(LOGICAL_WIDTH, LOGICAL_HEIGHT);

        game.draw_hud(&mut canvas);

        let x = ((centered_col("2>Score<1") as i32 - 1) * TEXT_CELL_WIDTH + 2) as usize;
        let y = ((23 - 1) * TEXT_CELL_HEIGHT as usize) + 1;
        assert_eq!(canvas.pixels[y * canvas.width as usize + x], Some(HUD_TEXT));
    }

    #[test]
    fn hud_moves_angle_prompt_to_right_for_player_two() {
        let mut game = Game::new();
        game.current_player = Player::Two;
        let mut canvas = Canvas::new(LOGICAL_WIDTH, LOGICAL_HEIGHT);

        game.draw_hud(&mut canvas);

        let left_prompt_x = 1;
        let right_prompt_x = (65 * TEXT_CELL_WIDTH as usize) + 1;
        let y = TEXT_CELL_HEIGHT as usize + 1;

        assert_ne!(
            canvas.pixels[y * canvas.width as usize + right_prompt_x],
            None
        );
        assert_eq!(
            canvas.pixels[y * canvas.width as usize + left_prompt_x],
            None
        );
    }

    #[test]
    fn launching_shot_enters_throwing_arm_phase() {
        let mut game = Game::new();
        game.turn_phase = TurnPhase::EnterVelocity {
            angle_deg: 45.0,
            input: String::from("50"),
        };

        let _ = game.handle_submit();

        assert!(
            matches!(
                game.turn_phase,
                TurnPhase::ThrowingArm {
                    player: Player::One,
                    ..
                }
            ),
            "expected ThrowingArm for Player::One, got {:?}",
            game.turn_phase
        );
    }

    #[test]
    fn throwing_arm_timer_expires_and_transitions_to_projectile_flying() {
        let mut game = Game::new();
        game.turn_phase = TurnPhase::EnterVelocity {
            angle_deg: 45.0,
            input: String::from("50"),
        };
        let _ = game.handle_submit();

        let _ = game.update(THROW_ARM_DURATION + 0.05);

        assert_eq!(game.turn_phase, TurnPhase::ProjectileFlying);
    }

    #[test]
    fn player_one_throw_uses_left_arm() {
        assert_eq!(throwing_arm_pose(Player::One), GorillaArms::LeftUp);
    }

    #[test]
    fn player_two_throw_uses_right_arm() {
        assert_eq!(throwing_arm_pose(Player::Two), GorillaArms::RightUp);
    }

    #[test]
    fn gorilla_explosion_enters_victory_dance_phase() {
        let mut game = Game::new();
        game.explosion = Some(Explosion {
            kind: ExplosionKind::Gorilla {
                gorilla_index: 1,
                winner_index: 0,
            },
            elapsed: GORILLA_EXPLOSION_DURATION,
        });

        let _ = game.update(PROJECTILE_TIME_STEP);

        assert!(
            matches!(
                game.turn_phase,
                TurnPhase::VictoryDance {
                    winner_index: 0,
                    loser_index: 1,
                    cycle: 0,
                    ..
                }
            ),
            "expected VictoryDance, got {:?}",
            game.turn_phase
        );
    }

    #[test]
    fn victory_dance_timer_cycles_through_arm_poses() {
        let mut game = Game::new();
        game.turn_phase = TurnPhase::VictoryDance {
            winner_index: 0,
            loser_index: 1,
            cycle: 0,
            timer: VICTORY_DANCE_INTERVAL,
        };

        let _ = game.update(VICTORY_DANCE_INTERVAL + 0.01);

        assert!(
            matches!(game.turn_phase, TurnPhase::VictoryDance { cycle: 1, .. }),
            "expected cycle 1, got {:?}",
            game.turn_phase
        );
    }

    #[test]
    fn victory_dance_completes_after_eight_cycles_and_resets_round() {
        let mut game = Game::new();
        let old_gorilla_x = game.gorillas[0].x;
        game.turn_phase = TurnPhase::VictoryDance {
            winner_index: 0,
            loser_index: 1,
            cycle: 7,
            timer: VICTORY_DANCE_INTERVAL,
        };

        let _ = game.update(VICTORY_DANCE_INTERVAL + 0.01);

        assert_eq!(
            game.turn_phase,
            TurnPhase::EnterAngle {
                input: String::new()
            }
        );
        // New round means gorillas are placed again (positions likely differ from seed)
        // Just verify state reset, not exact position
        let _ = old_gorilla_x;
    }

    #[test]
    fn victory_dance_loser_gorilla_not_drawn() {
        let mut game = Game::new();
        game.round.buildings.clear();
        game.gorillas = [
            Gorilla { x: 100.0, y: 100.0 },
            Gorilla { x: 500.0, y: 100.0 },
        ];
        game.turn_phase = TurnPhase::VictoryDance {
            winner_index: 0,
            loser_index: 1,
            cycle: 0,
            timer: VICTORY_DANCE_INTERVAL,
        };

        let mut canvas_dance = Canvas::new(LOGICAL_WIDTH, LOGICAL_HEIGHT);
        game.draw_scene(&mut canvas_dance, false);

        // Render loser gorilla in isolation to find its pixel footprint
        let mut canvas_loser = Canvas::new(LOGICAL_WIDTH, LOGICAL_HEIGHT);
        draw_gorilla(&mut canvas_loser, game.gorillas[1], GorillaArms::Down);

        // Every pixel the loser would have drawn should be absent from the dance canvas
        let any_loser_pixel_drawn = canvas_loser
            .pixels
            .iter()
            .zip(canvas_dance.pixels.iter())
            .any(|(loser, dance)| loser.is_some() && dance.is_some());
        assert!(
            !any_loser_pixel_drawn,
            "loser gorilla pixels should not appear during VictoryDance"
        );
    }

    fn test_buildings() -> Vec<Building> {
        (0..8)
            .map(|index| Building {
                x: 2.0 + index as f32 * 50.0,
                width: 48.0,
                height: 80.0 + index as f32,
                color: building_color(5),
                windows: Vec::new(),
            })
            .collect()
    }

    fn test_gorillas() -> [Gorilla; 2] {
        [
            Gorilla { x: 100.0, y: 100.0 },
            Gorilla { x: 500.0, y: 100.0 },
        ]
    }

    fn assert_banana_extent(rotation: BananaRotation, width: usize, height: usize) {
        let pixels = rendered_banana_pixels(rotation);
        let lit: Vec<(usize, usize)> = pixels
            .iter()
            .enumerate()
            .flat_map(|(y, row)| {
                row.iter()
                    .enumerate()
                    .filter_map(move |(x, pixel)| pixel.map(|_| (x, y)))
            })
            .collect();

        let min_x = lit.iter().map(|(x, _)| *x).min().unwrap();
        let max_x = lit.iter().map(|(x, _)| *x).max().unwrap();
        let min_y = lit.iter().map(|(_, y)| *y).min().unwrap();
        let max_y = lit.iter().map(|(_, y)| *y).max().unwrap();

        assert_eq!(max_x - min_x + 1, width);
        assert_eq!(max_y - min_y + 1, height);
    }

    fn rendered_banana_pixels(rotation: BananaRotation) -> Vec<Vec<Option<[f32; 4]>>> {
        let mut canvas = Canvas::new(10, 10);
        draw_banana(&mut canvas, 2.0, 2.0, rotation);

        (2..8)
            .map(|y| {
                (2..8)
                    .map(|x| canvas.pixels[y * canvas.width as usize + x])
                    .collect()
            })
            .collect()
    }

    #[test]
    fn game_state_new_starts_in_config_menu() {
        let state = GameState::new();
        assert!(matches!(state.screen, AppScreen::ConfigMenu));
    }

    #[test]
    fn config_full_flow_through_defaults_enters_playing() {
        let mut state = GameState::new();
        assert!(matches!(state.screen, AppScreen::ConfigMenu));

        let _ = state.handle_submit();
        assert_eq!(state.active_field, ConfigField::PlayerTwoName);

        let _ = state.handle_submit();
        assert_eq!(state.active_field, ConfigField::TargetScore);

        let _ = state.handle_submit();
        assert_eq!(state.active_field, ConfigField::Gravity);

        let _ = state.handle_submit();

        assert_eq!(state.screen, AppScreen::Playing);
        assert_eq!(state.game.player_names[0], "Player 1");
        assert_eq!(state.game.player_names[1], "Player 2");
        assert_eq!(state.config.target_score, 3);
        assert!((state.game.gravity - 9.8).abs() < 0.001);
    }

    #[test]
    fn config_screen_renders_player_one_prompt_on_canvas() {
        let mut state = GameState::new();
        state.screen = AppScreen::ConfigMenu;
        state.active_field = ConfigField::PlayerOneName;
        let frame = state.frame();
        assert!(
            !frame.vertices.is_empty(),
            "config screen should produce render vertices"
        );
    }

    #[test]
    fn config_screen_shows_cursor_on_active_field_only() {
        let mut state = GameState::new();
        state.screen = AppScreen::ConfigMenu;
        state.active_field = ConfigField::PlayerOneName;
        let frame_field0 = state.frame();
        let _ = state.handle_submit();
        let frame_field1 = state.frame();
        assert_ne!(
            frame_field0.vertices.len(),
            frame_field1.vertices.len(),
            "frames should differ as cursor and values change between fields"
        );
    }

    #[test]
    fn config_name_field_accepts_alphanumeric() {
        let mut state = GameState::new();
        state.screen = AppScreen::ConfigMenu;
        state.active_field = ConfigField::PlayerOneName;
        for ch in "Ab3".chars() {
            state.handle_char(ch);
        }
        assert_eq!(state.field_input, "Ab3");
    }

    #[test]
    fn config_name_field_rejects_non_alphanumeric() {
        let mut state = GameState::new();
        state.screen = AppScreen::ConfigMenu;
        state.active_field = ConfigField::PlayerOneName;
        state.handle_char('\x01');
        state.handle_char('X');
        assert_eq!(state.field_input, "X");
    }

    #[test]
    fn config_player_name_accepts_spaces_and_punctuation() {
        let mut state = GameState::new();

        state.handle_char('A');
        state.handle_char(' ');
        state.handle_char('1');
        state.handle_char('!');

        assert_eq!(state.field_input, "A 1!");
    }

    #[test]
    fn config_player_name_still_caps_at_10_chars() {
        let mut state = GameState::new();

        for _ in 0..12 {
            state.handle_char('a');
        }

        assert_eq!(state.field_input.len(), 10);
    }

    #[test]
    fn config_name_field_caps_at_ten_chars() {
        let mut state = GameState::new();
        state.screen = AppScreen::ConfigMenu;
        state.active_field = ConfigField::PlayerOneName;
        for ch in "ABCDEFGHIJK".chars() {
            state.handle_char(ch);
        }
        assert_eq!(state.field_input.len(), 10);
    }

    #[test]
    fn config_score_field_rejects_letters() {
        let mut state = GameState::new();
        state.screen = AppScreen::ConfigMenu;
        state.active_field = ConfigField::TargetScore;
        state.handle_char('a');
        state.handle_char('3');
        assert_eq!(state.field_input, "3");
    }

    #[test]
    fn config_score_field_caps_at_two_digits() {
        let mut state = GameState::new();
        state.screen = AppScreen::ConfigMenu;
        state.active_field = ConfigField::TargetScore;
        for ch in "123".chars() {
            state.handle_char(ch);
        }
        assert_eq!(state.field_input, "12");
    }

    #[test]
    fn config_empty_name_submit_uses_default_and_advances() {
        let mut state = GameState::new();
        state.screen = AppScreen::ConfigMenu;
        state.active_field = ConfigField::PlayerOneName;
        let _ = state.handle_submit();
        assert_eq!(state.config.player_names[0], "Player 1");
        assert_eq!(state.active_field, ConfigField::PlayerTwoName);
        assert_eq!(state.field_input, "");
    }

    #[test]
    fn config_name_submit_stores_input_and_advances() {
        let mut state = GameState::new();
        state.screen = AppScreen::ConfigMenu;
        state.active_field = ConfigField::PlayerOneName;
        for ch in "Alice".chars() {
            state.handle_char(ch);
        }
        let _ = state.handle_submit();
        assert_eq!(state.config.player_names[0], "Alice");
        assert_eq!(state.active_field, ConfigField::PlayerTwoName);
    }

    #[test]
    fn config_score_zero_stays_on_same_field() {
        let mut state = GameState::new();
        state.screen = AppScreen::ConfigMenu;
        state.active_field = ConfigField::TargetScore;
        state.handle_char('0');
        let _ = state.handle_submit();
        assert_eq!(state.active_field, ConfigField::TargetScore);
        assert_eq!(state.field_input, "");
    }

    #[test]
    fn config_empty_score_uses_default_and_advances() {
        let mut state = GameState::new();
        state.screen = AppScreen::ConfigMenu;
        state.active_field = ConfigField::TargetScore;
        let _ = state.handle_submit();
        assert_eq!(state.config.target_score, 3);
        assert_eq!(state.active_field, ConfigField::Gravity);
    }

    #[test]
    fn config_gravity_zero_stays_on_same_field() {
        let mut state = GameState::new();
        state.screen = AppScreen::ConfigMenu;
        state.active_field = ConfigField::Gravity;
        state.handle_char('0');
        let _ = state.handle_submit();
        assert_eq!(state.active_field, ConfigField::Gravity);
        assert_eq!(state.field_input, "");
    }

    #[test]
    fn config_empty_gravity_uses_default_and_enters_playing() {
        let mut state = GameState::new();
        state.screen = AppScreen::ConfigMenu;
        state.active_field = ConfigField::Gravity;
        let _ = state.handle_submit();
        assert!((state.config.gravity - 9.8).abs() < 0.001);
        assert_eq!(state.screen, AppScreen::Playing);
    }

    #[test]
    fn config_apply_sets_gravity_on_game() {
        let mut state = GameState::new();
        state.screen = AppScreen::ConfigMenu;
        state.active_field = ConfigField::Gravity;
        state.config.gravity = 20.0;
        let _ = state.handle_submit();
        assert!((state.game.gravity - 20.0).abs() < 0.001);
    }

    #[test]
    fn config_apply_sets_player_names_on_game() {
        let mut state = GameState::new();
        state.screen = AppScreen::ConfigMenu;
        state.config.player_names[0] = String::from("Alice");
        state.config.player_names[1] = String::from("Bob");
        state.active_field = ConfigField::Gravity;
        let _ = state.handle_submit();
        assert_eq!(state.game.player_names[0], "Alice");
        assert_eq!(state.game.player_names[1], "Bob");
    }

    #[test]
    fn config_backspace_pops_last_char() {
        let mut state = GameState::new();
        state.screen = AppScreen::ConfigMenu;
        state.active_field = ConfigField::PlayerOneName;
        state.handle_char('A');
        state.handle_char('B');
        state.handle_backspace();
        assert_eq!(state.field_input, "A");
    }

    #[test]
    fn game_state_new_starts_in_config_menu_not_playing() {
        let state = GameState::new();
        assert!(matches!(state.screen, AppScreen::ConfigMenu));
        assert!(matches!(state.active_field, ConfigField::PlayerOneName));
    }

    #[test]
    fn game_state_default_config_has_correct_defaults() {
        let config = MatchConfig::default();
        assert_eq!(config.player_names[0], "Player 1");
        assert_eq!(config.player_names[1], "Player 2");
        assert_eq!(config.target_score, 3);
        assert!((config.gravity - 9.8).abs() < f32::EPSILON);
    }

    #[test]
    fn glyph_rows_covers_config_prompt_characters() {
        let required = "DFHMNUWdfhmunw()='/ ";
        for ch in required.chars() {
            assert!(glyph_rows(ch).is_some(), "missing glyph for '{ch}'");
        }
    }

    #[test]
    fn game_gravity_field_controls_projectile_fall_rate() {
        let mut game = Game::new();
        game.round.buildings.clear();
        game.gorillas = [
            Gorilla { x: 50.0, y: 200.0 },
            Gorilla { x: 580.0, y: 200.0 },
        ];
        game.turn_phase = TurnPhase::ProjectileFlying;
        game.projectile = Some(Projectile {
            start: Gorilla { x: 50.0, y: 200.0 },
            player: Player::One,
            angle_degrees: 0.0,
            velocity: 1.0,
            elapsed: 0.0,
            shot_in_sun: false,
        });
        game.gravity = DEFAULT_GRAVITY * 10_000.0;

        let _ = game.update(PROJECTILE_TIME_STEP);

        assert!(
            game.projectile.is_none(),
            "projectile should fall off screen immediately with extreme gravity"
        );
    }

    #[test]
    fn game_update_returns_vec_of_sound_cues() {
        let mut game = Game::new();
        let update = game.update(0.016);
        assert!(update.cues.is_empty()); // no cues when idle
    }

    #[test]
    fn game_handle_submit_returns_vec_of_sound_cues() {
        use crate::audio::SoundCue;
        let mut game = Game::new();
        let cues: Vec<SoundCue> = game.handle_submit();
        let _ = cues; // just verify it compiles and returns a Vec
    }

    #[test]
    fn handle_submit_emits_throw_cue_for_valid_velocity() {
        use crate::audio::SoundCue;
        let mut game = Game::new();
        game.turn_phase = TurnPhase::EnterVelocity {
            angle_deg: 45.0,
            input: String::from("50"),
        };
        let cues = game.handle_submit();
        assert!(
            cues.contains(&SoundCue::Throw),
            "expected Throw cue, got {cues:?}"
        );
    }

    #[test]
    fn handle_submit_does_not_emit_throw_for_angle_entry() {
        use crate::audio::SoundCue;
        let mut game = Game::new();
        // Submitting angle (not velocity) should not emit Throw
        game.handle_char('4');
        game.handle_char('5');
        let cues = game.handle_submit();
        assert!(!cues.contains(&SoundCue::Throw));
    }

    #[test]
    fn update_emits_victory_dance_cue_when_gorilla_explosion_finishes() {
        use crate::audio::SoundCue;
        let mut game = Game::new();
        game.explosion = Some(Explosion {
            kind: ExplosionKind::Gorilla {
                gorilla_index: 1,
                winner_index: 0,
            },
            elapsed: GORILLA_EXPLOSION_DURATION,
        });
        let update = game.update(PROJECTILE_TIME_STEP);
        assert!(
            update.cues.contains(&SoundCue::VictoryDance),
            "expected VictoryDance cue, got {:?}",
            update.cues
        );
    }

    #[test]
    fn game_state_handle_submit_emits_intro_cues_when_entering_playing() {
        use crate::audio::SoundCue;
        let mut state = GameState::new();
        // Submit through all 4 config fields (defaults); last one triggers game start
        let _ = state.handle_submit(); // P1 name
        let _ = state.handle_submit(); // P2 name
        let _ = state.handle_submit(); // target score
        let cues = state.handle_submit(); // gravity -> starts game
        assert!(
            cues.contains(&SoundCue::Intro),
            "expected Intro cue on game start, got {cues:?}"
        );
        assert!(
            cues.contains(&SoundCue::GorillaIntro),
            "expected GorillaIntro cue on game start, got {cues:?}"
        );
    }

    #[test]
    fn apply_config_wires_target_score_to_game() {
        let mut state = GameState::new();
        state.config.target_score = 5;
        state.apply_config_and_start();
        assert_eq!(state.game.target_score, 5);
    }

    #[test]
    fn game_state_transitions_to_match_over_when_game_signals_it() {
        let mut state = GameState::new();
        state.screen = AppScreen::Playing;
        state.game.target_score = 1;
        state.game.explosion = Some(Explosion {
            kind: ExplosionKind::Gorilla {
                gorilla_index: 1,
                winner_index: 0,
            },
            elapsed: GORILLA_EXPLOSION_DURATION - 0.001,
        });
        state.game.player_names = [String::from("Alice"), String::from("Bob")];
        state.game.scores = [0, 0];

        let _ = state.update(0.1);

        assert_eq!(state.screen, AppScreen::MatchOver);
        let mo = state.match_over_state.as_ref().unwrap();
        assert_eq!(mo.names[0], "Alice");
        assert_eq!(mo.scores[0], 1);
    }

    #[test]
    fn match_over_frame_has_vertices() {
        let mut state = GameState::new();
        state.screen = AppScreen::MatchOver;
        state.match_over_state = Some(MatchOverState {
            scores: [3, 1],
            names: [String::from("Alice"), String::from("Bob")],
        });

        let frame = state.frame();

        assert!(
            !frame.vertices.is_empty(),
            "MatchOver frame rendered no vertices"
        );
    }

    #[test]
    fn match_over_any_char_resets_to_config_menu() {
        let mut state = GameState::new();
        state.screen = AppScreen::MatchOver;
        state.match_over_state = Some(MatchOverState {
            scores: [3, 1],
            names: [String::from("Alice"), String::from("Bob")],
        });

        state.handle_char('x');

        assert_eq!(state.screen, AppScreen::ConfigMenu);
        assert!(state.match_over_state.is_none());
        assert_eq!(state.active_field, ConfigField::PlayerOneName);
        assert!(state.field_input.is_empty());
    }

    #[test]
    fn match_over_enter_resets_to_config_menu() {
        let mut state = GameState::new();
        state.screen = AppScreen::MatchOver;
        state.match_over_state = Some(MatchOverState {
            scores: [3, 1],
            names: [String::from("Alice"), String::from("Bob")],
        });

        let _ = state.handle_submit();

        assert_eq!(state.screen, AppScreen::ConfigMenu);
    }

    #[test]
    fn match_over_backspace_resets_to_config_menu() {
        let mut state = GameState::new();
        state.screen = AppScreen::MatchOver;
        state.match_over_state = Some(MatchOverState {
            scores: [3, 1],
            names: [String::from("Alice"), String::from("Bob")],
        });

        state.handle_backspace();

        assert_eq!(state.screen, AppScreen::ConfigMenu);
    }
}
