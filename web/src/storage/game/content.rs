use std::collections::HashMap;

use super::{
    api::{search_xivapi, GameSheet, XivApiIcon},
    impl_game_data,
};
use crate::stopwatch::Stopwatch;
use reqwest::Client;
use serde::Deserialize;
use serenity::async_trait;
use sqlx::PgPool;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XivApiContentRoulette {
    pub category: String,
    pub icon: XivApiIcon,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XivApiContentFinderCondition {
    pub image: XivApiIcon,
    pub name: String,
}

struct ContentRouletteSheet;
struct ContentFinderConditionSheet;

#[async_trait]
impl GameSheet for ContentRouletteSheet {
    type Element = ContentRouletteInfo;
    const USES_DATABASE: bool = false;

    async fn get_xivapi(client: &Client) -> Result<Vec<Self::Element>, reqwest::Error> {
        Ok(search_xivapi::<XivApiContentRoulette>(
            client,
            "ContentRoulette",
            "IsInDutyFinder=1",
            "Name,Icon,Category",
        )
        .await?
        .into_iter()
        .flatten()
        .map(|r| {
            let id = r.row_id as u8;
            let name = r.fields.name;
            let _category = r.fields.category;
            let icon_path = r.fields.icon.path_hr1;
            ContentRouletteInfo {
                id,
                name,
                // category,
                icon_path,
            }
        })
        .collect())
    }
}

#[async_trait]
impl GameSheet for ContentFinderConditionSheet {
    type Element = ContentFinderInfo;
    const USES_DATABASE: bool = false;

    async fn get_xivapi(client: &Client) -> Result<Vec<Self::Element>, reqwest::Error> {
        Ok(search_xivapi::<XivApiContentFinderCondition>(
            client,
            "ContentFinderCondition",
            "-Image=0",
            "Name,Image",
        )
        .await?
        .into_iter()
        .flatten()
        .map(|r| {
            let id = r.row_id;
            let mut name = r.fields.name;
            // Capitalize first letter
            if let Some(c) = name.chars().next() {
                let ch = c.to_uppercase().to_string();
                name.replace_range(..c.len_utf8(), &ch);
            }
            let image_path = r.fields.image.path_hr1;
            ContentFinderInfo {
                id,
                name,
                image_path,
            }
        })
        .collect())
    }
}

pub struct ContentRouletteInfo {
    pub id: u8,
    pub name: String,
    pub icon_path: String,
}

pub struct ContentFinderInfo {
    pub id: u16,
    pub name: String,
    pub image_path: String,
}

pub struct ContentData {
    pub roulettes: HashMap<u8, ContentRouletteInfo>,
    pub content: HashMap<u16, ContentFinderInfo>,
}

impl ContentData {
    pub const DEFAULT_IMAGE: &'static str = "ui/icon/112000/112034_hr1.tex";

    pub async fn new(pool: &PgPool, client: &Client) -> Result<Self, super::GameDataError> {
        let _s = Stopwatch::new("Content Data Init");
        Ok(ContentData {
            roulettes: ContentRouletteSheet::get_and_upsert(pool, client)
                .await?
                .into_iter()
                .map(|r| (r.id, r))
                .collect(),
            content: ContentFinderConditionSheet::get_and_upsert(pool, client)
                .await?
                .into_iter()
                .map(|r| (r.id, r))
                .collect(),
        })
    }

    pub fn get_roulette_by_id(&self, id: u8) -> Option<&ContentRouletteInfo> {
        self.roulettes.get(&id)
    }

    pub fn get_content_by_id(&self, id: u16) -> Option<&ContentFinderInfo> {
        self.content.get(&id)
    }

    pub fn get_roulette_name(&self, id: u8) -> String {
        self.get_roulette_by_id(id)
            .map_or_else(|| format!("Roulette {}", id), |r| r.name.clone())
    }

    pub fn get_content_name(&self, id: u16) -> String {
        self.get_content_by_id(id)
            .map_or_else(|| format!("Content {}", id), |r| r.name.clone())
    }

    pub fn get_roulette_image(&self, id: u8) -> String {
        self.get_roulette_by_id(id)
            .map_or_else(|| Self::DEFAULT_IMAGE.to_string(), |r| r.icon_path.clone())
    }

    pub fn get_content_image(&self, id: u16) -> String {
        self.get_content_by_id(id)
            .map_or_else(|| Self::DEFAULT_IMAGE.to_string(), |r| r.image_path.clone())
    }
}

impl_game_data!(ContentData, CONTENT_DATA);
