//! QBasic GORILLAS.BAS (IBM Corporation, 1991), ported statement by statement onto an emulated
//! VGA card and PC speaker ([`game_toolkit_dosvideo`]).
//!
//! - [`qb`]: the QBasic runtime the port runs on (screen modes, `PRINT`/`INPUT`, `LINE`,
//!   `CIRCLE`, `PAINT`, `GET`/`PUT`, `PLAY`, `RND`, `TIMER`), with QBasic's timing.
//! - [`gorillas`]: the program.
//! - [`app`]: runs it in a window or browser canvas.
//! - [`capture`]: headless scripted runs, for comparing against the original in DOSBox.

pub mod app;
#[cfg(not(target_arch = "wasm32"))]
pub mod capture;
pub mod gorillas;
pub mod qb;

/// Seconds since local midnight, for `TIMER`.
pub fn seconds_since_midnight() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        let d = js_sys::Date::new_0();
        f64::from(d.get_hours()) * 3600.0
            + f64::from(d.get_minutes()) * 60.0
            + f64::from(d.get_seconds())
            + f64::from(d.get_milliseconds()) / 1000.0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0.0, |d| d.as_secs_f64() % 86_400.0)
    }
}

/// Browser entry point: run the game in the `<canvas>` with this id.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn start(canvas_id: String) -> Result<(), wasm_bindgen::JsValue> {
    app::run(app::Config {
        canvas_id: Some(canvas_id),
        ..Default::default()
    })
    .map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))
}
