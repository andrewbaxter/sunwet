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
