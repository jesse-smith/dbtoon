use std::time::Instant;

/// Emit a verbose diagnostic message to stderr.
pub fn emit(verbose: bool, msg: &str) {
    if verbose {
        eprintln!("[dbtoon] {}", msg);
    }
}

/// A timer for measuring durations in verbose mode.
pub struct Timer {
    start: Instant,
}

impl Timer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }
}
