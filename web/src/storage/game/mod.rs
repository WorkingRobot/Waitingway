use reqwest::Client;
use serenity::async_trait;
use sqlx::PgPool;
use thiserror::Error;
use tokio::task::JoinSet;

mod api;
pub mod jobs;
pub mod worlds;

pub use crate::impl_game_data;

#[derive(Debug, Error)]
pub enum GameDataError {
    #[error("Failed to fetch data from XIVAPI: {0}")]
    XivApiError(#[from] reqwest::Error),
    #[error("Failed to fetch data from the database: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Tokio join error")]
    JoinError(#[from] tokio::task::JoinError),
}

#[async_trait]
pub trait GameData: Sized {
    async fn new(pool: &PgPool, client: &Client) -> Result<Self, GameDataError>;
}

#[macro_export]
macro_rules! impl_game_data {
    ($ty:ty, $constval:ident) => {
        static $constval: tokio::sync::OnceCell<$ty> = tokio::sync::OnceCell::const_new();

        pub(super) async fn initialize(
            pool: &sqlx::PgPool,
            client: &reqwest::Client,
        ) -> Result<(), $crate::storage::game::GameDataError> {
            $constval
                .get_or_try_init(|| async { <$ty>::new(pool, client).await })
                .await?;
            Ok(())
        }

        pub fn get_data() -> &'static $ty {
            $constval.get().expect("Data not initialized")
        }
    };
}

pub async fn initialize(pool: &PgPool, client: &Client) -> Result<(), GameDataError> {
    let mut joinset = JoinSet::new();

    joinset.spawn({
        let pool = pool.clone();
        let client = client.clone();
        async move { worlds::initialize(&pool, &client).await }
    });
    joinset.spawn({
        let pool = pool.clone();
        let client = client.clone();
        async move { jobs::initialize(&pool, &client).await }
    });

    while let Some(ret) = joinset.join_next().await {
        ret??;
    }
    Ok(())
}
