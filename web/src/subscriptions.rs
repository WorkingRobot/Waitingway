use crate::{
    discord::{commands::create_travel_embed, utils::COLOR_SUCCESS, DiscordClient},
    storage::RedisClient,
    storage::{RedisKey, RedisValue},
    worlds::{Datacenter, World},
};
use futures_util::{stream, StreamExt};
use redis::{AsyncCommands, Cmd};
use serde::{Deserialize, Serialize};
use serenity::all::{CreateMessage, UserId};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Serenity error")]
    Serenity(#[from] serenity::Error),
    #[error("Redis error")]
    Redis(#[from] redis::RedisError),
    #[error("Postcard error")]
    Postcard(#[from] postcard::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Subscriber {
    Discord(u64),
}

impl RedisValue for Subscriber {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Endpoint {
    Datacenter(u16),
    World(u16),
}

impl RedisKey for Endpoint {
    const PREFIX: &'static str = "subscriptions";
}

#[derive(Debug, Clone)]
pub struct EndpointPublishData(pub Arc<EndpointPublish>);

impl From<EndpointPublish> for EndpointPublishData {
    fn from(endpoint: EndpointPublish) -> Self {
        Self(Arc::new(endpoint))
    }
}

#[derive(Debug, Clone)]
pub enum EndpointPublish {
    Datacenter {
        id: u16,
        data: &'static Datacenter,
        worlds: Vec<(&'static World, bool)>,
    },
    World {
        id: u16,
        data: &'static World,
    },
}

impl From<&EndpointPublish> for Endpoint {
    fn from(endpoint: &EndpointPublish) -> Self {
        match endpoint {
            EndpointPublish::Datacenter { id, .. } => Endpoint::Datacenter(*id),
            EndpointPublish::World { id, .. } => Endpoint::World(*id),
        }
    }
}

#[derive(Clone)]
pub struct SubscriptionManager {
    imp: Arc<SubscriptionManagerImp>,
}

pub struct SubscriptionManagerImp {
    discord: DiscordClient,
    redis: RedisClient,
}

impl SubscriptionManager {
    pub fn new(discord: DiscordClient, redis: RedisClient) -> Self {
        Self {
            imp: Arc::new(SubscriptionManagerImp { discord, redis }),
        }
    }

    #[must_use]
    fn redis(&self) -> &RedisClient {
        &self.imp.redis
    }

    pub async fn subscribe(
        &self,
        endpoint: Endpoint,
        subscriber: Subscriber,
    ) -> Result<bool, Error> {
        let ret = self
            .redis()
            .clone()
            .sadd(
                endpoint.to_key(self.redis().config())?,
                subscriber.to_value()?,
            )
            .await?;
        if ret {
            log::info!("User {:?} subscribed to {:?}", subscriber, endpoint);
        }
        Ok(ret)
    }

    pub async fn unsubscribe(
        &self,
        endpoint: Endpoint,
        subscriber: &Subscriber,
    ) -> Result<bool, Error> {
        let ret = self
            .redis()
            .clone()
            .srem(
                endpoint.to_key(self.redis().config())?,
                subscriber.to_value()?,
            )
            .await?;
        if ret {
            log::info!("User {:?} unsubscribed from {:?}", subscriber, endpoint);
        }
        Ok(ret)
    }

    /// Publishing errors will be printed to the log.
    pub async fn publish_endpoint(&self, publish_data: EndpointPublish) -> Result<(), Error> {
        const CHUNK_SIZE: usize = 32;

        let publish_data: EndpointPublishData = publish_data.into();
        let endpoint: Endpoint = (&*publish_data.0).into();

        let key = endpoint.to_key(self.redis().config())?;
        let mut redis = self.redis().clone();

        loop {
            let subscribers: Vec<Vec<u8>> = Cmd::spop(key.clone())
                .arg(CHUNK_SIZE)
                .query_async(&mut redis)
                .await?;
            if subscribers.is_empty() {
                break;
            }
            let should_break = subscribers.len() < CHUNK_SIZE;
            stream::iter(subscribers)
                .for_each_concurrent(None, |subscriber| {
                    let data = publish_data.clone();
                    async move {
                        let subscriber = match Subscriber::from_value(&subscriber) {
                            Ok(subscriber) => subscriber,
                            Err(e) => {
                                log::error!(
                                    "Failed to deserialize subscriber: {} (data = {:?})",
                                    e,
                                    subscriber
                                );
                                return;
                            }
                        };
                        if let Err(e) = self.publish_to(&subscriber, &data.0).await {
                            log::error!("Failed to publish to {:?}: {}", subscriber, e);
                        }
                    }
                })
                .await;
            if should_break {
                break;
            }
        }

        Ok(())
    }

    async fn publish_to(
        &self,
        subscriber: &Subscriber,
        publish_data: &EndpointPublish,
    ) -> Result<(), Error> {
        match subscriber {
            Subscriber::Discord(user_id) => {
                let config = self.imp.discord.config();
                let (name, embed) = match publish_data {
                    EndpointPublish::Datacenter {
                        id: _,
                        data,
                        worlds,
                    } => (
                        &data.to_string(),
                        create_travel_embed(&data.to_string(), worlds.clone(), config),
                    ),
                    EndpointPublish::World { id: _, data } => (
                        &data.to_string(),
                        create_travel_embed(&data.to_string(), vec![(data, false)], config),
                    ),
                };
                let embed = embed
                    .title(format!("{name} is now available for DC Travel"))
                    .color(COLOR_SUCCESS);

                UserId::new(*user_id)
                    .dm(&self.imp.discord.http(), CreateMessage::new().embed(embed))
                    .await?;
            }
        };
        Ok(())
    }
}
