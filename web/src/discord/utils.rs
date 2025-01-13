use serenity::all::{colours, Color};
use std::time::{Duration, Instant};

pub const COLOR_SUCCESS: Color = colours::css::POSITIVE;
pub const COLOR_ERROR: Color = colours::css::DANGER;
pub const COLOR_IN_QUEUE: Color = Color::BLITZ_BLUE;

pub const COLOR_DC_ALLOWED: Color = colours::css::POSITIVE;
pub const COLOR_DC_PROHIBITED: Color = colours::css::DANGER;
pub const COLOR_DC_MIXED: Color = colours::css::WARNING;

pub fn format_duration(duration: time::Duration) -> String {
    let seconds = duration.whole_seconds();
    let minutes = seconds / 60;
    let seconds = seconds % 60;
    let hours = minutes / 60;
    let minutes = minutes % 60;
    let days = hours / 24;
    let hours = hours % 24;

    if days > 0 {
        format!("{days}d {hours:02}:{minutes:02}:{seconds:02}")
    } else if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

pub struct Stopwatch {
    name: String,
    start: Instant,
}

impl Stopwatch {
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
