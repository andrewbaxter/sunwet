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
fn test_move() {
    assert_eq!(compile_query(r#""xyz" -> "owner" -> "name" { => a }"#.to_string()).unwrap(), Query {
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
