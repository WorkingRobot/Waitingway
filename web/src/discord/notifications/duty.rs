use crate::{
    config::DiscordEmoteConfig,
    discord::{
        utils::{
            format_duration_duty_eta, format_queue_duration, COLOR_ERROR, COLOR_IN_QUEUE,
            COLOR_QUEUE_POP, COLOR_SUCCESS,
        },
        DiscordClient,
    },
    models::{
        duty::{FillParam, RecapUpdate, RecapUpdateData, RoulettePosition, WaitTime},
        job_info::JobInfo,
    },
    storage::{
        db::wrappers::DatabaseU16,
        game::{content, get_icon_url, get_icon_url_from_id, jobs},
    },
};
use actix_web::Result;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serenity::all::{
    ChannelId, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter, CreateMessage, EditMessage,
    FormattedTimestamp, FormattedTimestampStyle, Message, MessageId, Timestamp, UserId,
};
use sqlx::Either;
use time::{Duration, OffsetDateTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueData {
    pub character_name: String,
    pub home_world_id: u16,
    pub queued_job: u8,
    pub queued_roulette: Option<u8>,
    pub queued_content: Option<Vec<u16>>,
}

impl QueueData {
    pub fn job(&self) -> &JobInfo {
        jobs::get_data()
            .get_job_by_id(self.queued_job)
            .expect("Invalid job ID")
    }

    pub fn queue_name(&self, expand: bool) -> String {
        let c = content::get_data();

        if let Some(names) = self
            .queued_roulette
            .map(|r| vec![c.get_roulette_name(r)])
            .or_else(|| {
                self.queued_content
                    .as_ref()
                    .map(|v| v.iter().map(|r| c.get_content_name(*r)).collect_vec())
            })
            .filter(|c| !c.is_empty())
        {
            if names.len() == 1 {
                names.first().unwrap().to_string()
            } else if !expand {
                format!("{} and {} more", names.first().unwrap(), names.len() - 1)
            } else if names.len() == 2 {
                names.join(" and ")
            } else {
                let mut iter = names.into_iter();
                let last = iter.next_back().unwrap();
                format!("{}, and {}", iter.join(", "), last)
            }
        } else {
            "Unknown".to_string()
        }
    }

    pub fn queue_image(&self) -> String {
        self.queued_roulette
            .map(|r| content::get_data().get_roulette_image(r))
            .or_else(|| {
                self.queued_content
                    .as_ref()
                    .and_then(|v| v.first().map(|r| content::get_data().get_content_image(*r)))
            })
            .unwrap_or_else(|| content::ContentData::DEFAULT_IMAGE.to_string())
    }

    pub fn embed_author(&self) -> CreateEmbedAuthor {
        CreateEmbedAuthor::new(&self.character_name)
            .icon_url(get_icon_url_from_id(self.job().icon_id()))
    }
}

fn format_position(pos: RoulettePosition) -> String {
    match pos {
        RoulettePosition::Position(p) => p.to_string(),
        RoulettePosition::After50 => "50+".to_string(),
        RoulettePosition::RetrievingInfo => "unknown".to_string(),
    }
}

pub async fn send_create(
    discord: &DiscordClient,
    user_id: UserId,
    queue_data: &QueueData,
    update: &RecapUpdate,
    estimated: Option<time::OffsetDateTime>,
) -> Result<Message, serenity::Error> {
    let channel = user_id.create_dm_channel(discord.http()).await?;

    channel
        .send_message(
            discord.http(),
            CreateMessage::new().embed(create_queue_embed(
                &discord.config().emotes,
                queue_data,
                update.time.0,
                update,
                estimated,
            )),
        )
        .await
}

pub async fn send_update(
    discord: &DiscordClient,
    message_id: MessageId,
    channel_id: ChannelId,
    queue_data: &QueueData,
    update: &RecapUpdate,
    start_time: time::OffsetDateTime,
    estimated: Option<time::OffsetDateTime>,
) -> Result<(), serenity::Error> {
    channel_id
        .edit_message(
            discord.http(),
            message_id,
            EditMessage::new().embed(create_queue_embed(
                &discord.config().emotes,
                queue_data,
                start_time,
                update,
                estimated,
            )),
        )
        .await?;
    Ok(())
}

pub async fn send_pop(
    discord: &DiscordClient,
    _message_id: MessageId,
    channel_id: ChannelId,
    queue_data: &QueueData,
    timestamp: time::OffsetDateTime,
    resulting_content: Option<u16>,
    in_progress_timestamp: Option<time::OffsetDateTime>,
) -> Result<Message, serenity::Error> {
    channel_id
        .send_message(
            discord.http(),
            CreateMessage::new().embed(create_pop_embed(
                queue_data,
                timestamp,
                resulting_content,
                in_progress_timestamp,
            )),
        )
        .await
}

#[allow(clippy::too_many_arguments)]
pub async fn send_delete(
    discord: &DiscordClient,
    message_id: MessageId,
    channel_id: ChannelId,
    queue_data: &QueueData,
    position_start: Option<RoulettePosition>,
    position_end: Option<RoulettePosition>,
    duration: Duration,
    resulting_content: Option<u16>,
    error_message: Option<String>,
    error_code: Option<u16>,
) -> Result<(), serenity::Error> {
    channel_id
        .edit_message(
            discord.http(),
            message_id,
            EditMessage::new().embed(create_completion_embed(
                queue_data,
                position_start,
                position_end,
                duration,
                resulting_content,
                match (error_message, error_code) {
                    (Some(message), Some(code)) => Some((message, code)),
                    _ => None,
                },
            )),
        )
        .await?;
    Ok(())
}

fn create_pop_embed(
    queue_data: &QueueData,
    timestamp: time::OffsetDateTime,
    resulting_content: Option<u16>,
    in_progress_timestamp: Option<time::OffsetDateTime>,
) -> CreateEmbed {
    let mut msg = String::new();

    msg.push_str("This queue pop ");
    if let Some(content) = resulting_content {
        msg.push_str(format!("for {}", content::get_data().get_content_name(content),).as_str());
    }

    msg.push_str(
        format!(
            " expires {}.",
            FormattedTimestamp::new(
                (timestamp + Duration::seconds(45)).into(),
                Some(FormattedTimestampStyle::RelativeTime)
            )
        )
        .as_str(),
    );

    if let Some(in_progress_timestamp) = in_progress_timestamp {
        msg.push_str(
            format!(
                "\nYou will be joining an in-progress duty that began {}",
                FormattedTimestamp::new(
                    in_progress_timestamp.into(),
                    Some(FormattedTimestampStyle::RelativeTime)
                ),
            )
            .as_str(),
        );
    }

    let ret = CreateEmbed::new()
        .title("Queue popped!")
        .description(msg)
        .author(queue_data.embed_author())
        .footer(CreateEmbedFooter::new("At"))
        .timestamp(timestamp)
        .color(COLOR_QUEUE_POP);

    if let Some(content) = resulting_content {
        ret.image(content::get_data().get_content_image(content))
    } else {
        ret
    }
}

fn create_completion_embed(
    queue_data: &QueueData,
    position_start: Option<RoulettePosition>,
    position_end: Option<RoulettePosition>,
    duration: Duration,
    resulting_content: Option<u16>,
    error: Option<(String, u16)>,
) -> CreateEmbed {
    match resulting_content {
        Some(content) => {
            create_completion_embed_successful(queue_data, position_start, content, duration)
        }
        None => create_completion_embed_unsuccessful(
            queue_data,
            match (position_start, position_end) {
                (Some(start), Some(end)) => Some(Either::Right((start, end))),
                (Some(start), None) => Some(Either::Left(start)),
                _ => None,
            },
            duration,
            error,
        ),
    }
}

fn create_completion_embed_successful(
    queue_data: &QueueData,
    start_position: Option<RoulettePosition>,
    content: u16,
    duration: Duration,
) -> CreateEmbed {
    let mut msg = format!(
        "You've entered {}! Thanks for using Waitingway!\n\n",
        content::get_data().get_content_name(content)
    );

    if queue_data.queued_roulette.is_some()
        || queue_data
            .queued_content
            .as_ref()
            .is_some_and(|q| q.len() > 1)
    {
        msg.push_str(format!("You were in queue for {}.\n", queue_data.queue_name(true)).as_str());
    }

    if let Some(position) = start_position {
        msg.push_str(
            format!(
                "Your queue size was {}, which was completed in {}.",
                format_position(position),
                format_queue_duration(duration),
            )
            .as_str(),
        );
    } else {
        msg.push_str(
            format!(
                "Your queue was completed in {}.",
                format_queue_duration(duration),
            )
            .as_str(),
        );
    }

    CreateEmbed::new()
        .title("Queue completed!")
        .description(msg)
        .image(get_icon_url(
            &content::get_data().get_content_image(content),
        ))
        .author(queue_data.embed_author())
        .footer(CreateEmbedFooter::new("At"))
        .timestamp(OffsetDateTime::now_utc())
        .color(COLOR_SUCCESS)
}

fn create_completion_embed_unsuccessful(
    queue_data: &QueueData,
    // Start => Queue popped; (Start, End) => No pop yet
    position: Option<Either<RoulettePosition, (RoulettePosition, RoulettePosition)>>,
    duration: Duration,
    error: Option<(String, u16)>,
) -> CreateEmbed {
    let mut msg = if let Some((message, _code)) = error {
        message
    } else {
        "You left the queue!".to_string()
    };

    msg.push_str("\n\n");

    let position = position.map(|p| match p {
        Either::Right((pos, end)) if pos == end => Either::Left(pos),
        _ => p,
    });
    match position {
        Some(Either::Right((start, end))) => {
            msg.push_str(
                format!(
                    "Your position started at {} and ended at {}, and you spent {} in queue.",
                    format_position(start),
                    format_position(end),
                    format_queue_duration(duration)
                )
                .as_str(),
            );
        }
        Some(Either::Left(pos)) => {
            msg.push_str(
                format!(
                    "You were in position {}, and spent {} in queue.",
                    format_position(pos),
                    format_queue_duration(duration)
                )
                .as_str(),
            );
        }
        _ => {
            msg.push_str(
                format!("You spent {} in queue.", format_queue_duration(duration)).as_str(),
            );
        }
    }

    msg.push_str(format!("\nYou were in queue for {}.", queue_data.queue_name(true)).as_str());

    CreateEmbed::new()
        .title("Unsuccessful Queue")
        .description(msg)
        .image(get_icon_url(&queue_data.queue_image()))
        .author(queue_data.embed_author())
        .footer(CreateEmbedFooter::new("At"))
        .timestamp(OffsetDateTime::now_utc())
        .color(COLOR_ERROR)
}

fn format_update(
    config: &DiscordEmoteConfig,
    update: &RecapUpdate,
    start_time: time::OffsetDateTime,
    estimated: Option<time::OffsetDateTime>,
) -> String {
    let reported_estimate = match update.update_data {
        Some(
            RecapUpdateData::WaitTime { wait_time }
            | RecapUpdateData::Players { wait_time, .. }
            | RecapUpdateData::Thd { wait_time, .. }
            | RecapUpdateData::Roulette { wait_time, .. },
        ) => Some(wait_time),
        _ => None,
    };
    let wait_time_sentence = reported_estimate.map(|e| match e {
        WaitTime::Hidden => "The game-reported ETA is unknown.".to_string(),
        WaitTime::Minutes(mins) => {
            format!(
                "The game-reported ETA is **{}**.",
                format_duration_duty_eta(Duration::minutes(mins.into()))
            )
        }
        WaitTime::Over30Minutes => "The game-reported ETA is **30m+**.".to_string(),
    });

    let position = match update.update_data {
        Some(RecapUpdateData::Roulette { position, .. }) => Some(position),
        _ => None,
    };
    let position_sentence = position.map(|p| match p {
        RoulettePosition::RetrievingInfo => "Your position is currently unknown.".to_string(),
        RoulettePosition::Position(position) => format!("You're in position **{}**.", position),
        RoulettePosition::After50 => "You're in position **50+**.".to_string(),
    });

    let fill_param_field = match update.update_data {
        Some(RecapUpdateData::Thd {
            tanks:
                FillParam {
                    found: DatabaseU16(tanks_found),
                    needed: DatabaseU16(tanks_total),
                },
            healers:
                FillParam {
                    found: DatabaseU16(healers_found),
                    needed: DatabaseU16(healers_total),
                },
            dps:
                FillParam {
                    found: DatabaseU16(dps_found),
                    needed: DatabaseU16(dps_total),
                },
            ..
        }) => Some(format!(
            "{} {}/{} {} {}/{} {} {}/{}",
            config.duty_tank,
            tanks_found,
            tanks_total,
            config.duty_healer,
            healers_found,
            healers_total,
            config.duty_dps,
            dps_found,
            dps_total
        )),
        Some(RecapUpdateData::Players {
            players:
                FillParam {
                    found: DatabaseU16(players_found),
                    needed: DatabaseU16(players_total),
                },
            ..
        }) => Some(format!(
            "{} {}/{}",
            config.duty_player, players_found, players_total
        )),
        _ => None,
    };

    let estimated: Option<Timestamp> = estimated
        .or_else(|| {
            if let Some(WaitTime::Minutes(mins)) = reported_estimate {
                Some((start_time + Duration::minutes(mins.into())).into())
            } else {
                None
            }
        })
        .map(|e| e.into());
    let estimated_sentence = estimated.map(|e| {
        format!(
            "Your queue will pop {} ({}).",
            FormattedTimestamp::new(e, Some(FormattedTimestampStyle::RelativeTime)),
            FormattedTimestamp::new(e, Some(FormattedTimestampStyle::ShortTime)),
        )
    });

    let elapsed_sentence = format!(
        "You began your queue {}.",
        FormattedTimestamp::new(
            start_time.into(),
            Some(FormattedTimestampStyle::RelativeTime)
        )
    );

    vec![
        position_sentence,
        wait_time_sentence,
        fill_param_field,
        estimated_sentence,
        Some(elapsed_sentence),
        Some("\nYou'll receive a DM from me when your queue pops.".to_string()),
    ]
    .into_iter()
    .flatten()
    .join("\n")
}

fn create_queue_embed(
    config: &DiscordEmoteConfig,
    queue_data: &QueueData,
    start_time: time::OffsetDateTime,
    update: &RecapUpdate,
    estimated: Option<time::OffsetDateTime>,
) -> CreateEmbed {
    CreateEmbed::new()
        .title(queue_data.queue_name(false))
        .description(format_update(config, update, start_time, estimated))
        .image(get_icon_url(&queue_data.queue_image()))
        .author(queue_data.embed_author())
        .footer(CreateEmbedFooter::new("Last updated"))
        .timestamp(Timestamp::from(update.time.0))
        .color(COLOR_IN_QUEUE)
}
