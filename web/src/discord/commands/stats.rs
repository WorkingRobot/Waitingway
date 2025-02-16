use poise::CreateReply;
use serenity::all::{
    Color, CreateEmbed, CreateEmbedFooter, FormattedTimestamp, FormattedTimestampStyle,
};
use thousands::Separable;
use time::OffsetDateTime;
use titlecase::titlecase;

use crate::{
    discord::utils::{format_duration, format_latency, install_stats, member_count, COLOR_SUCCESS},
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
    let cache = ctx.cache();
    let latency = time::Duration::try_from(ctx.ping().await);
    let (servers, users) = install_stats(ctx.data()).await;
    let members = member_count(ctx.data()).await;

    let embed = CreateEmbed::new()
        .title("Waitingway Statistics")
        .field("Servers", servers.separate_with_commas(), true)
        .field("Users", members.separate_with_commas(), true)
        .field("Installed Users", users.separate_with_commas(), true)
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
