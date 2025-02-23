use crate::{
    config::Config,
    discord::{notifications::login as notifs, DiscordClient},
    routes::api::notifications::{impl_notification_instance, NotificationInstance},
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
    pub character_name: String,
    pub home_world_id: u16,
    pub world_id: u16,
    #[serde(flatten)]
    pub update_data: UpdateData,
}

#[derive(Clone, Debug, Deserialize)]
struct UpdateData {
    pub position: u32,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub estimated_time: time::OffsetDateTime,
}

#[derive(Clone, Debug, Deserialize)]
struct DeleteData {
    pub successful: bool,
    pub queue_start_size: u32,
    pub queue_end_size: u32,
    #[serde(rename = "duration")]
    pub duration_secs: u32,
    pub error_message: Option<String>,
    pub error_code: Option<i32>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub identify_timeout: Option<time::OffsetDateTime>,
}

#[derive(Debug, Deserialize, Serialize)]
struct InstanceData {
    pub username: Uuid,
    pub messages: Vec<(MessageId, ChannelId)>,
    pub character_name: String,
    pub home_world_id: u16,
    pub world_id: u16,
}

#[async_trait]
impl NotificationInstance for InstanceData {
    type CreateData = CreateData;
    type UpdateData = UpdateData;
    type DeleteData = DeleteData;

    fn new(username: Uuid, messages: Vec<(MessageId, ChannelId)>, data: &Self::CreateData) -> Self {
        Self {
            username,
            character_name: data.character_name.clone(),
            home_world_id: data.home_world_id,
            world_id: data.world_id,
            messages,
        }
    }

    fn username(&self) -> &Uuid {
        &self.username
    }

    fn messages(&self) -> &Vec<(MessageId, ChannelId)> {
        &self.messages
    }

    fn passes_threshold(data: &CreateData, config: &Config) -> bool {
        data.update_data.position >= config.discord.queue_size_dm_threshold
    }

    async fn dispatch_create(
        data: &CreateData,
        discord: &DiscordClient,
        id: UserId,
    ) -> Result<Message, serenity::Error> {
        notifs::send_queue_position(
            discord,
            id,
            &data.character_name,
            data.update_data.position,
            data.update_data.updated_at,
            data.update_data.estimated_time,
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
        notifs::update_queue_position(
            discord,
            message,
            channel,
            &self.character_name,
            data.position,
            data.updated_at,
            data.estimated_time,
        )
        .await
    }

    async fn dispatch_delete(
        &self,
        data: &DeleteData,
        discord: &DiscordClient,
        message: MessageId,
        channel: ChannelId,
    ) -> Result<(), serenity::Error> {
        notifs::send_queue_completion(
            discord,
            message,
            channel,
            &self.character_name,
            data.queue_start_size,
            data.queue_end_size,
            time::Duration::new(data.duration_secs.into(), 0),
            data.error_message.clone(),
            data.error_code,
            data.identify_timeout,
            data.successful,
        )
        .await
    }
}

impl_notification_instance!(InstanceData);
