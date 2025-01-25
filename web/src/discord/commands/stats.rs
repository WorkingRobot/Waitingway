use futures_util::{stream::FuturesUnordered, StreamExt};
use poise::CreateReply;
use serenity::all::{
    Color, CreateEmbed, CreateEmbedFooter, FormattedTimestamp, FormattedTimestampStyle,
};
use time::OffsetDateTime;
use titlecase::titlecase;

use crate::{
    discord::utils::{format_duration, format_latency, COLOR_SUCCESS},
    natives::{self, VERSION_DATA},
};

use super::Context;
use super::Error;

/// Get some statistics about Waitingway
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn stats(ctx: Context<'_>) -> Result<(), Error> {
    let http = ctx.http();
    let cache = ctx.cache();
    let latency = time::Duration::try_from(ctx.ping().await);
    let app_info = ctx.http().get_current_application_info().await?;

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

    let embed = CreateEmbed::new()
        .title("Waitingway Statistics")
        .field(
            "Servers",
            app_info
                .approximate_guild_count
                .unwrap_or_default()
                .to_string(),
            true,
        )
        .field("Users", guild_member_sum.to_string(), true)
        .field(
            "Installed Users",
            app_info
                .approximate_user_install_count
                .unwrap_or_default()
                .to_string(),
            true,
        )
        .field(
            "Version",
            format!(
                "v{} ({})",
                VERSION_DATA.version,
                titlecase(VERSION_DATA.profile)
            ),
            true,
        )
        .field("OS Version", os_info::get().to_string(), true)
        .field(
            "Started At",
            FormattedTimestamp::new(
                natives::process_start_time()?.into(),
                Some(FormattedTimestampStyle::RelativeTime),
            )
            .to_string(),
            true,
        )
        .field("Uptime", format_duration(natives::process_uptime()?), true)
        .field(
            "Shard",
            format!(
                "{} / {}",
                ctx.serenity_context().shard_id.get() + 1,
                cache.shard_count()
            ),
            true,
        )
        .field(
            "Discord Latency",
            latency.map_or("Unknown".to_string(), format_latency),
            true,
        )
        .thumbnail(cache.current_user().face())
        .footer(CreateEmbedFooter::new("Last updated"))
        .timestamp(OffsetDateTime::now_utc())
        .color(Color::from(COLOR_SUCCESS));

    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}
