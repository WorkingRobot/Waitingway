use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize};
use serenity::async_trait;
use sqlx::PgPool;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct XivApiSearch<T> {
    pub next: Option<String>,
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

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct XivApiIcon {
    pub id: u32,
    pub path: String,
    pub path_hr1: String,
}

pub async fn search_xivapi<T: DeserializeOwned>(
    client: &Client,
    sheet: &str,
    query: &str,
    fields: &str,
) -> Result<Vec<Vec<XivApiRow<T>>>, reqwest::Error> {
    let mut ret = vec![];
    let mut cursor = None;
    loop {
        let mut builder = client
            .get("https://v2.xivapi.com/api/search")
            .query(&[("fields", fields)]);
        if let Some(cursor) = &cursor {
            builder = builder.query(&[("cursor", cursor)]);
        } else {
            builder = builder.query(&[("sheets", sheet), ("query", query)]);
        }
        let resp: XivApiSearch<T> = builder.send().await?.json().await?;
        ret.push(resp.results);
        cursor = resp.next;
        if cursor.is_none() {
            break;
        }
    }
    Ok(ret)
}

pub fn get_icon_url_from_id(icon_id: u32) -> String {
    get_icon_url(format!("ui/icon/{:03}000/{:06}_hr1.tex", icon_id / 1000, icon_id).as_str())
}

pub fn get_icon_url(path: &str) -> String {
    reqwest::Url::parse_with_params(
        "https://v2.xivapi.com/api/asset",
        &[("path", path), ("format", "png")],
    )
    .expect("Failed to parse URL")
    .to_string()
}

#[async_trait]
pub trait GameSheet {
    type Element: Send;

    const USES_DATABASE: bool;

    async fn get_xivapi(client: &Client) -> Result<Vec<Self::Element>, reqwest::Error>;

    #[allow(unused_variables)]
    async fn get_db(pool: &PgPool) -> Result<Vec<Self::Element>, sqlx::Error> {
        assert!(!Self::USES_DATABASE);
        Ok(vec![])
    }

    #[allow(unused_variables)]
    async fn upsert_db(pool: &PgPool, elements: Vec<Self::Element>) -> Result<(), sqlx::Error> {
        assert!(!Self::USES_DATABASE);
        Ok(())
    }

    async fn get_and_upsert(
        pool: &PgPool,
        client: &Client,
    ) -> Result<Vec<Self::Element>, super::GameDataError> {
        if Self::USES_DATABASE {
            let elements = Self::get_xivapi(client).await?;
            Self::upsert_db(pool, elements).await?;
            Ok(Self::get_db(pool).await?)
        } else {
            Ok(Self::get_xivapi(client).await?)
        }
    }
}
