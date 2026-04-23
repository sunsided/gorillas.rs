# Port Plan

This plan aligns `GOAL.md` with the current Rust implementation. The target is maximum practical fidelity to QBasic `GORILLAS.BAS`, while keeping the modern `winit`/`wgpu` implementation model.

## Current Implementation Snapshot

- [x] `winit` application shell exists in `src/app.rs`.
- [x] `wgpu` renderer exists in `src/render/mod.rs`.
- [x] Static scene drawing uses a fixed-size CPU raster canvas, then uploads contiguous pixel spans as GPU rectangles.
  - Done: BASIC-visible drawing primitives now live on the logical `640 x 350` canvas, so rendered pixels can also become the future collision mask.
  - Remaining: tune individual raster helpers where QBasic `LINE`, `CIRCLE`, arc, `PSET`, or `POINT` behavior needs closer parity.
- [x] Static scene frame is generated in `Game::frame`.
- [x] EGA-mode logical coordinate space is currently `640 x 350`.
  - Fidelity caveat: this is not yet a fixed low-resolution framebuffer or aspect-correct presentation path.
- [x] City skyline data is generated and rendered as a rough scene.
  - Fidelity caveat: generation is private, hard-seeded, missing wind, and not yet modeled as round data.
- [x] Gorillas are drawn procedurally with arm poses as a rough placeholder.
  - Fidelity caveat: this is not yet pixel-faithful to the BASIC `LINE`/`CIRCLE`/`PSET` drawing and `GET` sprite capture behavior.
- [x] Sun is drawn procedurally with a happy/shocked parameter in the drawing function.
  - Fidelity caveat: there is no sun state or gameplay path that can show the shocked face yet.
- [ ] There is no game state machine yet.
- [ ] There is no intro, config menu, gorilla intro, match-over screen, or play-again loop.
- [ ] There is no text rendering.
- [ ] There is no keyboard text/numeric input flow.
- [ ] There is no banana projectile.
- [ ] There is no wind value or wind arrow.
- [ ] There is no turn handling, scoring, hit detection, explosions, or victory dance.
- [ ] There is no audio.

## Fidelity Rules

- [ ] Prefer the original shape, color, coordinate, and timing behavior whenever it can be reproduced cleanly.
- [ ] Use `GORILLAS.BAS` as the behavior authority when `GOAL.md` is broad or ambiguous.
- [ ] Use `reference.png` for visual comparison after every rendering milestone.
- [ ] Keep deviations explicit in this file or near the code they affect.
- [ ] Avoid visual upgrades that make the game less like the original.

## Known Deviations To Fix

- [x] City generation used to rely on a hard-coded `SmallRng::seed_from_u64(1991)` skyline.
  - Original: `RANDOMIZE (TIMER)` creates a new city each round.
  - Done: generation is seedable for tests, while gameplay uses a fresh random seed.
- [x] Gorilla placement used to select the tallest building in broad left/right screen ranges.
  - Original: first gorilla is placed on the second or third building from the left; second gorilla is placed on the second or third building from the right.
  - Done: original building-index placement is implemented with `XAdj = 14`, `YAdj = 30` for EGA mode.
- [ ] Wind physics is absent.
  - Original: wind is generated per round, affects horizontal acceleration, and is shown as a bottom arrow.
  - Partial: wind is now generated per round and rendered as a bottom arrow at `ScrHeight - 5`; projectile physics is still pending.
- [x] Palette/color mapping is corrected for the current static scene.
  - Original EGA palette is configured in `SetScreen` with `PALETTE 0,1`, `1,46`, `2,44`, `3,54`, `5,7`, `6,4`, `7,3`, `9,63`.
  - Done: standard 4-bit EGA attributes are now separate from BASIC `PALETTE` remaps, so unmapped attributes like `WINDOWCOLOR = 14` resolve to the default EGA yellow instead of being misread as raw DAC values.
  - Done: the renderer now treats frame colors as sRGB palette bytes and linearizes them before writing to an sRGB surface, so DOS/EGA-like bytes survive presentation and screenshot capture.
  - Screenshot comparison: `target/screenshots/first-frame.ppm` is `640 x 350`; `reference.png` is `640 x 343`.
  - Screenshot comparison: the 7-pixel height difference in `reference.png` can be ignored for now; use it for palette, layout, and shape comparison.
  - Current screenshot colors now include canonical bytes such as `#0000AA`, `#00AAAA`, `#AAAAAA`, `#AA0000`, `#FFFF55`, and `#555555`.
- [x] Window scaling preserves the logical scene aspect ratio.
  - Original: fixed screen mode geometry.
  - Done: renderer now centers the logical frame in an aspect-correct viewport on resize.
  - Remaining: decide whether to render through a fixed logical framebuffer for closer pixel behavior.
- [x] Current sun drawing is close and verified against the original primitive coordinates.
  - Original: sun uses exact `DoSun` rays, happy arc, shocked mouth, and eye pixels.
  - Done: arc orientation and pixel placement were audited against `GORILLAS.BAS` and `reference.png`.
  - Remaining: connect shocked support to `SunHit` state.
- [x] Current gorilla drawing is a practical BASIC primitive translation.
  - Original: gorilla sprites are captured with `GET` after drawing with `LINE`, `CIRCLE`, `PSET`, and `Scl`.
  - Done: procedural primitive output for the three arm poses has been tuned to match the original closely enough for the current GPU primitive path.
- [x] Building colors used to be hand-picked approximations.
  - Original: building attributes are `FnRan(3) + 4`, remapped through the EGA palette.
  - Done: building colors now use the palette mapping rather than independent RGBA constants.
- [ ] Skyline slope behavior may already deviate from the literal BASIC source.
  - Original source: slope `6` is initialized as inverted-V, but the generation loop contains `CASE 3 TO 5` followed by an unreachable `CASE 4`.
  - Current Rust: slope `6` behaves as an inverted V, matching the comment's apparent intent rather than the literal unreachable BASIC branch.
  - Plan: compare against actual QBasic behavior if possible; otherwise document this as an intentional bug-for-intent choice.
- [ ] Collision will not use the `wgpu` framebuffer for QBasic `POINT`.
  - Original: projectile collision samples screen pixels.
  - Reason for deviation: `wgpu` framebuffers are not an appropriate gameplay collision source; the CPU raster canvas is the chosen replacement surface.
  - Fidelity risk: `PlotShot` does not collide a simple point or bounding box; it samples a short player-dependent banana probe through screen colors and treats `SUNATTR`, `OBJECTCOLOR`, and other non-background pixels differently.
  - Plan: use explicit geometry or a CPU collision mask for buildings, gorillas, sun, and bounds while matching the visible `POINT` probe behavior.
- [ ] Timing will not use `Rest`, `CalcDelay`, or busy loops.
  - Original: speed depends on calibrated busy waits.
  - Reason for deviation: modern apps should use frame delta time and event-loop redraws.
  - Current Rust has no `dt`, update method, or animation scheduler; it only redraws the static frame.
  - Plan: add explicit frame timing and animation state, then tune frame-based animation to match the apparent original pacing.
- [ ] Input will not block like `INKEY$`, `INPUT`, or `LINE INPUT`.
  - Original: QBasic blocks while reading text and numeric input.
  - Reason for deviation: `winit` is event-driven.
  - Plan: model equivalent prompts and validation with explicit app/turn states.
- [ ] NumLock `PEEK`/`POKE` behavior will not be ported.
  - Original: toggles/restores keyboard flags.
  - Reason for deviation: obsolete machine-specific behavior.

## Model And State Work

- [ ] Introduce app-level screens matching the original flow:
  - [ ] `Intro`
  - [ ] `ConfigMenu`
  - [ ] `GorillaIntro`
  - [ ] `Playing`
  - [ ] `MatchOver`
  - [ ] `PlayAgain`
- [ ] Introduce match configuration:
  - [ ] Player 1 name, default `Player 1`, max 10 chars.
  - [ ] Player 2 name, default `Player 2`, max 10 chars.
  - [ ] Number of rounds/games, default `3`.
    - BASIC labels this prompt as total points, but `PlayGame` runs exactly `FOR i = 1 TO NumGames`; it does not stop when a player reaches the value.
    - If this port intentionally changes to "play until target score", document that as a gameplay deviation.
  - [ ] Gravity, default `9.8`.
- [ ] Introduce match/round state:
  - [ ] Buildings and windows.
  - [ ] Wind.
  - [ ] Gorillas with positions, pose, and alive state.
  - [ ] Scores.
  - [ ] Current player.
  - [ ] Sun face and sun-hit state.
  - [ ] Projectile, explosion, and victory animation state.
- [ ] Introduce turn phases:
  - [ ] Enter angle.
  - [ ] Enter velocity.
  - [ ] Throw pose/sound delay.
  - [ ] Projectile flying.
  - [ ] Explosion or gorilla explosion.
  - [ ] Victory dance.
  - [ ] Round reset or match over.

## Rendering Work

- [x] Preserve fixed logical coordinates and add aspect-correct presentation.
- [ ] Add text rendering.
  - [ ] Prefer a bitmap or DOS-like font approach that matches the original terminal/game text.
  - [ ] Implement row/column positioning equivalent to `LOCATE` and `Center`.
  - [ ] Preserve 80-column text-screen layout where the BASIC switches back to `SCREEN 0`.
  - [ ] Render intro/config/game-over text using original strings and approximate rows.
- [x] Centralize palette.
  - [x] Represent original EGA attributes.
  - [x] Represent `PALETTE` remapping from BASIC attributes to DAC values.
  - [x] Do not treat unmapped attributes such as `14` as raw DAC values; `WINDOWCOLOR = 14` should resolve to standard EGA yellow `#FFFF55`.
  - [x] Fix rendered RGB levels so reference colors like `#0000AA`, `#00AAAA`, and `#AA0000` survive the wgpu surface path.
  - [x] Replace ad hoc `OBJECT`, `BACKGROUND`, `SUN`, `LIT_WINDOW`, and `DARK_WINDOW` color constants with palette-driven values.
- [x] Decide the fidelity strategy for BASIC raster primitives.
  - [x] Use a low-resolution CPU raster canvas for gameplay-visible sprites and future collision-mask reads, with `wgpu` limited to presenting pixel spans.
  - [ ] Verify inclusive `LINE` rectangle endpoints, circle arcs, filled circles, and one-pixel `PSET` behavior before treating visual primitives as parity.
- [ ] Finish original scene primitives.
  - [x] Verify sun happy/shocked mouth orientation.
  - [x] Verify gorilla arm pose shapes and pixel scale.
  - [x] Add banana drawing with four orientation frames.
  - [x] Add wind arrow.
  - [ ] Add building and gorilla explosion animations.
- [ ] Render HUD exactly enough for gameplay:
  - [ ] Player names at top left/top right.
  - [ ] Angle and velocity prompts at original columns.
  - [ ] Centered score line near row 23.
  - [ ] Game-over score table.

## Gameplay Work

- [ ] City generation:
  - [x] Expose pure, seedable generation APIs before adding tests; current generation is private and hard-wired to one seed.
  - [x] Preserve original EGA constants: bottom line `335`, height increment `10`, default width `37`, random height `120`, window rectangle parameters `WWidth = 3`, `WHeight = 6`, spacing `10 x 15`.
    - Note: BASIC filled `LINE` endpoints are inclusive, so EGA windows appear as `4 x 7` pixels.
  - [ ] Preserve slope selection and new-height changes.
  - [ ] Preserve RNG call order closely enough that seeded tests can compare intended sequences.
  - [ ] Store enough building geometry to reproduce original gorilla placement and collisions.
  - [x] Add tests for deterministic seeded city generation.
- [ ] Gorilla placement:
  - [x] Use second/third building from each edge, not tallest-in-region.
  - [x] Apply original EGA offsets.
  - [x] Add tests for placement from generated building coordinates.
- [ ] Input:
  - [ ] Implement numeric entry with digits, one decimal point, backspace, enter, and invalid-key feedback state.
  - [ ] Preserve original angle validation behavior: values above `360` reset.
  - [ ] Invert player 2 angle with `180 - angle`.
- [ ] Projectile physics:
  - [x] Spawn from original adjusted start position.
  - [x] Preserve player-specific launch offset: player 2 starts at `StartX + Scl(25)`; both start at `StartY - Scl(4) - 3`.
  - [x] Use `x = start_x + init_x_vel * t + 0.5 * (wind / 5) * t^2`.
  - [x] Use `y = start_y + (-init_y_vel * t + 0.5 * gravity * t^2) * (ScrHeight / 350)`.
  - [x] Preserve original simulation step semantics (`t += 0.1`) even if rendered with frame interpolation.
  - [x] Advance rotation frame as `(t * 10) % 4`.
  - [x] Treat velocity below `2` as self-hit.
  - [x] Add unit tests for representative trajectories.
  - [x] Render projectile samples as a temporary demo shot.
  - [ ] Wire projectile samples into turn state and rendering.
- [ ] Collision:
  - [ ] Detect out-of-bounds using original thresholds.
  - [ ] Emulate original banana collision probe: player-dependent leading edge, `LookX`/`LookY` diagonal samples, and sampled color priority.
  - [ ] Detect building hits and trigger normal explosion.
  - [ ] Detect gorilla hits and trigger gorilla explosion.
  - [ ] Detect sun hit as expression-only collision while allowing the banana to pass through until it leaves the sun region.
  - [ ] Tune explicit geometry or CPU masks to match visible BASIC `POINT` behavior.
- [ ] Scoring:
  - [ ] Direct hit awards the throwing player.
  - [ ] Self-hit awards the opponent.
  - [ ] Run the configured number of rounds/games, matching BASIC `FOR i = 1 TO NumGames`.
  - [ ] Add tests for scoring outcomes.

## Original Screens And Flow

- [ ] Intro screen:
  - [ ] Render original title and mission text.
  - [ ] Render `Press any key to continue`.
  - [ ] Implement sparkle border animation.
  - [ ] Optional: approximate intro music.
- [ ] Config screen:
  - [ ] Prompt for player names.
  - [ ] Prompt with original wording, `Play to how many total points (Default = 3)`, while preserving the original fixed-round behavior or explicitly documenting a changed interpretation.
  - [ ] Prompt for gravity.
- [ ] Gorilla intro:
  - [ ] Show `V = View Intro`, `P = Play Game`, and `Your Choice?`.
  - [ ] Generate gorilla frames before play, or make the modern equivalent explicit.
  - [ ] If `V`, play the original starring/dance sequence.
- [ ] Play loop:
  - [ ] Create a new city each round.
  - [ ] Alternate tosser.
  - [ ] Preserve the original cross-round tosser toggle (`J` is not reset inside each round).
  - [ ] Reset sun after a sun hit.
  - [ ] Pause briefly after a round win.
- [ ] Match over:
  - [ ] Render `GAME OVER!` score screen.
  - [ ] Wait for key with sparkle pause.
  - [ ] Ask `Would you like to play again?` and accept `y`/`n`.

## Audio Work

- [ ] Decide whether audio is in scope for the first playable milestone.
- [ ] If included, add a small audio subsystem.
- [ ] Approximate QBasic `PLAY` cues:
  - [ ] Intro music.
  - [ ] Throw sound.
  - [ ] Building explosion.
  - [ ] Gorilla explosion.
  - [ ] Victory dance.
- [ ] Document any omitted or approximate sound behavior.

## Suggested Iteration Order

- [x] Milestone 1: make the static scene more faithful.
  - [x] Extract seedable city/round data model.
  - [x] Palette centralization.
  - [x] Original gorilla placement.
  - [x] Aspect-correct scaling.
  - [x] Decide raster primitive fidelity strategy.
- [ ] Milestone 2: add text and state flow.
  - [ ] Text renderer.
  - [ ] Intro screen.
  - [ ] Config menu.
  - [ ] Basic playing HUD.
- [ ] Milestone 3: add playable turn mechanics.
  - [ ] Wind generation and arrow.
  - [ ] Angle/velocity input.
  - [ ] Projectile model and banana rendering.
  - [ ] Building/out-of-bounds collision.
- [ ] Milestone 4: complete round resolution.
  - [ ] Gorilla collision.
  - [ ] Explosions.
  - [ ] Scoring.
  - [ ] Victory dance.
- [ ] Milestone 5: complete original flow and polish.
  - [ ] Gorilla intro.
  - [ ] Game-over and play-again loop.
  - [ ] Sparkle pause.
  - [ ] Audio, if selected.

## Verification Checklist

- [ ] `cargo fmt --check`
- [ ] `cargo test`
- [ ] `cargo check`
- [ ] Run the app and compare the first gameplay scene to `reference.png`.
- [ ] For visual changes, capture or inspect `target/screenshots/first-frame.ppm` at `640 x 350` logical size and compare against `reference.png` at `640 x 343`; ignore the reference image's 7-pixel height difference for now.
- [ ] For gameplay changes, add focused tests where behavior does not require a window.
