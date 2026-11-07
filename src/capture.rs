//! Headless scripted runs, for comparing the port with the original in DOSBox.
//!
//! A timeline drives the keyboard and takes screenshots on the emulated clock, in the format
//! the DOSBox capture script uses, so the same timeline can drive both:
//!
//! ```text
//! 5 shot intro
//! 6 key space
//! 7.5 type Alice
//! 8.5 key Return
//! 61 end
//! ```
//!
//! Screenshots are the active display area as PNG (640x350 in `SCREEN 9`, 720x400 in text
//! mode, like a DOSBox window without scaling). The PC speaker goes to `audio.wav`, and with
//! `frames` set every 30th of a second is written as `frames/NNNNN.png`.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use game_toolkit_dosvideo::{FrameView, Keyboard, sc};

use crate::gorillas::Options;

/// One timeline entry.
#[derive(Clone, Debug)]
enum Event {
    Key(String),
    Type(String),
    Shot(String),
    End,
}

fn parse(timeline: &str) -> Result<Vec<(f64, Event)>> {
    let mut out = Vec::new();
    for line in timeline.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(3, ' ');
        let t: f64 = parts
            .next()
            .unwrap_or("")
            .parse()
            .context(line.to_string())?;
        let kind = parts.next().unwrap_or("");
        let arg = parts.next().unwrap_or("").to_string();
        out.push((
            t,
            match kind {
                "key" => Event::Key(arg),
                "type" => Event::Type(arg),
                "shot" => Event::Shot(arg),
                "end" => Event::End,
                _ => bail!("unknown event: {line}"),
            },
        ));
    }
    Ok(out)
}

/// An xdotool key name or a character, as `(scancode, shift)`.
fn scancode(name: &str) -> Option<(u8, bool)> {
    let named = match name {
        "Return" => Some(sc::ENTER),
        "space" => Some(sc::SPACE),
        "BackSpace" => Some(sc::BACKSPACE),
        "Escape" => Some(sc::ESC),
        "period" => Some(sc::PERIOD),
        "Up" => Some(sc::UP),
        "Down" => Some(sc::DOWN),
        "Left" => Some(sc::LEFT),
        "Right" => Some(sc::RIGHT),
        _ => None,
    };
    if let Some(s) = named {
        return Some((s, false));
    }
    let mut chars = name.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    (1..0x3Au8).find_map(|s| {
        if Keyboard::ascii_for(s, false) == c as u8 {
            Some((s, false))
        } else if Keyboard::ascii_for(s, true) == c as u8 {
            Some((s, true))
        } else {
            None
        }
    })
}

/// The active area of the last finished frame as PNG.
fn shot_png(frame: FrameView<'_>) -> Vec<u8> {
    let [ax, ay, aw, ah] = frame.active;
    let mut rgba = Vec::with_capacity((aw * ah * 4) as usize);
    for y in ay..ay + ah {
        let row = ((y * frame.width + ax) * 4) as usize;
        rgba.extend_from_slice(&frame.rgba[row..row + (aw * 4) as usize]);
    }
    FrameView {
        width: aw,
        height: ah,
        rgba: &rgba,
        active: [0, 0, aw, ah],
        number: frame.number,
    }
    .to_png()
}

/// Run `timeline` headless, writing screenshots, `audio.wav` and optionally a frame sequence
/// into `out`.
pub fn run(timeline: &Path, out: &Path, options: Options, frames: bool) -> Result<()> {
    let events = parse(&std::fs::read_to_string(timeline)?)?;
    std::fs::create_dir_all(out)?;
    let frame_dir: Option<PathBuf> = frames.then(|| out.join("frames"));
    if let Some(d) = &frame_dir {
        std::fs::create_dir_all(d)?;
    }
    let (m, mut program) = crate::app::boot(options);
    let end = events
        .iter()
        .find(|(_, e)| matches!(e, Event::End))
        .map_or_else(|| events.last().map_or(10.0, |e| e.0 + 1.0), |e| e.0);

    // Expand typing into key presses 60 ms apart; every key is held 40 ms.
    let mut presses: Vec<(f64, u8, bool)> = Vec::new();
    let mut shots: Vec<(f64, String)> = Vec::new();
    for (t, e) in &events {
        match e {
            Event::Key(k) => {
                let (s, shift) = scancode(k).with_context(|| format!("unknown key {k}"))?;
                presses.push((*t, s, shift));
            }
            Event::Type(text) => {
                for (i, c) in text.chars().enumerate() {
                    let (s, shift) =
                        scancode(&c.to_string()).with_context(|| format!("unknown key {c}"))?;
                    presses.push((t + i as f64 * 0.1, s, shift));
                }
            }
            Event::Shot(name) => shots.push((*t, name.clone())),
            Event::End => {}
        }
    }
    presses.sort_by(|a, b| a.0.total_cmp(&b.0));
    shots.sort_by(|a, b| a.0.total_cmp(&b.0));

    let rate = 44_100;
    let mut audio = Vec::new();
    let mut held: Vec<(f64, u8)> = Vec::new();
    let (mut pi, mut si) = (0, 0);
    let mut next_frame = 0.0;
    let mut frame_no = 0u32;
    loop {
        let now = m.borrow().dos.now();
        if now >= end {
            break;
        }
        {
            let mut mm = m.borrow_mut();
            let kb = mm.dos.keyboard_mut();
            held.retain(|&(t, s)| {
                if now >= t {
                    kb.release(s);
                    false
                } else {
                    true
                }
            });
            while pi < presses.len() && presses[pi].0 <= now {
                let (t, s, shift) = presses[pi];
                kb.press(s, Keyboard::ascii_for(s, shift));
                held.push((t + 0.04, s));
                pi += 1;
            }
            mm.flush();
            mm.dos.sync();
            while si < shots.len() && shots[si].0 <= now {
                std::fs::write(
                    out.join(format!("{}.png", shots[si].1)),
                    shot_png(mm.dos.frame()),
                )?;
                si += 1;
            }
            if let Some(d) = &frame_dir
                && now >= next_frame
            {
                std::fs::write(
                    d.join(format!("{frame_no:05}.png")),
                    shot_png(mm.dos.frame()),
                )?;
                frame_no += 1;
                next_frame += 1.0 / 30.0;
            }
        }
        let before = m.borrow().dos.now_ns();
        if program.step() {
            break;
        }
        let mut mm = m.borrow_mut();
        if mm.dos.now_ns() <= before {
            mm.advance(1e6);
        }
        mm.dos.speaker_render(rate, &mut audio);
    }
    std::fs::write(out.join("audio.wav"), wav16(&audio, rate))?;
    Ok(())
}

/// Mono 16-bit PCM WAV.
fn wav16(samples: &[f32], rate: u32) -> Vec<u8> {
    let data = samples.len() as u32 * 2;
    let mut w = Vec::with_capacity(44 + data as usize);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&rate.to_le_bytes());
    w.extend_from_slice(&(rate * 2).to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data.to_le_bytes());
    for s in samples {
        w.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    w
}
