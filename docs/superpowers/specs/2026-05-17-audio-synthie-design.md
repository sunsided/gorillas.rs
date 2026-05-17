# Audio Implementation: synthie 0.3.0

**Issue:** [#19 - Implement sound effects and music (PLAY strings)](https://github.com/sunsided/gorillas.rs/issues/19)
**Date:** 2026-05-17

## Goal

Add audio playback to gorillas.rs using synthie 0.3.0. The original QBasic game drives sound via `PLAY` strings; this implementation hand-translates the six known strings into pre-computed MIDI note sequences. The game loop remains silent if no audio device is available.

## PLAY Strings to Implement

| Cue | PLAY string | Trigger |
|-----|-------------|---------|
| Intro | `MBT160O1L8CDEDCDL4ECC` | GameState transitions to Playing (first round) |
| GorillaIntro | 4 melodic phrases | GameState transitions from ConfigMenu to Playing |
| Throw | `MBo0L32A-L64CL16BL64A+` | Player submits angle + velocity |
| BuildingExplosion | `MBO0L32EFGEFDC` | Projectile hits a building |
| GorillaExplosion | `MBO0L16EFGEFDC` | Projectile hits a gorilla |
| VictoryDance | `MFO0L32EFGEFDC` x8 | Winner gorilla starts dancing |

`MB` = music background (non-blocking in BASIC), `T160` = tempo 160 BPM, `O0`/`O1` = octave, `Ln` = note length (L4=quarter, L8=eighth, L16=sixteenth, L32=thirty-second, L64=sixty-fourth).

## Architecture

Four files touched:

```
Cargo.toml       add synthie = "0.3.0"
src/audio.rs     new — AudioScheduler, SoundCue, note sequences
src/game.rs      update/handle_submit return Vec<SoundCue>
src/app.rs       App holds Option<AudioScheduler>, ticks + dispatches
```

No synthie types appear in `game.rs`. The game emits domain-level `SoundCue` values; `app.rs` bridges them to audio.

## Components

### `SoundCue` (src/audio.rs)

```rust
pub enum SoundCue {
    Intro,
    GorillaIntro,
    Throw,
    BuildingExplosion,
    GorillaExplosion,
    VictoryDance,
}
```

### `NoteEvent` (src/audio.rs)

```rust
struct NoteEvent {
    deadline: Instant,
    event: AudioEvent,
}
```

Ordered by `deadline` (min-heap via `BinaryHeap`).

### `AudioScheduler` (src/audio.rs)

```rust
pub struct AudioScheduler {
    _stream: cpal::Stream,        // keeps audio thread alive
    tx: Sender<AudioEvent>,
    queue: BinaryHeap<NoteEvent>, // min-heap by deadline
}
```

- `AudioScheduler::new() -> anyhow::Result<Self>` - calls `synthie::prelude::setup_audio()`, loads a SID-style patch on channel 0.
- `fn play(&mut self, cue: SoundCue)` - converts cue to NoteOn/NoteOff pairs via `cue_to_events`, pushes into queue. Sends `AudioEvent::Panic` first to cut any already-playing notes.
- `fn tick(&mut self)` - called each frame before game update; drains all events with `deadline <= Instant::now()`, sends each via `tx`.

### `cue_to_events` (src/audio.rs)

Private function. Takes `SoundCue` and `start: Instant`, returns `Vec<NoteEvent>`.

Each cue is a `const` array of `(u8 /* MIDI note */, u64 /* note-on micros from start */, u64 /* note-off micros from start */)` triples, derived by hand from the PLAY string.

**MIDI note mapping** (QBasic PLAY O0 = MIDI octave C2, O1 = C3, O2 = C4 = middle C):
- C=0, D=2, E=4, F=5, G=7, A=9, B=11 (semitone offsets within octave)
- Flat (`-`) subtracts 1, sharp (`+`) adds 1
- MIDI note = 36 + (octave * 12) + semitone_offset
  - O0 C = MIDI 36, O0 A- = MIDI 44, O1 C = MIDI 48, O1 E = MIDI 52

**Duration calculation** (T160 BPM):
- Beat duration = 60,000,000 / 160 = 375,000 µs
- L4 = 375,000 µs, L8 = 187,500 µs, L16 = 93,750 µs, L32 = 46,875 µs, L64 = 23,437 µs

Note-off fires at note-on + (note_duration - 10,000 µs) to allow a short gap between notes.

### Game changes (src/game.rs)

- `Game::update(&mut self, dt: f32) -> Vec<SoundCue>` (was `()`)
- `Game::handle_submit(&mut self) -> Vec<SoundCue>` (was `()`)
- `Game::handle_backspace` stays `()` (no sound)
- `GameState::update`, `GameState::handle_submit` propagate cues upward

**Trigger points:**
- Throw: `handle_submit` when velocity is valid and projectile is created
- BuildingExplosion: `update` when `CollisionProbe::Impact { kind: ImpactKind::Building, .. }`
- GorillaExplosion: `update` when `CollisionProbe::Impact { kind: ImpactKind::Gorilla(..), .. }`
- VictoryDance: `finish_explosion` when `ExplosionKind::Gorilla` finishes
- Intro / GorillaIntro: `GameState::apply_config_and_start()` (called from `config_handle_submit` when config is complete)

### App changes (src/app.rs)

```rust
struct App {
    // existing fields ...
    audio: Option<AudioScheduler>,
}
```

- `App::new()`: `audio = AudioScheduler::new().ok()` - audio failure is non-fatal.
- `RedrawRequested` handler: call `audio.tick()` first, then `game.update(dt)` collecting cues, then `audio.play(cue)` for each.
- `handle_submit` / `handle_backspace` call paths similarly collect and dispatch cues.

## Patch Selection

All six cues share one SID-style patch from `synthie::presets::sid::default_patches()`. The "PWM Lead" preset (pulse wave, short decay) best approximates the original square-wave QBasic sound. Loaded once at `AudioScheduler::new()`.

## Error Handling

- `AudioScheduler::new()` returns `anyhow::Result<Self>`. On failure (no audio device), `App::audio` is `None`. Game continues silently.
- `tx.send()` errors (receiver gone) are discarded silently; the stream owns the receiver and only drops on program exit.
- No panics on audio path. All audio errors are logged via `eprintln!` at init time only.

## Testing

- Unit tests in `src/audio.rs`: verify each cue's note sequence has correct length, MIDI values in valid range (0-127), and that note-off deadlines are after note-on deadlines.
- `GameState` / `Game` method signatures change: existing tests updated to collect and ignore returned `Vec<SoundCue>`.
- No audio hardware required in tests; `AudioScheduler` is not constructed in unit tests.

## Out of Scope

- PLAY string parser (not needed for the 6 fixed strings)
- Per-cue patch variation (one patch for all is sufficient)
- Intro music cutoff when game starts before melody finishes (acceptable for first implementation)
