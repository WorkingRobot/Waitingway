use crate::{
    config::DiscordConfig,
    discord::utils::{
        format_queue_duration, COLOR_DC_ALLOWED, COLOR_DC_MIXED, COLOR_DC_PROHIBITED,
    },
    models::login::QueueEstimate,
    storage::game::worlds::{self, World},
};
use ::serenity::all::{
    Color, CreateEmbed, CreateEmbedFooter, FormattedTimestamp, FormattedTimestampStyle,
};
use itertools::Itertools;
use poise::serenity_prelude as serenity;
use time::OffsetDateTime;

use super::Context;

pub async fn autocomplete_world<'a>(
    _ctx: Context<'_>,
    query: &'a str,
) -> impl Iterator<Item = serenity::AutocompleteChoice> + 'a {
    worlds::get_data()
        .find_best_world_match(query)
        .into_iter()
        .map(|dc| serenity::AutocompleteChoice::new(dc.to_string(), dc.id))
}

pub fn create_travel_embed(
    name: &str,
    worlds: Vec<(&World, bool)>,
    config: &DiscordConfig,
) -> CreateEmbed {
    let color = match worlds.iter().filter(|(_, s)| *s).count() {
        0 => COLOR_DC_ALLOWED,
        n if n == worlds.len() => COLOR_DC_PROHIBITED,
        _ => COLOR_DC_MIXED,
    };

    let embed = CreateEmbed::new().title(format!("DC Travel for {name}"));

    let embed = if worlds.len() == 1 {
        let is_prohibited = worlds.first().expect("worlds is not empty").1;
        embed.description(format_travel_status(is_prohibited, config))
    } else {
        embed.fields(
            worlds
                .into_iter()
                .sorted_unstable_by_key(|(world, _)| world.id)
                .map(|(world, is_prohibited)| {
                    (
                        world.name.clone(),
                        format_travel_status(is_prohibited, config),
                        true,
                    )
                }),
        )
    };

    embed
        .footer(CreateEmbedFooter::new("Last updated"))
        .timestamp(OffsetDateTime::now_utc())
        .color(color)
}

fn format_travel_status(is_prohibited: bool, config: &DiscordConfig) -> String {
    format!(
        "{} {}",
        if !is_prohibited {
            &config.green_check_emoji
        } else {
            &config.red_cross_emoji
        },
        if !is_prohibited {
            "Allowed"
        } else {
            "Prohibited"
        }
    )
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum QueueColor {
    #[default]
    Green,
    Orange,
    Yellow,
    Red,
}

impl From<QueueColor> for Color {
    fn from(color: QueueColor) -> Self {
        match color {
            QueueColor::Green => COLOR_DC_ALLOWED,
            QueueColor::Orange => Color::ORANGE,
            QueueColor::Yellow => COLOR_DC_MIXED,
            QueueColor::Red => COLOR_DC_PROHIBITED,
        }
    }
}

impl QueueColor {
    fn from_estimate(estimate: &QueueEstimate) -> Self {
        let a = Self::from_duration(time::Duration::seconds_f64(estimate.last_duration));
        let b = Self::from_size(estimate.last_size);
        std::cmp::max(a, b)
    }

    fn from_duration(duration: time::Duration) -> Self {
        match duration {
            d if d < time::Duration::seconds(90) => QueueColor::Green,
            d if d < time::Duration::minutes(5) => QueueColor::Orange,
            d if d < time::Duration::minutes(10) => QueueColor::Yellow,
            _ => QueueColor::Red,
        }
    }

    fn from_size(size: i32) -> Self {
        match size {
            s if s < 100 => QueueColor::Green,
            s if s < 500 => QueueColor::Orange,
            s if s < 1000 => QueueColor::Yellow,
            _ => QueueColor::Red,
        }
    }
}

pub fn create_queue_embed(name: &str, worlds: Vec<(&World, QueueEstimate)>) -> CreateEmbed {
    let color = worlds
        .iter()
        .map(|(_, e)| QueueColor::from_estimate(e))
        .max()
        .unwrap_or_default();

    let embed = CreateEmbed::new().title(format!("Queue Times for {name}"));

    let embed = if worlds.len() == 1 {
        let estimate = &worlds.first().expect("worlds is not empty").1;
        embed
            .description(format_queue_time(estimate, false))
            .timestamp(estimate.last_update.0)
    } else {
        embed
            .fields(
                worlds
                    .into_iter()
                    .sorted_unstable_by_key(|(world, _)| world.id)
                    .map(|(world, estimate)| {
                        (world.name.clone(), format_queue_time(&estimate, true), true)
                    }),
            )
            .timestamp(OffsetDateTime::now_utc())
    };

    embed
        .footer(CreateEmbedFooter::new("Last updated"))
        .color(Color::from(color))
}

fn format_queue_time(estimate: &QueueEstimate, add_updated: bool) -> String {
    let mut result = if estimate.last_duration == 0f64 && estimate.last_size == 0 {
        "Instant".to_string()
    } else {
        format!(
            "Size: {}\nTime: {}",
            estimate.last_size,
            format_queue_duration(time::Duration::seconds_f64(estimate.last_duration))
        )
    };
    if add_updated {
        result.push_str(&format!(
            "\nUpdated {}",
            FormattedTimestamp::new(
                estimate.last_update.0.into(),
                Some(FormattedTimestampStyle::RelativeTime)
            ),
        ));
    }
    result
}
