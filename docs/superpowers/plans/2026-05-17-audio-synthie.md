# Audio Implementation (synthie 0.3.0) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add SID-style sound effects and music to gorillas.rs using synthie 0.3.0, triggered by game events emitted from `Game::update()` and `Game::handle_submit()`.

**Architecture:** `Game` and `GameState` return `Vec<SoundCue>` from mutating methods. `App` owns an `Option<AudioScheduler>` that converts each cue to a sequence of `(Instant, AudioEvent)` pairs, firing them each frame via wall-clock scheduling. No synthie types appear in `game.rs`.

**Tech Stack:** Rust edition 2024, synthie 0.3.0, cpal 0.17, crossbeam-channel 0.5

---

## MIDI Reference

Formula: `midi = 36 + octave * 12 + semitone`, where `C=0 D=2 E=4 F=5 G=7 A=9 B=11`, flat subtracts 1, sharp adds 1.

| Octave | C  | D  | E  | F  | G  | A  | B  |
|--------|----|----|----|----|----|----|-----|
| O0     | 36 | 38 | 40 | 41 | 43 | 45 | 47 |
| O1     | 48 | 50 | 52 | 53 | 55 | 57 | 59 |
| O2     | 60 | 62 | 64 | 65 | 67 | 69 | 71 |

Key accidentals: O0 A-flat=44, O0 A-sharp=46. O2 E-flat=63, O2 D-flat=61, O2 G-flat=66.

## Duration Reference

At T160 BPM (375 000 µs/beat): L4=375 000, L8=187 500, L16=93 750, L32=46 875, L64=23 437.
At T120 BPM (500 000 µs/beat): L9=222 222, L16=125 000, L32=62 500, L64=31 250.

Note-off fires at `note_on_us + dur_us - 10_000` (10 ms gap between notes).

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `Cargo.toml` | Modify | Add synthie, cpal, anyhow |
| `src/main.rs` | Modify | Add `mod audio;` |
| `src/audio.rs` | Create | `SoundCue`, `AudioScheduler`, note sequences, `cue_to_events` |
| `src/game.rs` | Modify | Return `Vec<SoundCue>` from `update`/`handle_submit`; emit cues at trigger points |
| `src/app.rs` | Modify | Add `audio: Option<AudioScheduler>`; tick + dispatch each frame |

---

## Task 1: Add dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add synthie, cpal, and anyhow to Cargo.toml**

```toml
[dependencies]
anyhow = "1"
bytemuck = { version = "1.15", features = ["derive"] }
cpal = "0.17"
pollster = "0.4.0"
rand = "0.10.1"
synthie = "0.3.0"
winit = "0.30.13"
wgpu = "29.0.1"
```

- [ ] **Step 2: Verify the project compiles**

```bash
cargo check
```

Expected: zero errors (new deps resolve, existing code unchanged).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add synthie 0.3.0 and cpal 0.17 dependencies"
```

---

## Task 2: SoundCue enum

**Files:**
- Create: `src/audio.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add `mod audio;` to src/main.rs**

```rust
mod app;
mod audio;
mod game;
mod render;
```

- [ ] **Step 2: Create src/audio.rs with a failing test**

```rust
use std::time::Instant;

use anyhow::Result;
use synthie::params::AudioEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundCue {
    Intro,
    GorillaIntro,
    Throw,
    BuildingExplosion,
    GorillaExplosion,
    VictoryDance,
}

pub(crate) struct AudioScheduler {
    _stream: cpal::Stream,
    tx: crossbeam_channel::Sender<AudioEvent>,
    queue: Vec<(Instant, AudioEvent)>,
}

pub(crate) fn cue_to_events(_cue: SoundCue, _start: Instant) -> Vec<(Instant, AudioEvent)> {
    todo!()
}

impl AudioScheduler {
    pub(crate) fn new() -> Result<Self> {
        todo!()
    }

    pub(crate) fn play(&mut self, _cue: SoundCue) {
        todo!()
    }

    pub(crate) fn tick(&mut self) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_cue_all_variants_can_be_constructed() {
        let _ = [
            SoundCue::Intro,
            SoundCue::GorillaIntro,
            SoundCue::Throw,
            SoundCue::BuildingExplosion,
            SoundCue::GorillaExplosion,
            SoundCue::VictoryDance,
        ];
    }
}
```

- [ ] **Step 3: Run tests — expect compilation success, test pass**

```bash
cargo test -p gorillas audio::tests
```

Expected: `sound_cue_all_variants_can_be_constructed` passes (enum exists).

- [ ] **Step 4: Commit**

```bash
git add src/audio.rs src/main.rs
git commit -m "feat(audio): add SoundCue enum and AudioScheduler skeleton"
```

---

## Task 3: Note sequences and cue_to_events

**Files:**
- Modify: `src/audio.rs`

- [ ] **Step 1: Write failing tests for event counts and MIDI validity**

Add inside `#[cfg(test)] mod tests` in `src/audio.rs`:

```rust
#[test]
fn intro_produces_18_events() {
    let events = cue_to_events(SoundCue::Intro, Instant::now());
    assert_eq!(events.len(), 18); // 9 notes × 2
}

#[test]
fn gorilla_intro_produces_120_events() {
    let events = cue_to_events(SoundCue::GorillaIntro, Instant::now());
    assert_eq!(events.len(), 120); // 4 phrases × 15 notes × 2
}

#[test]
fn throw_produces_8_events() {
    let events = cue_to_events(SoundCue::Throw, Instant::now());
    assert_eq!(events.len(), 8); // 4 notes × 2
}

#[test]
fn building_explosion_produces_14_events() {
    let events = cue_to_events(SoundCue::BuildingExplosion, Instant::now());
    assert_eq!(events.len(), 14); // 7 notes × 2
}

#[test]
fn gorilla_explosion_produces_14_events() {
    let events = cue_to_events(SoundCue::GorillaExplosion, Instant::now());
    assert_eq!(events.len(), 14);
}

#[test]
fn victory_dance_produces_112_events() {
    let events = cue_to_events(SoundCue::VictoryDance, Instant::now());
    assert_eq!(events.len(), 112); // 8 reps × 7 notes × 2
}

#[test]
fn all_cues_have_valid_midi_range() {
    let start = Instant::now();
    for cue in [
        SoundCue::Intro,
        SoundCue::GorillaIntro,
        SoundCue::Throw,
        SoundCue::BuildingExplosion,
        SoundCue::GorillaExplosion,
        SoundCue::VictoryDance,
    ] {
        for (_, event) in cue_to_events(cue, start) {
            let midi = match event {
                AudioEvent::NoteOn(n) | AudioEvent::NoteOff(n) => n.as_u8(),
                _ => continue,
            };
            assert!(midi <= 127, "MIDI {midi} out of range for {cue:?}");
        }
    }
}

#[test]
fn note_off_fires_after_note_on_for_throw() {
    let start = Instant::now();
    let events = cue_to_events(SoundCue::Throw, start);
    let mut ons: Vec<(u8, Instant)> = Vec::new();
    let mut offs: Vec<(u8, Instant)> = Vec::new();
    for (t, event) in &events {
        match event {
            AudioEvent::NoteOn(n) => ons.push((n.as_u8(), *t)),
            AudioEvent::NoteOff(n) => offs.push((n.as_u8(), *t)),
            _ => {}
        }
    }
    for (on_midi, on_t) in &ons {
        let has_later_off = offs
            .iter()
            .any(|(off_midi, off_t)| off_midi == on_midi && off_t > on_t);
        assert!(has_later_off, "no NoteOff after NoteOn for midi {on_midi}");
    }
}
```

- [ ] **Step 2: Run tests — expect failures (todo! panics)**

```bash
cargo test -p gorillas audio::tests
```

Expected: FAIL with `not yet implemented`.

- [ ] **Step 3: Implement note sequence constants**

Replace the `todo!()` in `cue_to_events` with full implementation. First add the constants before the function:

```rust
use std::time::Duration;

// (start_us, midi, dur_us) — NoteOff fires at start_us + dur_us - 10_000
//
// MIDI = 36 + octave*12 + semitone  (C=0 D=2 E=4 F=5 G=7 A=9 B=11, flat-1, sharp+1)
// T160: L4=375_000, L8=187_500, L16=93_750, L32=46_875, L64=23_437
// T120: L9=222_222, L16=125_000, L32=62_500, L64=31_250

// MBT160O1L8CDEDCDL4ECC
const INTRO_NOTES: [(u64, u8, u64); 9] = [
    (0,           48, 187_500), // C L8
    (187_500,     50, 187_500), // D
    (375_000,     52, 187_500), // E
    (562_500,     50, 187_500), // D
    (750_000,     48, 187_500), // C
    (937_500,     50, 187_500), // D
    (1_125_000,   52, 375_000), // E L4
    (1_500_000,   48, 375_000), // C
    (1_875_000,   48, 375_000), // C
];

// MBo0L32A-L64CL16BL64A+  (T120)
const THROW_NOTES: [(u64, u8, u64); 4] = [
    (0,       44, 62_500),  // A-flat L32
    (62_500,  36, 31_250),  // C L64
    (93_750,  47, 125_000), // B L16
    (218_750, 46, 31_250),  // A-sharp L64
];

// MBO0L32EFGEFDC  (T120)
const BUILDING_EXPLOSION_NOTES: [(u64, u8, u64); 7] = [
    (0,       40, 62_500), // E
    (62_500,  41, 62_500), // F
    (125_000, 43, 62_500), // G
    (187_500, 40, 62_500), // E
    (250_000, 41, 62_500), // F
    (312_500, 38, 62_500), // D
    (375_000, 36, 62_500), // C
];

// MBO0L16EFGEFDC  (T120) — same pitches, doubled duration
const GORILLA_EXPLOSION_NOTES: [(u64, u8, u64); 7] = [
    (0,       40, 125_000),
    (125_000, 41, 125_000),
    (250_000, 43, 125_000),
    (375_000, 40, 125_000),
    (500_000, 41, 125_000),
    (625_000, 38, 125_000),
    (750_000, 36, 125_000),
];

// MFO0L32EFGEFDC x8 — same pitches as building explosion, 8 repetitions
const VICTORY_DANCE_UNIT: [(u64, u8, u64); 7] = BUILDING_EXPLOSION_NOTES;
const VICTORY_UNIT_DUR_US: u64 = 437_500; // 7 × 62_500

// GorillaIntro: 4 melodic phrases, each with the same timing template.
// Derived from:
//   t120o1l16 b9n0baan0bn0bn0baaan0b9n0baan0b
//   o2l16     e-9n0e-d-d-n0e-n0e-n0e-d-d-d-n0e-9n0e-d-d-n0e-
//   o2l16     g-9n0g-een0g-n0g-n0g-eeen0g-9n0g-een0g-
//   o2l16     b9n0baan0g-n0g-n0g-eeen0o1b9n0baan0b
//
// All 4 phrases share this (start_us, dur_us) timing template (15 notes, rests omitted):
const GORILLA_INTRO_TIMING: [(u64, u64); 15] = [
    (0,           222_222), // L9
    (347_222,     125_000), // L16
    (472_222,     125_000),
    (597_222,     125_000),
    (847_222,     125_000),
    (1_097_222,   125_000),
    (1_347_222,   125_000),
    (1_472_222,   125_000),
    (1_597_222,   125_000),
    (1_722_222,   125_000),
    (1_972_222,   222_222), // L9
    (2_319_444,   125_000),
    (2_444_444,   125_000),
    (2_569_444,   125_000),
    (2_819_444,   125_000),
];
const GORILLA_INTRO_PHRASE_DUR_US: u64 = 2_944_444; // 2_819_444 + 125_000

// MIDI notes for each phrase (15 per phrase):
// P1 o1: B B A A B B B A A A B B A A B
// P2 o2: Eb Eb Db Db Eb Eb Eb Db Db Db Eb Eb Db Db Eb
// P3 o2: Gb Gb E E Gb Gb Gb E E E Gb Gb E E Gb
// P4 o2→o1: B2 B2 A2 A2 Gb Gb Gb E E E B1 B1 A1 A1 B1
const GORILLA_INTRO_PHRASES: [[u8; 15]; 4] = [
    [59, 59, 57, 57, 59, 59, 59, 57, 57, 57, 59, 59, 57, 57, 59],
    [63, 63, 61, 61, 63, 63, 63, 61, 61, 61, 63, 63, 61, 61, 63],
    [66, 66, 64, 64, 66, 66, 66, 64, 64, 64, 66, 66, 64, 64, 66],
    [71, 71, 69, 69, 66, 66, 66, 64, 64, 64, 59, 59, 57, 57, 59],
];
```

- [ ] **Step 4: Implement cue_to_events**

Replace the `todo!()` body:

```rust
pub(crate) fn cue_to_events(cue: SoundCue, start: Instant) -> Vec<(Instant, AudioEvent)> {
    use synthie::params::MidiNote;

    fn push_notes(
        out: &mut Vec<(Instant, AudioEvent)>,
        start: Instant,
        notes: &[(u64, u8, u64)],
    ) {
        for &(on_us, midi, dur_us) in notes {
            let on_t = start + Duration::from_micros(on_us);
            let off_t = on_t + Duration::from_micros(dur_us.saturating_sub(10_000));
            out.push((on_t, AudioEvent::NoteOn(MidiNote(midi))));
            out.push((off_t, AudioEvent::NoteOff(MidiNote(midi))));
        }
    }

    let mut events = Vec::new();

    match cue {
        SoundCue::Intro => {
            push_notes(&mut events, start, &INTRO_NOTES);
        }
        SoundCue::GorillaIntro => {
            for (phrase_idx, midi_notes) in GORILLA_INTRO_PHRASES.iter().enumerate() {
                let phrase_start =
                    start + Duration::from_micros(phrase_idx as u64 * GORILLA_INTRO_PHRASE_DUR_US);
                for (slot_idx, &(slot_us, dur_us)) in GORILLA_INTRO_TIMING.iter().enumerate() {
                    let midi = midi_notes[slot_idx];
                    let on_t = phrase_start + Duration::from_micros(slot_us);
                    let off_t = on_t + Duration::from_micros(dur_us.saturating_sub(10_000));
                    events.push((on_t, AudioEvent::NoteOn(MidiNote(midi))));
                    events.push((off_t, AudioEvent::NoteOff(MidiNote(midi))));
                }
            }
        }
        SoundCue::Throw => {
            push_notes(&mut events, start, &THROW_NOTES);
        }
        SoundCue::BuildingExplosion => {
            push_notes(&mut events, start, &BUILDING_EXPLOSION_NOTES);
        }
        SoundCue::GorillaExplosion => {
            push_notes(&mut events, start, &GORILLA_EXPLOSION_NOTES);
        }
        SoundCue::VictoryDance => {
            for rep in 0..8u64 {
                let offset_us = rep * VICTORY_UNIT_DUR_US;
                let rep_notes: [(u64, u8, u64); 7] = VICTORY_DANCE_UNIT.map(|(t, m, d)| {
                    (t + offset_us, m, d)
                });
                push_notes(&mut events, start, &rep_notes);
            }
        }
    }

    events
}
```

- [ ] **Step 5: Run tests — expect all pass**

```bash
cargo test -p gorillas audio::tests
```

Expected: all 9 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/audio.rs
git commit -m "feat(audio): implement note sequences and cue_to_events for all 6 cues"
```

---

## Task 4: Implement AudioScheduler

**Files:**
- Modify: `src/audio.rs`

AudioScheduler does not require audio hardware in tests; its methods are only tested indirectly via cue_to_events. No hardware-dependent tests here.

- [ ] **Step 1: Implement AudioScheduler::new()**

Replace the `todo!()` in `new()`:

```rust
pub(crate) fn new() -> Result<Self> {
    use synthie::params::AudioEvent;
    use synthie::prelude::setup_audio;
    use synthie::presets::sid::default_patches;

    let (stream, tx, _scope_rx) = setup_audio()?;

    let patch = default_patches()
        .into_iter()
        .find(|p| p.name == "PWM Lead")
        .expect("synthie 0.3.0 ships PWM Lead preset");
    let _ = tx.send(AudioEvent::LoadPatch(Box::new(patch.params)));

    Ok(Self {
        _stream: stream,
        tx,
        queue: Vec::new(),
    })
}
```

- [ ] **Step 2: Implement AudioScheduler::play()**

Replace the `todo!()` in `play()`:

```rust
pub(crate) fn play(&mut self, cue: SoundCue) {
    let _ = self.tx.send(AudioEvent::Panic);
    self.queue.clear();
    let events = cue_to_events(cue, Instant::now());
    self.queue.extend(events);
}
```

- [ ] **Step 3: Implement AudioScheduler::tick()**

Replace the `todo!()` in `tick()`:

```rust
pub(crate) fn tick(&mut self) {
    let now = Instant::now();
    let mut i = 0;
    while i < self.queue.len() {
        if self.queue[i].0 <= now {
            let (_, event) = self.queue.remove(i);
            let _ = self.tx.send(event);
        } else {
            i += 1;
        }
    }
}
```

- [ ] **Step 4: Verify the whole module compiles**

```bash
cargo check
```

Expected: zero errors.

- [ ] **Step 5: Commit**

```bash
git add src/audio.rs
git commit -m "feat(audio): implement AudioScheduler new/play/tick"
```

---

## Task 5: Change Game return types and fix existing tests

**Files:**
- Modify: `src/game.rs`

`Game::update` currently returns `()`. Change it and `Game::handle_submit` to return `Vec<SoundCue>`. Fix the `GameState` wrappers. Update all call sites in tests to discard the return value.

- [ ] **Step 1: Write a failing test asserting update returns something**

Add inside the `#[cfg(test)] mod tests` in `src/game.rs`:

```rust
#[test]
fn game_update_returns_vec_of_sound_cues() {
    use crate::audio::SoundCue;
    let mut game = Game::new();
    let cues: Vec<SoundCue> = game.update(0.016);
    assert!(cues.is_empty()); // no cues when idle
}

#[test]
fn game_handle_submit_returns_vec_of_sound_cues() {
    use crate::audio::SoundCue;
    let mut game = Game::new();
    let cues: Vec<SoundCue> = game.handle_submit();
    let _ = cues; // just verify it compiles and returns a Vec
}
```

- [ ] **Step 2: Run tests — expect compile error (wrong return type)**

```bash
cargo test -p gorillas 2>&1 | head -30
```

Expected: `error[E0308]: mismatched types` — `update` returns `()` but test expects `Vec<SoundCue>`.

- [ ] **Step 3: Change Game::update signature and return type**

In `src/game.rs`, find:

```rust
pub fn update(&mut self, dt: f32) {
```

Change to:

```rust
pub fn update(&mut self, dt: f32) -> Vec<crate::audio::SoundCue> {
    let cues = Vec::new();
```

Add `cues` as the return value at the end of the function body (currently all early returns use `return;` — change those to `return cues;`). The final implicit return changes to `cues`.

The complete updated `Game::update` body — find every `return;` in `update` and replace with `return cues;`, then add `cues` at the bottom:

```rust
pub fn update(&mut self, dt: f32) -> Vec<crate::audio::SoundCue> {
    let cues = Vec::new();

    if let Some(explosion) = self.explosion.as_mut() {
        explosion.advance(dt);
        if explosion.finished() {
            let explosion = self.explosion.take().unwrap();
            self.finish_explosion(explosion);
        }
        return cues;
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
        return cues;
    }

    if self.projectile.is_none() {
        return cues;
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
        return cues;
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
            self.explosion = Some(match kind {
                ImpactKind::Building => Explosion::building(x, y),
                ImpactKind::Gorilla(player_index) => {
                    Explosion::gorilla(player_index, projectile.player.index())
                }
            });
            self.sun_shocked = false;
        }
    }

    cues
}
```

- [ ] **Step 4: Change Game::handle_submit signature and return type**

Find:

```rust
pub fn handle_submit(&mut self) {
```

Change to:

```rust
pub fn handle_submit(&mut self) -> Vec<crate::audio::SoundCue> {
    let cues = Vec::new();
```

Add `cues` at the end of the function (all existing early `return;` statements change to `return cues;`). Full replacement:

```rust
pub fn handle_submit(&mut self) -> Vec<crate::audio::SoundCue> {
    let cues = Vec::new();

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
        }
        TurnPhase::ThrowingArm { .. }
        | TurnPhase::VictoryDance { .. }
        | TurnPhase::ProjectileFlying => {}
    }

    cues
}
```

- [ ] **Step 5: Update GameState::update and GameState::handle_submit**

Find `GameState::update`:

```rust
pub fn update(&mut self, dt: f32) {
    match self.screen {
        AppScreen::ConfigMenu => {}
        AppScreen::Playing => self.game.update(dt),
    }
}
```

Change to:

```rust
pub fn update(&mut self, dt: f32) -> Vec<crate::audio::SoundCue> {
    match self.screen {
        AppScreen::ConfigMenu => vec![],
        AppScreen::Playing => self.game.update(dt),
    }
}
```

Find `GameState::handle_submit`:

```rust
pub fn handle_submit(&mut self) {
    match self.screen {
        AppScreen::ConfigMenu => self.config_handle_submit(),
        AppScreen::Playing => self.game.handle_submit(),
    }
}
```

Change to:

```rust
pub fn handle_submit(&mut self) -> Vec<crate::audio::SoundCue> {
    match self.screen {
        AppScreen::ConfigMenu => {
            self.config_handle_submit();
            vec![]
        }
        AppScreen::Playing => self.game.handle_submit(),
    }
}
```

- [ ] **Step 6: Fix all test call sites — discard return values**

In `src/game.rs` tests, every call to `game.update(...)` or `game.handle_submit()` or `state.handle_submit()` that ignores the return value will now warn. Change each to discard explicitly:

Find pattern `game.update(` in tests and prepend `let _ = `:

```rust
// Before:
game.update(PROJECTILE_TIME_STEP);
// After:
let _ = game.update(PROJECTILE_TIME_STEP);
```

Same for `handle_submit`:

```rust
// Before:
game.handle_submit();
// After:
let _ = game.handle_submit();
```

And for `state.handle_submit()`:

```rust
// Before:
state.handle_submit();
// After:
let _ = state.handle_submit();
```

The affected test lines are (by approximate line numbers — verify with `grep`):
- `game.update` at lines ~2021, 2132, 2149, 2172, 2307, 2333, 2360, 2380
- `game.handle_submit` at lines ~2184, 2199, 2210, 2232, 2235, 2283, 2305
- `state.handle_submit` at lines ~2491, 2494, 2497, 2500, 2527

Also update `app.rs` — the call `self.game.update(dt)` currently discards the return:

```rust
// In RedrawRequested handler, find:
self.game.update(dt);
// Change to:
let _ = self.game.update(dt);
```

And `self.game.handle_submit()` in the key handler:

```rust
// Find:
Key::Named(NamedKey::Enter) => self.game.handle_submit(),
// Change to:
Key::Named(NamedKey::Enter) => { let _ = self.game.handle_submit(); }
```

- [ ] **Step 7: Run all tests — expect all pass**

```bash
cargo test -p gorillas
```

Expected: all tests pass, no warnings about unused `must_use`.

- [ ] **Step 8: Commit**

```bash
git add src/game.rs src/app.rs
git commit -m "refactor(game): change update/handle_submit to return Vec<SoundCue>"
```

---

## Task 6: Emit SoundCues at trigger points

**Files:**
- Modify: `src/game.rs`

Trigger map:
- `Throw` — `Game::handle_submit` when valid velocity and projectile is created
- `BuildingExplosion` — `Game::update` when `CollisionProbe::Impact { kind: ImpactKind::Building, .. }`
- `GorillaExplosion` — `Game::update` when `CollisionProbe::Impact { kind: ImpactKind::Gorilla(..), .. }`
- `VictoryDance` — `Game::update` when gorilla explosion finishes (before `finish_explosion`)
- `Intro` + `GorillaIntro` — `GameState::handle_submit` when transitioning `ConfigMenu → Playing`

- [ ] **Step 1: Write failing tests for cue emission**

Add inside `#[cfg(test)] mod tests` in `src/game.rs`:

```rust
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
    let cues = game.update(PROJECTILE_TIME_STEP);
    assert!(
        cues.contains(&SoundCue::VictoryDance),
        "expected VictoryDance cue, got {cues:?}"
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
    let cues = state.handle_submit(); // gravity → starts game
    assert!(
        cues.contains(&SoundCue::Intro),
        "expected Intro cue on game start, got {cues:?}"
    );
    assert!(
        cues.contains(&SoundCue::GorillaIntro),
        "expected GorillaIntro cue on game start, got {cues:?}"
    );
}
```

- [ ] **Step 2: Run tests — expect failures**

```bash
cargo test -p gorillas game::tests::handle_submit_emits_throw_cue_for_valid_velocity
cargo test -p gorillas game::tests::update_emits_victory_dance_cue_when_gorilla_explosion_finishes
cargo test -p gorillas game::tests::game_state_handle_submit_emits_intro_cues_when_entering_playing
```

Expected: FAIL — cue Vecs are empty.

- [ ] **Step 3: Emit Throw cue in Game::handle_submit**

In the `TurnPhase::EnterVelocity` arm, after the projectile is created (after `self.turn_phase = TurnPhase::ThrowingArm { ... }`), push the cue. Change `let cues = Vec::new();` at the top to `let mut cues = Vec::new();` and add:

```rust
TurnPhase::EnterVelocity { angle_deg, input } => {
    let velocity = parse_numeric_input(input);
    if velocity < MIN_THROW_VELOCITY {
        // ... existing low-velocity branch unchanged
        return cues;
    }

    // ... existing projectile creation unchanged ...

    self.turn_phase = TurnPhase::ThrowingArm {
        player,
        timer: THROW_ARM_DURATION,
    };
    cues.push(crate::audio::SoundCue::Throw); // ← add this line
}
```

- [ ] **Step 4: Emit BuildingExplosion and GorillaExplosion in Game::update**

In the `CollisionProbe::Impact` arm of `Game::update`, change `let cues = Vec::new();` to `let mut cues = Vec::new();` and push the appropriate cue:

```rust
CollisionProbe::Impact { kind, x, y } => {
    self.projectile = None;
    match kind {
        ImpactKind::Building => {
            cues.push(crate::audio::SoundCue::BuildingExplosion);
            self.explosion = Some(Explosion::building(x, y));
        }
        ImpactKind::Gorilla(player_index) => {
            cues.push(crate::audio::SoundCue::GorillaExplosion);
            self.explosion = Some(Explosion::gorilla(
                player_index,
                projectile.player.index(),
            ));
        }
    }
    self.sun_shocked = false;
}
```

- [ ] **Step 5: Emit VictoryDance cue when gorilla explosion finishes**

In `Game::update`, in the explosion branch, before calling `finish_explosion`, emit the cue:

```rust
if let Some(explosion) = self.explosion.as_mut() {
    explosion.advance(dt);
    if explosion.finished() {
        let explosion = self.explosion.take().unwrap();
        if matches!(explosion.kind, ExplosionKind::Gorilla { .. }) {
            cues.push(crate::audio::SoundCue::VictoryDance);
        }
        self.finish_explosion(explosion);
    }
    return cues;
}
```

Note: `cues` must be `mut` from the top of `update`. Update the declaration:

```rust
pub fn update(&mut self, dt: f32) -> Vec<crate::audio::SoundCue> {
    let mut cues = Vec::new();
```

- [ ] **Step 6: Emit Intro and GorillaIntro in GameState::handle_submit**

Change the `ConfigMenu` arm:

```rust
AppScreen::ConfigMenu => {
    let was_config = matches!(self.screen, AppScreen::ConfigMenu);
    self.config_handle_submit();
    if was_config && matches!(self.screen, AppScreen::Playing) {
        vec![
            crate::audio::SoundCue::Intro,
            crate::audio::SoundCue::GorillaIntro,
        ]
    } else {
        vec![]
    }
}
```

- [ ] **Step 7: Run all tests — expect all pass**

```bash
cargo test -p gorillas
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/game.rs
git commit -m "feat(game): emit SoundCue events at throw, explosion, and game-start triggers"
```

---

## Task 7: Wire AudioScheduler into App

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Add audio field to App**

In `src/app.rs`, add the import at the top:

```rust
use crate::audio::{AudioScheduler, SoundCue};
```

Add `audio` field to the `App` struct:

```rust
struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    game: GameState,
    last_update: Instant,
    first_screenshot_taken: bool,
    audio: Option<AudioScheduler>,
}
```

- [ ] **Step 2: Initialize AudioScheduler in App::new()**

```rust
fn new() -> Self {
    let audio = AudioScheduler::new()
        .map_err(|e| eprintln!("audio init failed: {e}"))
        .ok();
    Self {
        window: None,
        renderer: None,
        game: GameState::new(),
        last_update: Instant::now(),
        first_screenshot_taken: false,
        audio,
    }
}
```

- [ ] **Step 3: Tick and dispatch cues in the RedrawRequested handler**

Find the `RedrawRequested` match arm. Replace the current `self.game.update(dt)` call with:

```rust
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

        // ... rest of render + screenshot code unchanged ...
    }
}
```

Add the helper function (outside impl, in the module):

```rust
fn dispatch_cues(audio: &mut Option<AudioScheduler>, cues: Vec<SoundCue>) {
    if let Some(audio) = audio.as_mut() {
        for cue in cues {
            audio.play(cue);
        }
    }
}
```

- [ ] **Step 4: Dispatch cues from handle_submit**

Find the `Key::Named(NamedKey::Enter)` arm:

```rust
Key::Named(NamedKey::Enter) => {
    let cues = self.game.handle_submit();
    dispatch_cues(&mut self.audio, cues);
}
```

- [ ] **Step 5: Verify the project compiles**

```bash
cargo check
```

Expected: zero errors.

- [ ] **Step 6: Run tests**

```bash
cargo test -p gorillas
```

Expected: all pass (AudioScheduler::new() is never called in tests, so no hardware needed).

- [ ] **Step 7: Commit**

```bash
git add src/app.rs
git commit -m "feat(app): wire AudioScheduler into App; tick and dispatch SoundCues each frame"
```

---

## Task 8: Final verification

- [ ] **Step 1: Run full test suite**

```bash
cargo test -p gorillas
```

Expected: all tests pass, zero failures.

- [ ] **Step 2: Build in release mode**

```bash
cargo build --release
```

Expected: zero errors, zero warnings.

- [ ] **Step 3: Run the game and verify audio**

```bash
cargo run --release
```

Manual checks:
- Config screen appears silently
- Submit through config fields — Intro melody plays (C D E D C D E C C at ~1 sec)
- GorillaIntro phrases play immediately after (syncopated B/A motif, escalating up to O2)
- Enter angle and velocity, press Enter — short Throw sound (A-flat C B A-sharp)
- Banana hits building — BuildingExplosion flourish (E F G E F D C)
- Banana hits gorilla — longer GorillaExplosion (same notes, doubled)
- Victory dance — 8x repetition of the explosion motif
- No crash if audio device is unavailable (game continues silently)

- [ ] **Step 4: Final commit with issue reference**

```bash
git add -A
git commit -m "feat: implement audio via synthie 0.3.0 - closes #19

Hand-translated QBasic PLAY strings to MIDI note sequences. Wall-clock
AudioScheduler fires NoteOn/NoteOff events from a per-frame tick loop.
Game returns Vec<SoundCue> from update/handle_submit; App dispatches
to AudioScheduler. Audio failure is non-fatal."
```

---

## Self-Review Notes

**Spec coverage check:**
- SoundCue enum with 6 variants: Task 2 ✓
- cue_to_events with all 6 sequences: Task 3 ✓
- AudioScheduler new/play/tick: Task 4 ✓
- Game::update returns Vec<SoundCue>: Task 5 ✓
- Throw trigger in handle_submit: Task 6 ✓
- Building/GorillaExplosion triggers in update: Task 6 ✓
- VictoryDance trigger in update: Task 6 ✓
- Intro/GorillaIntro trigger in GameState::handle_submit: Task 6 ✓
- App wired with Option<AudioScheduler>: Task 7 ✓
- Audio failure is non-fatal: Task 7 step 2 ✓
- tx.send errors silently discarded: Tasks 4 + 7 ✓
- No audio hardware in tests: all test steps ✓
- Existing tests fixed: Task 5 ✓

**Type consistency:** `Vec<crate::audio::SoundCue>` used throughout game.rs. `SoundCue` imported in app.rs via `use crate::audio::{AudioScheduler, SoundCue}`. `dispatch_cues` helper uses same types. Consistent.

**Note on mut:** Tasks 5 and 6 both change `let cues = Vec::new()` to `let mut cues = Vec::new()`. Verify both `update` and `handle_submit` use `mut`. The plan shows `let mut cues` in Task 6 step 5 — make sure this carries over when implementing Task 5 first (Task 5 sets up `let cues` which Task 6 changes to `let mut cues`).
