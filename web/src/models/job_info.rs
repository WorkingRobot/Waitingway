use std::fmt::Display;

use super::{duty::RouletteRole, duty_db::DbRouletteRole};
use crate::storage::db::wrappers::DatabaseU16;

#[derive(Debug, sqlx::FromRow)]
pub struct DbJobInfo {
    pub id: DatabaseU16,
    pub name: String,
    pub abbreviation: String,
    pub disciple: JobDisciple,
    pub role: Option<DbRouletteRole>,
    pub can_queue: bool,
}

#[derive(Debug, Copy, Clone, sqlx::Type)]
#[sqlx(type_name = "job_disciple", rename_all = "lowercase")]
pub enum JobDisciple {
    War,
    Magic,
    Hand,
    Land,
}

impl Display for JobDisciple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Disciple of ")?;
        match self {
            JobDisciple::War => write!(f, "War"),
            JobDisciple::Magic => write!(f, "Magic"),
            JobDisciple::Hand => write!(f, "the Hand"),
            JobDisciple::Land => write!(f, "the Land"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct JobInfo {
    pub id: u8,
    pub name: String,
    pub abbreviation: String,
    pub disciple: JobDisciple,
    pub role: Option<RouletteRole>,
    pub can_queue_for_duty: bool,
}

impl JobInfo {
    pub fn icon_id(&self) -> u32 {
        62000 + u32::from(self.id)
    }
}

impl Display for JobInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({}{}) - {}",
            titlecase::titlecase(&self.name),
            self.abbreviation,
            if self.can_queue_for_duty { "" } else { "!" },
            self.disciple
        )
    }
}

impl From<DbJobInfo> for JobInfo {
    fn from(db: DbJobInfo) -> Self {
        Self {
            id: db.id.0 as u8,
            name: db.name,
            abbreviation: db.abbreviation,
            disciple: db.disciple,
            role: db.role.map(|r| r.into()),
            can_queue_for_duty: db.can_queue,
        }
    }
}
