use std::time::{Duration, Instant};

pub struct Stopwatch {
    name: String,
    start: Instant,
}

impl Stopwatch {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            start: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

impl Default for Stopwatch {
    fn default() -> Self {
        Self::new("Stopwatch")
    }
}

impl Drop for Stopwatch {
    fn drop(&mut self) {
        log::info!(
            "{}: {:.4}ms",
            self.name,
            self.elapsed().as_secs_f64() * 1_000.0
        );
    }
}
