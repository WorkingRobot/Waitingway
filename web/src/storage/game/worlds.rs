use super::{
    api::{search_xivapi, GameSheet, XivApiLink},
    impl_game_data, GameData,
};
use crate::{models::world_info::WorldInfo, stopwatch::Stopwatch, storage::db};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use poise::ChoiceParameter;
use reqwest::Client;
use serde::Deserialize;
use serenity::async_trait;
use sqlx::PgPool;
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XivApiWorld {
    pub is_public: bool,
    pub name: String,
    pub data_center: XivApiLink<XivApiDataCenter>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XivApiDataCenter {
    pub is_cloud: bool,
    pub name: String,
    pub region: u8,
}

struct WorldSheet;

#[async_trait]
impl GameSheet for WorldSheet {
    type Element = WorldInfo;
    const USES_DATABASE: bool = true;

    async fn get_xivapi(client: &Client) -> Result<Vec<Self::Element>, reqwest::Error> {
        Ok(search_xivapi::<XivApiWorld>(
            client,
            "World",
            // UserType 9 is NA Cloud Test which is public for some reason
            // 101 is China, 201 is Korea
            "-UserType=9 +(IsPublic=1 UserType=101 UserType=201)",
            "Name,DataCenter.Region,DataCenter.Name,DataCenter.IsCloud,IsPublic",
        )
        .await?
        .into_iter()
        .flatten()
        .map(|r| {
            let world_id = r.row_id;
            let world_name = r.fields.name;
            let datacenter_id = r.fields.data_center.row_id;
            let datacenter_name = r.fields.data_center.fields.name;
            let region_id = r.fields.data_center.fields.region;
            let (region_name, region_abbreviation) = match region_id {
                1 => ("Japan", "JP"),
                2 => ("North America", "NA"),
                3 => ("Europe", "EU"),
                4 => ("Oceania", "OC"),
                5 => ("China", "CN"),
                6 => ("Korea", "KR"),
                7 => ("Cloud", "CL"),
                _ => ("Unknown", "??"),
            };
            let is_cloud = r.fields.data_center.fields.is_cloud;
            let hidden = !r.fields.is_public;
            WorldInfo {
                world_id,
                world_name,
                datacenter_id,
                datacenter_name,
                region_id: region_id.into(),
                region_name: region_name.to_string(),
                region_abbreviation: region_abbreviation.to_string(),
                is_cloud,
                hidden,
            }
        })
        .collect_vec())
    }

    async fn get_db(pool: &PgPool) -> Result<Vec<Self::Element>, sqlx::Error> {
        db::world_info::get_worlds(pool).await
    }

    async fn upsert_db(pool: &PgPool, elements: Vec<Self::Element>) -> Result<(), sqlx::Error> {
        db::world_info::upsert_worlds(pool, elements).await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Datacenter {
    pub id: u16,
    pub name: String,
    pub region_id: u16,
    pub region_abbreviation: String,
}

impl Display for Datacenter {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.region_abbreviation)
    }
}

impl ChoiceParameter for Datacenter {
    fn list() -> Vec<poise::CommandParameterChoice> {
        get_data().datacenter_choices.clone()
    }

    fn from_index(index: usize) -> Option<Self> {
        get_data().datacenters.get(index).cloned()
    }

    fn from_name(name: &str) -> Option<Self> {
        get_data()
            .datacenters
            .iter()
            .find(|dc| {
                dc.name.eq_ignore_ascii_case(name) || dc.to_string().eq_ignore_ascii_case(name)
            })
            .cloned()
    }

    fn name(&self) -> &'static str {
        "Datacenter"
    }

    fn localized_name(&self, _locale: &str) -> Option<&'static str> {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct World {
    pub id: u16,
    pub name: String,
    pub datacenter: Datacenter,
}

impl World {
    pub(self) fn match_name(&self) -> String {
        format!(
            "{} {} {}",
            self.name, self.datacenter.name, self.datacenter.region_abbreviation
        )
    }
}

impl Display for World {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.datacenter.name)
    }
}

pub struct WorldData {
    pub datacenters: Vec<Datacenter>,
    pub datacenter_choices: Vec<poise::CommandParameterChoice>,
    pub worlds: Vec<World>,
    pub matcher: SkimMatcherV2,
}

#[async_trait]
impl GameData for WorldData {
    async fn new(pool: &PgPool, client: &Client) -> Result<Self, super::GameDataError> {
        let _s = Stopwatch::new("World Data Init");

        let worlds = WorldSheet::get_and_upsert(pool, client).await?;

        let datacenter_params = worlds
            .iter()
            .filter(|world| !world.hidden)
            .unique_by(|world| world.datacenter_id)
            .sorted_unstable_by_key(|world| world.datacenter_id)
            .sorted_by_key(|world| world.region_id)
            .map(|world| {
                let param = Datacenter {
                    id: world.datacenter_id,
                    name: world.datacenter_name.clone(),
                    region_id: world.region_id,
                    region_abbreviation: world.region_abbreviation.clone(),
                };
                (
                    param.clone(),
                    poise::CommandParameterChoice {
                        __non_exhaustive: (),
                        name: param.to_string(),
                        localizations: HashMap::default(),
                    },
                )
            });

        let world_params = worlds
            .iter()
            .filter(|world| !world.hidden)
            .sorted_unstable_by_key(|world| world.world_id)
            .map(|world| World {
                id: world.world_id,
                name: world.world_name.clone(),
                datacenter: Datacenter {
                    id: world.datacenter_id,
                    name: world.datacenter_name.clone(),
                    region_id: world.region_id,
                    region_abbreviation: world.region_abbreviation.clone(),
                },
            });

        let dc_params: (Vec<Datacenter>, Vec<poise::CommandParameterChoice>) =
            datacenter_params.collect();

        Ok(WorldData {
            datacenters: dc_params.0,
            datacenter_choices: dc_params.1,
            worlds: world_params.collect(),
            matcher: SkimMatcherV2::default(),
        })
    }
}

impl WorldData {
    pub fn find_best_world_match(&self, query: &str) -> Vec<&World> {
        self.worlds
            .iter()
            .filter_map(|s| {
                self.matcher
                    .fuzzy_match(&s.match_name(), query)
                    .map(|score| (score, s))
            })
            .sorted_unstable_by_key(|(score, _)| -score)
            .map(|(_, s)| s)
            .collect_vec()
    }

    pub fn get_datacenter_by_id(&self, id: u16) -> Option<&Datacenter> {
        self.datacenters.iter().find(|dc| dc.id == id)
    }

    pub fn get_world_by_id(&self, id: u16) -> Option<&World> {
        self.worlds.iter().find(|world| world.id == id)
    }
}

impl_game_data!(WorldData, WORLD_DATA);
