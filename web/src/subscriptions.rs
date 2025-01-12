use crate::discord::{
    commands::create_travel_embed,
    travel_param::{TravelDatacenterParam, TravelWorldParam},
    utils::COLOR_SUCCESS,
    DiscordClient,
};
use dashmap::DashMap;
use futures_util::{stream, StreamExt};
use serenity::all::{CreateMessage, UserId};
use std::{
    collections::HashSet,
    ops::Deref,
    sync::{Arc, OnceLock},
};

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("Serenity error")]
    Serenity(#[from] serenity::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Subscriber {
    Discord(UserId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Endpoint {
    Datacenter(u16),
    World(u16),
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
    subscriptions: DashMap<Endpoint, HashSet<Subscriber>>,

    discord_client: DiscordClient,
}

impl SubscriptionManager {
    pub fn new(discord_client: DiscordClient) -> Self {
        Self {
            imp: Arc::new(SubscriptionManagerImp {
                subscriptions: DashMap::new(),
                discord_client,
            }),
        }
    }

    pub fn subscribe(&self, endpoint: Endpoint, subscriber: Subscriber) -> bool {
        let ret = self
            .imp
            .subscriptions
            .entry(endpoint)
            .or_default()
            .insert(subscriber.clone());
        if ret {
            log::info!("User {:?} subscribed to {:?}", subscriber, endpoint);
        }
        ret
    }

    pub fn unsubscribe(&self, endpoint: Endpoint, subscriber: &Subscriber) -> bool {
        let mut success = false;
        self.imp
            .subscriptions
            .entry(endpoint)
            .and_modify(|subscribers| {
                success = subscribers.remove(subscriber);
            });
        if success {
            log::info!("User {:?} unsubscribed from {:?}", subscriber, endpoint);
        }
        success
    }

    // Reports the first error that occurred while publishing to subscribers.
    pub async fn publish_endpoint(
        &self,
        publish_data: impl Into<EndpointPublishData>,
    ) -> Result<(), PublishError> {
        let publish_data: EndpointPublishData = publish_data.into();
        let endpoint: Endpoint = (&*publish_data.0).into();
        let mut subscribers: Option<HashSet<Subscriber>> = None;
        self.imp.subscriptions.entry(endpoint).and_modify(|s| {
            subscribers = Some(std::mem::take(s));
        });
        if let Some(subscribers) = subscribers {
            let mut error = OnceLock::new();
            stream::iter(subscribers)
                .for_each_concurrent(Some(32), |subscriber| {
                    let err = &error;
                    let data = publish_data.clone();
                    async move {
                        if let Err(e) = self.publish_to(&subscriber, data.0.deref()).await {
                            let _ = err.set(e);
                        }
                    }
                })
                .await;
            match error.take() {
                Some(e) => Err(e),
                None => Ok(()),
            }
        } else {
            Ok(())
        }
    }

    async fn publish_to(
        &self,
        subscriber: &Subscriber,
        publish_data: &EndpointPublish,
    ) -> Result<(), PublishError> {
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
                    .title(format!("{} is now available for DC Travel", name))
                    .color(COLOR_SUCCESS);

                user_id
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
