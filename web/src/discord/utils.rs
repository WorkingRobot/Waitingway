use futures_util::{stream::FuturesUnordered, StreamExt};
use redis::AsyncCommands;
use serenity::all::{colours, Color};

use crate::storage::redis::utils::to_key;

use super::DiscordClient;

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

// Guild Count, User Count
pub async fn install_stats(client: &DiscordClient) -> (u32, u32) {
    client
        .http()
        .get_current_application_info()
        .await
        .map(|i| {
            (
                i.approximate_guild_count.unwrap_or_default(),
                i.approximate_user_install_count.unwrap_or_default(),
            )
        })
        .unwrap_or_default()
}

pub async fn member_count(client: &DiscordClient) -> u64 {
    let cache = client.cache();
    let http = client.http();
    let mut guilds: FuturesUnordered<_> = cache
        .guilds()
        .into_iter()
        .map(|g| async move {
            let count = cache.guild(g).map(|c| c.member_count);
            match count {
                Some(count) => count,
                None => g
                    .to_partial_guild_with_counts(http)
                    .await
                    .ok()
                    .and_then(|g| g.approximate_member_count)
                    .unwrap_or_default(),
            }
        })
        .collect();
    let mut guild_member_sum = 0;
    while let Some(count) = guilds.next().await {
        guild_member_sum += count;
    }
    guild_member_sum
}

pub async fn increment_command_invokes(client: &DiscordClient) {
    let redis = client.redis();
    let result = redis.clone().incr(to_key("cmd_calls", redis), 1).await;
    let _: () = match result {
        Ok(e) => e,
        Err(e) => {
            log::error!("Redis increment error: {:?}", e);
        }
    };
}

pub async fn command_invokes(client: &DiscordClient) -> u64 {
    let redis = client.redis();
    redis
        .clone()
        .get(to_key("cmd_calls", redis))
        .await
        .unwrap_or_default()
}
