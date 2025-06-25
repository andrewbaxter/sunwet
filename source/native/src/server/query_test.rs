#![cfg(test)]

use {
    super::defaultviews::node_is_album,
    crate::{
        interface::triple::DbNode,
        server::{
            db,
            query::{
                build_root_chain,
                execute_sql_query,
            },
        },
    },
    chrono::{
        DateTime,
        Duration,
        TimeZone,
        Utc,
    },
    shared::{
        query_parser::compile_query,
        interface::{
            ont::{
                PREDICATE_ADD_TIMESTAMP,
                PREDICATE_ARTIST,
                PREDICATE_IS,
                PREDICATE_NAME,
                PREDICATE_TRACK,
            },
            query::{
                Chain,
                ChainBody,
                ChainRoot,
                FilterExpr,
                FilterExprExistance,
                FilterExprExistsType,
                FilterSuffixSimple,
                FilterSuffixSimpleOperator,
                JunctionType,
                MoveDirection,
                Query,
                Step,
                StepJunction,
                StepMove,
                StepRecurse,
                StrValue,
                Value,
            },
            triple::Node,
            wire::TreeNode,
        },
    },
    std::{
        collections::{
            BTreeMap,
            HashMap,
        },
        fs::read_to_string,
        path::PathBuf,
    },
};

fn s(value: impl AsRef<str>) -> Node {
    return Node::Value(serde_json::Value::String(value.as_ref().to_string()));
}

fn i(value: i32) -> Node {
    return Node::Value(serde_json::Value::Number(serde_json::Number::from(value)));
}

fn n() -> Node {
    return Node::Value(serde_json::Value::Null);
}

fn execute(triples: &[(&Node, &str, &Node)], want: &[&[(&str, TreeNode)]], query: Query) {
    let sort = query.sort;
    let (query, query_values) = build_root_chain(query.chain, HashMap::new()).unwrap();
    let mut db = rusqlite::Connection::open_in_memory().unwrap();
    db::migrate(&mut db).unwrap();
    for (s, p, o) in triples {
        db::triple_insert(&db, &DbNode((*s).clone()), p, &DbNode((*o).clone()), Utc::now().into(), true).unwrap();
    }

    //.    {
    //.        let prettier_root = PathBuf::from("/home/andrew/temp/soft/node/node_modules/");
    //.        let mut prettier = Command::new(prettier_root.join(".bin/prettier"));
    //.        prettier
    //.            .arg("--parser")
    //.            .arg("sql")
    //.            .arg("--plugin")
    //.            .arg(prettier_root.join("prettier-plugin-sql/lib/index.cjs"));
    //.        prettier.stdin(Stdio::piped());
    //.        prettier.stdout(Stdio::piped());
    //.        let mut child = prettier.spawn().unwrap();
    //.        let mut child_stdin = child.stdin.take().unwrap();
    //.        child_stdin.write_all(query.as_bytes()).unwrap();
    //.        drop(child_stdin);
    //.        let output = child.wait_with_output().unwrap();
    //.        if !output.status.success() {
    //.            panic!();
    //.        }
    //.        println!("Query: {}", String::from_utf8(output.stdout).unwrap());
    //.    }
    println!("Query: {}", query);
    {
        let mut s = db.prepare(&format!("explain query plan {}", query)).unwrap();
        let mut results = s.query(&*query_values.as_params()).unwrap();
        loop {
            let Some(row) = results.next().unwrap() else {
                break;
            };
            println!("explain row: {:?}", row);
        }
    }
    let got =
        execute_sql_query(&db, query, query_values, sort, None)
            .unwrap()
            .into_iter()
            .map(|x| x.value)
            .collect::<Vec<_>>();
    let want =
        want
            .into_iter()
            .map(|m| m.into_iter().map(|(k, v)| (k.to_string(), v.clone())).collect::<BTreeMap<_, _>>())
            .collect::<Vec<_>>();
    assert_eq!(want, got);
}

fn src_query_dir() -> PathBuf {
    return PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/server");
}

#[test]
fn test_base() {
    let query_dir = src_query_dir();
    execute(
        &[
            (&s("a"), PREDICATE_IS, &node_is_album()),
            (&s("a"), PREDICATE_ADD_TIMESTAMP, &s(DateTime::UNIX_EPOCH.to_rfc3339())),
            (&s("a"), PREDICATE_NAME, &s("a_name")),
            (&s("a"), PREDICATE_ARTIST, &s("a_a")),
            (&s("a_a"), PREDICATE_NAME, &s("a_a_name")),
        ],
        &[
            &[
                ("album_id", TreeNode::Scalar(s("a"))),
                ("album_add_timestamp", TreeNode::Scalar(s(DateTime::UNIX_EPOCH.to_rfc3339()))),
                ("album_name", TreeNode::Scalar(s("a_name"))),
                ("album_artist_id", TreeNode::Scalar(s("a_a"))),
                ("album_artist_name", TreeNode::Scalar(s("a_a_name"))),
                ("cover", TreeNode::Scalar(n())),
            ],
        ],
        compile_query(
            Some((&query_dir, |p| read_to_string(p).map_err(|e| e.to_string()))),
            &read_to_string(&query_dir.join("query_audio_albums_by_add_date.txt")).unwrap(),
        ).unwrap(),
    );
}

#[test]
fn test_versions() {
    let query = compile_query(None, "\"x\" -> \"y\" { => y }").unwrap();
    let (query, query_values) = build_root_chain(query.chain, HashMap::new()).unwrap();
    let mut db = rusqlite::Connection::open_in_memory().unwrap();
    db::migrate(&mut db).unwrap();
    db::triple_insert(
        &db,
        &DbNode(s("x")),
        "y",
        &DbNode(s("no")),
        DateTime::from_timestamp_nanos(1),
        true,
    ).unwrap();
    db::triple_insert(
        &db,
        &DbNode(s("x")),
        "y",
        &DbNode(s("no")),
        DateTime::from_timestamp_nanos(2),
        true,
    ).unwrap();
    println!("Query: {}", query);
    {
        let mut s = db.prepare(&format!("explain query plan {}", query)).unwrap();
        let mut results = s.query(&*query_values.as_params()).unwrap();
        loop {
            let Some(row) = results.next().unwrap() else {
                break;
            };
            println!("explain row: {:?}", row);
        }
    }
    let got =
        execute_sql_query(&db, query, query_values, None, None)
            .unwrap()
            .into_iter()
            .map(|x| x.value)
            .collect::<Vec<_>>();
    assert_eq!(
        got,
        vec![[("y".to_string(), TreeNode::Scalar(s("no")))].into_iter().collect::<BTreeMap<_, _>>()]
    );
}

#[test]
fn test_delete() {
    let query = compile_query(None, "\"x\" -> \"y\" { => y }").unwrap();
    let (query, query_values) = build_root_chain(query.chain, HashMap::new()).unwrap();
    let mut db = rusqlite::Connection::open_in_memory().unwrap();
    db::migrate(&mut db).unwrap();
    db::triple_insert(
        &db,
        &DbNode(s("x")),
        "y",
        &DbNode(s("no")),
        DateTime::from_timestamp_nanos(1),
        true,
    ).unwrap();
    db::triple_insert(
        &db,
        &DbNode(s("x")),
        "y",
        &DbNode(s("no")),
        DateTime::from_timestamp_nanos(2),
        false,
    ).unwrap();
    println!("Query: {}", query);
    {
        let mut s = db.prepare(&format!("explain query plan {}", query)).unwrap();
        let mut results = s.query(&*query_values.as_params()).unwrap();
        loop {
            let Some(row) = results.next().unwrap() else {
                break;
            };
            println!("explain row: {:?}", row);
        }
    }
    let got =
        execute_sql_query(&db, query, query_values, None, None)
            .unwrap()
            .into_iter()
            .map(|x| x.value)
            .collect::<Vec<_>>();
    assert_eq!(got, vec![]);
}

#[test]
fn test_undelete() {
    let query = compile_query(None, "\"x\" -> \"y\" { => y }").unwrap();
    let (query, query_values) = build_root_chain(query.chain, HashMap::new()).unwrap();
    let mut db = rusqlite::Connection::open_in_memory().unwrap();
    db::migrate(&mut db).unwrap();
    db::triple_insert(
        &db,
        &DbNode(s("x")),
        "y",
        &DbNode(s("no")),
        DateTime::from_timestamp_nanos(1),
        true,
    ).unwrap();
    db::triple_insert(
        &db,
        &DbNode(s("x")),
        "y",
        &DbNode(s("no")),
        DateTime::from_timestamp_nanos(2),
        false,
    ).unwrap();
    db::triple_insert(
        &db,
        &DbNode(s("x")),
        "y",
        &DbNode(s("no")),
        DateTime::from_timestamp_nanos(3),
        true,
    ).unwrap();
    println!("Query: {}", query);
    {
        let mut s = db.prepare(&format!("explain query plan {}", query)).unwrap();
        let mut results = s.query(&*query_values.as_params()).unwrap();
        loop {
            let Some(row) = results.next().unwrap() else {
                break;
            };
            println!("explain row: {:?}", row);
        }
    }
    let got =
        execute_sql_query(&db, query, query_values, None, None)
            .unwrap()
            .into_iter()
            .map(|x| x.value)
            .collect::<Vec<_>>();
    assert_eq!(
        got,
        vec![[("y".to_string(), TreeNode::Scalar(s("no")))].into_iter().collect::<BTreeMap<_, _>>()]
    );
}

#[test]
fn test_recurse() {
    execute(
        &[
            (&s("a"), PREDICATE_IS, &node_is_album()),
            (&s("a"), PREDICATE_NAME, &s("a_name")),
            (&s("b"), PREDICATE_IS, &node_is_album()),
            (&s("b_p"), PREDICATE_TRACK, &s("b")),
            (&s("b_p"), PREDICATE_NAME, &s("b_name")),
        ],
        &[&[("name", TreeNode::Scalar(s("a_name")))], &[("name", TreeNode::Scalar(s("b_name")))]],
        Query {
            chain: Chain {
                body: ChainBody {
                    root: Some(ChainRoot::Value(Value::Literal(node_is_album()))),
                    steps: vec![
                        //. .
                        Step::Move(StepMove {
                            dir: MoveDirection::Backward,
                            predicate: StrValue::Literal(PREDICATE_IS.to_string()),
                            filter: None,
                            first: false,
                        }),
                        Step::Recurse(StepRecurse {
                            subchain: ChainBody {
                                root: None,
                                steps: vec![Step::Move(StepMove {
                                    dir: MoveDirection::Backward,
                                    predicate: StrValue::Literal(PREDICATE_TRACK.to_string()),
                                    filter: None,
                                    first: false,
                                })],
                            },
                            first: false,
                        }),
                        Step::Move(StepMove {
                            dir: MoveDirection::Forward,
                            predicate: StrValue::Literal(PREDICATE_NAME.to_string()),
                            filter: None,
                            first: false,
                        })
                    ],
                },
                bind: Some("name".to_string()),
                subchains: vec![],
            },
            sort: None,
        },
    );
}

#[test]
fn test_filter_eq() {
    execute(
        &[
            (&s("a"), PREDICATE_IS, &s("sunwet/1/album")),
            (&s("a"), PREDICATE_NAME, &s("a_name")),
            (&s("b"), PREDICATE_IS, &s("sunwet/1/album")),
            (&s("b"), PREDICATE_NAME, &s("b_name")),
        ],
        &[&[("id", TreeNode::Scalar(s("a")))]],
        Query {
            chain: Chain {
                body: ChainBody {
                    root: Some(ChainRoot::Value(Value::Literal(node_is_album()))),
                    steps: vec![
                        //. .
                        Step::Move(StepMove {
                            dir: MoveDirection::Backward,
                            predicate: StrValue::Literal(PREDICATE_IS.to_string()),
                            filter: Some(FilterExpr::Exists(FilterExprExistance {
                                type_: FilterExprExistsType::Exists,
                                subchain: ChainBody {
                                    root: None,
                                    steps: vec![Step::Move(StepMove {
                                        dir: MoveDirection::Forward,
                                        predicate: StrValue::Literal(PREDICATE_NAME.to_string()),
                                        filter: None,
                                        first: false,
                                    })],
                                },
                                suffix: Some(shared::interface::query::FilterSuffix::Simple(FilterSuffixSimple {
                                    op: FilterSuffixSimpleOperator::Eq,
                                    value: Value::Literal(s("a_name")),
                                })),
                            })),
                            first: false,
                        })
                    ],
                },
                bind: Some("id".to_string()),
                subchains: vec![],
            },
            sort: None,
        },
    );
}

#[test]
fn test_filter_lt() {
    execute(
        &[
            (&s("a"), PREDICATE_IS, &node_is_album()),
            (&s("a"), "sunwet/1/q", &i(12)),
            (&s("b"), PREDICATE_IS, &node_is_album()),
            (&s("b"), "sunwet/1/q", &i(47)),
        ],
        &[&[("id", TreeNode::Scalar(s("b")))]],
        Query {
            chain: Chain {
                body: ChainBody {
                    root: Some(ChainRoot::Value(Value::Literal(node_is_album()))),
                    steps: vec![
                        //. .
                        Step::Move(StepMove {
                            dir: MoveDirection::Backward,
                            predicate: StrValue::Literal(PREDICATE_IS.to_string()),
                            filter: Some(FilterExpr::Exists(FilterExprExistance {
                                type_: FilterExprExistsType::Exists,
                                subchain: ChainBody {
                                    root: None,
                                    steps: vec![Step::Move(StepMove {
                                        dir: MoveDirection::Forward,
                                        predicate: StrValue::Literal("sunwet/1/q".to_string()),
                                        filter: None,
                                        first: false,
                                    })],
                                },
                                suffix: Some(shared::interface::query::FilterSuffix::Simple(FilterSuffixSimple {
                                    op: FilterSuffixSimpleOperator::Gte,
                                    value: Value::Literal(i(30)),
                                })),
                            })),
                            first: false,
                        })
                    ],
                },
                bind: Some("id".to_string()),
                subchains: vec![],
            },
            sort: None,
        },
    );
}

#[test]
fn test_chain_union() {
    execute(
        &[
            (&s("a"), PREDICATE_IS, &node_is_album()),
            (&s("b"), PREDICATE_IS, &s("sunwet/1/dog")),
            (&s("d"), PREDICATE_IS, &s("sunwet/1/what")),
        ],
        &[
            //. .
            &[("id", TreeNode::Scalar(s("b")))],
            &[("id", TreeNode::Scalar(s("d")))],
        ],
        Query {
            chain: Chain {
                body: ChainBody {
                    root: None,
                    steps: vec![
                        //. .
                        Step::Junction(StepJunction {
                            type_: JunctionType::Or,
                            subchains: vec![
                                //. .
                                ChainBody {
                                    root: Some(ChainRoot::Value(Value::Literal(s("sunwet/1/dog")))),
                                    steps: vec![Step::Move(StepMove {
                                        dir: MoveDirection::Backward,
                                        predicate: StrValue::Literal(PREDICATE_IS.to_string()),
                                        filter: None,
                                        first: false,
                                    })],
                                },
                                ChainBody {
                                    root: Some(ChainRoot::Value(Value::Literal(s("sunwet/1/what")))),
                                    steps: vec![Step::Move(StepMove {
                                        dir: MoveDirection::Backward,
                                        predicate: StrValue::Literal(PREDICATE_IS.to_string()),
                                        filter: None,
                                        first: false,
                                    })],
                                }
                            ],
                        })
                    ],
                },
                bind: Some("id".to_string()),
                subchains: vec![],
            },
            sort: None,
        },
    );
}

#[test]
fn test_gc() {
    let mut db = rusqlite::Connection::open_in_memory().unwrap();
    db::migrate(&mut db).unwrap();
    let stamp1 = chrono::Local.with_ymd_and_hms(2014, 10, 1, 1, 1, 1).unwrap().into();
    let stamp2 = chrono::Local.with_ymd_and_hms(2014, 11, 1, 1, 1, 1).unwrap().into();
    let stamp3 = chrono::Local.with_ymd_and_hms(2014, 12, 1, 1, 1, 1).unwrap().into();

    // Newest is after epoch
    db::triple_insert(&db, &DbNode(s("a")), "b", &DbNode(s("c")), stamp1, true).unwrap();
    db::triple_insert(&db, &DbNode(s("a")), "b", &DbNode(s("c")), stamp2, false).unwrap();
    db::triple_insert(&db, &DbNode(s("a")), "b", &DbNode(s("c")), stamp3, true).unwrap();

    // Newest is before epoch, but exists
    db::triple_insert(&db, &DbNode(s("d")), "e", &DbNode(s("f")), stamp1, false).unwrap();
    db::triple_insert(&db, &DbNode(s("d")), "e", &DbNode(s("f")), stamp2, true).unwrap();

    // Newest is before epoch, but doesn't exist
    db::triple_insert(&db, &DbNode(s("g")), "h", &DbNode(s("i")), stamp1, true).unwrap();
    db::triple_insert(&db, &DbNode(s("g")), "h", &DbNode(s("i")), stamp1, false).unwrap();

    // Gc
    db::triple_gc_deleted(&db, stamp2 + Duration::seconds(1)).unwrap();
    let want = vec![
        //. .
        format!("{:?}", (s("a"), "b".to_string(), s("c"), stamp3, true)),
        format!("{:?}", (s("d"), "e".to_string(), s("f"), stamp2, true))
    ];
    let mut have =
        db::triple_list_all(
            &db,
            DateTime::<Utc>::MAX_UTC,
            &DbNode(Node::Value(serde_json::Value::Null)),
            "",
            &DbNode(Node::Value(serde_json::Value::Null)),
        )
            .unwrap()
            .into_iter()
            .map(|r| format!("{:?}", (r.subject.0, r.predicate, r.object.0, r.commit_, r.exists)))
            .collect::<Vec<_>>();
    have.sort();
    pretty_assertions::assert_eq!(want, have);
    db::triple_gc_deleted(&db, stamp2 + Duration::seconds(1)).unwrap();
    let mut have =
        db::triple_list_all(
            &db,
            DateTime::<Utc>::MAX_UTC,
            &DbNode(Node::Value(serde_json::Value::Null)),
            "",
            &DbNode(Node::Value(serde_json::Value::Null)),
        )
            .unwrap()
            .into_iter()
            .map(|r| format!("{:?}", (r.subject.0, r.predicate, r.object.0, r.commit_, r.exists)))
            .collect::<Vec<_>>();
    have.sort();
    pretty_assertions::assert_eq!(want, have);
}
