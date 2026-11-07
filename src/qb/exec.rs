//! A minimal executor: the BASIC program is one `async` function, polled once per host step.
//!
//! BASIC code blocks (busy loops on `TIMER`, `INKEY$` polls, `PLAY` in the foreground); a
//! game loop must not. Every statement that spends emulated time awaits, and after about a
//! millisecond of emulated time the program yields back to the host, which shows frames,
//! plays audio and feeds the keyboard before polling again.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

/// A running program.
pub struct Program {
    fut: Option<Pin<Box<dyn Future<Output = ()>>>>,
}

impl Program {
    pub fn new(f: impl Future<Output = ()> + 'static) -> Self {
        Self {
            fut: Some(Box::pin(f)),
        }
    }

    /// Run until the program yields. Returns `true` once it has finished.
    pub fn step(&mut self) -> bool {
        let Some(fut) = self.fut.as_mut() else {
            return true;
        };
        let mut cx = Context::from_waker(Waker::noop());
        if fut.as_mut().poll(&mut cx).is_ready() {
            self.fut = None;
            return true;
        }
        false
    }
}

/// Yields once.
pub struct Yield(bool);

impl Yield {
    pub fn new() -> Self {
        Self(false)
    }
}

impl Future for Yield {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            Poll::Pending
        }
    }
}
