//! The clock the composing path stamps learning with. Stays with the
//! desktop manager; the store seams moved to the engine `userdata` crate.

/// Milliseconds since the Unix epoch. Injectable because the engine's
/// association window is a comparison against this clock, and a test that
/// cannot move it can only ever exercise one side of it.
pub trait Clock: Send + Sync {
    fn now_ms(&self) -> i64;
}

/// The wall clock.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| {
                i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
            })
    }
}
