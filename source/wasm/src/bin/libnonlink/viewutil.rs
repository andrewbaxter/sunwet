use {
    flowcontrol::exenum,
    shared::{
        interface::{
            config::view::{
                FieldOrLiteral,
                FieldOrLiteralString,
            },
            triple::{
                FileHash,
                Node,
            },
            wire::{
                NodeMeta,
                TreeNode,
            },
        },
        stringpattern::node_to_text,
    },
    std::{
        collections::HashMap,
        rc::Rc,
    },
};

#[derive(Clone)]
pub struct DataStackLevel {
    pub data: TreeNode,
    pub node_meta: Rc<HashMap<Node, NodeMeta>>,
}

pub fn maybe_get_meta<'a>(data_stack: &'a Vec<Rc<DataStackLevel>>, node: &Node) -> Option<&'a NodeMeta> {
    for data_at in data_stack.iter().rev() {
        if let Some(meta) = data_at.node_meta.get(node) {
            return Some(meta);
        }
    }
    return None;
}

pub fn maybe_get_field(config_at: &String, data_stack: &Vec<Rc<DataStackLevel>>) -> Option<TreeNode> {
    for data_at in data_stack.iter().rev() {
        let TreeNode::Record(data_at) = &data_at.data else {
            continue;
        };
        let Some(data_at) = data_at.get(config_at) else {
            continue;
        };
        if exenum!(data_at, TreeNode:: Scalar(Node::Value(serde_json::Value::Null)) =>()).is_some() {
            continue;
        }
        return Some(data_at.clone());
    }
    return None;
}

pub fn maybe_get_field_or_literal(
    config_at: &FieldOrLiteral,
    data_stack: &Vec<Rc<DataStackLevel>>,
) -> Option<TreeNode> {
    match config_at {
        FieldOrLiteral::Field(config_at) => return maybe_get_field(config_at, data_stack),
        FieldOrLiteral::Literal(config_at) => return Some(TreeNode::Scalar(config_at.clone())),
    }
}

pub fn maybe_get_field_or_literal_string(
    config_at: &FieldOrLiteralString,
    data_stack: &Vec<Rc<DataStackLevel>>,
) -> Option<TreeNode> {
    match config_at {
        FieldOrLiteralString::Field(config_at) => return maybe_get_field(config_at, data_stack),
        FieldOrLiteralString::Literal(config_at) => return Some(
            TreeNode::Scalar(Node::Value(serde_json::Value::String(config_at.clone()))),
        ),
    }
}

pub fn tree_node_to_text(data_at: &TreeNode) -> String {
    match data_at {
        TreeNode::Array(v) => return serde_json::to_string(v).unwrap(),
        TreeNode::Record(v) => return serde_json::to_string(v).unwrap(),
        TreeNode::Scalar(v) => node_to_text(v),
    }
}

pub fn unwrap_value_media_hash(data_at: &Node) -> Result<FileHash, String> {
    match data_at {
        Node::File(v) => return Ok(v.clone()),
        _ => return Err(format!("Media source is not a file: {}", serde_json::to_string(data_at).unwrap())),
    }
}
