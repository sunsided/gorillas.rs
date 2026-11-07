//! `gorillas`: play in a window.
//!
//! `gorillas capture TIMELINE OUTDIR [--frames]`: run a scripted session headless (see
//! `capture`).
//!
//! Environment: `GORILLAS_SEED=n` replaces `RANDOMIZE (TIMER)` with `RANDOMIZE n`,
//! `GORILLAS_CRT=0..6` picks the CRT filter preset (0 = off, default 2 = soft glow).

#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<()> {
    use gorillas::{app, gorillas::Options};

    let options = Options {
        seed: std::env::var("GORILLAS_SEED")
            .ok()
            .and_then(|s| s.parse().ok()),
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("capture") {
        let (Some(timeline), Some(out)) = (args.get(1), args.get(2)) else {
            anyhow::bail!("usage: gorillas capture TIMELINE OUTDIR [--frames]");
        };
        let frames = args.iter().any(|a| a == "--frames");
        return gorillas::capture::run(timeline.as_ref(), out.as_ref(), options, frames);
    }
    env_logger_init();
    println!("QBasic Gorillas: Alt+Enter fullscreen, Alt+C cycles the CRT filter");
    app::run(app::Config {
        options,
        crt: std::env::var("GORILLAS_CRT")
            .ok()
            .and_then(|v| v.parse().ok()),
        ..Default::default()
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn env_logger_init() {}

#[cfg(target_arch = "wasm32")]
fn main() {}
