use serenity::all::{colours, Color};

pub const COLOR_SUCCESS: Color = colours::css::POSITIVE;
pub const COLOR_ERROR: Color = colours::css::DANGER;
pub const COLOR_IN_QUEUE: Color = Color::BLITZ_BLUE;
pub const COLOR_QUEUE_POP: Color = Color::GOLD;

pub const COLOR_DC_ALLOWED: Color = colours::css::POSITIVE;
pub const COLOR_DC_PROHIBITED: Color = colours::css::DANGER;
pub const COLOR_DC_MIXED: Color = colours::css::WARNING;

pub fn format_queue_duration(duration: time::Duration) -> String {
    format_duration_default(duration, true, "Instant")
}

pub fn format_duration(duration: time::Duration) -> String {
    format_duration_default(duration, true, "0s")
}

pub fn format_duration_duty_eta(duration: time::Duration) -> String {
    format_duration_default(duration, false, "0m")
}

fn format_duration_default(duration: time::Duration, add_seconds: bool, default: &str) -> String {
    if duration.is_zero() {
        return default.to_string();
    }

    let seconds = duration.whole_seconds();
    let minutes = seconds / 60;
    let seconds = seconds % 60;
    let hours = minutes / 60;
    let minutes = minutes % 60;
    let days = hours / 24;
    let hours = hours % 24;

    let mut data = vec![];
    let mut write = false;

    if days > 0 {
        data.push(format!("{}d", days));
        write = true;
    }
    if hours > 0 || write {
        data.push(format!("{}h", hours));
        write = true;
    }
    if minutes > 0 || write {
        data.push(format!("{}m", minutes));
        write = true;
    }
    if (seconds > 0 || write) && add_seconds {
        data.push(format!("{}s", seconds));
    }

    data.join(" ")
}

pub fn format_latency(duration: time::Duration) -> String {
    format!("{:.2}ms", duration.as_seconds_f32() * 1000.)
}
