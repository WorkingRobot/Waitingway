use super::duty_db::{DbRouletteEstimate, DbRouletteRole};
use crate::{
    middleware::version::UserAgentVersion,
    storage::db::wrappers::{DatabaseDateTime, DatabaseU16},
};
use num_enum::{FromPrimitive, IntoPrimitive};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Recap {
    // Unique id of the recap
    #[serde(skip)]
    pub id: Uuid,
    // User/player id that the recap was from
    #[serde(skip)]
    pub user_id: Uuid,

    pub queued_roulette: Option<u8>,
    pub queued_content: Option<Vec<u16>>,
    pub queued_job: u8,
    pub queued_flags: ContentFlags,
    pub queued_languages: QueueLanguage,

    // World id that the recap was for
    pub world_id: u16,

    pub party: Option<PartyMakeup>,
    // Time the queue was started
    pub start_time: DatabaseDateTime,
    // Time the queue was left
    pub end_time: DatabaseDateTime,
    pub withdraw_message: Option<u16>,

    pub updates: Vec<RecapUpdate>,
    pub pops: Vec<RecapPop>,

    #[serde(skip)]
    pub client_version: UserAgentVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecapUpdate {
    #[serde(rename = "timestamp")]
    pub time: DatabaseDateTime,
    pub is_reserving_server: bool,

    #[serde(flatten)]
    pub update_data: Option<RecapUpdateData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$type", rename_all = "snake_case")]
pub enum RecapUpdateData {
    Roulette {
        wait_time: WaitTime,
        position: RoulettePosition,
    },
    Thd {
        wait_time: WaitTime,
        tanks: FillParam,
        healers: FillParam,
        dps: FillParam,
    },
    Players {
        wait_time: WaitTime,
        players: FillParam,
    },
    WaitTime {
        wait_time: WaitTime,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecapPop {
    #[serde(rename = "timestamp")]
    pub time: DatabaseDateTime,
    pub resulting_flags: ContentFlags,
    pub resulting_content: Option<u16>,
    #[serde(rename = "in_progress_begin_timestamp")]
    pub in_progress_time: Option<DatabaseDateTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "u8", from = "u8")]
pub enum WaitTime {
    Minutes(u8),
    Over30Minutes,
    Hidden,
}

impl From<WaitTime> for u8 {
    fn from(value: WaitTime) -> u8 {
        match value {
            WaitTime::Minutes(min) => min,
            WaitTime::Over30Minutes => 0,
            WaitTime::Hidden => 255,
        }
    }
}

impl From<u8> for WaitTime {
    fn from(value: u8) -> Self {
        match value {
            0 => WaitTime::Over30Minutes,
            255 => WaitTime::Hidden,
            _ => WaitTime::Minutes(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "u8", from = "u8")]
pub enum RoulettePosition {
    Position(u8),
    After50,
    RetrievingInfo,
}

impl From<RoulettePosition> for u8 {
    fn from(value: RoulettePosition) -> u8 {
        match value {
            RoulettePosition::Position(pos) => pos,
            RoulettePosition::After50 => 254,
            RoulettePosition::RetrievingInfo => 255,
        }
    }
}

impl From<u8> for RoulettePosition {
    fn from(value: u8) -> Self {
        match value {
            254 => RoulettePosition::After50,
            0 | 255 => RoulettePosition::RetrievingInfo,
            _ => RoulettePosition::Position(value),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "duty_fill_param")]
pub struct FillParam {
    pub found: DatabaseU16,
    pub needed: DatabaseU16,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PartyMakeup {
    pub is_party_leader: bool,
    pub members: Vec<PartyMember>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "duty_party_member")]
pub struct PartyMember {
    pub job: DatabaseU16,
    pub level: DatabaseU16,
    pub world: DatabaseU16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ContentFlags {
    pub loot_rule: LootRule,
    #[serde(rename = "is_unrestricted_party")]
    pub is_unrestricted: bool,
    pub is_min_ilvl: bool,
    pub is_silence_echo: bool,
    pub is_explorer: bool,
    pub is_level_synced: bool,
    pub is_limited_leveling: bool,
    pub in_progress_party: bool,
}

impl ContentFlags {
    pub fn as_flags(self, languages: QueueLanguage) -> u16 {
        // Bit 0
        let is_unrestricted = u16::from(self.is_unrestricted);
        // Bit 1
        let is_min_ilvl = u16::from(self.is_min_ilvl);
        // Bit 2
        let is_silence_echo = u16::from(self.is_silence_echo);
        // Bit 3
        let is_explorer = u16::from(self.is_explorer);
        // Bit 4
        let is_level_synced = u16::from(self.is_level_synced);
        // Bit 5
        let is_limited_leveling = u16::from(self.is_limited_leveling);
        // Bit 6
        let in_progress_party = u16::from(self.in_progress_party);
        // Bit 7+8
        let loot_rule = u16::from(u8::from(self.loot_rule));
        // Bit 9+10+11+12
        let languages = u16::from(u8::from(languages));

        is_unrestricted
            | (is_min_ilvl << 1)
            | (is_silence_echo << 2)
            | (is_explorer << 3)
            | (is_level_synced << 4)
            | (is_limited_leveling << 5)
            | (in_progress_party << 6)
            | (loot_rule << 7)
            | (languages << 9)
    }

    #[allow(dead_code)]
    pub fn from_flags(flags: u16) -> (Self, QueueLanguage) {
        let is_unrestricted = (flags & 1) != 0;
        let is_min_ilvl = (flags & 2) != 0;
        let is_silence_echo = (flags & 4) != 0;
        let is_explorer = (flags & 8) != 0;
        let is_level_synced = (flags & 16) != 0;
        let is_limited_leveling = (flags & 32) != 0;
        let in_progress_party = (flags & 64) != 0;
        let loot_rule = (flags >> 7) & 3;
        let languages = (flags >> 9) & 15;

        (
            Self {
                loot_rule: u8::try_from(loot_rule).unwrap_or_default().into(),
                is_unrestricted,
                is_min_ilvl,
                is_silence_echo,
                is_explorer,
                is_level_synced,
                is_limited_leveling,
                in_progress_party,
            },
            u8::try_from(languages).unwrap_or_default().into(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum RecapUpdateType {
    Roulette = 0,
    Thd = 1,
    Players = 2,
    WaitTime = 3,
}

#[derive(
    Debug, Default, Clone, Copy, Serialize_repr, Deserialize_repr, FromPrimitive, IntoPrimitive,
)]
#[repr(u8)]
// Poor man's bitflags
pub enum QueueLanguage {
    #[default]
    None = 0,
    Jp = 1,
    En = 2,
    JpEn = 3,
    De = 4,
    JpDe = 5,
    EnDe = 6,
    JpEnDe = 7,
    Fr = 8,
    JpFr = 9,
    EnFr = 10,
    JpEnFr = 11,
    DeFr = 12,
    JpDeFr = 13,
    EnDeFr = 14,
    JpEnDeFr = 15,
}

#[derive(
    Debug, Default, Clone, Copy, Serialize_repr, Deserialize_repr, FromPrimitive, IntoPrimitive,
)]
#[repr(u8)]
pub enum LootRule {
    #[default]
    None = 0,
    GreedOnly = 1,
    Lootmaster = 2,
}

#[derive(Debug, Copy, Clone, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum RouletteRole {
    Tank = 1,
    Healer = 2,
    Dps = 3,
}

impl RouletteRole {
    pub fn as_db(self) -> DbRouletteRole {
        match self {
            RouletteRole::Tank => DbRouletteRole::Tank,
            RouletteRole::Healer => DbRouletteRole::Healer,
            RouletteRole::Dps => DbRouletteRole::Dps,
        }
    }
}

impl From<DbRouletteRole> for RouletteRole {
    fn from(role: DbRouletteRole) -> Self {
        match role {
            DbRouletteRole::Tank => RouletteRole::Tank,
            DbRouletteRole::Healer => RouletteRole::Healer,
            DbRouletteRole::Dps => RouletteRole::Dps,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouletteSize {
    // User/player id that the size update was from
    #[serde(skip)]
    pub user_id: Uuid,
    // World id that the size is for
    pub world_id: u16,

    pub roulette_id: u8,
    pub role: RouletteRole,

    pub size: Option<RoulettePosition>,
    pub estimated_wait_time: Option<WaitTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouletteEstimate {
    pub datacenter_id: u16,
    pub roulette_id: u8,
    pub role: RouletteRole,

    pub last_update: DatabaseDateTime,
    pub wait_time: f64,
    pub size: RoulettePosition,
    pub estimated_wait_time: WaitTime,
}

impl From<DbRouletteEstimate> for RouletteEstimate {
    fn from(db: DbRouletteEstimate) -> Self {
        Self {
            datacenter_id: db.datacenter_id as u16,
            roulette_id: (db.roulette_id as u16).try_into().unwrap_or_default(),
            role: db.role.into(),
            last_update: db.time.unwrap_or_default(),
            wait_time: db.duration.unwrap_or_default(),
            size: u8::try_from(db.size.unwrap_or_default() as u16)
                .unwrap_or_default()
                .into(),
            estimated_wait_time: u8::try_from(db.wait_time.unwrap_or_default() as u16)
                .unwrap_or_default()
                .into(),
        }
    }
}
