use {
    chrono::Utc,
    good_ormning_runtime::sqlite::GoodOrmningCustomString,
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
        SeaRc,
        SqliteQueryBuilder,
        TableRef,
        UnionType,
    },
    sea_query_rusqlite::RusqliteBinder,
    serde::{
        de,
        Deserialize,
        Serialize,
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

pub mod db;
pub mod interface;

struct SubchainBuildState<'a> {
    query_state: &'a mut QueryBuildState,
    unique: usize,
}

struct QueryBuildState {
    // # Immutable
    ident_col_start: sea_query::DynIden,
    ident_col_end: sea_query::DynIden,
    ident_col_subject: sea_query::DynIden,
    ident_col_predicate: sea_query::DynIden,
    ident_col_object: sea_query::DynIden,
    triple_table: TableRef,
    // # Mutable
    global_unique: usize,
    ctes: Vec<sea_query::CommonTableExpression>,
    cte_lookup: HashMap<Step, BuiltCte>,
}

#[derive(Clone)]
struct BuiltCte {
    cte: sea_query::TableRef,
}

#[derive(Clone)]
struct BuiltSubchain {
    cte_name: sea_query::DynIden,
    cte: sea_query::TableRef,
    col_start: sea_query::ColumnRef,
    col_end: sea_query::ColumnRef,
    plural: bool,
}

#[derive(Clone)]
struct BuiltChain {
    cte_name: sea_query::DynIden,
    cte: sea_query::TableRef,
    selects: Vec<(String, bool)>,
}

#[derive(Clone)]
struct BuildStepRes {
    table: sea_query::TableRef,
    table_as: sea_query::DynIden,
    clause: Option<sea_query::SimpleExpr>,
    col_start: sea_query::ColumnRef,
    col_end: sea_query::ColumnRef,
}

impl BuildStepRes {
    fn select(
        self,
        query_state: &QueryBuildState,
    ) -> (sea_query::SelectStatement, sea_query::ColumnRef, sea_query::ColumnRef) {
        let mut sel = sea_query::Query::select();
        sel.from_as(self.table, self.table_as);
        if let Some(clause) = self.clause {
            sel.and_where(clause);
        }
        sel.expr_as(self.col_start.clone(), query_state.ident_col_start.clone());
        return (sel, self.col_start, self.col_end);
    }

    fn join(self, sel: &mut sea_query::SelectStatement, join_col: &ColumnRef) -> sea_query::ColumnRef {
        sel.join_as(
            sea_query::JoinType::InnerJoin,
            self.table,
            self.table_as,
            sea_query::Expr::col(self.col_start).eq(join_col.clone()),
        );
        if let Some(clause) = self.clause {
            sel.and_where(clause);
        }
        return self.col_end;
    }
}

fn build_step<'x>(chain_state: &'x mut SubchainBuildState, step: &Step) -> BuildStepRes {
    let table_as = SeaRc::new(Alias::new(format!("t{}", chain_state.unique)));
    chain_state.unique += 1;
    match step {
        Step::Move(n) => {
            let start;
            let end;
            match n.dir {
                MoveDirection::Down => {
                    start = &chain_state.query_state.ident_col_subject;
                    end = &chain_state.query_state.ident_col_object;
                },
                MoveDirection::Up => {
                    start = &chain_state.query_state.ident_col_object;
                    end = &chain_state.query_state.ident_col_subject;
                },
            }
            let col_start = ColumnRef::TableColumn(table_as.clone(), start.clone());
            let col_end = ColumnRef::TableColumn(table_as.clone(), end.clone());
            return BuildStepRes {
                table: chain_state.query_state.triple_table.clone(),
                table_as: table_as.clone(),
                clause: Some(
                    sea_query::Expr::col(
                        ColumnRef::TableColumn(table_as, chain_state.query_state.ident_col_predicate.clone()),
                    ).eq(n.predicate.clone()),
                ),
                col_start: col_start.clone(),
                col_end: col_end.clone(),
            };
        },
        Step::Recurse0(n_recurse) => {
            let cte = if let Some(cte) = chain_state.query_state.cte_lookup.get(step) {
                cte.clone()
            } else {
                let cte_name = SeaRc::new(Alias::new(format!("step_r0_{}", chain_state.query_state.global_unique)));
                chain_state.query_state.global_unique += 1;
                let mut cte = sea_query::CommonTableExpression::new();
                let cte_table = TableRef::Table(cte_name.clone());
                let cte_col_start =
                    ColumnRef::TableColumn(cte_name.clone(), chain_state.query_state.ident_col_start.clone());
                let cte_col_end =
                    ColumnRef::TableColumn(cte_name.clone(), chain_state.query_state.ident_col_end.clone());
                let built = BuiltCte { cte: cte_table.clone() };
                chain_state.query_state.cte_lookup.insert(step.clone(), built.clone());
                cte.table_name(cte_name.clone());
                cte.column(chain_state.query_state.ident_col_start.clone());
                cte.column(chain_state.query_state.ident_col_end.clone());

                // Base, select all subjects + predicates
                let mut sel_base = sea_query::Query::select();
                let triple_local_name = SeaRc::new(Alias::new(format!("t{}", chain_state.unique)));
                chain_state.unique += 1;
                sel_base.from_as(chain_state.query_state.triple_table.clone(), triple_local_name.clone());
                sel_base.column(
                    ColumnRef::TableColumn(
                        triple_local_name.clone(),
                        chain_state.query_state.ident_col_subject.clone(),
                    ),
                );
                sel_base.column(
                    ColumnRef::TableColumn(
                        triple_local_name.clone(),
                        chain_state.query_state.ident_col_subject.clone(),
                    ),
                );
                sel_base.union(sea_query::UnionType::Distinct, {
                    let mut q = sea_query::Query::select();
                    q.from_as(chain_state.query_state.triple_table.clone(), triple_local_name.clone());
                    q.column(
                        ColumnRef::TableColumn(
                            triple_local_name.clone(),
                            chain_state.query_state.ident_col_object.clone(),
                        ),
                    );
                    q.column(
                        ColumnRef::TableColumn(
                            triple_local_name.clone(),
                            chain_state.query_state.ident_col_object.clone(),
                        ),
                    );
                    q
                });

                // Recurse
                sel_base.union(UnionType::Distinct, {
                    let mut sel_recurse = sea_query::Query::select();
                    sel_recurse.from(cte_name.clone());
                    sel_recurse.column(cte_col_start);
                    let mut ident_prev_end = cte_col_end;
                    for next in &n_recurse.chain {
                        ident_prev_end = build_step(chain_state, next).join(&mut sel_recurse, &ident_prev_end);
                    }
                    sel_recurse.column(ident_prev_end);
                    sel_recurse
                });

                // Assemble, return
                cte.query(sel_base);
                chain_state.query_state.ctes.push(cte);
                built
            };
            return BuildStepRes {
                table: cte.cte,
                table_as: table_as.clone(),
                clause: None,
                col_start: ColumnRef::TableColumn(table_as.clone(), chain_state.query_state.ident_col_start.clone()),
                col_end: ColumnRef::TableColumn(table_as.clone(), chain_state.query_state.ident_col_end.clone()),
            };
        },
        Step::Recurse1(n_recurse) => {
            let cte = if let Some(cte) = chain_state.query_state.cte_lookup.get(step) {
                cte.clone()
            } else {
                let cte_name = SeaRc::new(Alias::new(format!("step_r1_{}", chain_state.query_state.global_unique)));
                chain_state.query_state.global_unique += 1;
                let mut cte = sea_query::CommonTableExpression::new();
                let cte_table = TableRef::Table(cte_name.clone());
                let cte_col_start =
                    ColumnRef::TableColumn(cte_name.clone(), chain_state.query_state.ident_col_start.clone());
                let cte_col_end =
                    ColumnRef::TableColumn(cte_name.clone(), chain_state.query_state.ident_col_end.clone());
                let built = BuiltCte { cte: cte_table.clone() };
                chain_state.query_state.cte_lookup.insert(step.clone(), built.clone());
                cte.table_name(cte_name.clone());
                cte.column(chain_state.query_state.ident_col_start.clone());
                cte.column(chain_state.query_state.ident_col_end.clone());

                // Base
                let mut sel_base = {
                    let mut chain = n_recurse.chain.iter();
                    let (mut sel_base, ident_prev_start, mut ident_prev_end) =
                        build_step(chain_state, chain.next().unwrap()).select(&chain_state.query_state);
                    for next in chain {
                        ident_prev_end = build_step(chain_state, next).join(&mut sel_base, &ident_prev_end);
                    }
                    sel_base.column(ident_prev_end);
                    sel_base
                };

                // Recurse
                sel_base.union(UnionType::Distinct, {
                    let mut sel_recurse = sea_query::Query::select();
                    sel_recurse.from(cte_name.clone());
                    sel_recurse.column(cte_col_start);
                    let mut ident_prev_end = cte_col_end;
                    for next in &n_recurse.chain {
                        ident_prev_end = build_step(chain_state, next).join(&mut sel_recurse, &ident_prev_end);
                    }
                    sel_recurse.column(ident_prev_end);
                    sel_recurse
                });

                // Assemble, return
                cte.query(sel_base);
                chain_state.query_state.ctes.push(cte);
                built
            };
            return BuildStepRes {
                table: cte.cte,
                table_as: table_as.clone(),
                clause: None,
                col_start: ColumnRef::TableColumn(table_as.clone(), chain_state.query_state.ident_col_start.clone()),
                col_end: ColumnRef::TableColumn(table_as.clone(), chain_state.query_state.ident_col_end.clone()),
            };
        },
        Step::First => unreachable!(),
    }
}

// Produces (sequence of) CTEs from steps, returning the last CTE. CTE has start
// and end fields only.
fn build_subchain(
    query_state: &mut QueryBuildState,
    mut prev_subchain_seg: Option<(sea_query::DynIden, sea_query::ColumnRef, sea_query::ColumnRef)>,
    steps: &[Step],
) -> BuiltSubchain {
    let chain_index = query_state.global_unique;
    query_state.global_unique += 1;
    let mut subchain_slice_index = 0;
    let mut step_index = 0;
    let mut plural = true;
    loop {
        let cte_name = SeaRc::new(Alias::new(format!("chain{}_{}", chain_index, subchain_slice_index)));
        subchain_slice_index += 1;
        let mut sql_cte = sea_query::CommonTableExpression::new();
        sql_cte.table_name(cte_name.clone());
        let mut chain_state = SubchainBuildState {
            query_state: query_state,
            unique: Default::default(),
        };
        let mut sql_sel;
        let mut ident_prev_end: ColumnRef;
        let ident_prev_start;
        if let Some((ident_prev_subchain, ident_prev_start0, ident_prev_end0)) = prev_subchain_seg {
            sql_sel = sea_query::Query::select();
            sql_sel.from(TableRef::Table(ident_prev_subchain.clone()));
            sql_sel.column(ColumnRef::TableAsterisk(ident_prev_subchain.clone()));
            ident_prev_start = ident_prev_start0;
            ident_prev_end = match ident_prev_end0 {
                ColumnRef::TableColumn(_, ident_end) => ColumnRef::TableColumn(
                    ident_prev_subchain.clone(),
                    ident_end,
                ),
                _ => unreachable!(),
            };
        } else {
            let first_step = &steps[0];
            step_index += 1;
            if let Step::First = first_step {
                panic!();
            }
            (sql_sel, ident_prev_start, ident_prev_end) =
                build_step(&mut chain_state, &first_step).select(&chain_state.query_state);
        }
        sql_sel.expr_as(ident_prev_start.clone(), chain_state.query_state.ident_col_start.clone());
        while step_index < steps.len() {
            let step = &steps[step_index];
            if let Step::First = step {
                sql_sel.limit(1);
                plural = false;
                break;
            }
            step_index += 1;
            ident_prev_end = build_step(&mut chain_state, step).join(&mut sql_sel, &ident_prev_end);
            plural = true;
        }
        sql_sel.expr_as(ident_prev_end.clone(), chain_state.query_state.ident_col_end.clone());
        if step_index >= steps.len() {
            sql_cte.query(sql_sel);
            query_state.ctes.push(sql_cte);
            return BuiltSubchain {
                cte_name: cte_name.clone(),
                cte: TableRef::Table(cte_name.clone()),
                col_start: ColumnRef::TableColumn(cte_name.clone(), query_state.ident_col_start.clone()),
                col_end: ColumnRef::TableColumn(cte_name.clone(), query_state.ident_col_end.clone()),
                plural: plural,
            };
        } else {
            sql_cte.query(sql_sel);
            query_state.ctes.push(sql_cte);
            prev_subchain_seg = Some((cte_name, ident_prev_start, ident_prev_end));
        }
    }
}

/// Produces CTE with `_` selects, no aggregation.
fn build_chain(
    query_state: &mut QueryBuildState,
    prev_subchain_seg: Option<(sea_query::DynIden, sea_query::ColumnRef, sea_query::ColumnRef)>,
    chain: Chain,
) -> BuiltChain {
    let chain_index = query_state.global_unique;
    query_state.global_unique += 1;
    let cte_name = SeaRc::new(Alias::new(format!("chain{}", chain_index)));
    let mut sql_cte = sea_query::CommonTableExpression::new();
    sql_cte.table_name(cte_name.clone());
    let mut sql_sel = sea_query::Query::select();
    let base_subchain = build_subchain(query_state, prev_subchain_seg, &chain.steps);
    sql_sel.from(base_subchain.cte);
    sql_sel.expr_as(base_subchain.col_start.clone(), query_state.ident_col_start.clone());
    sql_sel.group_by_col(base_subchain.col_start.clone());
    sql_sel.expr_as(base_subchain.col_end.clone(), query_state.ident_col_end.clone());
    sql_sel.group_by_col(base_subchain.col_end.clone());
    let mut selects = vec![];
    if let Some(name) = chain.select {
        sql_sel.expr_as(base_subchain.col_end.clone(), SeaRc::new(Alias::new(format!("_{}", name))));
        selects.push((name, base_subchain.plural));
    }
    for child in chain.children {
        let child =
            build_chain(
                query_state,
                Some(
                    (base_subchain.cte_name.clone(), base_subchain.col_start.clone(), base_subchain.col_end.clone()),
                ),
                child,
            );
        sql_sel.join(
            sea_query::JoinType::LeftJoin,
            child.cte,
            sea_query::Expr::col(
                base_subchain.col_end.clone(),
            ).eq(ColumnRef::TableColumn(child.cte_name.clone(), query_state.ident_col_start.clone())),
        );
        for (name, plural) in child.selects {
            let ident_name = SeaRc::new(Alias::new(format!("_{}", name)));
            sql_sel.expr_as(ColumnRef::TableColumn(child.cte_name.clone(), ident_name.clone()), ident_name);
            selects.push((name, base_subchain.plural || plural));
        }
    }
    return BuiltChain {
        cte_name: cte_name.clone(),
        cte: TableRef::Table(cte_name),
        selects: selects,
    };
}

fn build_node(v: Node) -> sea_query::Value {
    return sea_query::Value::Json(Some(Box::new(serde_json::to_value(&v).unwrap())));
}

fn build_query(q: Query) -> (String, sea_query_rusqlite::RusqliteValues) {
    let mut query_state = QueryBuildState {
        ident_col_start: SeaRc::new(Alias::new("start")),
        ident_col_end: SeaRc::new(Alias::new("end")),
        ident_col_subject: SeaRc::new(Alias::new("subject")),
        ident_col_predicate: SeaRc::new(Alias::new("predicate")),
        ident_col_object: SeaRc::new(Alias::new("object")),
        triple_table: TableRef::Table(SeaRc::new(Alias::new("triple"))),
        global_unique: Default::default(),
        ctes: Default::default(),
        cte_lookup: Default::default(),
    };
    let cte = build_chain(&mut query_state, None, q.chain);
    let mut sel_root = sea_query::Query::select();
    sel_root.from(cte.cte);
    if let Some(root) = q.root {
        sel_root.and_where(
            sea_query::Expr::col(
                ColumnRef::TableColumn(cte.cte_name.clone(), query_state.ident_col_start.clone()),
            ).eq(build_node(root)),
        );
    }
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
            })],
            select: Some("id".to_string()),
            children: vec![
                //. .
                Chain {
                    steps: vec![
                        //. .
                        Step::Recurse0(StepRecurse { chain: vec![Step::Move(StepMove {
                            dir: MoveDirection::Up,
                            predicate: "sunwet/1/element".to_string(),
                        })] }),
                        Step::Move(StepMove {
                            dir: MoveDirection::Down,
                            predicate: "sunwet/1/name".to_string(),
                        })
                    ],
                    select: Some("name".to_string()),
                    children: Default::default(),
                },
                Chain {
                    steps: vec![
                        //. .
                        Step::Recurse0(StepRecurse { chain: vec![Step::Move(StepMove {
                            dir: MoveDirection::Up,
                            predicate: "sunwet/1/element".to_string(),
                        })] }),
                        Step::Move(StepMove {
                            dir: MoveDirection::Down,
                            predicate: "sunwet/1/artist".to_string(),
                        }),
                        Step::Recurse0(StepRecurse { chain: vec![Step::Move(StepMove {
                            dir: MoveDirection::Up,
                            predicate: "sunwet/1/element".to_string(),
                        })] }),
                        Step::Move(StepMove {
                            dir: MoveDirection::Down,
                            predicate: "sunwet/1/name".to_string(),
                        })
                    ],
                    select: Some("artist".to_string()),
                    children: Default::default(),
                },
                Chain {
                    steps: vec![
                        //. .
                        Step::Recurse0(StepRecurse { chain: vec![Step::Move(StepMove {
                            dir: MoveDirection::Up,
                            predicate: "sunwet/1/element".to_string(),
                        })] }),
                        Step::Move(StepMove {
                            dir: MoveDirection::Down,
                            predicate: "sunwet/1/cover".to_string(),
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
