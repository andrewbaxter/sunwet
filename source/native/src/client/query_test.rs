#![cfg(test)]

use {
    crate::client::query::compile_query,
    shared::interface::query::{
        Chain,
        ChainBody,
        Query,
        Step,
        StepMove,
        StrValue,
    },
    std::{
        fs::read_to_string,
        path::PathBuf,
    },
};

#[test]
fn test_rt_move() {
    assert_eq!(compile_query(None, r#""xyz" -> "owner" -> "name" { => a }"#).unwrap(), Query {
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
                    dir: shared::interface::query::MoveDirection::Forward,
                    predicate: StrValue::Literal("owner".to_string()),
                    filter: None,
                    first: false,
                }), Step::Move(StepMove {
                    dir: shared::interface::query::MoveDirection::Forward,
                    predicate: StrValue::Literal("name".to_string()),
                    filter: None,
                    first: false,
                })],
            },
            bind: Some("a".to_string()),
            subchains: vec![],
        },
        sort: None,
    });
}

fn src_query_dir() -> PathBuf {
    return PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/server");
}

#[test]
fn test_default_albums() {
    let query_dir = src_query_dir();
    compile_query(
        Some(&query_dir),
        &read_to_string(query_dir.join("query_audio_albums_by_add_date.txt")).unwrap(),
    ).unwrap();
}

#[test]
fn test_default_albums_tracks() {
    let query_dir = src_query_dir();
    compile_query(
        Some(&query_dir),
        &read_to_string(query_dir.join("query_audio_tracks_search_name.txt")).unwrap(),
    ).unwrap();
}

#[test]
fn test_default_notes() {
    let query_dir = src_query_dir();
    compile_query(
        Some(&query_dir),
        &read_to_string(query_dir.join("query_notes_by_add_date.txt")).unwrap(),
    ).unwrap();
}
