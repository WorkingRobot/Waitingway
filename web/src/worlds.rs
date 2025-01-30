use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use poise::ChoiceParameter;
use sqlx::PgPool;

use crate::storage::db;

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
        WORLD_DATA
            .get()
            .map(|v| v.datacenter_choices.clone())
            .unwrap_or_default()
    }

    fn from_index(index: usize) -> Option<Self> {
        WORLD_DATA
            .get()
            .and_then(|v| v.datacenters.get(index).cloned())
    }

    fn from_name(name: &str) -> Option<Self> {
        WORLD_DATA.get().and_then(|v| {
            v.datacenters
                .iter()
                .find(|dc| {
                    dc.name.eq_ignore_ascii_case(name) || dc.to_string().eq_ignore_ascii_case(name)
                })
                .cloned()
        })
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

impl WorldData {
    pub async fn new(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let worlds = db::get_worlds(pool).await?;

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

static WORLD_DATA: tokio::sync::OnceCell<WorldData> = tokio::sync::OnceCell::const_new();

pub async fn initialize(pool: &PgPool) -> Result<(), sqlx::Error> {
    WORLD_DATA
        .get_or_try_init(|| async { WorldData::new(pool).await })
        .await?;
    Ok(())
}

pub fn get_world_data() -> Option<&'static WorldData> {
    WORLD_DATA.get()
}
