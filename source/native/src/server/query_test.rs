#![cfg(test)]

use {
    super::defaultviews::node_is_album,
    crate::{
        interface::triple::DbNode,
        server::{
            db,
            dbutil,
            dbwrite,
            defaultviews::node_media_audio,
            migrate,
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
    htwrap::htserve::viserr::VisErr,
    shared::{
        interface::{
            ont::{
                PREDICATE_ADD_TIMESTAMP,
                PREDICATE_ARTIST,
                PREDICATE_IS,
                PREDICATE_MEDIA,
                PREDICATE_NAME,
                PREDICATE_TRACK,
            },
            query::{
                ChainHead,
                ChainRoot,
                ChainTail,
                FilterExpr,
                FilterExprExistance,
                FilterExprExistsType,
                FilterSuffixSimple,
                FilterSuffixSimpleOperator,
                JunctionType,
                MoveDirection,
                Query,
                QuerySuffix,
                Step,
                StepJunction,
                StepMove,
                StepRecurse,
                StepSpecific,
                StrValue,
                Value,
            },
            triple::Node,
            wire::TreeNode,
        },
        query_parser::compile_query,
    },
    rusqlite::OptionalExtension,
    std::{
        collections::{
            BTreeMap,
            HashMap,
            HashSet,
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
    let (query_string, query_values) = build_root_chain(&query, HashMap::new()).map_err(|e| {
        panic!("{}", match e {
            VisErr::Internal(e) => e.to_string(),
            VisErr::External(e) => e,
        })
    }).unwrap();
    let mut db = rusqlite::Connection::open_in_memory().unwrap();
    let mut db = db::migrate(db, None).unwrap();
    for (s, p, o) in triples {
        dbwrite::write_triple(
            &mut db,
            &DbNode((*s).clone()),
            p,
            &DbNode((*o).clone()),
            Utc::now().into(),
            true,
        ).unwrap();
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
    println!("Query: {}", query_string);

    //.    {
    //.        let mut s = db.0.prepare(&format!("explain query plan {}", query)).unwrap();
    //.        let mut results = s.query(&*query_values.as_params()).unwrap();
    //.        loop {
    //.            let Some(row) = results.next().unwrap() else {
    //.                break;
    //.            };
    //.            println!("explain row: {:?}", row);
    //.        }
    //.    }
    let got =
        execute_sql_query(&mut db::Db(&mut db.0.transaction().unwrap()), query_string, query_values, &query, None)
            .unwrap()
            .into_iter()
            .map(|x| x.tail_data)
            .collect::<Vec<_>>();
    let want = want.into_iter().map(|m| {
        m.into_iter().map(|(k, v)| (k.to_string(), v.clone())).collect::<BTreeMap<_, _>>()
    }).collect::<Vec<_>>();
    assert_eq!(want, got);
}

fn src_query_dir() -> PathBuf {
    return PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../queries");
}

#[test]
fn test_base() {
    let query_dir = src_query_dir();
    let query_head = compile_query(&read_to_string(&query_dir.join("query_audio_albums.txt")).unwrap()).unwrap();
    let query_tail =
        compile_query(&read_to_string(&query_dir.join("query_audio_albums_select.txt")).unwrap()).unwrap();
    execute(
        &[
            (&s("a"), PREDICATE_IS, &node_is_album()),
            (&s("a"), PREDICATE_MEDIA, &node_media_audio()),
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
        Query {
            chain_head: query_head.chain_head,
            suffix: query_tail.suffix,
        },
    );
}

#[test]
fn test_versions() {
    let query = compile_query("\"x\" -> \"y\" { => y }").unwrap();
    let (query_sql, query_values) = build_root_chain(&query, HashMap::new()).map_err(|e| {
        panic!("{}", match e {
            VisErr::Internal(e) => e.to_string(),
            VisErr::External(e) => e,
        })
    }).unwrap();
    let mut db = rusqlite::Connection::open_in_memory().unwrap();
    let mut db = db::migrate(db, None).unwrap();
    dbwrite::write_triple(
        &mut db,
        &DbNode(s("x")),
        "y",
        &DbNode(s("no")),
        DateTime::from_timestamp_nanos(1),
        true,
    ).unwrap();
    dbwrite::write_triple(
        &mut db,
        &DbNode(s("x")),
        "y",
        &DbNode(s("no")),
        DateTime::from_timestamp_nanos(2),
        true,
    ).unwrap();
    println!("Query: {}", query_sql);
    {
        let mut s = db.0.prepare(&format!("explain query plan {}", query_sql)).unwrap();
        let mut results = s.query(&*query_values.as_params()).unwrap();
        loop {
            let Some(row) = results.next().unwrap() else {
                break;
            };
            println!("explain row: {:?}", row);
        }
    }
    let got =
        execute_sql_query(&mut db::Db(&mut db.0.transaction().unwrap()), query_sql, query_values, &query, None)
            .unwrap()
            .into_iter()
            .map(|x| x.tail_data)
            .collect::<Vec<_>>();
    assert_eq!(got, vec![[("y".to_string(), TreeNode::Scalar(s("no")))].into_iter().collect::<BTreeMap<_, _>>()]);
}

#[test]
fn test_delete() {
    let query = compile_query("\"x\" -> \"y\" { => y }").unwrap();
    let (query_sql, query_values) = build_root_chain(&query, HashMap::new()).map_err(|e| {
        panic!("{}", match e {
            VisErr::Internal(e) => e.to_string(),
            VisErr::External(e) => e,
        })
    }).unwrap();
    let mut db = rusqlite::Connection::open_in_memory().unwrap();
    let mut db = db::migrate(db, None).unwrap();
    dbwrite::write_triple(
        &mut db,
        &DbNode(s("x")),
        "y",
        &DbNode(s("no")),
        DateTime::from_timestamp_nanos(1),
        true,
    ).unwrap();
    dbwrite::write_triple(
        &mut db,
        &DbNode(s("x")),
        "y",
        &DbNode(s("no")),
        DateTime::from_timestamp_nanos(2),
        false,
    ).unwrap();
    println!("Query: {}", query_sql);
    {
        let mut s = db.0.prepare(&format!("explain query plan {}", query_sql)).unwrap();
        let mut results = s.query(&*query_values.as_params()).unwrap();
        loop {
            let Some(row) = results.next().unwrap() else {
                break;
            };
            println!("explain row: {:?}", row);
        }
    }
    let got =
        execute_sql_query(&mut db::Db(&mut db.0.transaction().unwrap()), query_sql, query_values, &query, None)
            .unwrap()
            .into_iter()
            .map(|x| x.tail_data)
            .collect::<Vec<_>>();
    assert_eq!(got, vec![]);
}

#[test]
fn test_undelete() {
    let query = compile_query("\"x\" -> \"y\" { => y }").unwrap();
    let (query_sql, query_values) = build_root_chain(&query, HashMap::new()).map_err(|e| {
        panic!("{}", match e {
            VisErr::Internal(e) => e.to_string(),
            VisErr::External(e) => e,
        })
    }).unwrap();
    let mut db = rusqlite::Connection::open_in_memory().unwrap();
    let mut db = db::migrate(db, None).unwrap();
    dbwrite::write_triple(
        &mut db,
        &DbNode(s("x")),
        "y",
        &DbNode(s("no")),
        DateTime::from_timestamp_nanos(1),
        true,
    ).unwrap();
    dbwrite::write_triple(
        &mut db,
        &DbNode(s("x")),
        "y",
        &DbNode(s("no")),
        DateTime::from_timestamp_nanos(2),
        false,
    ).unwrap();
    dbwrite::write_triple(
        &mut db,
        &DbNode(s("x")),
        "y",
        &DbNode(s("no")),
        DateTime::from_timestamp_nanos(3),
        true,
    ).unwrap();
    println!("Query: {}", query_sql);
    {
        let mut s = db.0.prepare(&format!("explain query plan {}", query_sql)).unwrap();
        let mut results = s.query(&*query_values.as_params()).unwrap();
        loop {
            let Some(row) = results.next().unwrap() else {
                break;
            };
            println!("explain row: {:?}", row);
        }
    }
    let got =
        execute_sql_query(&mut db::Db(&mut db.0.transaction().unwrap()), query_sql, query_values, &query, None)
            .unwrap()
            .into_iter()
            .map(|x| x.tail_data)
            .collect::<Vec<_>>();
    assert_eq!(got, vec![[("y".to_string(), TreeNode::Scalar(s("no")))].into_iter().collect::<BTreeMap<_, _>>()]);
}

#[test]
fn test_recurse() {
    execute(
        &[
            (&s("a"), PREDICATE_IS, &node_is_album()),
            (&s("a"), PREDICATE_MEDIA, &node_media_audio()),
            (&s("a"), PREDICATE_NAME, &s("a_name")),
            (&s("b"), PREDICATE_IS, &node_is_album()),
            (&s("b"), PREDICATE_MEDIA, &node_media_audio()),
            (&s("b_p"), PREDICATE_TRACK, &s("b")),
            (&s("b_p"), PREDICATE_NAME, &s("b_name")),
        ],
        &[&[("name", TreeNode::Scalar(s("a_name")))], &[("name", TreeNode::Scalar(s("b_name")))]],
        Query {
            chain_head: ChainHead {
                root: Some(ChainRoot::Value(Value::Literal(node_is_album()))),
                steps: vec![
                    //. .
                    Step {
                        specific: StepSpecific::Move(StepMove {
                            dir: MoveDirection::Backward,
                            predicate: StrValue::Literal(PREDICATE_IS.to_string()),
                            filter: None,
                        }),
                        sort: None,
                        first: false,
                    },
                    Step {
                        specific: StepSpecific::Recurse(StepRecurse { subchain: ChainHead {
                            root: None,
                            steps: vec![Step {
                                specific: StepSpecific::Move(StepMove {
                                    dir: MoveDirection::Backward,
                                    predicate: StrValue::Literal(PREDICATE_TRACK.to_string()),
                                    filter: None,
                                }),
                                sort: None,
                                first: false,
                            }],
                        }, }),
                        sort: None,
                        first: false,
                    },
                    Step {
                        specific: StepSpecific::Move(StepMove {
                            dir: MoveDirection::Forward,
                            predicate: StrValue::Literal(PREDICATE_NAME.to_string()),
                            filter: None,
                        }),
                        sort: None,
                        first: false,
                    },
                ],
            },
            suffix: Some(QuerySuffix {
                chain_tail: ChainTail {
                    bind: Some("name".to_string()),
                    subchains: vec![],
                },
                sort: None,
            }),
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
            chain_head: ChainHead {
                root: Some(ChainRoot::Value(Value::Literal(node_is_album()))),
                steps: vec![
                    //. .
                    Step {
                        specific: StepSpecific::Move(StepMove {
                            dir: MoveDirection::Backward,
                            predicate: StrValue::Literal(PREDICATE_IS.to_string()),
                            filter: Some(FilterExpr::Exists(FilterExprExistance {
                                type_: FilterExprExistsType::Exists,
                                subchain: ChainHead {
                                    root: None,
                                    steps: vec![Step {
                                        specific: StepSpecific::Move(StepMove {
                                            dir: MoveDirection::Forward,
                                            predicate: StrValue::Literal(PREDICATE_NAME.to_string(),),
                                            filter: None,
                                        }),
                                        sort: None,
                                        first: false,
                                    }],
                                },
                                suffix: Some(shared::interface::query::FilterSuffix::Simple(FilterSuffixSimple {
                                    op: FilterSuffixSimpleOperator::Eq,
                                    value: Value::Literal(s("a_name")),
                                },)),
                            })),
                        }),
                        sort: None,
                        first: false,
                    },
                ],
            },
            suffix: Some(QuerySuffix {
                chain_tail: ChainTail {
                    bind: Some("id".to_string()),
                    subchains: vec![],
                },
                sort: None,
            }),
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
            chain_head: ChainHead {
                root: Some(ChainRoot::Value(Value::Literal(node_is_album()))),
                steps: vec![
                    //. .
                    Step {
                        specific: StepSpecific::Move(StepMove {
                            dir: MoveDirection::Backward,
                            predicate: StrValue::Literal(PREDICATE_IS.to_string()),
                            filter: Some(FilterExpr::Exists(FilterExprExistance {
                                type_: FilterExprExistsType::Exists,
                                subchain: ChainHead {
                                    root: None,
                                    steps: vec![Step {
                                        specific: StepSpecific::Move(StepMove {
                                            dir: MoveDirection::Forward,
                                            predicate: StrValue::Literal("sunwet/1/q".to_string()),
                                            filter: None,
                                        }),
                                        sort: None,
                                        first: false,
                                    }],
                                },
                                suffix: Some(shared::interface::query::FilterSuffix::Simple(FilterSuffixSimple {
                                    op: FilterSuffixSimpleOperator::Gte,
                                    value: Value::Literal(i(30)),
                                },)),
                            })),
                        }),
                        sort: None,
                        first: false,
                    },
                ],
            },
            suffix: Some(QuerySuffix {
                chain_tail: ChainTail {
                    bind: Some("id".to_string()),
                    subchains: vec![],
                },
                sort: None,
            }),
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
            chain_head: ChainHead {
                root: None,
                steps: vec![
                    //. .
                    Step {
                        specific: StepSpecific::Junction(StepJunction {
                            type_: JunctionType::Or,
                            subchains: vec![
                                //. .
                                ChainHead {
                                    root: Some(ChainRoot::Value(Value::Literal(s("sunwet/1/dog")))),
                                    steps: vec![Step {
                                        specific: StepSpecific::Move(StepMove {
                                            dir: MoveDirection::Backward,
                                            predicate: StrValue::Literal(PREDICATE_IS.to_string()),
                                            filter: None,
                                        }),
                                        sort: None,
                                        first: false,
                                    }],
                                },
                                ChainHead {
                                    root: Some(ChainRoot::Value(Value::Literal(s("sunwet/1/what",)))),
                                    steps: vec![Step {
                                        specific: StepSpecific::Move(StepMove {
                                            dir: MoveDirection::Backward,
                                            predicate: StrValue::Literal(PREDICATE_IS.to_string()),
                                            filter: None,
                                        }),
                                        sort: None,
                                        first: false,
                                    }],
                                },
                            ],
                        }),
                        sort: None,
                        first: false,
                    },
                ],
            },
            suffix: Some(QuerySuffix {
                chain_tail: ChainTail {
                    bind: Some("id".to_string()),
                    subchains: vec![],
                },
                sort: None,
            }),
        },
    );
}

#[test]
fn test_gc() {
    let mut db = rusqlite::Connection::open_in_memory().unwrap();
    let mut db = db::migrate(db, None).unwrap();
    let stamp1 = chrono::Local.with_ymd_and_hms(2014, 10, 1, 1, 1, 1).unwrap().into();
    let stamp1b = stamp1 + Duration::seconds(1);
    let stamp2 = chrono::Local.with_ymd_and_hms(2014, 11, 1, 1, 1, 1).unwrap().into();
    let stamp3 = chrono::Local.with_ymd_and_hms(2014, 12, 1, 1, 1, 1).unwrap().into();

    // Newest is after epoch
    dbwrite::write_triple(&mut db, &DbNode(s("a")), "b", &DbNode(s("c")), stamp1, true).unwrap();
    dbwrite::write_triple(&mut db, &DbNode(s("a")), "b", &DbNode(s("c")), stamp2, false).unwrap();
    dbwrite::write_triple(&mut db, &DbNode(s("a")), "b", &DbNode(s("c")), stamp3, true).unwrap();

    // Newest is before epoch, but exists
    dbwrite::write_triple(&mut db, &DbNode(s("d")), "e", &DbNode(s("f")), stamp1, false).unwrap();
    dbwrite::write_triple(&mut db, &DbNode(s("d")), "e", &DbNode(s("f")), stamp2, true).unwrap();

    // Newest is before epoch, but doesn't exist
    dbwrite::write_triple(&mut db, &DbNode(s("g")), "h", &DbNode(s("i")), stamp1, true).unwrap();
    dbwrite::write_triple(&mut db, &DbNode(s("g")), "h", &DbNode(s("i")), stamp1b, false).unwrap();

    // Gc
    dbutil::triple_gc_deleted(&mut db, stamp2 + Duration::seconds(1)).unwrap();
    let want = vec![
        //. .
        format!("{:?}", (s("a"), "b".to_string(), s("c"), stamp3, true)),
        format!("{:?}", (s("d"), "e".to_string(), s("f"), stamp2, true)),
    ];
    let mut have = dbutil::hist_list_all(&mut db).unwrap().into_iter().map(|r| {
        format!("{:?}", (r.subject.0, r.predicate, r.object.0, r.commit_, r.exists))
    }).collect::<Vec<_>>();
    have.sort();
    pretty_assertions::assert_eq!(want, have);
    dbutil::triple_gc_deleted(&mut db, stamp2 + Duration::seconds(1)).unwrap();
    let mut have = dbutil::hist_list_all(&mut db).unwrap().into_iter().map(|r| {
        format!("{:?}", (r.subject.0, r.predicate, r.object.0, r.commit_, r.exists))
    }).collect::<Vec<_>>();
    have.sort();
    pretty_assertions::assert_eq!(want, have);
}

/// Test that file GC correctly finds files referenced as objects (not just subjects).
/// Before the fix, file GC only checked triple_snapshot.subject, so files appearing
/// only as objects (e.g. cover images, media files) would be incorrectly deleted.
#[test]
fn test_file_gc_finds_object_files() {
    let mut db = rusqlite::Connection::open_in_memory().unwrap();
    let mut db = db::migrate(db, None).unwrap();
    let file_node = Node::File(shared::interface::triple::FileHash::Sha256("deadbeef".to_string()));
    let file_node_sql = serde_json_canonicalizer::to_string(&file_node).unwrap();
    // File appears only as object (typical: album -> cover -> file_hash)
    dbwrite::write_triple(
        &mut db,
        &DbNode(s("album1")),
        "sunwet/1/cover",
        &DbNode(file_node.clone()),
        Utc::now().into(),
        true,
    )
    .unwrap();
    // Verify the file is found via object check on triple_snapshot (through subjobj join)
    let found_as_object: bool = db
        .0
        .query_row(
            r#"SELECT 1 FROM "triple_snapshot" ts
               JOIN "subjobj" so ON ts."object" = so."id"
               WHERE so."value" = ?1"#,
            [&file_node_sql],
            |_| Ok(true),
        )
        .optional()
        .unwrap()
        .unwrap_or(false);
    assert!(
        found_as_object,
        "File referenced as object must be findable in triple_snapshot. \
         GC would incorrectly delete this file if it only checked subjects."
    );
    // Also verify it is NOT found as subject (to prove the bug scenario)
    let found_as_subject: bool = db
        .0
        .query_row(
            r#"SELECT 1 FROM "triple_snapshot" ts
               JOIN "subjobj" so ON ts."subject" = so."id"
               WHERE so."value" = ?1"#,
            [&file_node_sql],
            |_| Ok(true),
        )
        .optional()
        .unwrap()
        .unwrap_or(false);
    assert!(
        !found_as_subject,
        "File should NOT appear as subject - it's only referenced as object"
    );
}

/// Helper to set up a V1-schema database with test data for migration testing.
fn setup_v1_db_with_data(conn: &rusqlite::Connection, triples: &[(&str, &str, &str, &str, bool)]) {
    // The V1 schema is the same as V0: just the triple table + supporting tables.
    // good_ormning's __good_version table tells it we're at version 1.
    conn.execute_batch(
        r#"
        CREATE TABLE __good_version (rid int primary key, version bigint not null, lock int not null);
        INSERT INTO __good_version VALUES (0, 1, 0);

        CREATE TABLE IF NOT EXISTS "triple" (
            "predicate" text not null,
            "subject" text not null,
            "commit_" text not null,
            "object" text not null,
            "exists" integer not null,
            constraint "triple_pk" primary key ("subject", "predicate", "object", "commit_")
        );
        CREATE UNIQUE INDEX "triple_index_obj_pred_subj" on "triple" ("object", "predicate", "subject", "commit_");
        CREATE INDEX "triple_index_pred_subj" on "triple" ("predicate", "subject", "commit_");
        CREATE INDEX "triple_index_pred_obj" on "triple" ("predicate", "object", "commit_");
        CREATE INDEX "triple_commit_exists" on "triple" ("commit_", "exists");

        CREATE TABLE IF NOT EXISTS "commit" (
            "idtimestamp" text not null,
            "description" text not null,
            constraint "commit_timestamp" primary key ("idtimestamp")
        );

        CREATE TABLE IF NOT EXISTS "meta" (
            "mimetype" text,
            "fulltext" text not null,
            "node" text not null,
            constraint "meta_node" primary key ("node")
        );

        CREATE TABLE IF NOT EXISTS "generated" (
            "node" text not null,
            "mimetype" text not null,
            "gentype" text not null,
            constraint "generated_pk" primary key ("node", "gentype")
        );

        CREATE TABLE IF NOT EXISTS "file_access" (
            "spec_hash" integer not null,
            "access_source" text not null,
            "file" text not null,
            constraint "file_access_pk" primary key ("file", "access_source", "spec_hash")
        );
        "#,
    )
    .unwrap();
    let mut stmt = conn
        .prepare(
            r#"INSERT INTO "triple" ("subject", "predicate", "object", "commit_", "exists")
               VALUES (?1, ?2, ?3, ?4, ?5)"#,
        )
        .unwrap();
    for (subj, pred, obj, commit, exists) in triples {
        stmt.execute(rusqlite::params![subj, pred, obj, commit, *exists as i64])
            .unwrap();
    }
}

/// Test migration from V1 schema with fake data. This is the permanent in-memory test.
/// Verifies: triple count preserved, snapshot correct, queries work, GC doesn't delete live data.
#[test]
fn test_migration_v1_to_latest() {
    let file_hash_json = r#"{"t":"f","v":{"sha256":"abc123"}}"#;
    let album_json = r#"{"t":"v","v":"album-uuid-1"}"#;
    let artist_json = r#"{"t":"v","v":"artist-uuid-1"}"#;
    let album_name_json = r#"{"t":"v","v":"My Album"}"#;
    let artist_name_json = r#"{"t":"v","v":"Some Artist"}"#;
    let is_album_json = r#"{"t":"v","v":"sunwet/1/album"}"#;
    // commit_ stored as RFC3339 datetime string in the DB
    let stamp1 = "2024-01-01T00:00:00+00:00";
    let stamp2 = "2024-01-01T00:00:01+00:00";
    let stamp3 = "2024-01-01T00:00:02+00:00";
    let triples: Vec<(&str, &str, &str, &str, bool)> = vec![
        // Album exists
        (album_json, "sunwet/1/is", is_album_json, stamp1, true),
        // Album has a name
        (album_json, "sunwet/1/name", album_name_json, stamp1, true),
        // Album has a cover file (file as OBJECT - this is the critical case for GC)
        (album_json, "sunwet/1/cover", file_hash_json, stamp1, true),
        // Album has artist
        (album_json, "sunwet/1/artist", artist_json, stamp1, true),
        // Artist name
        (artist_json, "sunwet/1/name", artist_name_json, stamp1, true),
        // A triple that was created then deleted (should NOT appear in snapshot)
        (album_json, "sunwet/1/name", r#"{"t":"v","v":"Old Name"}"#, stamp2, true),
        (album_json, "sunwet/1/name", r#"{"t":"v","v":"Old Name"}"#, stamp3, false),
    ];
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    setup_v1_db_with_data(&conn, &triples);

    // Run migration
    let mut db = db::migrate(conn, Some(&|v| migrate::migrate(v))).unwrap();

    // 1. Triple count: all 7 rows should be in the history table
    let hist = dbutil::hist_list_all(&mut db).unwrap();
    assert_eq!(hist.len(), triples.len(), "All triple rows must be preserved in history");

    // 2. All distinct subjects, predicates, objects preserved
    // After normalization, triple stores integer IDs - count distinct values via subjobj/predicate tables
    let orig_subjects: HashSet<&str> = triples.iter().map(|t| t.0).collect();
    let orig_predicates: HashSet<&str> = triples.iter().map(|t| t.1).collect();
    let orig_objects: HashSet<&str> = triples.iter().map(|t| t.2).collect();
    let distinct_subjects: i64 = db
        .0
        .query_row(
            r#"SELECT count(DISTINCT s."value") FROM "triple" t JOIN "subjobj" s ON t."subject" = s."id""#,
            [],
            |r| r.get(0),
        )
        .unwrap();
    let distinct_predicates: i64 = db
        .0
        .query_row(
            r#"SELECT count(DISTINCT p."value") FROM "triple" t JOIN "predicate" p ON t."predicate" = p."id""#,
            [],
            |r| r.get(0),
        )
        .unwrap();
    let distinct_objects: i64 = db
        .0
        .query_row(
            r#"SELECT count(DISTINCT o."value") FROM "triple" t JOIN "subjobj" o ON t."object" = o."id""#,
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        orig_subjects.len() as i64,
        distinct_subjects,
        "Distinct subject count must match"
    );
    assert_eq!(
        orig_predicates.len() as i64,
        distinct_predicates,
        "Distinct predicate count must match"
    );
    assert_eq!(
        orig_objects.len() as i64,
        distinct_objects,
        "Distinct object count must match"
    );

    // 3. Snapshot: deleted triple should NOT be in snapshot
    let album_node = DbNode(Node::Value(serde_json::Value::String("album-uuid-1".to_string())));
    assert!(
        !dbutil::triple_snapshot_exists(
            &mut db,
            &album_node,
            "sunwet/1/name",
            &DbNode(Node::Value(serde_json::Value::String("Old Name".to_string()))),
        )
        .unwrap(),
        "Deleted triple should not be in snapshot"
    );
    // But live triples should be in snapshot
    let file_node = DbNode(Node::File(shared::interface::triple::FileHash::Sha256("abc123".to_string())));
    assert!(
        dbutil::triple_snapshot_exists(&mut db, &album_node, "sunwet/1/cover", &file_node).unwrap(),
        "Live triple (album -> cover -> file) must be in snapshot"
    );

    // 4. File referenced as object must be discoverable in snapshot (GC correctness)
    let file_node_sql = serde_json_canonicalizer::to_string(&file_node.0).unwrap();
    let found_as_object: bool = db
        .0
        .query_row(
            r#"SELECT 1 FROM "triple_snapshot" ts
               JOIN "subjobj" so ON ts."object" = so."id"
               WHERE so."value" = ?1"#,
            [&file_node_sql],
            |_| Ok(true),
        )
        .optional()
        .unwrap()
        .unwrap_or(false);
    assert!(
        found_as_object,
        "File node referenced as object must be findable in triple_snapshot - \
         GC would incorrectly delete this file if it only checked subjects"
    );

    // 5. Dynamic query works after migration
    let query = compile_query("\"album-uuid-1\" -> \"sunwet/1/name\" { => name }").unwrap();
    let (query_sql, query_values) = build_root_chain(&query, HashMap::new())
        .map_err(|e| match e {
            VisErr::Internal(e) => panic!("{}", e),
            VisErr::External(e) => panic!("{}", e),
        })
        .unwrap();
    let got = execute_sql_query(
        &mut db::Db(&mut db.0.transaction().unwrap()),
        query_sql,
        query_values,
        &query,
        None,
    )
    .unwrap();
    assert_eq!(got.len(), 1, "Query should find album name");

    // 6. GC should not delete live data
    let epoch = Utc::now();
    dbutil::triple_gc_deleted(&mut db, epoch).unwrap();
    // The live triples (5 of them) should still exist
    let hist_after_gc = dbutil::hist_list_all(&mut db).unwrap();
    assert!(
        hist_after_gc.len() >= 5,
        "GC should preserve live triples, got {} rows",
        hist_after_gc.len()
    );
    // Cover file triple must survive
    assert!(
        dbutil::triple_snapshot_exists(&mut db, &album_node, "sunwet/1/cover", &file_node).unwrap(),
        "Cover file triple must survive GC"
    );
}

/// Test migration from example.sqlite3.bak (real data).
/// This test copies the backup, runs migration, and verifies data integrity.
#[test]
fn test_migration_example_db() {
    let bak_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../example.sqlite3.bak");
    if !bak_path.exists() {
        eprintln!("Skipping test_migration_example_db: example.sqlite3.bak not found");
        return;
    }
    // Copy to temp file
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::copy(&bak_path, tmp.path()).unwrap();
    let conn = rusqlite::Connection::open(tmp.path()).unwrap();

    // Count data before migration
    let triple_count_before: i64 = conn
        .query_row("SELECT count(*) FROM triple", [], |r| r.get(0))
        .unwrap();
    let distinct_subjects_before: i64 = conn
        .query_row("SELECT count(distinct subject) FROM triple", [], |r| r.get(0))
        .unwrap();
    let distinct_predicates_before: i64 = conn
        .query_row("SELECT count(distinct predicate) FROM triple", [], |r| {
            r.get(0)
        })
        .unwrap();
    let distinct_objects_before: i64 = conn
        .query_row("SELECT count(distinct object) FROM triple", [], |r| r.get(0))
        .unwrap();
    assert!(triple_count_before > 0, "Backup DB should have data");

    // Count file nodes that appear as objects (these are the ones GC must not delete)
    let file_objects_before: i64 = conn
        .query_row(
            r#"SELECT count(distinct object) FROM triple
               WHERE object LIKE '%"t":"f"%'
               AND "exists" = 1
               AND commit_ = (
                   SELECT max(t2.commit_) FROM triple t2
                   WHERE t2.subject = triple.subject
                   AND t2.predicate = triple.predicate
                   AND t2.object = triple.object
               )"#,
            [],
            |r| r.get(0),
        )
        .unwrap();

    // Run migration
    let mut db = db::migrate(conn, Some(&|v| migrate::migrate(v))).unwrap();

    // 1. Triple count preserved
    let hist = dbutil::hist_list_all(&mut db).unwrap();
    // hist_list_all is paginated (limit 100), so use raw SQL for total count
    let triple_count_after: i64 = db
        .0
        .query_row("SELECT count(*) FROM triple", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        triple_count_before, triple_count_after,
        "Triple count must be preserved after migration"
    );

    // 2. Distinct subjects/predicates/objects preserved (via subjobj/predicate joins)
    let distinct_subjects_after: i64 = db
        .0
        .query_row(
            r#"SELECT count(DISTINCT s."value") FROM "triple" t JOIN "subjobj" s ON t."subject" = s."id""#,
            [],
            |r| r.get(0),
        )
        .unwrap();
    let distinct_predicates_after: i64 = db
        .0
        .query_row(
            r#"SELECT count(DISTINCT p."value") FROM "triple" t JOIN "predicate" p ON t."predicate" = p."id""#,
            [],
            |r| r.get(0),
        )
        .unwrap();
    let distinct_objects_after: i64 = db
        .0
        .query_row(
            r#"SELECT count(DISTINCT o."value") FROM "triple" t JOIN "subjobj" o ON t."object" = o."id""#,
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(distinct_subjects_before, distinct_subjects_after);
    assert_eq!(distinct_predicates_before, distinct_predicates_after);
    assert_eq!(distinct_objects_before, distinct_objects_after);

    // 3. Snapshot has entries
    let snapshot_count: i64 = db
        .0
        .query_row("SELECT count(*) FROM triple_snapshot", [], |r| r.get(0))
        .unwrap();
    assert!(
        snapshot_count > 0,
        "Snapshot should have entries after migration"
    );

    // 4. File objects in snapshot - critical GC correctness test.
    // Files appear as objects (e.g., album -> cover -> file_hash).
    // The GC must find them in triple_snapshot via object column (through subjobj join).
    let file_objects_in_snapshot: i64 = db
        .0
        .query_row(
            r#"SELECT count(DISTINCT so."value") FROM "triple_snapshot" ts
               JOIN "subjobj" so ON ts."object" = so."id"
               WHERE so."value" LIKE '%"t":"f"%'"#,
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        file_objects_before, file_objects_in_snapshot,
        "All live file objects must appear in triple_snapshot. \
         If this fails, GC would delete these files because they are only referenced as objects, \
         not subjects. The file GC query must check both subject AND object columns."
    );

    // 5. subjobj table populated
    let subjobj_count: i64 = db
        .0
        .query_row("SELECT count(*) FROM subjobj", [], |r| r.get(0))
        .unwrap();
    assert!(subjobj_count > 0, "subjobj table should be populated");

    // 6. predicate table populated
    let predicate_count: i64 = db
        .0
        .query_row("SELECT count(*) FROM predicate", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        distinct_predicates_before, predicate_count,
        "All predicates should be in predicate table"
    );
}
