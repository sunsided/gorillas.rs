# Play-Again Prompt Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** After GAME OVER is dismissed, show "Would you like to play again?" and exit on `n` or return to ConfigMenu on `y`.

**Architecture:** Add `AppScreen::PlayAgain` variant and `GameState::exit_requested` flag. `MatchOver` any-key transitions to `PlayAgain` instead of resetting. `app.rs` calls `event_loop.exit()` when the flag is set.

**Tech Stack:** Rust, winit, existing `draw_text`/`Canvas` render helpers in `src/game.rs`.

---

### Task 1: Add `PlayAgain` variant and `exit_requested` field

**Files:**
- Modify: `src/game.rs:542-545` (`AppScreen` enum)
- Modify: `src/game.rs:567-574` (`GameState` struct + `new()`)
- Modify: `src/game.rs:588-602` (`update()`)
- Modify: `src/game.rs:605-648` (`frame()` - stub arm only)

- [ ] **Step 1: Add `PlayAgain` to `AppScreen`**

In `src/game.rs`, change:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppScreen {
    ConfigMenu,
    Playing,
    MatchOver,
}
```
to:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppScreen {
    ConfigMenu,
    Playing,
    MatchOver,
    PlayAgain,
}
```

- [ ] **Step 2: Add `exit_requested` to `GameState` and initialise it**

In `src/game.rs`, change:
```rust
pub struct GameState {
    pub screen: AppScreen,
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
            config: MatchConfig::default(),
            active_field: ConfigField::PlayerOneName,
            field_input: String::new(),
            game: Game::new(),
            match_over_state: None,
        }
    }
```
to:
```rust
pub struct GameState {
    pub screen: AppScreen,
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
```

- [ ] **Step 3: Add `PlayAgain` arm to `update()`**

In `src/game.rs`, change:
```rust
AppScreen::ConfigMenu | AppScreen::MatchOver => vec![],
```
to:
```rust
AppScreen::ConfigMenu | AppScreen::MatchOver | AppScreen::PlayAgain => vec![],
```

- [ ] **Step 4: Add stub `PlayAgain` arm to `frame()`**

In `src/game.rs`, inside the `frame()` match, after the `AppScreen::MatchOver` arm add a stub that returns an empty frame (we render it properly in Task 5):

Change:
```rust
        }
    }

    fn render_config_screen(&self, canvas: &mut Canvas) {
```
(i.e., the closing braces of `frame()`) to:

```rust
            AppScreen::PlayAgain => Frame {
                logical_width: LOGICAL_WIDTH,
                logical_height: LOGICAL_HEIGHT,
                clear_color: BACKGROUND,
                vertices: vec![],
            },
        }
    }

    fn render_config_screen(&self, canvas: &mut Canvas) {
```

- [ ] **Step 5: Add stub `PlayAgain` arms to `handle_char`, `handle_backspace`, `handle_submit`**

`handle_char` - add arm (ignores all input for now):
```rust
AppScreen::PlayAgain => {}
```

`handle_backspace` - add arm:
```rust
AppScreen::PlayAgain => {}
```

`handle_submit` - add arm:
```rust
AppScreen::PlayAgain => vec![]
```

- [ ] **Step 6: Verify compilation**

```bash
cargo check --all-features --all-targets
```
Expected: no errors.

---

### Task 2: `MatchOver` transitions to `PlayAgain` on any key

**Files:**
- Modify: `src/game.rs:718-755` (`handle_char`, `handle_backspace`, `handle_submit`)
- Modify: `src/game.rs` (existing MatchOver tests - update expectations)

- [ ] **Step 1: Update the three existing MatchOver tests**

Find and replace the three tests that currently assert `AppScreen::ConfigMenu` after MatchOver key events. They now expect `AppScreen::PlayAgain` and no `match_over_state`. Replace all three:

```rust
#[test]
fn match_over_any_char_transitions_to_play_again() {
    let mut state = GameState::new();
    state.screen = AppScreen::MatchOver;
    state.match_over_state = Some(MatchOverState {
        scores: [3, 1],
        names: [String::from("Alice"), String::from("Bob")],
    });

    state.handle_char('x');

    assert_eq!(state.screen, AppScreen::PlayAgain);
    assert!(state.match_over_state.is_none());
}

#[test]
fn match_over_enter_transitions_to_play_again() {
    let mut state = GameState::new();
    state.screen = AppScreen::MatchOver;
    state.match_over_state = Some(MatchOverState {
        scores: [3, 1],
        names: [String::from("Alice"), String::from("Bob")],
    });

    let _ = state.handle_submit();

    assert_eq!(state.screen, AppScreen::PlayAgain);
    assert!(state.match_over_state.is_none());
}

#[test]
fn match_over_backspace_transitions_to_play_again() {
    let mut state = GameState::new();
    state.screen = AppScreen::MatchOver;
    state.match_over_state = Some(MatchOverState {
        scores: [3, 1],
        names: [String::from("Alice"), String::from("Bob")],
    });

    state.handle_backspace();

    assert_eq!(state.screen, AppScreen::PlayAgain);
    assert!(state.match_over_state.is_none());
}
```

- [ ] **Step 2: Run tests and confirm the three updated tests fail**

```bash
cargo test --all-features match_over
```
Expected: the three renamed tests FAIL (still going to `ConfigMenu`).

- [ ] **Step 3: Change `MatchOver` arms in `handle_char`, `handle_backspace`, `handle_submit`**

`handle_char`:
```rust
AppScreen::MatchOver => self.reset_to_config(),
```
becomes:
```rust
AppScreen::MatchOver => {
    self.match_over_state = None;
    self.screen = AppScreen::PlayAgain;
}
```

`handle_backspace`:
```rust
AppScreen::MatchOver => self.reset_to_config(),
```
becomes:
```rust
AppScreen::MatchOver => {
    self.match_over_state = None;
    self.screen = AppScreen::PlayAgain;
}
```

`handle_submit`:
```rust
AppScreen::MatchOver => {
    self.reset_to_config();
    vec![]
}
```
becomes:
```rust
AppScreen::MatchOver => {
    self.match_over_state = None;
    self.screen = AppScreen::PlayAgain;
    vec![]
}
```

- [ ] **Step 4: Run tests and confirm they pass**

```bash
cargo test --all-features match_over
```
Expected: all three tests PASS.

- [ ] **Step 5: Run full test suite to confirm no regressions**

```bash
cargo test --all-features
```
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/game.rs
git commit -m "feat(game): MatchOver any-key transitions to PlayAgain"
```

---

### Task 3: `PlayAgain` `y`/`Y` returns to ConfigMenu

**Files:**
- Modify: `src/game.rs` (`handle_char` `PlayAgain` arm + new tests)

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `src/game.rs`:

```rust
#[test]
fn play_again_y_resets_to_config_menu() {
    let mut state = GameState::new();
    state.screen = AppScreen::PlayAgain;

    state.handle_char('y');

    assert_eq!(state.screen, AppScreen::ConfigMenu);
    assert_eq!(state.active_field, ConfigField::PlayerOneName);
    assert!(state.field_input.is_empty());
}

#[test]
fn play_again_upper_y_resets_to_config_menu() {
    let mut state = GameState::new();
    state.screen = AppScreen::PlayAgain;

    state.handle_char('Y');

    assert_eq!(state.screen, AppScreen::ConfigMenu);
}
```

- [ ] **Step 2: Run to confirm tests fail**

```bash
cargo test --all-features play_again_y
```
Expected: both tests FAIL (PlayAgain arm currently ignores all input).

- [ ] **Step 3: Implement `y`/`Y` in the `PlayAgain` arm of `handle_char`**

Change:
```rust
AppScreen::PlayAgain => {}
```
to:
```rust
AppScreen::PlayAgain => match ch {
    'y' | 'Y' => self.reset_to_config(),
    _ => {}
},
```

- [ ] **Step 4: Run tests and confirm they pass**

```bash
cargo test --all-features play_again_y
```
Expected: both tests PASS.

- [ ] **Step 5: Run full suite**

```bash
cargo test --all-features
```
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/game.rs
git commit -m "feat(game): PlayAgain y/Y resets to ConfigMenu"
```

---

### Task 4: `PlayAgain` `n`/`N` sets `exit_requested`

**Files:**
- Modify: `src/game.rs` (`handle_char` `PlayAgain` arm + new tests)

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn play_again_n_sets_exit_requested() {
    let mut state = GameState::new();
    state.screen = AppScreen::PlayAgain;

    state.handle_char('n');

    assert!(state.exit_requested);
}

#[test]
fn play_again_upper_n_sets_exit_requested() {
    let mut state = GameState::new();
    state.screen = AppScreen::PlayAgain;

    state.handle_char('N');

    assert!(state.exit_requested);
}
```

- [ ] **Step 2: Run to confirm tests fail**

```bash
cargo test --all-features play_again_n
```
Expected: both tests FAIL (`exit_requested` stays `false`).

- [ ] **Step 3: Add `n`/`N` to the `PlayAgain` arm**

Change:
```rust
AppScreen::PlayAgain => match ch {
    'y' | 'Y' => self.reset_to_config(),
    _ => {}
},
```
to:
```rust
AppScreen::PlayAgain => match ch {
    'y' | 'Y' => self.reset_to_config(),
    'n' | 'N' => self.exit_requested = true,
    _ => {}
},
```

- [ ] **Step 4: Run tests and confirm they pass**

```bash
cargo test --all-features play_again_n
```
Expected: both tests PASS.

- [ ] **Step 5: Add test that other keys are ignored**

```rust
#[test]
fn play_again_other_keys_ignored() {
    let mut state = GameState::new();
    state.screen = AppScreen::PlayAgain;

    state.handle_char('x');

    assert_eq!(state.screen, AppScreen::PlayAgain);
    assert!(!state.exit_requested);
}
```

- [ ] **Step 6: Run to confirm it passes**

```bash
cargo test --all-features play_again_other_keys_ignored
```
Expected: PASS.

- [ ] **Step 7: Run full suite**

```bash
cargo test --all-features
```
Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/game.rs
git commit -m "feat(game): PlayAgain n/N sets exit_requested flag"
```

---

### Task 5: `frame()` renders `PlayAgain` screen

**Files:**
- Modify: `src/game.rs` (`frame()` `PlayAgain` arm + new test)

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn play_again_frame_has_vertices() {
    let mut state = GameState::new();
    state.screen = AppScreen::PlayAgain;

    let frame = state.frame();

    assert!(
        !frame.vertices.is_empty(),
        "PlayAgain frame should render text vertices"
    );
}
```

- [ ] **Step 2: Run to confirm test fails**

```bash
cargo test --all-features play_again_frame_has_vertices
```
Expected: FAIL (stub returns empty `vertices`).

- [ ] **Step 3: Implement `PlayAgain` rendering**

Replace the stub `PlayAgain` arm in `frame()`:
```rust
AppScreen::PlayAgain => Frame {
    logical_width: LOGICAL_WIDTH,
    logical_height: LOGICAL_HEIGHT,
    clear_color: BACKGROUND,
    vertices: vec![],
},
```
with:
```rust
AppScreen::PlayAgain => {
    let mut canvas = Canvas::new(LOGICAL_WIDTH, LOGICAL_HEIGHT);
    draw_text(
        &mut canvas,
        11,
        24,
        "Would you like to play again?",
        HUD_TEXT,
    );
    Frame {
        logical_width: LOGICAL_WIDTH,
        logical_height: LOGICAL_HEIGHT,
        clear_color: BACKGROUND,
        vertices: canvas.into_vertices(),
    }
}
```

Row 11, col 24 matches BASIC `LOCATE 11, 24`.

- [ ] **Step 4: Run test and confirm it passes**

```bash
cargo test --all-features play_again_frame_has_vertices
```
Expected: PASS.

- [ ] **Step 5: Run full suite**

```bash
cargo test --all-features
```
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/game.rs
git commit -m "feat(game): render PlayAgain screen with play-again prompt"
```

---

### Task 6: `app.rs` exits when `exit_requested` is set

**Files:**
- Modify: `src/app.rs:122-137` (`window_event` keyboard handler)

No unit test is possible here - the check hooks into winit's `ActiveEventLoop`, which requires a live event loop. Verified manually after the change.

- [ ] **Step 1: Add the exit check inside the `KeyboardInput` arm**

In `src/app.rs`, locate this block (around line 122):
```rust
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.logical_key {
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    Key::Named(NamedKey::Backspace) => self.game.handle_backspace(),
                    Key::Named(NamedKey::Enter) => {
                        let cues = self.game.handle_submit();
                        dispatch_cues(&mut self.audio, cues);
                    }
                    Key::Character(ref text) => {
                        for ch in text.chars() {
                            self.game.handle_char(ch);
                        }
                    }
                    _ => {}
                }
            }
```

Change to:
```rust
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.logical_key {
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    Key::Named(NamedKey::Backspace) => self.game.handle_backspace(),
                    Key::Named(NamedKey::Enter) => {
                        let cues = self.game.handle_submit();
                        dispatch_cues(&mut self.audio, cues);
                    }
                    Key::Character(ref text) => {
                        for ch in text.chars() {
                            self.game.handle_char(ch);
                        }
                    }
                    _ => {}
                }
                if self.game.exit_requested {
                    event_loop.exit();
                }
            }
```

- [ ] **Step 2: Build**

```bash
cargo build
```
Expected: compiles with no errors.

- [ ] **Step 3: Run full test suite**

```bash
cargo test --all-features
```
Expected: all tests pass.

- [ ] **Step 4: Smoke test manually**

Run the game, play a round to completion, observe:
1. GAME OVER screen shows with scores.
2. Any key transitions to the "Would you like to play again?" screen.
3. `y` returns to the config menu.
4. Play again to completion, then press `n` - window closes.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat(app): exit process when GameState signals exit_requested"
```
