use std::path::Path;
use shared::model::cli::{
    CliNode,
    CliTriple,
};
use uuid::Uuid;

pub fn node_id() -> String {
    return Uuid::new_v4().hyphenated().to_string();
}

pub fn node_upload(root: &Path, p: &Path) -> CliNode {
    return CliNode::Upload(p.strip_prefix(root).unwrap().to_path_buf());
}

pub fn node_value_str(v: &str) -> CliNode {
    return CliNode::Value(serde_json::Value::String(v.to_string()));
}

pub fn node_value_usize(v: usize) -> CliNode {
    return CliNode::Value(serde_json::Value::Number(serde_json::Number::from(v as i64)));
}

pub fn triple(sub: &CliNode, pred: &str, obj: &CliNode) -> CliTriple {
    return CliTriple {
        subject: sub.clone(),
        predicate: pred.to_string(),
        object: obj.clone(),
    };
}

const PREFIX_SUNWET1: &str = "sunwet/1";

// Link to file node from metadata node representing file
pub fn pred_file() -> String {
    return format!("{PREFIX_SUNWET1}/file");
}

// Human-known name for something
pub fn pred_name() -> String {
    return format!("{PREFIX_SUNWET1}/name");
}

// A mangling of the human-known name that can be unambiguously sorted by a
// computer (ex: hiragana/katagana instead of kanji)
pub fn pred_name_sort() -> String {
    return format!("{PREFIX_SUNWET1}/name_sort");
}

// Link to artist
pub fn pred_artist() -> String {
    return format!("{PREFIX_SUNWET1}/artist");
}

// Link to cover (file node)
pub fn pred_image() -> String {
    return format!("{PREFIX_SUNWET1}/cover");
}

// Link to booklet (file node)
pub fn pred_document() -> String {
    return format!("{PREFIX_SUNWET1}/booklet");
}

pub fn pred_media() -> String {
    return format!("{PREFIX_SUNWET1}/media");
}

pub fn pred_index() -> String {
    return format!("{PREFIX_SUNWET1}/index");
}

pub fn pred_element() -> String {
    return format!("{PREFIX_SUNWET1}/element");
}

/// Typing, can be chained to form hierarchy
pub fn pred_is() -> String {
    return format!("{PREFIX_SUNWET1}/is");
}

pub fn root_albumset_value() -> CliNode {
    return CliNode::Value(serde_json::Value::String(format!("{PREFIX_SUNWET1}/albumset")));
}

pub fn root_album_value() -> CliNode {
    return CliNode::Value(serde_json::Value::String(format!("{PREFIX_SUNWET1}/album")));
}

pub fn root_track_value() -> CliNode {
    return CliNode::Value(serde_json::Value::String(format!("{PREFIX_SUNWET1}/track")));
}

pub fn root_artist_value() -> CliNode {
    return CliNode::Value(serde_json::Value::String(format!("{PREFIX_SUNWET1}/artist")));
}

pub fn root_audio_value() -> CliNode {
    return CliNode::Value(serde_json::Value::String(format!("{PREFIX_SUNWET1}/audio")));
}

pub fn root_video_value() -> CliNode {
    return CliNode::Value(serde_json::Value::String(format!("{PREFIX_SUNWET1}/video")));
}
