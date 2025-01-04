#![cfg(test)]
use {
    crate::client::query::compile_query,
    shared::interface::query::{
        Chain,
        ChainBody,
        Query,
        Step,
        StepMove,
    },
};

#[test]
fn test_rt_move() {
    assert_eq!(compile_query(r#""xyz" -> "owner" -> "name" { => a }"#).unwrap(), Query {
        chain: Chain {
            body: ChainBody {
                root: Some(
                    shared::interface::query::ChainRoot::Value(
                        shared::interface::query::Value::Literal(
                            shared::interface::triple::Node::Value(serde_json::Value::String("xyz".to_string())),
                        ),
                    ),
                ),
                steps: vec![Step::Move(StepMove {
                    dir: shared::interface::query::MoveDirection::Down,
                    predicate: "owner".to_string(),
                    filter: None,
                    first: false,
                }), Step::Move(StepMove {
                    dir: shared::interface::query::MoveDirection::Down,
                    predicate: "name".to_string(),
                    filter: None,
                    first: false,
                })],
            },
            select: Some("a".to_string()),
            subchains: vec![],
        },
        sort: vec![],
    });
}

#[test]
fn test_default_albums() {
    compile_query(include_str!("../server/defaultview_query_albums.txt")).unwrap();
}

#[test]
fn test_default_albums_tracks() {
    compile_query(include_str!("../server/defaultview_query_albums_tracks.txt")).unwrap();
}

#[test]
fn test_default_notes() {
    compile_query(include_str!("../server/defaultview_query_notes.txt")).unwrap();
}
