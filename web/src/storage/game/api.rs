use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize};
use serenity::async_trait;
use sqlx::PgPool;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct XivApiSearch<T> {
    pub schema: String,
    pub results: Vec<XivApiRow<T>>,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct XivApiRow<T> {
    pub score: f32,
    pub sheet: String,
    pub row_id: u16,
    pub fields: T,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct XivApiLink<T> {
    pub value: u16,
    pub sheet: String,
    pub row_id: u16,
    pub fields: T,
}

pub async fn search_xivapi<T: DeserializeOwned>(
    client: &Client,
    sheet: &str,
    query: &str,
    fields: &str,
) -> Result<XivApiSearch<T>, reqwest::Error> {
    client
        .get("https://v2.xivapi.com/api/search")
        .query(&[("sheets", sheet), ("query", query), ("fields", fields)])
        .send()
        .await?
        .json()
        .await
}

#[async_trait]
pub trait GameSheet {
    type Element;

    async fn get_xivapi(client: &Client) -> Result<Vec<Self::Element>, reqwest::Error>;

    async fn get_db(pool: &PgPool) -> Result<Vec<Self::Element>, sqlx::Error>;

    async fn upsert_db(pool: &PgPool, elements: Vec<Self::Element>) -> Result<(), sqlx::Error>;

    async fn get_and_upsert(
        pool: &PgPool,
        client: &Client,
    ) -> Result<Vec<Self::Element>, super::GameDataError> {
        let elements = Self::get_xivapi(client).await?;
        Self::upsert_db(pool, elements).await?;
        Ok(Self::get_db(pool).await?)
    }
}
