use {
    shared::interface::{
        triple::Node,
    },
};

pub fn node_is_album() -> Node {
    return Node::Value(serde_json::Value::String("sunwet/1/album".to_string()));
}

pub fn node_is_track() -> Node {
    return Node::Value(serde_json::Value::String("sunwet/1/track".to_string()));
}

pub const PREDICATE_IS: &str = "sunwet/1/is";
pub const PREDICATE_ELEMENT: &str = "sunwet/1/element";
pub const PREDICATE_INDEX: &str = "sunwet/1/index";
pub const PREDICATE_NAME: &str = "sunwet/1/name";
pub const PREDICATE_ARTIST: &str = "sunwet/1/artist";
pub const PREDICATE_COVER: &str = "sunwet/1/cover";
pub const PREDICATE_FILE: &str = "sunwet/1/file";
pub const PREDICATE_MEDIA: &str = "sunwet/1/media";
pub const ALBUMS_RECORD_KEY_ID: &str = "id";
pub const ALBUMS_RECORD_KEY_NAME: &str = "name";
pub const ALBUMS_RECORD_KEY_COVER: &str = "cover";
pub const ALBUMS_RECORD_KEY_ARTIST_ID: &str = "artist_id";
pub const ALBUMS_RECORD_KEY_ARTIST_NAME: &str = "artist_name";
pub const TRACKS_PARAM_ALBUM: &str = "album_id";
pub const TRACKS_RECORD_KEY_ID: &str = "id";
pub const TRACKS_RECORD_KEY_FILE: &str = "file";
pub const TRACKS_RECORD_KEY_NAME: &str = "name";
pub const TRACKS_RECORD_KEY_MEDIA: &str = "media";
pub const TRACKS_RECORD_KEY_INDEX: &str = "index";
pub const TRACKS_RECORD_KEY_ARTIST_ID: &str = "artist_id";
pub const TRACKS_RECORD_KEY_ARTIST_NAME: &str = "artist_name";
