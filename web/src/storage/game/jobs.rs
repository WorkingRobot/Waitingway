use std::collections::HashMap;

use super::{
    api::{GameSheet, XivApiLink, search_xivapi},
    impl_game_data,
};
use crate::{
    models::{
        duty::RouletteRole,
        job_info::{JobDisciple, JobInfo},
    },
    stopwatch::Stopwatch,
    storage::db,
};
use reqwest::Client;
use serde::Deserialize;
use serenity::async_trait;
use sqlx::PgPool;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XivApiClassJob {
    pub abbreviation: String,
    pub can_queue_for_duty: bool,
    pub class_job_category: XivApiLink<XivApiClassJobCategory>,
    pub name: String,
    pub role: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XivApiClassJobCategory {
    #[allow(unused)]
    pub name: String,
}

struct ClassJobSheet;

#[async_trait]
impl GameSheet for ClassJobSheet {
    type Element = JobInfo;
    const USES_DATABASE: bool = true;

    async fn get_xivapi(client: &Client) -> Result<Vec<Self::Element>, reqwest::Error> {
        Ok(search_xivapi::<XivApiClassJob>(
            client,
            "ClassJob",
            "-ClassJobParent=0",
            "Name,Abbreviation,ClassJobCategory.Name,Role,CanQueueForDuty",
        )
        .await?
        .into_iter()
        .flatten()
        .map(|r| {
            let id = r.row_id as u8;
            let name = r.fields.name;
            let abbreviation = r.fields.abbreviation;
            let disciple = match r.fields.class_job_category.row_id {
                30 => JobDisciple::War,
                31 => JobDisciple::Magic,
                32 => JobDisciple::Land,
                33 => JobDisciple::Hand,
                _ => panic!(
                    "Unknown disciple ID: {}",
                    r.fields.class_job_category.row_id
                ),
            };
            let role = match r.fields.role {
                0 => None, // DoH/DoL
                1 => Some(RouletteRole::Tank),
                2 | 3 => Some(RouletteRole::Dps),
                4 => Some(RouletteRole::Healer),
                _ => panic!("Unknown role ID: {}", r.fields.role),
            };
            let can_queue_for_duty = r.fields.can_queue_for_duty;
            JobInfo {
                id,
                name,
                abbreviation,
                disciple,
                role,
                can_queue_for_duty,
            }
        })
        .collect())
    }

    async fn get_db(pool: &PgPool) -> Result<Vec<Self::Element>, sqlx::Error> {
        db::job_info::get_jobs(pool).await
    }

    async fn upsert_db(pool: &PgPool, elements: Vec<Self::Element>) -> Result<(), sqlx::Error> {
        db::job_info::upsert_jobs(pool, elements).await
    }
}

pub struct JobData {
    pub jobs: HashMap<u8, JobInfo>,
}

impl JobData {
    pub async fn new(pool: &PgPool, client: &Client) -> Result<Self, super::GameDataError> {
        let _s = Stopwatch::new("Job Data Init");
        Ok(JobData {
            jobs: ClassJobSheet::get_and_upsert(pool, client)
                .await?
                .into_iter()
                .map(|j| (j.id, j))
                .collect(),
        })
    }

    pub fn get_job_by_id(&self, id: u8) -> Option<&JobInfo> {
        self.jobs.get(&id)
    }
}

impl_game_data!(JobData, JOB_DATA);
