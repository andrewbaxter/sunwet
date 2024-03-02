use chrono::{
    DateTime,
    Utc,
};
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Serialize, Deserialize)]
pub enum WsMessage {
    Notify(serde_json::Value),
    Request(serde_json::Value),
}

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
pub struct WsL2SReq {
    pub session_id: String,
    pub mode: WsL2SReqMode,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WsL2SReqMode {
    Ready(DateTime<Utc>),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct WsC2SNotify {
    pub session_id: String,
    pub mode: WsC2SNotifyMode,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WsC2SNotifyMode {
    Prepare(Prepare),
    Pause,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct WsC2SReq {
    pub session_id: String,
    pub mode: WsC2SReqMode,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WsC2SReqMode {
    Ready(DateTime<Utc>),
}
