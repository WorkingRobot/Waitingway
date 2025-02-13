use crate::discord::{
    utils::{format_duration, format_queue_duration, COLOR_ERROR, COLOR_IN_QUEUE, COLOR_SUCCESS},
    DiscordClient,
};
use actix_web::Result;
use serenity::all::{
    ChannelId, CreateEmbed, CreateEmbedFooter, CreateMessage, EditMessage, FormattedTimestamp,
    FormattedTimestampStyle, Message, MessageId, Timestamp, UserId,
};
use time::{Duration, OffsetDateTime};

pub async fn send_queue_position(
    discord: &DiscordClient,
    user_id: UserId,
    character_name: &str,
    position: u32,
    now: time::OffsetDateTime,
    estimated: time::OffsetDateTime,
) -> Result<Message, serenity::Error> {
    let channel = user_id.create_dm_channel(discord.http()).await?;

    channel
        .send_message(
            discord.http(),
            CreateMessage::new().embed(create_queue_embed(
                character_name,
                position,
                now,
                estimated,
            )),
        )
        .await
}

pub async fn update_queue_position(
    discord: &DiscordClient,
    message_id: MessageId,
    channel_id: ChannelId,
    character_name: &str,
    position: u32,
    now: time::OffsetDateTime,
    estimated: time::OffsetDateTime,
) -> Result<(), serenity::Error> {
    channel_id
        .edit_message(
            discord.http(),
            message_id,
            EditMessage::new().embed(create_queue_embed(character_name, position, now, estimated)),
        )
        .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn send_queue_completion(
    discord: &DiscordClient,
    message_id: MessageId,
    channel_id: ChannelId,
    character_name: &str,
    queue_start_size: u32,
    queue_end_size: u32,
    duration: Duration,
    error_message: Option<String>,
    error_code: Option<i32>,
    identify_timeout: Option<time::OffsetDateTime>,
    successful: bool,
) -> Result<(), serenity::Error> {
    let delete_client = discord.clone();
    let delete_task = tokio::task::spawn(async move {
        channel_id
            .delete_message(delete_client.http(), message_id)
            .await
    });

    if successful {
        send_queue_completion_successful(
            discord,
            channel_id,
            character_name,
            queue_start_size,
            duration,
        )
        .await?;
    } else {
        send_queue_completion_unsuccessful(
            discord,
            channel_id,
            character_name,
            queue_start_size,
            queue_end_size,
            duration,
            error_message,
            error_code,
            identify_timeout,
        )
        .await?;
    }

    delete_task
        .await
        .map_err(|_| serenity::Error::Other("Failed to delete message"))??;

    Ok(())
}

async fn send_queue_completion_successful(
    discord: &DiscordClient,
    channel: ChannelId,
    character_name: &str,
    queue_start_size: u32,
    duration: Duration,
) -> Result<(), serenity::Error> {
    let embed = CreateEmbed::new()
            .title("Queue completed!")
            .description(format!("{} has been logged in successfully! Thanks for using Waitingway!\n\nYour queue size was {}, which was completed in {}.", character_name, queue_start_size, format_duration(duration)))
            .footer(CreateEmbedFooter::new("At"))
            .timestamp(OffsetDateTime::now_utc())
            .color(COLOR_SUCCESS);

    channel
        .send_message(discord.http(), CreateMessage::new().embed(embed))
        .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn send_queue_completion_unsuccessful(
    discord: &DiscordClient,
    channel: ChannelId,
    character_name: &str,
    queue_start_size: u32,
    queue_end_size: u32,
    duration: Duration,
    error_message: Option<String>,
    error_code: Option<i32>,
    identify_timeout: Option<time::OffsetDateTime>,
) -> Result<(), serenity::Error> {
    let mut description = if let Some(identify_timeout) = identify_timeout {
        let identify_timeout: Timestamp = identify_timeout.into();
        format!(
                    "{} left the queue prematurely. If you didn't mean to, try queueing again by {} ({}) to not lose your spot.\n",
                    character_name,
                    FormattedTimestamp::new(identify_timeout, Some(FormattedTimestampStyle::LongTime)),
                    FormattedTimestamp::new(identify_timeout, Some(FormattedTimestampStyle::RelativeTime)),
                )
    } else {
        format!("{character_name} left the queue prematurely. If you didn't mean to, try queueing again.\n")
    };
    if let Some(error_message) = error_message {
        if let Some(error_code) = error_code {
            description.push_str(format!("Error: {error_message} ({error_code})\n").as_str());
        }
    }
    description.push('\n');
    description.push_str(
        if queue_start_size == queue_end_size {
            format!(
                "Your queue size was {}, and you were in queue for {}.",
                queue_start_size,
                format_queue_duration(duration)
            )
        } else {
            format!(
                "Your queue size started at {} and ended at {}, and you were in queue for {}.",
                queue_start_size,
                queue_end_size,
                format_queue_duration(duration)
            )
        }
        .as_str(),
    );
    let embed = CreateEmbed::new()
        .title("Unsuccessful Queue")
        .description(description)
        .footer(CreateEmbedFooter::new("At"))
        .timestamp(OffsetDateTime::now_utc())
        .color(COLOR_ERROR);

    channel
        .send_message(discord.http(), CreateMessage::new().embed(embed))
        .await?;
    Ok(())
}

fn create_queue_embed(
    character_name: &str,
    position: u32,
    now: time::OffsetDateTime,
    estimated: time::OffsetDateTime,
) -> CreateEmbed {
    let estimated: Timestamp = estimated.into();
    CreateEmbed::new()
            .title(format!("{character_name}'s Queue"))
            .description(format!(
                "You're in position {}. You'll login {} ({})\n\nYou'll receive a DM from me when your queue completes.",
                position,
                FormattedTimestamp::new(estimated, Some(FormattedTimestampStyle::RelativeTime)),
                FormattedTimestamp::new(estimated, Some(FormattedTimestampStyle::LongTime)),
            ))
            .footer(CreateEmbedFooter::new("Last updated"))
            .timestamp(now)
            .color(COLOR_IN_QUEUE)
}
