# GorillaIntro Screen Design

**Date:** 2026-05-19
**Issue:** #18 - Implement GorillaIntro screen (V = intro animation, P = play)

## Overview

Insert a new `AppScreen::GorillaIntro` between `ConfigMenu` and `Playing`. It presents a
choice prompt ("V = View Intro" / "P = Play Game") and optionally plays the gorilla animation
with the GorillaIntro music before starting gameplay.

## Screen Flow

```
Intro → ConfigMenu → GorillaIntro → Playing
```

- `config_handle_submit` (Gravity confirmed): set `AppScreen::GorillaIntro`, initialize
  `gorilla_intro: Some(GorillaIntroState::new(player_names))`. No sound cue yet.
- `GorillaIntro` `handle_char('v'/'V')`: transition to `Animation` phase, return
  `vec![SoundCue::GorillaIntro]`.
- `GorillaIntro` any other key / `handle_submit` / `handle_backspace`: skip directly to
  `Playing` via `apply_config_and_start()`. No music.

## State Structure

```rust
enum GorillaIntroPhase {
    ChoiceMenu,
    Animation { timer: f32, phrase: u8, arm: GorillaArms },
}

struct GorillaIntroState {
    phase: GorillaIntroPhase,
    player_names: [String; 2],
}
```

`player_names` is copied from config at construction for use in the "STARRING:" line.

`Animation` fields:
- `timer`: accumulates `dt`; triggers arm toggle and phrase advance at each phrase boundary
- `phrase`: 0-3; reaching 4 triggers auto-advance to `Playing`
- `arm`: starts `LeftUp`; toggles to `RightUp` each phrase (both gorillas use same pose)

Gorilla positions are fixed constants (not from game round):
```rust
const GORILLA_INTRO_X1: f32 = 265.0;
const GORILLA_INTRO_X2: f32 = 325.0;
const GORILLA_INTRO_Y: f32 = 300.0; // exact value to be confirmed visually
```

## Rendering

### ChoiceMenu Phase

Black screen, three centered text lines:

```
row 10:  "V = View Intro"
row 12:  "P = Play Game"
row 14:  "Your Choice?"
```

(Row numbers to be confirmed against original BASIC output; vertical center of 25-row grid.)

### Animation Phase

Black screen with:

```
row 3:  "Q B A S I C   G O R I L L A S"   (centered)
row 5:  "STARRING:"                         (centered)
row 6:  "<name1>  AND  <name2>"             (centered)
```

Both gorillas drawn at `(GORILLA_INTRO_X1, GORILLA_INTRO_Y)` and
`(GORILLA_INTRO_X2, GORILLA_INTRO_Y)` using the current `arm` pose. Both gorillas share the
same arm pose and switch together each phrase.

Rendering via new method:
`fn render_gorilla_intro_screen(&self, state: &GorillaIntroState, canvas: &mut Canvas)`

## Update Logic

`GameState::update` for `AppScreen::GorillaIntro`:

- `ChoiceMenu`: no timer work, return `vec![]`
- `Animation`:
  - `timer += dt`
  - When `timer >= GORILLA_INTRO_PHRASE_DUR_S` (~2.9444s):
    - `phrase += 1`
    - `timer -= GORILLA_INTRO_PHRASE_DUR_S`
    - Toggle arm: `LeftUp` -> `RightUp` -> `LeftUp` -> ...
  - When `phrase >= 4`: clear `gorilla_intro`, call `apply_config_and_start()`
  - Return `vec![]` (sound already emitted on V keypress)

`GORILLA_INTRO_PHRASE_DUR_S` derived from existing audio constant:
`GORILLA_INTRO_PHRASE_DUR_US / 1_000_000` as `f32` = `2.944444`.

## Integration Points

| Site | Change |
|------|--------|
| `AppScreen` enum | Add `GorillaIntro` variant |
| `GameState` | Add `gorilla_intro: Option<GorillaIntroState>` field |
| `config_handle_submit` (Gravity arm) | Set `GorillaIntro` instead of `Playing` |
| `apply_config_and_start` | Unchanged; called from skip path and animation-complete path |
| `update` | Add `GorillaIntro` arm |
| `frame` | Add `GorillaIntro` arm |
| `handle_char` | Add `GorillaIntro` arm |
| `handle_submit` | Add `GorillaIntro` arm |
| `handle_backspace` | Add `GorillaIntro` arm |
| `reset_to_config` | Clear `gorilla_intro` |

## Out of Scope

- Background/cityscape behind gorillas during animation (original BASIC shows black)
- Any sound other than `SoundCue::GorillaIntro` on the V path
- Gorilla arm pre-rendering to sprite arrays (not needed in our draw-on-demand model)
