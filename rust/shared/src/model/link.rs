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
pub enum WsL2S {
    Ready(DateTime<Utc>),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WsS2L {
    Prepare(Prepare),
    Play(DateTime<Utc>),
    Pause,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WsC2S {
    Prepare(Prepare),
    Ready(DateTime<Utc>),
    Pause,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WsS2C {
    Play(DateTime<Utc>),
}
