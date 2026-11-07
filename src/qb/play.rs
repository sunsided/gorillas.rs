//! `PLAY`, `SOUND` and `BEEP`: QBasic's music macro language on the PC speaker.
//!
//! Unlike a one-shot tune player, QBasic keeps the octave, note length, tempo, articulation
//! and foreground/background mode from one `PLAY` statement to the next (GORILLAS.BAS relies
//! on that: `PLAY "MBO0L32EFGEFDC"` plays at whatever tempo an earlier string set). In
//! background mode (`MB`) notes go into a queue of [`QUEUE_NOTES`] that drains while the
//! program runs on; `PLAY` only waits when the queue is full. In foreground mode (`MF`, the
//! default) `PLAY` returns once its last note has started (measured in DOSBox: the victory
//! dance runs at that pace).

use std::collections::VecDeque;

/// Notes the background music buffer holds. QBasic documents 32; what it does in DOSBox (the
/// pace of the gorilla dance, where each `PLAY` waits for the previous phrase) fits 24.
pub const QUEUE_NOTES: usize = 24;

/// One queued note or rest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Note {
    /// Frequency in Hz, `None` for a rest.
    pub hz: Option<f64>,
    /// Sounding part in seconds.
    pub on: f64,
    /// Whole duration in seconds.
    pub dur: f64,
}

/// Music state that persists between `PLAY` statements, and the note queue.
#[derive(Clone, Debug)]
pub struct Music {
    octave: i32,
    length: f64,
    tempo: f64,
    articulation: f64,
    /// `MB` was given more recently than `MF`.
    pub background: bool,
    queue: VecDeque<Note>,
    /// The note playing now: its end time and the time its tone stops (ns).
    current: Option<(f64, f64)>,
    sounding: bool,
}

impl Default for Music {
    fn default() -> Self {
        Self {
            octave: 4,
            length: 4.0,
            tempo: 120.0,
            articulation: 7.0 / 8.0,
            background: false,
            queue: VecDeque::new(),
            current: None,
            sounding: false,
        }
    }
}

/// What the speaker has to do at an event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Speaker {
    Tone(f64),
    Off,
}

impl Music {
    /// Parse a `PLAY` string, updating the persistent state, and return its notes.
    pub fn parse(&mut self, s: &str) -> Vec<Note> {
        let b = s.to_ascii_uppercase().into_bytes();
        let mut i = 0;
        let mut out = Vec::new();
        fn num(b: &[u8], i: &mut usize) -> Option<u32> {
            let st = *i;
            while *i < b.len() && b[*i].is_ascii_digit() {
                *i += 1;
            }
            std::str::from_utf8(&b[st..*i]).ok()?.parse().ok()
        }
        fn dots(b: &[u8], i: &mut usize) -> f64 {
            let mut m = 1.0;
            let mut add = 0.5;
            while *i < b.len() && b[*i] == b'.' {
                m += add;
                add /= 2.0;
                *i += 1;
            }
            m
        }
        // Note number n: 1 = C in octave 0 ... 84; A in octave 3 (n = 46) is 440 Hz.
        let freq = |n: i32| 440.0 * 2f64.powf(f64::from(n - 46) / 12.0);
        while i < b.len() {
            let c = b[i];
            i += 1;
            let secs = |l: f64, tempo: f64| 60.0 / tempo * 4.0 / l;
            match c {
                b'A'..=b'G' => {
                    let semis = [9, 11, 0, 2, 4, 5, 7][(c - b'A') as usize];
                    let mut n = self.octave * 12 + semis + 1;
                    if i < b.len() && (b[i] == b'#' || b[i] == b'+') {
                        n += 1;
                        i += 1;
                    } else if i < b.len() && b[i] == b'-' {
                        n -= 1;
                        i += 1;
                    }
                    let l = num(&b, &mut i).map_or(self.length, |v| f64::from(v.clamp(1, 64)));
                    let d = secs(l, self.tempo) * dots(&b, &mut i);
                    out.push(Note {
                        hz: Some(freq(n)),
                        on: d * self.articulation,
                        dur: d,
                    });
                }
                b'N' => {
                    let n = num(&b, &mut i).unwrap_or(0) as i32;
                    let d = secs(self.length, self.tempo) * dots(&b, &mut i);
                    out.push(Note {
                        hz: (n > 0).then(|| freq(n)),
                        on: if n > 0 { d * self.articulation } else { 0.0 },
                        dur: d,
                    });
                }
                b'P' => {
                    let l = num(&b, &mut i).map_or(self.length, |v| f64::from(v.clamp(1, 64)));
                    let d = secs(l, self.tempo) * dots(&b, &mut i);
                    out.push(Note {
                        hz: None,
                        on: 0.0,
                        dur: d,
                    });
                }
                b'O' => self.octave = num(&b, &mut i).unwrap_or(4).min(6) as i32,
                b'<' => self.octave = (self.octave - 1).max(0),
                b'>' => self.octave = (self.octave + 1).min(6),
                b'L' => self.length = f64::from(num(&b, &mut i).unwrap_or(4).clamp(1, 64)),
                b'T' => self.tempo = f64::from(num(&b, &mut i).unwrap_or(120).clamp(32, 255)),
                b'M' if i < b.len() => {
                    match b[i] {
                        b'S' => self.articulation = 3.0 / 4.0,
                        b'N' => self.articulation = 7.0 / 8.0,
                        b'L' => self.articulation = 1.0,
                        b'B' => self.background = true,
                        b'F' => self.background = false,
                        _ => {}
                    }
                    i += 1;
                }
                _ => {}
            }
        }
        out
    }

    /// Notes waiting to start.
    pub fn waiting(&self) -> usize {
        self.queue.len()
    }

    /// Notes waiting or playing.
    pub fn pending(&self) -> usize {
        self.queue.len() + usize::from(self.current.is_some())
    }

    /// Queue a note (the caller makes sure there is room).
    pub fn push(&mut self, note: Note) {
        self.queue.push_back(note);
    }

    /// When the speaker next has to change, if anything is playing or queued.
    pub fn next_event(&self) -> Option<f64> {
        match self.current {
            Some((end, off)) if self.sounding && off < end => Some(off),
            Some((end, _)) => Some(end),
            None if !self.queue.is_empty() => Some(f64::NEG_INFINITY),
            None => None,
        }
    }

    /// Advance the player to machine time `now` (ns). Returns what the speaker must do.
    pub fn fire(&mut self, now: f64) -> Option<Speaker> {
        let mut action = None;
        loop {
            match self.current {
                Some((end, off)) => {
                    if now >= end {
                        self.current = None;
                        // The next note starts exactly where this one ended.
                        if let Some(n) = self.queue.pop_front() {
                            self.start(n, end, &mut action);
                            continue;
                        }
                        if self.sounding {
                            self.sounding = false;
                            action = Some(Speaker::Off);
                        }
                    } else if self.sounding && now >= off {
                        self.sounding = false;
                        action = Some(Speaker::Off);
                    }
                    return action;
                }
                None => {
                    let Some(n) = self.queue.pop_front() else {
                        return action;
                    };
                    self.start(n, now, &mut action);
                }
            }
        }
    }

    fn start(&mut self, n: Note, t: f64, action: &mut Option<Speaker>) {
        self.current = Some((t + n.dur * 1e9, t + n.on * 1e9));
        match n.hz {
            Some(hz) if n.on > 0.0 => {
                self.sounding = true;
                *action = Some(Speaker::Tone(hz));
            }
            _ => {
                if self.sounding {
                    *action = Some(Speaker::Off);
                }
                self.sounding = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_persists_between_statements() {
        let mut m = Music::default();
        let a = m.parse("MBT160O1L8C");
        assert!(m.background);
        assert!((a[0].dur - 60.0 / 160.0 * 0.5).abs() < 1e-9);
        // No tempo or length here: they carry over.
        let b = m.parse("D");
        assert!((b[0].dur - a[0].dur).abs() < 1e-9);
        assert!(b[0].hz.unwrap() > a[0].hz.unwrap());
    }

    #[test]
    fn gorilla_intro_lengths_and_rests() {
        let mut m = Music::default();
        let n = m.parse("t120o1l16b9n0baa");
        assert_eq!(n.len(), 5);
        // b9: a ninth note at 120 bpm.
        assert!((n[0].dur - 0.5 * 4.0 / 9.0).abs() < 1e-9);
        assert_eq!(n[1].hz, None);
        assert!((n[1].dur - 0.125).abs() < 1e-9);
    }

    #[test]
    fn notes_play_back_to_back() {
        let mut m = Music::default();
        for n in m.parse("T120L4CD") {
            m.push(n);
        }
        assert!(matches!(m.fire(0.0), Some(Speaker::Tone(_))));
        assert_eq!(m.fire(0.5e9 * 7.0 / 8.0), Some(Speaker::Off));
        assert!(matches!(m.fire(0.5e9), Some(Speaker::Tone(_))));
        assert_eq!(m.fire(1.0e9), Some(Speaker::Off));
        assert_eq!(m.pending(), 0);
    }
}
