use crate::{
    config::Config,
    discord::{
        notifications::duty::{self as notifs, QueueData},
        DiscordClient,
    },
    models::duty::RecapUpdate,
    routes::api::notifications::{impl_notification_instance, NotificationInstance},
    storage::db::wrappers::DatabaseDateTime,
};
use actix_web::{
    dev::{HttpServiceFactory, Payload},
    web, FromRequest, HttpRequest, Result,
};
use serde::{Deserialize, Serialize};
use serenity::{
    all::{ChannelId, Message, MessageId, UserId},
    async_trait,
};
use uuid::Uuid;

pub fn service() -> impl HttpServiceFactory {
    web::scope("/notifications").service(InstanceData::service())
}

#[derive(Clone, Debug, Deserialize)]
struct CreateData {
    #[serde(flatten)]
    pub data: QueueData,
    pub update: RecapUpdate,
    pub estimated_time: Option<DatabaseDateTime>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum UpdateData {
    Queue {
        estimated_time: Option<DatabaseDateTime>,
        update: RecapUpdate,
    },
    Pop {
        timestamp: DatabaseDateTime,
        resulting_content: Option<u16>,
        #[serde(rename = "in_progress_begin_timestamp")]
        in_progress_timestamp: Option<DatabaseDateTime>,
    },
}

#[derive(Clone, Debug, Deserialize)]
struct DeleteData {
    pub position_start: Option<u8>,
    pub position_end: Option<u8>,
    #[serde(rename = "duration")]
    pub duration_secs: u32,
    pub resulting_content: Option<u16>,
    pub error_message: Option<String>,
    pub error_code: Option<u16>,
}

#[derive(Debug, Deserialize, Serialize)]
struct InstanceData {
    pub username: Uuid,
    pub messages: Vec<(MessageId, ChannelId)>,
    pub start_time: time::OffsetDateTime,
    pub data: QueueData,
}

#[async_trait]
impl NotificationInstance for InstanceData {
    type CreateData = CreateData;
    type UpdateData = UpdateData;
    type DeleteData = DeleteData;

    fn new(username: Uuid, messages: Vec<(MessageId, ChannelId)>, data: &Self::CreateData) -> Self {
        Self {
            username,
            messages,
            start_time: data.update.time.0,
            data: data.data.clone(),
        }
    }

    fn username(&self) -> &Uuid {
        &self.username
    }

    fn messages(&self) -> &Vec<(MessageId, ChannelId)> {
        &self.messages
    }

    fn passes_threshold(_data: &CreateData, _config: &Config) -> bool {
        true
        // data.update_data.position < config.discord.queue_size_dm_threshold
    }

    async fn dispatch_create(
        data: &CreateData,
        discord: &DiscordClient,
        id: UserId,
    ) -> Result<Message, serenity::Error> {
        notifs::send_create(
            discord,
            id,
            &data.data,
            &data.update,
            data.estimated_time.map(|t| t.0),
        )
        .await
    }

    async fn dispatch_update(
        &self,
        data: &UpdateData,
        discord: &DiscordClient,
        message: MessageId,
        channel: ChannelId,
    ) -> Result<(), serenity::Error> {
        match data {
            UpdateData::Queue {
                estimated_time,
                update,
            } => {
                notifs::send_update(
                    discord,
                    message,
                    channel,
                    &self.data,
                    update,
                    self.start_time,
                    estimated_time.map(|t| t.0),
                )
                .await?;
            }
            UpdateData::Pop {
                timestamp,
                resulting_content,
                in_progress_timestamp,
            } => {
                notifs::send_pop(
                    discord,
                    message,
                    channel,
                    &self.data,
                    timestamp.0,
                    *resulting_content,
                    in_progress_timestamp.map(|t| t.0),
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn dispatch_delete(
        &self,
        data: &DeleteData,
        discord: &DiscordClient,
        message: MessageId,
        channel: ChannelId,
    ) -> Result<(), serenity::Error> {
        notifs::send_delete(
            discord,
            message,
            channel,
            &self.data,
            data.position_start.map(|p| p.into()),
            data.position_end.map(|p| p.into()),
            time::Duration::new(data.duration_secs.into(), 0),
            data.resulting_content,
            data.error_message.clone(),
            data.error_code,
        )
        .await
    }
}

impl_notification_instance!(InstanceData);
