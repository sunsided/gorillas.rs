//! Audio scheduling layer. Translates [`SoundCue`] values into timestamped MIDI events
//! and feeds them to the [`synthie`] synthesizer at the correct wall-clock time.
//!
//! Every note table is transcribed from a QBasic `PLAY` string in GORILLAS.BAS;
//! each constant's doc comment cites the originating line.

use std::time::{Duration, Instant};

use anyhow::Result;
use synthie::params::AudioEvent;

/// A named sound effect, each corresponding to a QBasic `PLAY` statement in GORILLAS.BAS.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundCue {
    /// Title-screen melody. GORILLAS.BAS line 696: `PLAY "MBT160O1L8CDEDCDL4ECC"`.
    Intro,
    /// Four-phrase animation music played during the gorilla intro screen.
    /// GORILLAS.BAS lines 645-661 (`GorillaIntro` sub).
    GorillaIntro,
    /// Banana-throw sound. GORILLAS.BAS line 937: `PLAY "MBo0L32A-L64CL16BL64A+"`.
    Throw,
    /// Explosion when the banana hits a building.
    /// GORILLAS.BAS line 274: `PLAY "MBO0L32EFGEFDC"`.
    BuildingExplosion,
    /// Explosion when a gorilla takes a direct hit.
    /// GORILLAS.BAS line 482: `PLAY "MBO0L16EFGEFDC"`.
    GorillaExplosion,
    /// Victory dance melody, played eight times after a hit.
    /// GORILLAS.BAS line 1144: `PLAY "MFO0L32EFGEFDC"`.
    VictoryDance,
}

/// Wall-clock audio scheduler. Converts [`SoundCue`]s into timestamped MIDI events
/// and drains them on each [`tick`](AudioScheduler::tick) call.
#[allow(dead_code)]
pub(crate) struct AudioScheduler {
    /// Keeps the audio output stream alive for the lifetime of the scheduler.
    _stream: cpal::Stream,
    /// Channel to the synthie audio thread.
    tx: crossbeam_channel::Sender<AudioEvent>,
    /// Pending `(fire_time, event)` pairs, drained in [`tick`](AudioScheduler::tick).
    queue: Vec<(Instant, AudioEvent)>,
}

// Each entry: (start_us from cue start, MIDI note, note duration µs)
// NoteOff fires at start_us + dur_us - 10_000

/// Note table for [`SoundCue::Intro`].
/// GORILLAS.BAS line 696: `PLAY "MBT160O1L8CDEDCDL4ECC"` — T160, O1 (base=48).
const INTRO_NOTES: [(u64, u8, u64); 9] = [
    (0, 48, 187_500),         // C L8
    (187_500, 50, 187_500),   // D
    (375_000, 52, 187_500),   // E
    (562_500, 50, 187_500),   // D
    (750_000, 48, 187_500),   // C
    (937_500, 50, 187_500),   // D
    (1_125_000, 52, 375_000), // E L4
    (1_500_000, 48, 375_000), // C
    (1_875_000, 48, 375_000), // C
];

/// Note table for [`SoundCue::Throw`].
/// GORILLAS.BAS line 937: `PLAY "MBo0L32A-L64CL16BL64A+"` — T120, O0 (base=36).
const THROW_NOTES: [(u64, u8, u64); 4] = [
    (0, 44, 62_500),       // A-flat L32
    (62_500, 36, 31_250),  // C L64
    (93_750, 47, 125_000), // B L16
    (218_750, 46, 31_250), // A-sharp L64
];

/// Note table for [`SoundCue::BuildingExplosion`].
/// GORILLAS.BAS line 274: `PLAY "MBO0L32EFGEFDC"` — T120, O0 (base=36), all L32.
const BUILDING_EXPLOSION_NOTES: [(u64, u8, u64); 7] = [
    (0, 40, 62_500),       // E
    (62_500, 41, 62_500),  // F
    (125_000, 43, 62_500), // G
    (187_500, 40, 62_500), // E
    (250_000, 41, 62_500), // F
    (312_500, 38, 62_500), // D
    (375_000, 36, 62_500), // C
];

/// Note table for [`SoundCue::GorillaExplosion`].
/// GORILLAS.BAS line 482: `PLAY "MBO0L16EFGEFDC"` — T120, O0 (base=36), all L16.
const GORILLA_EXPLOSION_NOTES: [(u64, u8, u64); 7] = [
    (0, 40, 125_000),
    (125_000, 41, 125_000),
    (250_000, 43, 125_000),
    (375_000, 40, 125_000),
    (500_000, 41, 125_000),
    (625_000, 38, 125_000),
    (750_000, 36, 125_000),
];

/// One repetition of the victory dance melody; same pitches as [`BUILDING_EXPLOSION_NOTES`].
/// GORILLAS.BAS line 1144: `PLAY "MFO0L32EFGEFDC"` — used eight times in [`SoundCue::VictoryDance`].
const VICTORY_DANCE_UNIT: [(u64, u8, u64); 7] = BUILDING_EXPLOSION_NOTES;
/// Duration of one victory dance repetition in microseconds (7 × 62_500 µs).
const VICTORY_UNIT_DUR_US: u64 = 437_500; // 7 × 62_500

/// Shared note-slot timing template for all four [`SoundCue::GorillaIntro`] phrases.
/// 15 slots per phrase; rests are omitted. Each entry is `(start_us, duration_us)`.
/// GORILLAS.BAS lines 645-661 (`GorillaIntro` sub).
const GORILLA_INTRO_TIMING: [(u64, u64); 15] = [
    (0, 222_222),       // L9
    (347_222, 125_000), // L16
    (472_222, 125_000),
    (597_222, 125_000),
    (847_222, 125_000),
    (1_097_222, 125_000),
    (1_347_222, 125_000),
    (1_472_222, 125_000),
    (1_597_222, 125_000),
    (1_722_222, 125_000),
    (1_972_222, 222_222), // L9
    (2_319_444, 125_000),
    (2_444_444, 125_000),
    (2_569_444, 125_000),
    (2_819_444, 125_000),
];
/// Total duration of one gorilla intro phrase in microseconds.
const GORILLA_INTRO_PHRASE_DUR_US: u64 = 2_944_444;

/// MIDI note numbers for each of the four gorilla intro phrases (15 notes each).
/// GORILLAS.BAS lines 645-661 (`GorillaIntro` sub):
/// - P1 O1: B B A A B B B A A A B B A A B
/// - P2 O2: Eb Eb Db Db Eb Eb Eb Db Db Db Eb Eb Db Db Eb
/// - P3 O2: Gb Gb E E Gb Gb Gb E E E Gb Gb E E Gb
/// - P4 O2-O1: B2 B2 A2 A2 Gb Gb Gb E E E B1 B1 A1 A1 B1
const GORILLA_INTRO_PHRASES: [[u8; 15]; 4] = [
    [59, 59, 57, 57, 59, 59, 59, 57, 57, 57, 59, 59, 57, 57, 59],
    [63, 63, 61, 61, 63, 63, 63, 61, 61, 61, 63, 63, 61, 61, 63],
    [66, 66, 64, 64, 66, 66, 66, 64, 64, 64, 66, 66, 64, 64, 66],
    [71, 71, 69, 69, 66, 66, 66, 64, 64, 64, 59, 59, 57, 57, 59],
];

/// Converts a [`SoundCue`] into a list of `(fire_time, AudioEvent)` pairs
/// relative to `start`, ready to be appended to the scheduler queue.
#[allow(dead_code)]
pub(crate) fn cue_to_events(cue: SoundCue, start: Instant) -> Vec<(Instant, AudioEvent)> {
    use synthie::params::MidiNote;

    fn push_notes(out: &mut Vec<(Instant, AudioEvent)>, start: Instant, notes: &[(u64, u8, u64)]) {
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
                let rep_notes: [(u64, u8, u64); 7] =
                    VICTORY_DANCE_UNIT.map(|(t, m, d)| (t + offset_us, m, d));
                push_notes(&mut events, start, &rep_notes);
            }
        }
    }

    events
}

#[allow(dead_code)]
impl AudioScheduler {
    /// Creates the audio output stream and loads the "PWM Lead" synthesizer patch.
    pub(crate) fn new() -> Result<Self> {
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

    /// Cancels any in-flight cue, clears the queue, and starts playing `cue` immediately.
    pub(crate) fn play(&mut self, cue: SoundCue) {
        let _ = self.tx.send(AudioEvent::Panic);
        self.queue.clear();
        let events = cue_to_events(cue, Instant::now());
        self.queue.extend(events);
    }

    /// Dispatches all queued events whose scheduled fire time has elapsed.
    /// Call once per frame from the render loop.
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

    /// Appends `cue` to start 10 ms after the last pending event,
    /// or immediately if the queue is empty.
    pub(crate) fn enqueue_after(&mut self, cue: SoundCue) {
        let start = self
            .queue
            .iter()
            .map(|(t, _)| *t)
            .max()
            .map(|last| last + Duration::from_millis(10))
            .unwrap_or_else(Instant::now);
        self.queue.extend(cue_to_events(cue, start));
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
}
