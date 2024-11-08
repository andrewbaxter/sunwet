#![cfg(test)]

use {
    crate::{
        db,
        interface::{
            iam::IamConfig,
            query::{
                Chain,
                FilterExpr,
                FilterExprComparison,
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
        query::{
            build_query,
            IAM_TARGET_ADMIN_ONLY,
        },
    },
    chrono::{
        Duration,
        TimeZone,
        Utc,
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

#[derive(PartialEq, Eq, PartialOrd, Debug, Clone)]
enum ResVal {
    Scalar(Node),
    Array(Vec<Node>),
}

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

fn execute(triples: &[(&Node, &str, &Node)], want: &[&[(&str, ResVal)]], query: Query) {
    let (query, query_values) = build_query(query, HashMap::new()).unwrap();
    let mut db = rusqlite::Connection::open_in_memory().unwrap();
    db::migrate(&mut db).unwrap();
    db::singleton_init(&db, &IamConfig {
        targets: vec![],
        access: vec![],
        roles: vec![],
        members: vec![],
    }).unwrap();
    for (s, p, o) in triples {
        db::triple_insert(&db, &s, p, &o, Utc::now().into(), true, 0).unwrap();
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
    let mut s = db.prepare(&query).unwrap();
    let column_names = s.column_names().into_iter().map(|k| k.to_string()).collect::<Vec<_>>();
    let got = {
        let mut got_rows = s.query(&*query_values.as_params()).unwrap();
        let mut got1 = vec![];
        loop {
            let Some(got_row) = got_rows.next().unwrap() else {
                break;
            };
            let mut got_row1 = HashMap::new();
            for (i, name) in column_names.iter().enumerate() {
                let value: ResVal;
                match got_row.get::<usize, Option<String>>(i).unwrap() {
                    Some(v) => {
                        let json_value = serde_json::from_str::<serde_json::Value>(&v).unwrap();
                        if let serde_json::Value::Array(arr) = json_value {
                            let mut elements = vec![];
                            for v in arr {
                                elements.push(serde_json::from_value::<Node>(v).unwrap());
                            }
                            value = ResVal::Array(elements);
                        } else {
                            value = ResVal::Scalar(serde_json::from_value(json_value).unwrap());
                        };
                    },
                    None => {
                        value = ResVal::Scalar(Node::Value(serde_json::Value::Null));
                    },
                }
                got_row1.insert(name.to_string(), value);
            }
            got1.push(got_row1);
        }
        got1
    };
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
            (&id("a"), "sunwet/1/is", &id("sunwet/1/album")),
            (&id("a"), "sunwet/1/name", &s("a_name")),
            (&id("a"), "sunwet/1/artist", &id("a_a")),
            (&id("a_a"), "sunwet/1/name", &s("a_a_name")),
        ],
        &[
            &[
                ("id", ResVal::Scalar(id("a"))),
                ("name", ResVal::Scalar(s("a_name"))),
                ("artist", ResVal::Scalar(s("a_a_name"))),
                ("cover", ResVal::Scalar(n())),
            ],
        ],
        Query {
            chain: Chain {
                subchain: Subchain {
                    root: Some(Value::Literal(Node::Id("sunwet/1/album".to_string()))),
                    steps: vec![Step::Move(StepMove {
                        dir: MoveDirection::Up,
                        predicate: "sunwet/1/is".to_string(),
                        first: false,
                        filter: None,
                    })],
                },
                select: Some("id".to_string()),
                children: vec![
                    //. .
                    Chain {
                        subchain: Subchain {
                            root: None,
                            steps: vec![
                                //. .
                                Step::Recurse(StepRecurse {
                                    subchain: Subchain {
                                        root: None,
                                        steps: vec![Step::Move(StepMove {
                                            dir: MoveDirection::Up,
                                            predicate: "sunwet/1/element".to_string(),
                                            first: false,
                                            filter: None,
                                        })],
                                    },
                                    first: false,
                                }),
                                Step::Move(StepMove {
                                    dir: MoveDirection::Down,
                                    predicate: "sunwet/1/name".to_string(),
                                    first: true,
                                    filter: None,
                                })
                            ],
                        },
                        select: Some("name".to_string()),
                        children: Default::default(),
                    },
                    Chain {
                        subchain: Subchain {
                            root: None,
                            steps: vec![
                                //. .
                                Step::Recurse(StepRecurse {
                                    subchain: Subchain {
                                        root: None,
                                        steps: vec![Step::Move(StepMove {
                                            dir: MoveDirection::Up,
                                            predicate: "sunwet/1/element".to_string(),
                                            first: false,
                                            filter: None,
                                        })],
                                    },
                                    first: false,
                                }),
                                Step::Move(StepMove {
                                    dir: MoveDirection::Down,
                                    predicate: "sunwet/1/artist".to_string(),
                                    first: true,
                                    filter: None,
                                }),
                                Step::Recurse(StepRecurse {
                                    subchain: Subchain {
                                        root: None,
                                        steps: vec![Step::Move(StepMove {
                                            dir: MoveDirection::Up,
                                            predicate: "sunwet/1/element".to_string(),
                                            first: false,
                                            filter: None,
                                        })],
                                    },
                                    first: false,
                                }),
                                Step::Move(StepMove {
                                    dir: MoveDirection::Down,
                                    predicate: "sunwet/1/name".to_string(),
                                    first: true,
                                    filter: None,
                                })
                            ],
                        },
                        select: Some("artist".to_string()),
                        children: Default::default(),
                    },
                    Chain {
                        subchain: Subchain {
                            root: None,
                            steps: vec![
                                //. .
                                Step::Recurse(StepRecurse {
                                    subchain: Subchain {
                                        root: None,
                                        steps: vec![Step::Move(StepMove {
                                            dir: MoveDirection::Up,
                                            predicate: "sunwet/1/element".to_string(),
                                            first: false,
                                            filter: None,
                                        })],
                                    },
                                    first: false,
                                }),
                                Step::Move(StepMove {
                                    dir: MoveDirection::Down,
                                    predicate: "sunwet/1/cover".to_string(),
                                    first: true,
                                    filter: None,
                                })
                            ],
                        },
                        select: Some("cover".to_string()),
                        children: Default::default(),
                    }
                ],
            },
            sort: vec![],
        },
    );
}

#[test]
fn test_recurse() {
    execute(
        &[
            (&id("a"), "sunwet/1/is", &id("sunwet/1/album")),
            (&id("a"), "sunwet/1/name", &s("a_name")),
            (&id("b"), "sunwet/1/is", &id("sunwet/1/album")),
            (&id("b_p"), "sunwet/1/element", &id("b")),
            (&id("b_p"), "sunwet/1/name", &s("b_name")),
        ],
        &[&[("name", ResVal::Scalar(s("a_name")))], &[("name", ResVal::Scalar(s("b_name")))]],
        Query {
            chain: Chain {
                subchain: Subchain {
                    root: Some(Value::Literal(id("sunwet/1/album"))),
                    steps: vec![
                        //. .
                        Step::Move(StepMove {
                            dir: MoveDirection::Up,
                            predicate: "sunwet/1/is".to_string(),
                            filter: None,
                            first: false,
                        }),
                        Step::Recurse(StepRecurse {
                            subchain: Subchain {
                                root: None,
                                steps: vec![Step::Move(StepMove {
                                    dir: MoveDirection::Up,
                                    predicate: "sunwet/1/element".to_string(),
                                    filter: None,
                                    first: false,
                                })],
                            },
                            first: false,
                        }),
                        Step::Move(StepMove {
                            dir: MoveDirection::Down,
                            predicate: "sunwet/1/name".to_string(),
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
            (&id("a"), "sunwet/1/is", &id("sunwet/1/album")),
            (&id("a"), "sunwet/1/name", &s("a_name")),
            (&id("b"), "sunwet/1/is", &id("sunwet/1/album")),
            (&id("b"), "sunwet/1/name", &s("b_name")),
        ],
        &[&[("id", ResVal::Scalar(id("a")))]],
        Query {
            chain: Chain {
                subchain: Subchain {
                    root: Some(Value::Literal(id("sunwet/1/album"))),
                    steps: vec![
                        //. .
                        Step::Move(StepMove {
                            dir: MoveDirection::Up,
                            predicate: "sunwet/1/is".to_string(),
                            filter: Some(FilterExpr::Comparison(FilterExprComparison {
                                type_: crate::interface::query::FilterExprComparisonType::Exists,
                                subchain: Subchain {
                                    root: None,
                                    steps: vec![Step::Move(StepMove {
                                        dir: MoveDirection::Down,
                                        predicate: "sunwet/1/name".to_string(),
                                        filter: None,
                                        first: false,
                                    })],
                                },
                                operator: crate::interface::query::FilterChainComparisonOperator::Eq,
                                value: Value::Literal(s("a_name")),
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
            (&id("a"), "sunwet/1/is", &id("sunwet/1/album")),
            (&id("a"), "sunwet/1/q", &i(12)),
            (&id("b"), "sunwet/1/is", &id("sunwet/1/album")),
            (&id("b"), "sunwet/1/q", &i(47)),
        ],
        &[&[("id", ResVal::Scalar(id("b")))]],
        Query {
            chain: Chain {
                subchain: Subchain {
                    root: Some(Value::Literal(id("sunwet/1/album"))),
                    steps: vec![
                        //. .
                        Step::Move(StepMove {
                            dir: MoveDirection::Up,
                            predicate: "sunwet/1/is".to_string(),
                            filter: Some(FilterExpr::Comparison(FilterExprComparison {
                                type_: crate::interface::query::FilterExprComparisonType::Exists,
                                subchain: Subchain {
                                    root: None,
                                    steps: vec![Step::Move(StepMove {
                                        dir: MoveDirection::Down,
                                        predicate: "sunwet/1/q".to_string(),
                                        filter: None,
                                        first: false,
                                    })],
                                },
                                operator: crate::interface::query::FilterChainComparisonOperator::Gte,
                                value: Value::Literal(i(30)),
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
            (&id("a"), "sunwet/1/is", &id("sunwet/1/album")),
            (&id("b"), "sunwet/1/is", &id("sunwet/1/dog")),
            (&id("d"), "sunwet/1/is", &id("sunwet/1/what")),
        ],
        &[
            //. .
            &[("id", ResVal::Scalar(id("b")))],
            &[("id", ResVal::Scalar(id("d")))],
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
                                    predicate: "sunwet/1/is".to_string(),
                                    filter: None,
                                    first: false,
                                })],
                            }, Subchain {
                                root: Some(Value::Literal(id("sunwet/1/what"))),
                                steps: vec![Step::Move(StepMove {
                                    dir: MoveDirection::Up,
                                    predicate: "sunwet/1/is".to_string(),
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
    db::singleton_init(&db, &IamConfig {
        targets: vec![],
        access: vec![],
        roles: vec![],
        members: vec![],
    }).unwrap();
    let stamp1 = chrono::Local.with_ymd_and_hms(2014, 10, 1, 1, 1, 1).unwrap().into();
    let stamp2 = chrono::Local.with_ymd_and_hms(2014, 11, 1, 1, 1, 1).unwrap().into();
    let stamp3 = chrono::Local.with_ymd_and_hms(2014, 12, 1, 1, 1, 1).unwrap().into();

    // Newest is after epoch
    db::triple_insert(&db, &s("a"), "b", &s("c"), stamp1, true, IAM_TARGET_ADMIN_ONLY).unwrap();
    db::triple_insert(&db, &s("a"), "b", &s("c"), stamp2, false, IAM_TARGET_ADMIN_ONLY).unwrap();
    db::triple_insert(&db, &s("a"), "b", &s("c"), stamp3, true, IAM_TARGET_ADMIN_ONLY).unwrap();

    // Newest is before epoch, but exists
    db::triple_insert(&db, &s("d"), "e", &s("f"), stamp1, false, IAM_TARGET_ADMIN_ONLY).unwrap();
    db::triple_insert(&db, &s("d"), "e", &s("f"), stamp2, true, IAM_TARGET_ADMIN_ONLY).unwrap();

    // Newest is before epoch, but doesn't exist
    db::triple_insert(&db, &s("g"), "h", &s("i"), stamp1, true, IAM_TARGET_ADMIN_ONLY).unwrap();
    db::triple_insert(&db, &s("g"), "h", &s("i"), stamp1, false, IAM_TARGET_ADMIN_ONLY).unwrap();

    // Gc
    db::triple_gc_deleted(&db, stamp2 + Duration::seconds(1)).unwrap();
    let want = vec![
        //. .
        format!("{:?}", (s("a"), "b".to_string(), s("c"), stamp3, true, IAM_TARGET_ADMIN_ONLY)),
        format!("{:?}", (s("d"), "e".to_string(), s("f"), stamp2, true, IAM_TARGET_ADMIN_ONLY))
    ];
    let mut have =
        db::triple_get_all(&db)
            .unwrap()
            .into_iter()
            .map(|r| format!("{:?}", (r.subject, r.predicate, r.object, r.timestamp, r.exists, r.iam_target)))
            .collect::<Vec<_>>();
    have.sort();
    pretty_assertions::assert_eq!(want, have);
    db::triple_gc_deleted(&db, stamp2 + Duration::seconds(1)).unwrap();
    let mut have =
        db::triple_get_all(&db)
            .unwrap()
            .into_iter()
            .map(|r| format!("{:?}", (r.subject, r.predicate, r.object, r.timestamp, r.exists, r.iam_target)))
            .collect::<Vec<_>>();
    have.sort();
    pretty_assertions::assert_eq!(want, have);
}
