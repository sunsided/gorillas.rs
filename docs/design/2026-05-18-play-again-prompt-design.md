# Play-Again Prompt After Match Ends

**Issue:** #16
**Date:** 2026-05-18

## Summary

After the GAME OVER screen is dismissed, display "Would you like to play again (Y or N)?" and wait for `y`/`n`. `y` returns to `ConfigMenu` (resets config, faithful to BASIC `GOTO spam`). `n` exits the process.

## BASIC Reference

```basic
spam:
  GetInputs Name1$, Name2$, NumGames
  GorillaIntro Name1$, Name2$
  PlayGame Name1$, Name2$, NumGames

LOCATE 11, 24
PRINT "Would you like to play again?"
DO
  again$ = INKEY$
LOOP UNTIL (again$ = "y") OR (again$ = "n")
IF again$ = "y" THEN GOTO spam
```

## State Machine Changes (`game.rs`)

Add `PlayAgain` variant to `AppScreen`:

```rust
pub enum AppScreen {
    ConfigMenu,
    Playing,
    MatchOver,
    PlayAgain,
}
```

Add exit flag to `GameState`:

```rust
pub struct GameState {
    pub exit_requested: bool,
    // ... existing fields
}
```

### Transitions

- `MatchOver` + any key -> `PlayAgain` (replaces current `reset_to_config()`)
- `PlayAgain` + `y`/`Y` -> `reset_to_config()` -> `ConfigMenu`
- `PlayAgain` + `n`/`N` -> sets `exit_requested = true`
- `PlayAgain` + any other key -> ignored

`match_over_state` is not needed once in `PlayAgain`; it can be cleared on transition.

## Rendering (`game.rs` `frame()`)

`PlayAgain` renders a single line centered at row 11, matching BASIC `LOCATE 11, 24`:

```
Would you like to play again (Y or N)?
```

No scores or other content on this screen.

## Exit Signal (`app.rs`)

After each key dispatch in `window_event`, check:

```rust
if self.game.exit_requested {
    event_loop.exit();
}
```

## `update()` Behavior

`PlayAgain` returns `vec![]` - no audio cues, no animation. Same as `MatchOver`.

## Tests

- `PlayAgain` is reached from `MatchOver` via any char/backspace/submit
- `y` and `Y` transition to `ConfigMenu` with reset config
- `n` and `N` set `exit_requested = true`
- Other keys in `PlayAgain` are ignored (no state change)
