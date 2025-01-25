use std::collections::HashMap;

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use poise::ChoiceParameter;
use sqlx::PgPool;

use crate::db;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TravelDatacenterParam {
    pub id: u16,
    pub name: String,
    pub region_id: u16,
    pub region_abbreviation: String,
}

impl TravelDatacenterParam {
    pub fn name(&self) -> String {
        format!("{} ({})", self.name, self.region_abbreviation)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TravelWorldParam {
    pub id: u16,
    pub name: String,
    pub datacenter: TravelDatacenterParam,
}

impl TravelWorldParam {
    pub fn match_name(&self) -> String {
        format!(
            "{} {} {}",
            self.name, self.datacenter.name, self.datacenter.region_abbreviation
        )
    }

    pub fn name(&self) -> String {
        format!("{} ({})", self.name, self.datacenter.name)
    }
}

pub struct TravelData {
    pub datacenters: Vec<TravelDatacenterParam>,
    pub datacenter_choices: Vec<poise::CommandParameterChoice>,
    pub worlds: Vec<TravelWorldParam>,
    pub matcher: SkimMatcherV2,
}

impl TravelData {
    pub async fn new(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let worlds = db::get_worlds(pool).await?;

        let datacenter_params = worlds
            .iter()
            .filter(|world| !world.hidden)
            .unique_by(|world| world.datacenter_id)
            .sorted_unstable_by_key(|world| world.datacenter_id)
            .sorted_by_key(|world| world.region_id)
            .map(|world| {
                let param = TravelDatacenterParam {
                    id: world.datacenter_id,
                    name: world.datacenter_name.clone(),
                    region_id: world.region_id,
                    region_abbreviation: world.region_abbreviation.clone(),
                };
                (
                    param.clone(),
                    poise::CommandParameterChoice {
                        __non_exhaustive: (),
                        name: param.name(),
                        localizations: HashMap::default(),
                    },
                )
            });

        let world_params = worlds
            .iter()
            .filter(|world| !world.hidden)
            .sorted_unstable_by_key(|world| world.world_id)
            .map(|world| TravelWorldParam {
                id: world.world_id,
                name: world.world_name.clone(),
                datacenter: TravelDatacenterParam {
                    id: world.datacenter_id,
                    name: world.datacenter_name.clone(),
                    region_id: world.region_id,
                    region_abbreviation: world.region_abbreviation.clone(),
                },
            });

        let dc_params: (
            Vec<TravelDatacenterParam>,
            Vec<poise::CommandParameterChoice>,
        ) = datacenter_params.collect();

        Ok(TravelData {
            datacenters: dc_params.0,
            datacenter_choices: dc_params.1,
            worlds: world_params.collect(),
            matcher: SkimMatcherV2::default(),
        })
    }

    pub fn find_best_world_match(&self, query: &str) -> Vec<&TravelWorldParam> {
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

    pub fn get_datacenter_by_id(&self, id: u16) -> Option<&TravelDatacenterParam> {
        self.datacenters.iter().find(|dc| dc.id == id)
    }

    pub fn get_world_by_id(&self, id: u16) -> Option<&TravelWorldParam> {
        self.worlds.iter().find(|world| world.id == id)
    }
}

static TRAVEL_PARAMS: tokio::sync::OnceCell<TravelData> = tokio::sync::OnceCell::const_new();

pub async fn init_travel_params(pool: &PgPool) -> Result<(), sqlx::Error> {
    TRAVEL_PARAMS
        .get_or_try_init(|| async { TravelData::new(pool).await })
        .await?;
    Ok(())
}

pub fn get_travel_params() -> Option<&'static TravelData> {
    TRAVEL_PARAMS.get()
}

impl ChoiceParameter for TravelDatacenterParam {
    fn list() -> Vec<poise::CommandParameterChoice> {
        TRAVEL_PARAMS
            .get()
            .map(|v| v.datacenter_choices.clone())
            .unwrap_or_default()
    }

    fn from_index(index: usize) -> Option<Self> {
        TRAVEL_PARAMS
            .get()
            .and_then(|v| v.datacenters.get(index).cloned())
    }

    fn from_name(name: &str) -> Option<Self> {
        TRAVEL_PARAMS.get().and_then(|v| {
            v.datacenters
                .iter()
                .find(|dc| {
                    dc.name.eq_ignore_ascii_case(name) || dc.name().eq_ignore_ascii_case(name)
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
