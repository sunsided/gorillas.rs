//! GORILLAS.RS — a Rust port of QBasic GORILLAS.BAS (IBM Corporation, 1991).
//!
//! Two players throw exploding bananas at each other across a procedurally generated
//! city skyline. Angle and velocity are adjustable; wind and gravity affect the arc.
//! See [`app::run`] for the entry point and [`game::GameState`] for the game loop.

mod app;
mod audio;
mod game;
mod render;

/// Starts the game; delegates to [`app::run`] and exits with code 1 on fatal error.
fn main() {
    if let Err(error) = app::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
