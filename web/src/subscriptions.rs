use crate::{
    config::RedisConfig,
    discord::{
        commands::create_travel_embed,
        travel_param::{TravelDatacenterParam, TravelWorldParam},
        utils::COLOR_SUCCESS,
        DiscordClient,
    },
    redis_utils::{RedisKey, RedisValue},
};
use futures_util::{stream, StreamExt};
use redis::{aio::ConnectionManager, AsyncCommands, Cmd};
use serde::{Deserialize, Serialize};
use serenity::all::{CreateMessage, UserId};
use std::{ops::Deref, sync::Arc};

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
        data: &'static TravelDatacenterParam,
        worlds: Vec<(&'static TravelWorldParam, bool)>,
    },
    World {
        id: u16,
        data: &'static TravelWorldParam,
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
    discord_client: DiscordClient,
    redis: ConnectionManager,
    redis_config: RedisConfig,
}

impl SubscriptionManager {
    pub fn new(discord_client: DiscordClient, redis_config: RedisConfig) -> Self {
        Self {
            imp: Arc::new(SubscriptionManagerImp {
                redis: discord_client.redis().clone(),
                discord_client,
                redis_config,
            }),
        }
    }

    #[must_use]
    fn redis(&self) -> ConnectionManager {
        self.imp.redis.clone()
    }

    fn redis_config(&self) -> &RedisConfig {
        &self.imp.redis_config
    }

    pub async fn subscribe(
        &self,
        endpoint: Endpoint,
        subscriber: Subscriber,
    ) -> Result<bool, Error> {
        let ret = self
            .redis()
            .sadd(
                endpoint.to_key(self.redis_config())?,
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
            .srem(
                endpoint.to_key(self.redis_config())?,
                subscriber.to_value()?,
            )
            .await?;
        if ret {
            log::info!("User {:?} unsubscribed from {:?}", subscriber, endpoint);
        }
        Ok(ret)
    }

    /// Publishing errors will be printed to the log.
    pub async fn publish_endpoint(
        &self,
        publish_data: impl Into<EndpointPublishData>,
    ) -> Result<(), Error> {
        let publish_data: EndpointPublishData = publish_data.into();
        let endpoint: Endpoint = (&*publish_data.0).into();

        let key = endpoint.to_key(self.redis_config())?;
        let mut redis = self.redis();

        const CHUNK_SIZE: usize = 32;
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
                        if let Err(e) = self.publish_to(&subscriber, data.0.deref()).await {
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
                let config = self.imp.discord_client.config();
                let (name, embed) = match publish_data {
                    EndpointPublish::Datacenter {
                        id: _,
                        data,
                        worlds,
                    } => (
                        &data.name(),
                        create_travel_embed(&data.name(), worlds.clone(), config),
                    ),
                    EndpointPublish::World { id: _, data } => (
                        &data.name(),
                        create_travel_embed(&data.name(), vec![(data, false)], config),
                    ),
                };
                let embed = embed
                    .title(format!("{name} is now available for DC Travel"))
                    .color(COLOR_SUCCESS);

                UserId::new(*user_id)
                    .dm(
                        &self.imp.discord_client.http(),
                        CreateMessage::new().embed(embed),
                    )
                    .await?;
            }
        };
        Ok(())
    }
}
