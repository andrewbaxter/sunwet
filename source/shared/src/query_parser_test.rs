#![cfg(test)]

use {
    crate::{
        interface::{
            query::{
                Chain,
                ChainHead,
                ChainRoot,
                ChainTail,
                MoveDirection,
                Query,
                Step,
                StepMove,
                StepSpecific,
                StrValue,
                Value,
            },
            triple::Node,
        },
        query_parser::{
            compile_fragment_query_head,
            compile_fragment_query_tail,
            compile_query,
        },
    },
    std::{
        fs::read_to_string,
        path::PathBuf,
    },
};

#[test]
fn test_rt_move() {
    assert_eq!(compile_query(r#""xyz" -> "owner" -> "name" { => a }"#).unwrap(), Query {
        chain: Chain {
            head: ChainHead {
                root: Some(
                    ChainRoot::Value(Value::Literal(Node::Value(serde_json::Value::String("xyz".to_string())))),
                ),
                steps: vec![
                    //. .
                    Step {
                        specific: StepSpecific::Move(StepMove {
                            dir: MoveDirection::Forward,
                            predicate: StrValue::Literal("owner".to_string()),
                            filter: None,
                        }),
                        sort: None,
                        first: false,
                    },
                    Step {
                        specific: StepSpecific::Move(StepMove {
                            dir: MoveDirection::Forward,
                            predicate: StrValue::Literal("name".to_string()),
                            filter: None,
                        }),
                        sort: None,
                        first: false,
                    }
                ],
            },
            tail: ChainTail {
                bind: Some("a".to_string()),
                subchains: vec![],
            },
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
    compile_fragment_query_head(&read_to_string(query_dir.join("query_audio_albums.txt")).unwrap()).unwrap();
    compile_fragment_query_tail(&read_to_string(query_dir.join("query_audio_albums_suffix.txt")).unwrap()).unwrap();
}

#[test]
fn test_default_albums_tracks() {
    let query_dir = src_query_dir();
    compile_fragment_query_head(
        &read_to_string(query_dir.join("query_audio_tracks_search_name.txt")).unwrap(),
    ).unwrap();
    compile_fragment_query_tail(&read_to_string(query_dir.join("query_audio_tracks_suffix.txt")).unwrap()).unwrap();
}

#[test]
fn test_default_notes() {
    let query_dir = src_query_dir();
    compile_fragment_query_head(&read_to_string(query_dir.join("query_notes_by_add_date.txt")).unwrap()).unwrap();
    compile_fragment_query_tail(&read_to_string(query_dir.join("query_notes_suffix.txt")).unwrap()).unwrap();
}
