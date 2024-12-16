#![cfg(test)]

use {
    super::defaultviews::{
        default_query_albums,
        node_is_album,
        PREDICATE_CREATOR,
        PREDICATE_ELEMENT,
        PREDICATE_IS,
        PREDICATE_NAME,
    },
    crate::{
        db,
        query::{
            build_query,
            QueryAccess,
            QueryResVal,
        },
        serverlib::query::execute_sql_query,
    },
    chrono::{
        Duration,
        TimeZone,
        Utc,
    },
    native::interface::triple::DbNode,
    shared::interface::{
        iam::IAM_TARGET_ADMIN,
        query::{
            Chain,
            FilterChainComparisonOperator,
            FilterExpr,
            FilterExprExists,
            FilterExprExistsType,
            JunctionType,
            MoveDirection,
            Query,
            Step,
            StepJunction,
            StepMove,
            StepRecurse,
            Subchain,
            Value,
        },
        triple::Node,
    },
    std::{
        collections::HashMap,
        io::Write,
        path::PathBuf,
        process::{
            Command,
            Stdio,
        },
    },
};

fn id(value: impl AsRef<str>) -> Node {
    return Node::Id(value.as_ref().to_string());
}

fn s(value: impl AsRef<str>) -> Node {
    return Node::Value(serde_json::Value::String(value.as_ref().to_string()));
}

fn i(value: i32) -> Node {
    return Node::Value(serde_json::Value::Number(serde_json::Number::from(value)));
}

fn n() -> Node {
    return Node::Value(serde_json::Value::Null);
}

fn execute(triples: &[(&Node, &str, &Node)], want: &[&[(&str, QueryResVal)]], query: Query) {
    let (query, query_values) = build_query(QueryAccess::Admin, query, HashMap::new()).unwrap();
    let mut db = rusqlite::Connection::open_in_memory().unwrap();
    db::migrate(&mut db).unwrap();
    for (s, p, o) in triples {
        db::triple_insert(
            &db,
            &DbNode((*s).clone()),
            p,
            &DbNode((*o).clone()),
            Utc::now().into(),
            true,
            0,
        ).unwrap();
    }
    {
        let prettier_root = PathBuf::from("/home/andrew/temp/soft/node/node_modules/");
        let mut prettier = Command::new(prettier_root.join(".bin/prettier"));
        prettier
            .arg("--parser")
            .arg("sql")
            .arg("--plugin")
            .arg(prettier_root.join("prettier-plugin-sql/lib/index.cjs"));
        prettier.stdin(Stdio::piped());
        prettier.stdout(Stdio::piped());
        let mut child = prettier.spawn().unwrap();
        let mut child_stdin = child.stdin.take().unwrap();
        child_stdin.write_all(query.as_bytes()).unwrap();
        drop(child_stdin);
        let output = child.wait_with_output().unwrap();
        if !output.status.success() {
            panic!();
        }
        println!("Query: {}", String::from_utf8(output.stdout).unwrap());
    }
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
    let got = execute_sql_query(&db, query, query_values).unwrap();
    let want =
        want
            .into_iter()
            .map(|m| m.into_iter().map(|(k, v)| (k.to_string(), v.clone())).collect::<HashMap<_, _>>())
            .collect::<Vec<_>>();
    pretty_assertions::assert_eq!(want, got);
}

#[test]
fn test_base() {
    execute(
        &[
            (&id("a"), PREDICATE_IS, &node_is_album()),
            (&id("a"), PREDICATE_NAME, &s("a_name")),
            (&id("a"), PREDICATE_CREATOR, &id("a_a")),
            (&id("a_a"), PREDICATE_NAME, &s("a_a_name")),
        ],
        &[
            &[
                ("id", QueryResVal::Scalar(id("a"))),
                ("name", QueryResVal::Scalar(s("a_name"))),
                ("artist", QueryResVal::Scalar(s("a_a_name"))),
                ("cover", QueryResVal::Scalar(n())),
            ],
        ],
        default_query_albums(),
    );
}

#[test]
fn test_recurse() {
    execute(
        &[
            (&id("a"), PREDICATE_IS, &node_is_album()),
            (&id("a"), PREDICATE_NAME, &s("a_name")),
            (&id("b"), PREDICATE_IS, &node_is_album()),
            (&id("b_p"), PREDICATE_ELEMENT, &id("b")),
            (&id("b_p"), PREDICATE_NAME, &s("b_name")),
        ],
        &[&[("name", QueryResVal::Scalar(s("a_name")))], &[("name", QueryResVal::Scalar(s("b_name")))]],
        Query {
            chain: Chain {
                subchain: Subchain {
                    root: Some(Value::Literal(node_is_album())),
                    steps: vec![
                        //. .
                        Step::Move(StepMove {
                            dir: MoveDirection::Up,
                            predicate: PREDICATE_IS.to_string(),
                            filter: None,
                            first: false,
                        }),
                        Step::Recurse(StepRecurse {
                            subchain: Subchain {
                                root: None,
                                steps: vec![Step::Move(StepMove {
                                    dir: MoveDirection::Up,
                                    predicate: PREDICATE_ELEMENT.to_string(),
                                    filter: None,
                                    first: false,
                                })],
                            },
                            first: false,
                        }),
                        Step::Move(StepMove {
                            dir: MoveDirection::Down,
                            predicate: PREDICATE_NAME.to_string(),
                            filter: None,
                            first: false,
                        })
                    ],
                },
                select: Some("name".to_string()),
                children: vec![],
            },
            sort: vec![],
        },
    );
}

#[test]
fn test_filter_eq() {
    execute(
        &[
            (&id("a"), PREDICATE_IS, &id("sunwet/1/album")),
            (&id("a"), PREDICATE_NAME, &s("a_name")),
            (&id("b"), PREDICATE_IS, &id("sunwet/1/album")),
            (&id("b"), PREDICATE_NAME, &s("b_name")),
        ],
        &[&[("id", QueryResVal::Scalar(id("a")))]],
        Query {
            chain: Chain {
                subchain: Subchain {
                    root: Some(Value::Literal(id("sunwet/1/album"))),
                    steps: vec![
                        //. .
                        Step::Move(StepMove {
                            dir: MoveDirection::Up,
                            predicate: PREDICATE_IS.to_string(),
                            filter: Some(FilterExpr::Exists(FilterExprExists {
                                type_: FilterExprExistsType::Exists,
                                subchain: Subchain {
                                    root: None,
                                    steps: vec![Step::Move(StepMove {
                                        dir: MoveDirection::Down,
                                        predicate: PREDICATE_NAME.to_string(),
                                        filter: None,
                                        first: false,
                                    })],
                                },
                                filter: Some((FilterChainComparisonOperator::Eq, Value::Literal(s("a_name")))),
                            })),
                            first: false,
                        })
                    ],
                },
                select: Some("id".to_string()),
                children: vec![],
            },
            sort: vec![],
        },
    );
}

#[test]
fn test_filter_lt() {
    execute(
        &[
            (&id("a"), PREDICATE_IS, &node_is_album()),
            (&id("a"), "sunwet/1/q", &i(12)),
            (&id("b"), PREDICATE_IS, &node_is_album()),
            (&id("b"), "sunwet/1/q", &i(47)),
        ],
        &[&[("id", QueryResVal::Scalar(id("b")))]],
        Query {
            chain: Chain {
                subchain: Subchain {
                    root: Some(Value::Literal(node_is_album())),
                    steps: vec![
                        //. .
                        Step::Move(StepMove {
                            dir: MoveDirection::Up,
                            predicate: PREDICATE_IS.to_string(),
                            filter: Some(FilterExpr::Exists(FilterExprExists {
                                type_: FilterExprExistsType::Exists,
                                subchain: Subchain {
                                    root: None,
                                    steps: vec![Step::Move(StepMove {
                                        dir: MoveDirection::Down,
                                        predicate: "sunwet/1/q".to_string(),
                                        filter: None,
                                        first: false,
                                    })],
                                },
                                filter: Some((FilterChainComparisonOperator::Gte, Value::Literal(i(30)))),
                            })),
                            first: false,
                        })
                    ],
                },
                select: Some("id".to_string()),
                children: vec![],
            },
            sort: vec![],
        },
    );
}

#[test]
fn test_chain_union() {
    execute(
        &[
            (&id("a"), PREDICATE_IS, &node_is_album()),
            (&id("b"), PREDICATE_IS, &id("sunwet/1/dog")),
            (&id("d"), PREDICATE_IS, &id("sunwet/1/what")),
        ],
        &[
            //. .
            &[("id", QueryResVal::Scalar(id("b")))],
            &[("id", QueryResVal::Scalar(id("d")))],
        ],
        Query {
            chain: Chain {
                subchain: Subchain {
                    root: None,
                    steps: vec![
                        //. .
                        Step::Junction(StepJunction {
                            type_: JunctionType::Or,
                            subchains: vec![Subchain {
                                root: Some(Value::Literal(id("sunwet/1/dog"))),
                                steps: vec![Step::Move(StepMove {
                                    dir: MoveDirection::Up,
                                    predicate: PREDICATE_IS.to_string(),
                                    filter: None,
                                    first: false,
                                })],
                            }, Subchain {
                                root: Some(Value::Literal(id("sunwet/1/what"))),
                                steps: vec![Step::Move(StepMove {
                                    dir: MoveDirection::Up,
                                    predicate: PREDICATE_IS.to_string(),
                                    filter: None,
                                    first: false,
                                })],
                            }],
                        })
                    ],
                },
                select: Some("id".to_string()),
                children: vec![],
            },
            sort: vec![],
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
    db::triple_insert(&db, &DbNode(s("a")), "b", &DbNode(s("c")), stamp1, true, IAM_TARGET_ADMIN.0).unwrap();
    db::triple_insert(&db, &DbNode(s("a")), "b", &DbNode(s("c")), stamp2, false, IAM_TARGET_ADMIN.0).unwrap();
    db::triple_insert(&db, &DbNode(s("a")), "b", &DbNode(s("c")), stamp3, true, IAM_TARGET_ADMIN.0).unwrap();

    // Newest is before epoch, but exists
    db::triple_insert(&db, &DbNode(s("d")), "e", &DbNode(s("f")), stamp1, false, IAM_TARGET_ADMIN.0).unwrap();
    db::triple_insert(&db, &DbNode(s("d")), "e", &DbNode(s("f")), stamp2, true, IAM_TARGET_ADMIN.0).unwrap();

    // Newest is before epoch, but doesn't exist
    db::triple_insert(&db, &DbNode(s("g")), "h", &DbNode(s("i")), stamp1, true, IAM_TARGET_ADMIN.0).unwrap();
    db::triple_insert(&db, &DbNode(s("g")), "h", &DbNode(s("i")), stamp1, false, IAM_TARGET_ADMIN.0).unwrap();

    // Gc
    db::triple_gc_deleted(&db, stamp2 + Duration::seconds(1)).unwrap();
    let want = vec![
        //. .
        format!("{:?}", (s("a"), "b".to_string(), s("c"), stamp3, true, IAM_TARGET_ADMIN.0)),
        format!("{:?}", (s("d"), "e".to_string(), s("f"), stamp2, true, IAM_TARGET_ADMIN.0))
    ];
    let mut have =
        db::triple_list_all(&db)
            .unwrap()
            .into_iter()
            .map(|r| format!("{:?}", (r.subject, r.predicate, r.object, r.timestamp, r.exists, r.iam_target)))
            .collect::<Vec<_>>();
    have.sort();
    pretty_assertions::assert_eq!(want, have);
    db::triple_gc_deleted(&db, stamp2 + Duration::seconds(1)).unwrap();
    let mut have =
        db::triple_list_all(&db)
            .unwrap()
            .into_iter()
            .map(|r| format!("{:?}", (r.subject, r.predicate, r.object, r.timestamp, r.exists, r.iam_target)))
            .collect::<Vec<_>>();
    have.sort();
    pretty_assertions::assert_eq!(want, have);
}
