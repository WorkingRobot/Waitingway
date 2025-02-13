use serde::Deserialize;

#[derive(Debug, sqlx::FromRow)]
pub struct DbTravelState {
    pub world_id: i16,
    pub prohibit: bool,
}

#[derive(Debug, Deserialize)]
pub struct DCTravelResponse {
    pub error: Option<String>,
    pub result: DCTravelResult,
}

#[derive(Debug, Deserialize)]
pub struct DCTravelResult {
    #[serde(rename = "return_code")]
    pub code: String,
    #[serde(rename = "return_status")]
    pub status: String,
    #[serde(rename = "return_errcode")]
    pub errcode: String,

    pub data: Option<DCTravelData>,
}

#[derive(Debug, Deserialize)]
pub struct DCTravelData {
    #[serde(rename = "homeDC")]
    #[allow(dead_code)]
    pub home_dc: u8,
    #[serde(rename = "homeWorldId")]
    pub home_world_id: u16,
    #[serde(rename = "worldInfos")]
    pub datacenters: Vec<DCTravelDCInfo>,
    #[serde(rename = "averageElapsedTime")]
    pub average_elapsed_time: i32,
}

#[derive(Debug, Deserialize)]
pub struct DCTravelDCInfo {
    #[allow(dead_code)]
    pub dc: u8,
    #[serde(rename = "worldIds")]
    pub worlds: Vec<DCTravelWorldInfo>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DCTravelWorldInfo {
    pub id: u16,
    #[serde(rename = "travelFlag")]
    pub travel: u8,
    #[serde(rename = "acceptFlag")]
    pub accept: u8,
    #[serde(rename = "prohibitFlag")]
    pub prohibit: u8,
}
