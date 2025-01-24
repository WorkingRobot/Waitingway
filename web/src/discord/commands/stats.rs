use futures_util::{stream::FuturesUnordered, StreamExt};
use poise::CreateReply;
use serenity::all::{CreateEmbed, FormattedTimestamp, FormattedTimestampStyle};
use titlecase::titlecase;

use crate::{
    discord::utils::{format_duration, format_latency},
    natives::{self, VERSION_DATA},
};

use super::Context;
use super::Error;

#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn stats(ctx: Context<'_>) -> Result<(), Error> {
    let http = ctx.http();
    let cache = ctx.cache();
    let latency = ctx
        .data()
        .client()
        .await
        .shard_manager
        .runners
        .lock()
        .await
        .get(&ctx.serenity_context().shard_id)
        .and_then(|r| r.latency)
        .and_then(|r| time::Duration::try_from(r).ok());
    let app_info = ctx.http().get_current_application_info().await?;
    let mut guilds: FuturesUnordered<_> = cache
        .guilds()
        .into_iter()
        .map(|g| async move {
            cache
                .guild(g)
                .map(|c| Ok(c.approximate_member_count.unwrap_or(c.member_count)))
                .unwrap_or(
                    g.to_partial_guild_with_counts(http)
                        .await
                        .map(|g| g.approximate_member_count.unwrap_or_default()),
                )
        })
        .collect();
    let mut guild_member_sum = 0;
    while let Some(count) = guilds.next().await {
        if let Ok(count) = count {
            guild_member_sum += count;
        }
    }

    let embed = CreateEmbed::new()
        .title("Waitingway Statistics")
        .field(
            "Server Count",
            app_info
                .approximate_guild_count
                .unwrap_or_default()
                .to_string(),
            true,
        )
        .field("User Count", guild_member_sum.to_string(), true)
        .field(
            "Installed User Count",
            app_info
                .approximate_user_install_count
                .unwrap_or_default()
                .to_string(),
            true,
        )
        .field(
            "Started At",
            FormattedTimestamp::new(
                natives::process_start_time()?.into(),
                Some(FormattedTimestampStyle::RelativeTime),
            )
            .to_string(),
            true,
        )
        .field(
            "Shard",
            format!(
                "{} / {}",
                ctx.serenity_context().shard_id.get(),
                cache.shard_count()
            ),
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
        .field("Uptime", format_duration(natives::process_uptime()?), true)
        .field(
            "Discord Latency",
            latency.map(format_latency).unwrap_or("Unknown".to_string()),
            true,
        );
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}
