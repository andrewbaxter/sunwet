use chrono::{
    DateTime,
    Utc,
};
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PrepareAudio {
    pub cover_url: String,
    pub audio_url: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PrepareMedia {
    Audio(PrepareAudio),
    Video(String),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Prepare {
    pub album: String,
    pub artist: String,
    pub name: String,
    pub media: PrepareMedia,
    pub media_time: f64,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WsS2LNotify {
    Prepare(Prepare),
    Pause,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WsL2SReq {
    Ready(DateTime<Utc>),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WsC2SNotify {
    Prepare(Prepare),
    Pause,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WsC2SReq {
    Ready(DateTime<Utc>),
}
