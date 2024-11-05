use {
    chrono::Utc,
    interface::{
        iam::IamConfig,
        query::{
            Chain,
            MoveDirection,
            Query,
            Step,
            StepMove,
            StepRecurse,
            Subchain,
            Value,
        },
        triple::Node,
    },
    query::build_query,
    std::{
        io::Write,
        path::PathBuf,
        process::{
            Command,
            Stdio,
        },
    },
};

pub mod interface;
pub mod db;
pub mod query;
pub mod query_test;

fn main() {
    let (query, query_values) = build_query(Query {
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
    }, Default::default()).unwrap();

    //. let mut db = rusqlite::Connection::open_in_memory().unwrap();
    let mut db = rusqlite::Connection::open("./test.sqlite3").unwrap();
    db::migrate(&mut db).unwrap();
    db::singleton_init(&db, &IamConfig {
        targets: vec![],
        access: vec![],
        roles: vec![],
        members: vec![],
    }).unwrap();
    for (s, p, o) in [
        //. .
        ("a", "sunwet/1/is", "sunwet/1/album"),
        ("a", "sunwet/1/name", "a_name"),
        ("a", "sunwet/1/artist", "a_a"),
        ("a_a", "sunwet/1/name", "a_a_name"),
    ] {
        db::triple_insert(
            &db,
            &Node::Id(s.to_string()),
            p,
            &Node::Id(o.to_string()),
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
    let mut s = db.prepare(&query).unwrap();
    let mut results = s.query(&*query_values.as_params()).unwrap();
    let mut count = 0;
    loop {
        let Some(row) = results.next().unwrap() else {
            break;
        };
        println!("row: {:?}", row);
        count += 1;
    }
    println!("Result count: {}", count);
}
