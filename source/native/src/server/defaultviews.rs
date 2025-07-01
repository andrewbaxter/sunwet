use {
    shared::interface::{
        ont::{
            OBJ_IS_ALBUM,
            OBJ_IS_TRACK,
            OBJ_MEDIA_AUDIO,
        },
        triple::Node,
    },
};

pub fn node_is_album() -> Node {
    return Node::Value(serde_json::Value::String(OBJ_IS_ALBUM.to_string()));
}

pub fn node_media_audio() -> Node {
    return Node::Value(serde_json::Value::String(OBJ_MEDIA_AUDIO.to_string()));
}

pub fn node_is_track() -> Node {
    return Node::Value(serde_json::Value::String(OBJ_IS_TRACK.to_string()));
}
