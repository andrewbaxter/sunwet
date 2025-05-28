#![cfg(test)]

use {
    super::query::IncludeContext,
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
    assert_eq!(
        compile_query(
            IncludeContext::Preloaded(Default::default()),
            r#""xyz" -> "owner" -> "name" { => a }"#,
        ).unwrap(),
        Query {
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
            sort: None,
        }
    );
}

#[test]
fn test_default_albums() {
    compile_query(
        IncludeContext::Preloaded(
            [
                (
                    format!("query_audio_albums_suffix.txt"),
                    include_str!("../server/query_audio_albums_suffix.txt").to_string(),
                ),
            ]
                .into_iter()
                .collect(),
        ),
        include_str!("../server/query_audio_albums_by_add_date.txt"),
    ).unwrap();
}

#[test]
fn test_default_albums_tracks() {
    compile_query(
        IncludeContext::Preloaded(
            [
                (
                    format!("query_audio_tracks_suffix.txt"),
                    include_str!("../server/query_audio_tracks_suffix.txt").to_string(),
                ),
            ]
                .into_iter()
                .collect(),
        ),
        include_str!("../server/query_audio_tracks_search_name.txt"),
    ).unwrap();
}

#[test]
fn test_default_notes() {
    compile_query(Default::default(), include_str!("../server/query_notes.txt")).unwrap();
}
