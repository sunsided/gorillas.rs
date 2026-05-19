# GorillaIntro Screen Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `AppScreen::GorillaIntro` between ConfigMenu and Playing: a choice prompt (V = View Intro / P = Play Game) followed by an optional gorilla animation in sync with the GorillaIntro music.

**Architecture:** All changes are confined to `src/game.rs`. A new `GorillaIntroState` struct (holding a `GorillaIntroPhase`) is stored as `Option<GorillaIntroState>` on `GameState`. Config submit redirects to GorillaIntro instead of directly to Playing. Playing is reached either via a non-V keypress (skip path) or automatically after 4 music phrases complete.

**Tech Stack:** Rust, existing `draw_gorilla` / `draw_text` / `Canvas` / `centered_col` helpers in `src/game.rs`, `SoundCue::GorillaIntro` (already defined in `src/audio.rs`).

---

### Task 1: Scaffold GorillaIntroPhase, GorillaIntroState, AppScreen::GorillaIntro

**Files:**
- Modify: `src/game.rs`

- [ ] **Step 1: Add four constants near the other constants at the top of game.rs (after `SPARKLE_FRAME_DURATION`)**

```rust
const GORILLA_INTRO_PHRASE_DUR_S: f32 = 2.944_444;
const GORILLA_INTRO_X1: f32 = 265.0;
const GORILLA_INTRO_X2: f32 = 325.0;
const GORILLA_INTRO_Y: f32 = 290.0;
```

- [ ] **Step 2: Add GorillaIntroPhase and GorillaIntroState after the GorillaArms enum**

```rust
#[derive(Debug)]
enum GorillaIntroPhase {
    ChoiceMenu,
    Animation { timer: f32, phrase: u8, arm: GorillaArms },
}

#[derive(Debug)]
struct GorillaIntroState {
    phase: GorillaIntroPhase,
    player_names: [String; 2],
    sound_pending: bool,
}
```

- [ ] **Step 3: Add GorillaIntro variant to AppScreen**

Change `AppScreen` from:
```rust
pub enum AppScreen {
    Intro,
    ConfigMenu,
    Playing,
    MatchOver,
    PlayAgain,
}
```
to:
```rust
pub enum AppScreen {
    Intro,
    ConfigMenu,
    GorillaIntro,
    Playing,
    MatchOver,
    PlayAgain,
}
```

- [ ] **Step 4: Add gorilla_intro field to GameState struct**

In the `GameState` struct body, add:
```rust
gorilla_intro: Option<GorillaIntroState>,
```

- [ ] **Step 5: Initialize gorilla_intro in GameState::new()**

In the `Self { ... }` initializer inside `GameState::new()`, add:
```rust
gorilla_intro: None,
```

- [ ] **Step 6: Add GorillaIntro to the update() no-op group**

In `GameState::update()`, change:
```rust
AppScreen::ConfigMenu | AppScreen::MatchOver | AppScreen::PlayAgain => vec![],
```
to:
```rust
AppScreen::ConfigMenu | AppScreen::MatchOver | AppScreen::PlayAgain | AppScreen::GorillaIntro => vec![],
```

- [ ] **Step 7: Add GorillaIntro stub arm to frame()**

In `GameState::frame()`, add before the `AppScreen::Playing` arm:
```rust
AppScreen::GorillaIntro => Frame {
    logical_width: LOGICAL_WIDTH,
    logical_height: LOGICAL_HEIGHT,
    clear_color: BACKGROUND,
    vertices: vec![],
},
```

- [ ] **Step 8: Add GorillaIntro stub arm to handle_char()**

In `GameState::handle_char()`, add:
```rust
AppScreen::GorillaIntro => {}
```

- [ ] **Step 9: Add GorillaIntro stub arm to handle_backspace()**

In `GameState::handle_backspace()`, add:
```rust
AppScreen::GorillaIntro => {}
```

- [ ] **Step 10: Add GorillaIntro stub arm to handle_submit()**

In `GameState::handle_submit()`, add:
```rust
AppScreen::GorillaIntro => vec![],
```

- [ ] **Step 11: Clear gorilla_intro in reset_to_config()**

In `GameState::reset_to_config()`, add:
```rust
self.gorilla_intro = None;
```

- [ ] **Step 12: Verify it compiles**

```bash
cargo check 2>&1
```
Expected: no errors.

- [ ] **Step 13: Run tests — expect no regressions**

```bash
cargo test 2>&1
```
Expected: all existing tests pass (no behavioral change yet).

- [ ] **Step 14: Commit**

```bash
git add src/game.rs
git commit -m "feat(game): scaffold GorillaIntro types and AppScreen variant"
```

---

### Task 2: Config submit transitions to GorillaIntro

**Files:**
- Modify: `src/game.rs`

- [ ] **Step 1: Add a private helper and two failing tests in the mod tests block**

```rust
fn advance_state_to_gorilla_intro() -> GameState {
    let mut state = GameState::new();
    state.screen = AppScreen::ConfigMenu;
    let _ = state.handle_submit(); // P1 name (default)
    let _ = state.handle_submit(); // P2 name (default)
    let _ = state.handle_submit(); // target score (default)
    let _ = state.handle_submit(); // gravity (default) -> GorillaIntro
    state
}

#[test]
fn config_submit_transitions_to_gorilla_intro() {
    let state = advance_state_to_gorilla_intro();
    assert_eq!(state.screen, AppScreen::GorillaIntro);
}

#[test]
fn config_submit_initializes_gorilla_intro_state() {
    let state = advance_state_to_gorilla_intro();
    let gi = state.gorilla_intro.as_ref().unwrap();
    assert!(matches!(gi.phase, GorillaIntroPhase::ChoiceMenu));
    assert!(!gi.sound_pending);
    assert_eq!(gi.player_names[0], "Player 1");
    assert_eq!(gi.player_names[1], "Player 2");
}
```

- [ ] **Step 2: Run the two new tests to confirm they fail**

```bash
cargo test config_submit_transitions_to_gorilla_intro config_submit_initializes_gorilla_intro_state 2>&1
```
Expected: FAIL — screen is `AppScreen::Playing`, not `AppScreen::GorillaIntro`.

- [ ] **Step 3: Replace the ConfigField::Gravity arm in config_handle_submit()**

Old:
```rust
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
```

New:
```rust
ConfigField::Gravity => {
    if !self.field_input.is_empty() {
        let grav: f32 = self.field_input.parse().unwrap_or(0.0);
        if grav <= 0.0 {
            self.field_input.clear();
            return;
        }
        self.config.gravity = grav;
    }
    self.gorilla_intro = Some(GorillaIntroState {
        phase: GorillaIntroPhase::ChoiceMenu,
        player_names: self.config.player_names.clone(),
        sound_pending: false,
    });
    self.screen = AppScreen::GorillaIntro;
}
```

- [ ] **Step 4: Remove the GorillaIntro cue from the ConfigMenu arm in handle_submit()**

Old:
```rust
AppScreen::ConfigMenu => {
    self.config_handle_submit();
    if matches!(self.screen, AppScreen::Playing) {
        vec![crate::audio::SoundCue::GorillaIntro]
    } else {
        vec![]
    }
}
```

New:
```rust
AppScreen::ConfigMenu => {
    self.config_handle_submit();
    vec![]
}
```

- [ ] **Step 5: Run the two new tests to confirm they pass**

```bash
cargo test config_submit_transitions_to_gorilla_intro config_submit_initializes_gorilla_intro_state 2>&1
```
Expected: PASS.

- [ ] **Step 6: Update the three now-broken existing tests**

**Replace `config_full_flow_through_defaults_enters_playing`** — the final submit now goes to GorillaIntro (the Playing assertion is restored in Task 3 after the skip path exists):
```rust
#[test]
fn config_full_flow_through_defaults_enters_playing() {
    let mut state = GameState::new();
    state.screen = AppScreen::ConfigMenu;

    let _ = state.handle_submit();
    assert_eq!(state.active_field, ConfigField::PlayerTwoName);

    let _ = state.handle_submit();
    assert_eq!(state.active_field, ConfigField::TargetScore);

    let _ = state.handle_submit();
    assert_eq!(state.active_field, ConfigField::Gravity);

    let _ = state.handle_submit();
    assert_eq!(state.screen, AppScreen::GorillaIntro);
}
```

**Replace `game_state_handle_submit_emits_gorilla_intro_cue_when_entering_playing`** — cue no longer fires on config submit:
```rust
#[test]
fn config_submit_emits_no_gorilla_intro_cue() {
    use crate::audio::SoundCue;
    let mut state = GameState::new();
    state.screen = AppScreen::ConfigMenu;
    let _ = state.handle_submit();
    let _ = state.handle_submit();
    let _ = state.handle_submit();
    let cues = state.handle_submit();
    assert!(
        !cues.contains(&SoundCue::GorillaIntro),
        "GorillaIntro cue must not fire on config submit, got {cues:?}"
    );
}
```

**Replace `config_to_playing_only_emits_gorilla_intro_cue`** — screen is now GorillaIntro and no cues fire:
```rust
#[test]
fn config_to_gorilla_intro_emits_no_cues() {
    let mut state = GameState::new();
    state.screen = AppScreen::ConfigMenu;
    let _ = state.handle_submit();
    let _ = state.handle_submit();
    let _ = state.handle_submit();
    let cues = state.handle_submit();
    assert_eq!(state.screen, AppScreen::GorillaIntro);
    assert!(
        !cues.contains(&crate::audio::SoundCue::Intro),
        "Intro cue must not fire here"
    );
    assert!(
        !cues.contains(&crate::audio::SoundCue::GorillaIntro),
        "GorillaIntro cue must not fire on config submit"
    );
}
```

- [ ] **Step 7: Run all tests**

```bash
cargo test 2>&1
```
Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add src/game.rs
git commit -m "feat(game): config submit transitions to GorillaIntro choice screen"
```

---

### Task 3: Non-V key / submit / backspace skip directly to Playing

**Files:**
- Modify: `src/game.rs`

- [ ] **Step 1: Add failing tests for all skip paths**

```rust
#[test]
fn gorilla_intro_p_key_goes_to_playing() {
    let mut state = advance_state_to_gorilla_intro();
    state.handle_char('p');
    assert_eq!(state.screen, AppScreen::Playing);
    assert!(state.gorilla_intro.is_none());
}

#[test]
fn gorilla_intro_any_non_v_key_goes_to_playing() {
    let mut state = advance_state_to_gorilla_intro();
    state.handle_char('x');
    assert_eq!(state.screen, AppScreen::Playing);
    assert!(state.gorilla_intro.is_none());
}

#[test]
fn gorilla_intro_submit_goes_to_playing() {
    let mut state = advance_state_to_gorilla_intro();
    let cues = state.handle_submit();
    assert_eq!(state.screen, AppScreen::Playing);
    assert!(state.gorilla_intro.is_none());
    assert!(cues.is_empty());
}

#[test]
fn gorilla_intro_backspace_goes_to_playing() {
    let mut state = advance_state_to_gorilla_intro();
    state.handle_backspace();
    assert_eq!(state.screen, AppScreen::Playing);
    assert!(state.gorilla_intro.is_none());
}

#[test]
fn gorilla_intro_skip_wires_config_to_game() {
    let mut state = GameState::new();
    state.screen = AppScreen::ConfigMenu;
    for ch in "Alice".chars() {
        state.handle_char(ch);
    }
    let _ = state.handle_submit(); // P1 -> "Alice"
    let _ = state.handle_submit(); // P2 default
    let _ = state.handle_submit(); // score default
    let _ = state.handle_submit(); // gravity -> GorillaIntro
    state.handle_char('p');        // skip to Playing
    assert_eq!(state.game.player_names[0], "Alice");
}
```

- [ ] **Step 2: Run new tests to confirm they fail**

```bash
cargo test gorilla_intro_p_key gorilla_intro_any_non_v gorilla_intro_submit_goes gorilla_intro_backspace gorilla_intro_skip_wires 2>&1
```
Expected: FAIL — stubs do nothing, screen stays GorillaIntro.

- [ ] **Step 3: Replace handle_char GorillaIntro stub with real implementation**

In `GameState::handle_char()`, replace `AppScreen::GorillaIntro => {}` with:
```rust
AppScreen::GorillaIntro => match ch.to_ascii_uppercase() {
    'V' => {
        if let Some(state) = self.gorilla_intro.as_mut() {
            state.phase = GorillaIntroPhase::Animation {
                timer: 0.0,
                phrase: 0,
                arm: GorillaArms::LeftUp,
            };
            state.sound_pending = true;
        }
    }
    _ => {
        self.gorilla_intro = None;
        self.apply_config_and_start();
    }
},
```

- [ ] **Step 4: Replace handle_submit GorillaIntro stub**

In `GameState::handle_submit()`, replace `AppScreen::GorillaIntro => vec![]` with:
```rust
AppScreen::GorillaIntro => {
    self.gorilla_intro = None;
    self.apply_config_and_start();
    vec![]
}
```

- [ ] **Step 5: Replace handle_backspace GorillaIntro stub**

In `GameState::handle_backspace()`, replace `AppScreen::GorillaIntro => {}` with:
```rust
AppScreen::GorillaIntro => {
    self.gorilla_intro = None;
    self.apply_config_and_start();
}
```

- [ ] **Step 6: Complete config_full_flow_through_defaults_enters_playing with Playing assertions**

Replace the test updated in Task 2 with the full version:
```rust
#[test]
fn config_full_flow_through_defaults_enters_playing() {
    let mut state = GameState::new();
    state.screen = AppScreen::ConfigMenu;

    let _ = state.handle_submit();
    assert_eq!(state.active_field, ConfigField::PlayerTwoName);

    let _ = state.handle_submit();
    assert_eq!(state.active_field, ConfigField::TargetScore);

    let _ = state.handle_submit();
    assert_eq!(state.active_field, ConfigField::Gravity);

    let _ = state.handle_submit();
    assert_eq!(state.screen, AppScreen::GorillaIntro);

    state.handle_char('p');
    assert_eq!(state.screen, AppScreen::Playing);
    assert_eq!(state.game.player_names[0], "Player 1");
    assert_eq!(state.game.player_names[1], "Player 2");
    assert_eq!(state.config.target_score, 3);
    assert!((state.game.gravity - 9.8).abs() < 0.001);
}
```

- [ ] **Step 7: Run all tests**

```bash
cargo test 2>&1
```
Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add src/game.rs
git commit -m "feat(game): GorillaIntro non-V key/submit/backspace skips to Playing"
```

---

### Task 4: V key transitions to Animation phase with deferred GorillaIntro cue

**Files:**
- Modify: `src/game.rs`

- [ ] **Step 1: Add failing tests for V key and deferred cue**

```rust
#[test]
fn gorilla_intro_v_key_transitions_to_animation() {
    let mut state = advance_state_to_gorilla_intro();
    state.handle_char('v');
    let gi = state.gorilla_intro.as_ref().unwrap();
    assert!(matches!(
        gi.phase,
        GorillaIntroPhase::Animation { phrase: 0, arm: GorillaArms::LeftUp, .. }
    ));
}

#[test]
fn gorilla_intro_uppercase_v_also_starts_animation() {
    let mut state = advance_state_to_gorilla_intro();
    state.handle_char('V');
    let gi = state.gorilla_intro.as_ref().unwrap();
    assert!(matches!(gi.phase, GorillaIntroPhase::Animation { .. }));
}

#[test]
fn gorilla_intro_v_key_sets_sound_pending() {
    let mut state = advance_state_to_gorilla_intro();
    state.handle_char('v');
    assert!(state.gorilla_intro.as_ref().unwrap().sound_pending);
}

#[test]
fn gorilla_intro_update_emits_gorilla_intro_cue_when_sound_pending() {
    let mut state = advance_state_to_gorilla_intro();
    state.handle_char('v');
    let cues = state.update(0.01);
    assert!(
        cues.contains(&crate::audio::SoundCue::GorillaIntro),
        "expected GorillaIntro cue from update after V press, got {cues:?}"
    );
}

#[test]
fn gorilla_intro_cue_not_emitted_twice() {
    let mut state = advance_state_to_gorilla_intro();
    state.handle_char('v');
    let _ = state.update(0.01);
    let cues = state.update(0.01);
    assert!(
        !cues.contains(&crate::audio::SoundCue::GorillaIntro),
        "GorillaIntro cue must not fire on second update"
    );
}
```

- [ ] **Step 2: Run new tests to confirm which fail**

```bash
cargo test gorilla_intro_v_key gorilla_intro_uppercase gorilla_intro_update_emits gorilla_intro_cue_not 2>&1
```
Expected: `gorilla_intro_v_key_transitions_to_animation`, `gorilla_intro_uppercase_v_also_starts_animation`, and `gorilla_intro_v_key_sets_sound_pending` pass (handle_char 'V' was already implemented in Task 3). The two `update`-based tests fail because the GorillaIntro update arm still returns `vec![]`.

- [ ] **Step 3: Replace the GorillaIntro arm in update() to drain sound_pending**

In `GameState::update()`, separate `GorillaIntro` from the no-op group and give it its own arm:

Change:
```rust
AppScreen::ConfigMenu | AppScreen::MatchOver | AppScreen::PlayAgain | AppScreen::GorillaIntro => vec![],
```
to:
```rust
AppScreen::ConfigMenu | AppScreen::MatchOver | AppScreen::PlayAgain => vec![],
AppScreen::GorillaIntro => {
    let mut cues = vec![];
    if let Some(state) = self.gorilla_intro.as_mut() {
        if state.sound_pending {
            cues.push(crate::audio::SoundCue::GorillaIntro);
            state.sound_pending = false;
        }
    }
    cues
}
```

- [ ] **Step 4: Run all tests**

```bash
cargo test 2>&1
```
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/game.rs
git commit -m "feat(game): GorillaIntro V key starts animation with deferred GorillaIntro cue"
```

---

### Task 5: Animation update — phrase advance, arm toggle, auto-advance to Playing

**Files:**
- Modify: `src/game.rs`

- [ ] **Step 1: Add a helper and failing tests for animation update**

```rust
fn advance_state_to_gorilla_intro_animation() -> GameState {
    let mut state = advance_state_to_gorilla_intro();
    state.handle_char('v');
    let _ = state.update(0.01); // drain sound_pending; timer now = 0.01
    state
}

#[test]
fn gorilla_intro_animation_starts_phrase_0_left_up() {
    let state = advance_state_to_gorilla_intro_animation();
    let gi = state.gorilla_intro.as_ref().unwrap();
    assert!(matches!(
        gi.phase,
        GorillaIntroPhase::Animation { phrase: 0, arm: GorillaArms::LeftUp, .. }
    ));
}

#[test]
fn gorilla_intro_animation_advances_phrase_after_duration() {
    let mut state = advance_state_to_gorilla_intro_animation();
    // timer is 0.01; add enough to cross one phrase boundary
    let _ = state.update(GORILLA_INTRO_PHRASE_DUR_S + 0.01);
    let gi = state.gorilla_intro.as_ref().unwrap();
    assert!(matches!(
        gi.phase,
        GorillaIntroPhase::Animation { phrase: 1, arm: GorillaArms::RightUp, .. }
    ));
}

#[test]
fn gorilla_intro_animation_toggles_arm_each_phrase() {
    let mut state = advance_state_to_gorilla_intro_animation();
    let _ = state.update(GORILLA_INTRO_PHRASE_DUR_S + 0.01); // phrase 0->1, LeftUp->RightUp
    let _ = state.update(GORILLA_INTRO_PHRASE_DUR_S);         // phrase 1->2, RightUp->LeftUp
    let gi = state.gorilla_intro.as_ref().unwrap();
    assert!(matches!(
        gi.phase,
        GorillaIntroPhase::Animation { phrase: 2, arm: GorillaArms::LeftUp, .. }
    ));
}

#[test]
fn gorilla_intro_animation_auto_advances_to_playing_after_4_phrases() {
    let mut state = advance_state_to_gorilla_intro_animation();
    // One large dt drives all 4 phrase boundaries at once
    let _ = state.update(GORILLA_INTRO_PHRASE_DUR_S * 4.0 + 0.01);
    assert_eq!(state.screen, AppScreen::Playing);
    assert!(state.gorilla_intro.is_none());
}
```

- [ ] **Step 2: Run new tests to confirm which fail**

```bash
cargo test gorilla_intro_animation 2>&1
```
Expected: `gorilla_intro_animation_starts_phrase_0_left_up` passes; the remaining three fail (update does not advance the timer).

- [ ] **Step 3: Expand the GorillaIntro arm in update() to advance animation**

Replace the current `AppScreen::GorillaIntro` arm in `GameState::update()` with:
```rust
AppScreen::GorillaIntro => {
    let mut cues = vec![];
    let mut advance_to_playing = false;

    if let Some(state) = self.gorilla_intro.as_mut() {
        if state.sound_pending {
            cues.push(crate::audio::SoundCue::GorillaIntro);
            state.sound_pending = false;
        }
        if let GorillaIntroPhase::Animation { timer, phrase, arm } = &mut state.phase {
            *timer += dt;
            while *timer >= GORILLA_INTRO_PHRASE_DUR_S && *phrase < 4 {
                *timer -= GORILLA_INTRO_PHRASE_DUR_S;
                *phrase += 1;
                *arm = if *arm == GorillaArms::LeftUp {
                    GorillaArms::RightUp
                } else {
                    GorillaArms::LeftUp
                };
            }
            if *phrase >= 4 {
                advance_to_playing = true;
            }
        }
    }

    if advance_to_playing {
        self.gorilla_intro = None;
        self.apply_config_and_start();
    }

    cues
}
```

- [ ] **Step 4: Run all tests**

```bash
cargo test 2>&1
```
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/game.rs
git commit -m "feat(game): GorillaIntro animation advances phrases, toggles arms, auto-advances to Playing"
```

---

### Task 6: Rendering — choice menu and animation

**Files:**
- Modify: `src/game.rs`

- [ ] **Step 1: Add failing render tests**

```rust
#[test]
fn gorilla_intro_choice_menu_frame_has_vertices() {
    let state = advance_state_to_gorilla_intro();
    let frame = state.frame();
    assert!(
        !frame.vertices.is_empty(),
        "GorillaIntro choice menu should render vertices"
    );
}

#[test]
fn gorilla_intro_animation_frame_has_vertices() {
    let mut state = advance_state_to_gorilla_intro();
    state.handle_char('v');
    let frame = state.frame();
    assert!(
        !frame.vertices.is_empty(),
        "GorillaIntro animation should render vertices"
    );
}
```

- [ ] **Step 2: Run new tests to confirm they fail**

```bash
cargo test gorilla_intro_choice_menu_frame gorilla_intro_animation_frame 2>&1
```
Expected: FAIL — frame() stub returns empty vertices.

- [ ] **Step 3: Add render_gorilla_intro_screen() method on GameState**

Place this after `render_intro_screen()`:
```rust
fn render_gorilla_intro_screen(&self, state: &GorillaIntroState, canvas: &mut Canvas) {
    match &state.phase {
        GorillaIntroPhase::ChoiceMenu => {
            draw_text(canvas, 10, centered_col("V = View Intro"), "V = View Intro", HUD_TEXT);
            draw_text(canvas, 12, centered_col("P = Play Game"), "P = Play Game", HUD_TEXT);
            draw_text(canvas, 14, centered_col("Your Choice?"), "Your Choice?", HUD_TEXT);
        }
        GorillaIntroPhase::Animation { arm, .. } => {
            let title = "Q B A S I C   G O R I L L A S";
            draw_text(canvas, 3, centered_col(title), title, HUD_TEXT);
            draw_text(canvas, 5, centered_col("STARRING:"), "STARRING:", HUD_TEXT);
            let starring = format!("{}  AND  {}", state.player_names[0], state.player_names[1]);
            draw_text(canvas, 6, centered_col(&starring), &starring, HUD_TEXT);
            draw_gorilla(canvas, Gorilla { x: GORILLA_INTRO_X1, y: GORILLA_INTRO_Y }, *arm);
            draw_gorilla(canvas, Gorilla { x: GORILLA_INTRO_X2, y: GORILLA_INTRO_Y }, *arm);
        }
    }
}
```

- [ ] **Step 4: Wire render_gorilla_intro_screen into frame()**

In `GameState::frame()`, replace the stub `AppScreen::GorillaIntro` arm with:
```rust
AppScreen::GorillaIntro => {
    let mut canvas = Canvas::new(LOGICAL_WIDTH, LOGICAL_HEIGHT);
    let state = self.gorilla_intro.as_ref().unwrap();
    self.render_gorilla_intro_screen(state, &mut canvas);
    Frame {
        logical_width: LOGICAL_WIDTH,
        logical_height: LOGICAL_HEIGHT,
        clear_color: BACKGROUND,
        vertices: canvas.into_vertices(),
    }
}
```

- [ ] **Step 5: Run all tests**

```bash
cargo test 2>&1
```
Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add src/game.rs
git commit -m "feat(game): render GorillaIntro choice menu and animation with gorillas"
```

---

### Task 7: Visual smoke test and gorilla Y-position adjustment

**Files:**
- Modify: `src/game.rs` (only if Y position needs adjustment)

- [ ] **Step 1: Build and run**

```bash
cargo run 2>&1
```

- [ ] **Step 2: Test the choice screen**

Navigate: Intro (any key) -> ConfigMenu (Enter x 4) -> GorillaIntro choice screen.

Verify "V = View Intro", "P = Play Game", "Your Choice?" are readable and vertically centered on the black screen.

- [ ] **Step 3: Test the skip path**

Press 'P'. Verify gameplay starts with no music.

- [ ] **Step 4: Test the V path**

Restart, navigate to GorillaIntro, press 'V'.

Verify:
- Title "Q B A S I C   G O R I L L A S" visible near top
- "STARRING: Player 1  AND  Player 2" visible below title
- Both gorillas visible, arm pose switches every ~2.94s
- After ~11.8s (4 phrases), gameplay starts automatically

- [ ] **Step 5: Adjust GORILLA_INTRO_Y if gorillas are poorly placed**

If gorillas are cut off or float too high, edit the constant near the top of `src/game.rs`:
```rust
const GORILLA_INTRO_Y: f32 = 290.0; // increase to move down, decrease to move up
```
Re-run until the gorillas look correct (feet near bottom third of screen).

- [ ] **Step 6: Commit only if changes were made**

```bash
git add src/game.rs
git commit -m "fix(game): adjust GorillaIntro gorilla Y position for visual correctness"
```
