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
        },
        triple::Node,
    },
    sea_query::{
        Alias,
        ColumnRef,
        OverStatement,
        SeaRc,
        SqliteQueryBuilder,
        TableRef,
        WindowStatement,
    },
    sea_query_rusqlite::RusqliteBinder,
    std::{
        io::Write,
        path::PathBuf,
        process::{
            Command,
            Stdio,
        },
    },
};

pub mod db;
pub mod interface;

struct QueryBuildState {
    // # Immutable
    ident_table_primary: sea_query::DynIden,
    ident_table_prev: sea_query::DynIden,
    ident_col_start: sea_query::DynIden,
    ident_col_end: sea_query::DynIden,
    ident_col_subject: sea_query::DynIden,
    ident_col_predicate: sea_query::DynIden,
    ident_col_object: sea_query::DynIden,
    ident_col_timestamp: sea_query::DynIden,
    ident_col_exists: sea_query::DynIden,
    triple_table: TableRef,
    // # Mutable
    global_unique: usize,
    ctes: Vec<sea_query::CommonTableExpression>,
}

#[derive(Clone)]
struct BuildChainRes {
    cte_name: sea_query::DynIden,
    cte: sea_query::TableRef,
    plural: bool,
    selects: Vec<(String, bool)>,
}

#[derive(Clone)]
struct BuildStepRes {
    ident_table: sea_query::DynIden,
    col_start: sea_query::DynIden,
    col_end: sea_query::DynIden,
    plural: bool,
}

fn build_step<'x>(query_state: &mut QueryBuildState, previous: Option<BuildStepRes>, step: &Step) -> BuildStepRes {
    match step {
        Step::Move(step) => {
            let seg_name = format!("seg{}_move", query_state.global_unique);
            query_state.global_unique += 1;
            let mut out;
            {
                let ident_cte = SeaRc::new(Alias::new(seg_name.clone()));
                let mut sql_sel = sea_query::Query::select();
                let local_ident_table_primary = query_state.ident_table_primary.clone();
                sql_sel.from_as(query_state.triple_table.clone(), local_ident_table_primary.clone());

                // Direction selection
                let from_ident_primary_start;
                let from_ident_primary_end;
                match step.dir {
                    MoveDirection::Down => {
                        from_ident_primary_start = &query_state.ident_col_subject;
                        from_ident_primary_end = &query_state.ident_col_object;
                    },
                    MoveDirection::Up => {
                        from_ident_primary_start = &query_state.ident_col_object;
                        from_ident_primary_end = &query_state.ident_col_subject;
                    },
                }
                let local_col_primary_start =
                    ColumnRef::TableColumn(local_ident_table_primary.clone(), from_ident_primary_start.clone());
                let local_col_primary_end =
                    ColumnRef::TableColumn(local_ident_table_primary.clone(), from_ident_primary_end.clone());

                // Only get latest event
                sql_sel.group_by_col(local_col_primary_start.clone());
                sql_sel.group_by_col(
                    ColumnRef::TableColumn(local_ident_table_primary.clone(), query_state.ident_col_predicate.clone()),
                );
                sql_sel.group_by_col(local_col_primary_end.clone());
                sql_sel.order_by(
                    ColumnRef::TableColumn(local_ident_table_primary.clone(), query_state.ident_col_timestamp.clone()),
                    sea_query::Order::Desc,
                );

                // Only consider elements with perm to view
                { }

                // Movement
                sql_sel.and_where(
                    sea_query::Expr::col(
                        ColumnRef::TableColumn(
                            local_ident_table_primary.clone(),
                            query_state.ident_col_predicate.clone(),
                        ),
                    ).eq(step.predicate.clone()),
                );

                // Subset of previous results
                let out_col_start;
                if let Some(previous) = previous {
                    let local_ident_table_prev = query_state.ident_table_prev.clone();
                    sql_sel.join_as(
                        sea_query::JoinType::InnerJoin,
                        previous.ident_table,
                        local_ident_table_prev.clone(),
                        sea_query::Expr::col(
                            ColumnRef::TableColumn(local_ident_table_prev.clone(), previous.col_end.clone()),
                        ).eq(local_col_primary_start.clone()),
                    );
                    out_col_start = ColumnRef::TableColumn(local_ident_table_prev.clone(), previous.col_start);
                } else {
                    out_col_start = local_col_primary_start.clone();
                }

                // Trim
                if step.first {
                    sql_sel.limit(1);
                }

                // Assemble
                sql_sel.column(out_col_start);
                sql_sel.column(local_col_primary_end.clone());
                sql_sel.column(
                    ColumnRef::TableColumn(local_ident_table_primary.clone(), query_state.ident_col_exists.clone()),
                );
                let mut sql_cte = sea_query::CommonTableExpression::new();
                sql_cte.table_name(ident_cte.clone());
                sql_cte.column(query_state.ident_col_start.clone());
                sql_cte.column(query_state.ident_col_end.clone());
                sql_cte.column(query_state.ident_col_exists.clone());
                sql_cte.query(sql_sel);
                query_state.ctes.push(sql_cte);
                out = BuildStepRes {
                    ident_table: ident_cte.clone(),
                    col_start: query_state.ident_col_start.clone(),
                    col_end: query_state.ident_col_end.clone(),
                    plural: !step.first,
                };
            }

            // Filter out deletions
            {
                let ident_cte = SeaRc::new(Alias::new(format!("{}b", seg_name)));
                let mut sql_sel = sea_query::Query::select();
                let local_ident_table_primary = query_state.ident_table_primary.clone();
                sql_sel.from_as(out.ident_table.clone(), local_ident_table_primary.clone());
                sql_sel.column(ColumnRef::TableColumn(local_ident_table_primary.clone(), out.col_start));
                sql_sel.column(ColumnRef::TableColumn(local_ident_table_primary.clone(), out.col_end));
                sql_sel.and_where(
                    sea_query::Expr::col(
                        ColumnRef::TableColumn(
                            local_ident_table_primary.clone(),
                            query_state.ident_col_exists.clone(),
                        ),
                    ).eq(true),
                );

                // Assemble
                let mut sql_cte = sea_query::CommonTableExpression::new();
                sql_cte.table_name(ident_cte.clone());
                sql_cte.column(query_state.ident_col_start.clone());
                sql_cte.column(query_state.ident_col_end.clone());
                sql_cte.query(sql_sel);
                query_state.ctes.push(sql_cte);
                out = BuildStepRes {
                    ident_table: ident_cte.clone(),
                    col_start: query_state.ident_col_start.clone(),
                    col_end: query_state.ident_col_end.clone(),
                    plural: out.plural,
                };
            }
            return out;
        },
        Step::Recurse(step) => {
            let seg_name = format!("seg{}_recurse", query_state.global_unique);
            query_state.global_unique += 1;
            let mut out;
            {
                let previous = previous.unwrap();
                let global_ident_table_cte = SeaRc::new(Alias::new(seg_name.clone()));
                let table_cte = TableRef::Table(global_ident_table_cte.clone());

                // Base case
                let mut sql_sel = sea_query::Query::select();
                {
                    let local_ident_table_prev = query_state.ident_table_prev.clone();
                    sql_sel.from_as(previous.ident_table, local_ident_table_prev.clone());
                    sql_sel.column(ColumnRef::TableColumn(local_ident_table_prev.clone(), previous.col_start));
                    sql_sel.column(ColumnRef::TableColumn(local_ident_table_prev.clone(), previous.col_end));
                }

                // Recursive case
                sql_sel.union(sea_query::UnionType::Distinct, {
                    let mut sql_sel = sea_query::Query::select();
                    sql_sel.from(table_cte);
                    sql_sel.column(
                        ColumnRef::TableColumn(global_ident_table_cte.clone(), query_state.ident_col_start.clone()),
                    );
                    let subchain = build_subchain(query_state, None, &step.chain);
                    let local_ident_table_primary = query_state.ident_table_primary.clone();
                    sql_sel.join_as(
                        sea_query::JoinType::InnerJoin,
                        TableRef::Table(subchain.ident_table.clone()),
                        local_ident_table_primary.clone(),
                        sea_query::Expr::col(
                            ColumnRef::TableColumn(local_ident_table_primary.clone(), subchain.col_start),
                        ).eq(
                            ColumnRef::TableColumn(global_ident_table_cte.clone(), query_state.ident_col_end.clone()),
                        ),
                    );
                    sql_sel.column(ColumnRef::TableColumn(local_ident_table_primary, subchain.col_end));
                    sql_sel
                });

                // Assemble
                let mut sql_cte = sea_query::CommonTableExpression::new();
                sql_cte.table_name(global_ident_table_cte.clone());
                let ident_col_start = query_state.ident_col_start.clone();
                let ident_col_end = query_state.ident_col_end.clone();
                sql_cte.column(ident_col_start.clone());
                sql_cte.column(ident_col_end.clone());
                sql_cte.query(sql_sel);
                query_state.ctes.push(sql_cte);
                out = BuildStepRes {
                    ident_table: global_ident_table_cte.clone(),
                    col_start: ident_col_start,
                    col_end: ident_col_end,
                    plural: true,
                };
            }
            if step.first {
                let global_ident_table_cte = SeaRc::new(Alias::new(format!("{}b", seg_name)));
                let ident_col_start = query_state.ident_col_start.clone();
                let ident_col_end = query_state.ident_col_end.clone();
                let mut sql_sel = sea_query::Query::select();
                sql_sel.from(TableRef::Table(out.ident_table.clone()));
                sql_sel.column(ColumnRef::TableColumn(out.ident_table.clone(), out.col_start));
                sql_sel.column(ColumnRef::TableColumn(out.ident_table.clone(), out.col_end));
                sql_sel.limit(1);

                // Assemble
                let mut sql_cte = sea_query::CommonTableExpression::new();
                sql_cte.table_name(global_ident_table_cte.clone());
                sql_cte.column(ident_col_start.clone());
                sql_cte.column(ident_col_end.clone());
                sql_cte.query(sql_sel);
                query_state.ctes.push(sql_cte);
                out = BuildStepRes {
                    ident_table: global_ident_table_cte.clone(),
                    col_start: ident_col_start,
                    col_end: ident_col_end,
                    plural: false,
                };
            }
            return out;
        },
    }
}

// Produces (sequence of) CTEs from steps, returning the last CTE. CTE has start
// and end fields only.
fn build_subchain(
    query_state: &mut QueryBuildState,
    prev_subchain_seg: Option<BuildStepRes>,
    steps: &[Step],
) -> BuildStepRes {
    let mut prev_subchain_seg = build_step(query_state, prev_subchain_seg, &steps[0]);
    for step in &steps[1..] {
        prev_subchain_seg = build_step(query_state, Some(prev_subchain_seg), step);
    }
    return prev_subchain_seg;
}

/// Produces CTE with `_` selects, no aggregation.
fn build_chain(
    query_state: &mut QueryBuildState,
    prev_subchain_seg: Option<BuildStepRes>,
    chain: Chain,
) -> BuildChainRes {
    let cte_name = format!("chain{}", query_state.global_unique);
    query_state.global_unique += 1;
    let mut sql_sel = sea_query::Query::select();
    let primary_subchain = build_subchain(query_state, prev_subchain_seg, &chain.steps);
    sql_sel.from(TableRef::Table(primary_subchain.ident_table.clone()));
    let global_col_primary_start =
        ColumnRef::TableColumn(primary_subchain.ident_table.clone(), primary_subchain.col_start.clone());
    let global_col_primary_end =
        ColumnRef::TableColumn(primary_subchain.ident_table.clone(), primary_subchain.col_end.clone());
    sql_sel.expr_as(global_col_primary_start.clone(), query_state.ident_col_start.clone());
    sql_sel.group_by_col(global_col_primary_start.clone());
    sql_sel.expr_as(global_col_primary_end.clone(), query_state.ident_col_end.clone());
    sql_sel.group_by_col(global_col_primary_end.clone());

    // Add dest as selection
    let mut selects = vec![];
    if let Some(name) = chain.select {
        sql_sel.expr_as(global_col_primary_end.clone(), SeaRc::new(Alias::new(format!("_{}", name))));
        selects.push((name, false));
    }

    // Process children
    let child_prev;
    {
        let global_ident_table_cte = SeaRc::new(Alias::new(format!("{}_childsrc", cte_name)));
        query_state.global_unique += 1;
        let ident_col_start = query_state.ident_col_start.clone();
        let ident_col_end = query_state.ident_col_end.clone();
        let mut sql_sel = sea_query::Query::select();
        sql_sel.from(TableRef::Table(primary_subchain.ident_table.clone()));
        sql_sel.column(
            ColumnRef::TableColumn(primary_subchain.ident_table.clone(), primary_subchain.col_end.clone()),
        );
        sql_sel.column(ColumnRef::TableColumn(primary_subchain.ident_table.clone(), primary_subchain.col_end));

        // Assemble
        let mut sql_cte = sea_query::CommonTableExpression::new();
        sql_cte.table_name(global_ident_table_cte.clone());
        sql_cte.column(ident_col_start.clone());
        sql_cte.column(ident_col_end.clone());
        sql_cte.query(sql_sel);
        query_state.ctes.push(sql_cte);
        child_prev = Some(BuildStepRes {
            ident_table: global_ident_table_cte.clone(),
            col_start: ident_col_start,
            col_end: ident_col_end,
            plural: false,
        });
    }
    for child in chain.children {
        let child_chain = build_chain(query_state, child_prev.clone(), child);
        sql_sel.join(
            sea_query::JoinType::LeftJoin,
            child_chain.cte,
            sea_query::Expr::col(
                global_col_primary_end.clone(),
            ).eq(ColumnRef::TableColumn(child_chain.cte_name.clone(), query_state.ident_col_start.clone())),
        );
        for (name, plural) in child_chain.selects {
            let ident_name = SeaRc::new(Alias::new(format!("_{}", name)));
            sql_sel.expr_as(ColumnRef::TableColumn(child_chain.cte_name.clone(), ident_name.clone()), ident_name);
            selects.push((name, child_chain.plural || plural));
        }
    }

    // Assemble
    let mut sql_cte = sea_query::CommonTableExpression::new();
    let ident_table_cte = SeaRc::new(Alias::new(cte_name));
    sql_cte.table_name(ident_table_cte.clone());
    sql_cte.query(sql_sel);
    query_state.ctes.push(sql_cte);
    return BuildChainRes {
        cte_name: ident_table_cte.clone(),
        cte: TableRef::Table(ident_table_cte),
        selects: selects,
        plural: primary_subchain.plural,
    };
}

fn build_node(v: Node) -> sea_query::Value {
    return sea_query::Value::Json(Some(Box::new(serde_json::to_value(&v).unwrap())));
}

fn build_query(q: Query) -> (String, sea_query_rusqlite::RusqliteValues) {
    let mut query_state = QueryBuildState {
        ident_table_primary: SeaRc::new(Alias::new("primary")),
        ident_table_prev: SeaRc::new(Alias::new("prev")),
        ident_col_start: SeaRc::new(Alias::new("start")),
        ident_col_end: SeaRc::new(Alias::new("end")),
        ident_col_subject: SeaRc::new(Alias::new("subject")),
        ident_col_predicate: SeaRc::new(Alias::new("predicate")),
        ident_col_object: SeaRc::new(Alias::new("object")),
        ident_col_timestamp: SeaRc::new(Alias::new("timestamp")),
        ident_col_exists: SeaRc::new(Alias::new("exists")),
        triple_table: TableRef::Table(SeaRc::new(Alias::new("triple"))),
        global_unique: Default::default(),
        ctes: Default::default(),
    };
    let root = if let Some(root) = q.root {
        let ident_table_root = SeaRc::new(Alias::new("root"));
        let mut sql_sel = sea_query::Query::select();
        let root_expr = build_node(root);
        sql_sel.expr(root_expr.clone());
        sql_sel.expr(root_expr.clone());
        let mut sql_cte = sea_query::CommonTableExpression::new();
        sql_cte.table_name(ident_table_root.clone());
        sql_cte.query(sql_sel);
        sql_cte.column(query_state.ident_col_start.clone());
        sql_cte.column(query_state.ident_col_end.clone());
        query_state.ctes.push(sql_cte);
        Some(BuildStepRes {
            ident_table: ident_table_root,
            col_start: query_state.ident_col_start.clone(),
            col_end: query_state.ident_col_end.clone(),
            plural: false,
        })
    } else {
        None
    };
    let cte = build_chain(&mut query_state, root, q.chain);
    let mut sel_root = sea_query::Query::select();
    sel_root.from(cte.cte);
    for (name, plural) in cte.selects {
        let expr =
            sea_query::ColumnRef::TableColumn(cte.cte_name.clone(), SeaRc::new(Alias::new(format!("_{}", name))));
        let ident_name = SeaRc::new(Alias::new(name));
        if plural {
            sel_root.expr_as(
                sea_query::SimpleExpr::FunctionCall(
                    sea_query::Func::cust(SeaRc::new(Alias::new("json_group_array"))).arg(expr),
                ),
                ident_name,
            );
        } else {
            sel_root.expr_as(expr, ident_name);
        }
    }
    let mut sel = sea_query::WithQuery::new();
    sel.recursive(true);
    sel.query(sel_root);
    for cte in query_state.ctes {
        sel.cte(cte);
    }
    return sel.build_rusqlite(SqliteQueryBuilder);
}

fn main() {
    let (query, query_values) = build_query(Query {
        root: Some(Node::Id("sunwet/1/album".to_string())),
        chain: Chain {
            steps: vec![Step::Move(StepMove {
                dir: MoveDirection::Up,
                predicate: "sunwet/1/is".to_string(),
                first: false,
            })],
            select: Some("id".to_string()),
            children: vec![
                //. .
                Chain {
                    steps: vec![
                        //. .
                        Step::Recurse(StepRecurse {
                            chain: vec![Step::Move(StepMove {
                                dir: MoveDirection::Up,
                                predicate: "sunwet/1/element".to_string(),
                                first: false,
                            })],
                            first: false,
                        }),
                        Step::Move(StepMove {
                            dir: MoveDirection::Down,
                            predicate: "sunwet/1/name".to_string(),
                            first: true,
                        })
                    ],
                    select: Some("name".to_string()),
                    children: Default::default(),
                },
                Chain {
                    steps: vec![
                        //. .
                        Step::Recurse(StepRecurse {
                            chain: vec![Step::Move(StepMove {
                                dir: MoveDirection::Up,
                                predicate: "sunwet/1/element".to_string(),
                                first: false,
                            })],
                            first: false,
                        }),
                        Step::Move(StepMove {
                            dir: MoveDirection::Down,
                            predicate: "sunwet/1/artist".to_string(),
                            first: true,
                        }),
                        Step::Recurse(StepRecurse {
                            chain: vec![Step::Move(StepMove {
                                dir: MoveDirection::Up,
                                predicate: "sunwet/1/element".to_string(),
                                first: false,
                            })],
                            first: false,
                        }),
                        Step::Move(StepMove {
                            dir: MoveDirection::Down,
                            predicate: "sunwet/1/name".to_string(),
                            first: true,
                        })
                    ],
                    select: Some("artist".to_string()),
                    children: Default::default(),
                },
                Chain {
                    steps: vec![
                        //. .
                        Step::Recurse(StepRecurse {
                            chain: vec![Step::Move(StepMove {
                                dir: MoveDirection::Up,
                                predicate: "sunwet/1/element".to_string(),
                                first: false,
                            })],
                            first: false,
                        }),
                        Step::Move(StepMove {
                            dir: MoveDirection::Down,
                            predicate: "sunwet/1/cover".to_string(),
                            first: true,
                        })
                    ],
                    select: Some("cover".to_string()),
                    children: Default::default(),
                }
            ],
        },
    });

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
