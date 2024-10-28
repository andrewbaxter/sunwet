use {
    good_ormning_runtime::sqlite::GoodOrmningCustomString,
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum FileHash {
    Sha256(String),
}

#[derive(Clone)]
pub enum Node {
    Id(String),
    File(FileHash),
    Value(serde_json::Value),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SerdeNodeType {
    I,
    F,
    V,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
struct SerdeNode {
    t: SerdeNodeType,
    v: serde_json::Value,
}

impl Serialize for Node {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        return Ok(match self {
            Node::Id(n) => SerdeNode {
                t: SerdeNodeType::I,
                v: serde_json::to_value(n).unwrap(),
            },
            Node::File(n) => SerdeNode {
                t: SerdeNodeType::F,
                v: serde_json::to_value(n).unwrap(),
            },
            Node::Value(n) => SerdeNode {
                t: SerdeNodeType::V,
                v: n.clone(),
            },
        }.serialize(serializer)?);
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        let n = SerdeNode::deserialize(deserializer)?;
        match n.t {
            SerdeNodeType::I => {
                let serde_json::Value::String(v) = n.v else {
                    return Err(de::Error::custom(format!("ID node value is not a string")));
                };
                return Ok(Node::Id(v));
            },
            SerdeNodeType::F => {
                let v = serde_json::from_value::<FileHash>(n.v).map_err(|e| de::Error::custom(e.to_string()))?;
                return Ok(Node::File(v));
            },
            SerdeNodeType::V => {
                return Ok(Node::Value(n.v));
            },
        }
    }
}

impl GoodOrmningCustomString<Node> for Node {
    fn to_sql<'a>(value: &'a Node) -> std::borrow::Cow<'a, str> {
        return serde_json::to_string(value).unwrap().into();
    }

    fn from_sql(value: String) -> Result<Node, String> {
        return serde_json::from_str(&value).map_err(|e| e.to_string())?;
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
enum MoveDirection {
    Down,
    Up,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
struct SegMove {
    dir: MoveDirection,
    predicate: String,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
struct SegRecurse {
    chain: Vec<Seg>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
enum Seg {
    Move(SegMove),
    Recurse0(SegRecurse),
    Recurse1(SegRecurse),
}

struct Chain {
    segments: Vec<Seg>,
    select: Option<String>,
    children: Vec<Chain>,
}

struct Query {
    root: Option<Node>,
    chain: Chain,
}

struct ChainBuildState<'a> {
    query_state: &'a mut QueryBuildState,
    unique: usize,
    select: HashMap<String, sea_query::ColumnRef>,
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
    cte_lookup: HashMap<Seg, BuiltCte>,
}

#[derive(Clone)]
struct BuiltCte {
    cte: sea_query::TableRef,
}

#[derive(Clone)]
struct BuiltChain {
    cte_name: sea_query::DynIden,
    cte: sea_query::TableRef,
    selects: Vec<String>,
}

#[derive(Clone)]
struct BuildSegRes {
    table: sea_query::TableRef,
    table_as: sea_query::DynIden,
    clause: Option<sea_query::SimpleExpr>,
    col_start: sea_query::ColumnRef,
    col_end: sea_query::ColumnRef,
}

impl BuildSegRes {
    fn select(self, query_state: &QueryBuildState) -> (sea_query::SelectStatement, sea_query::ColumnRef) {
        let mut sel = sea_query::Query::select();
        sel.from_as(self.table, self.table_as);
        if let Some(clause) = self.clause {
            sel.and_where(clause);
        }
        sel.expr_as(self.col_start.clone(), query_state.ident_col_start.clone());
        return (sel, self.col_end);
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

fn build_seg<'x>(s: &'x mut ChainBuildState, n: &Seg) -> BuildSegRes {
    let table_as = SeaRc::new(Alias::new(format!("t{}", s.unique)));
    s.unique += 1;
    match n {
        Seg::Move(n) => {
            let start;
            let end;
            match n.dir {
                MoveDirection::Down => {
                    start = &s.query_state.ident_col_subject;
                    end = &s.query_state.ident_col_object;
                },
                MoveDirection::Up => {
                    start = &s.query_state.ident_col_object;
                    end = &s.query_state.ident_col_subject;
                },
            }
            let col_start = ColumnRef::TableColumn(table_as.clone(), start.clone());
            let col_end = ColumnRef::TableColumn(table_as.clone(), end.clone());
            return BuildSegRes {
                table: s.query_state.triple_table.clone(),
                table_as: table_as.clone(),
                clause: Some(
                    sea_query::Expr::col(
                        ColumnRef::TableColumn(table_as, s.query_state.ident_col_predicate.clone()),
                    ).eq(n.predicate.clone()),
                ),
                col_start: col_start.clone(),
                col_end: col_end.clone(),
            };
        },
        Seg::Recurse0(n_recurse) => {
            let cte = if let Some(cte) = s.query_state.cte_lookup.get(n) {
                cte.clone()
            } else {
                let cte_name = SeaRc::new(Alias::new(format!("seg_r0_{}", s.query_state.global_unique)));
                s.query_state.global_unique += 1;
                let mut cte = sea_query::CommonTableExpression::new();
                let cte_table = TableRef::Table(cte_name.clone());
                let cte_col_start = ColumnRef::TableColumn(cte_name.clone(), s.query_state.ident_col_start.clone());
                let cte_col_end = ColumnRef::TableColumn(cte_name.clone(), s.query_state.ident_col_end.clone());
                let built = BuiltCte { cte: cte_table.clone() };
                s.query_state.cte_lookup.insert(n.clone(), built.clone());
                cte.table_name(cte_name.clone());
                cte.column(s.query_state.ident_col_start.clone());
                cte.column(s.query_state.ident_col_end.clone());

                // Base, select all subjects + predicates
                let mut sel_base = sea_query::Query::select();
                let triple_local_name = SeaRc::new(Alias::new(format!("t{}", s.unique)));
                s.unique += 1;
                sel_base.from_as(s.query_state.triple_table.clone(), triple_local_name.clone());
                sel_base.column(
                    ColumnRef::TableColumn(triple_local_name.clone(), s.query_state.ident_col_subject.clone()),
                );
                sel_base.column(
                    ColumnRef::TableColumn(triple_local_name.clone(), s.query_state.ident_col_subject.clone()),
                );
                sel_base.union(sea_query::UnionType::Distinct, {
                    let mut q = sea_query::Query::select();
                    q.from_as(s.query_state.triple_table.clone(), triple_local_name.clone());
                    q.column(
                        ColumnRef::TableColumn(triple_local_name.clone(), s.query_state.ident_col_object.clone()),
                    );
                    q.column(
                        ColumnRef::TableColumn(triple_local_name.clone(), s.query_state.ident_col_object.clone()),
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
                        ident_prev_end = build_seg(s, next).join(&mut sel_recurse, &ident_prev_end);
                    }
                    sel_recurse.column(ident_prev_end);
                    sel_recurse
                });

                // Assemble, return
                cte.query(sel_base);
                s.query_state.ctes.push(cte);
                built
            };
            return BuildSegRes {
                table: cte.cte,
                table_as: table_as.clone(),
                clause: None,
                col_start: ColumnRef::TableColumn(table_as.clone(), s.query_state.ident_col_start.clone()),
                col_end: ColumnRef::TableColumn(table_as.clone(), s.query_state.ident_col_end.clone()),
            };
        },
        Seg::Recurse1(n_recurse) => {
            let cte = if let Some(cte) = s.query_state.cte_lookup.get(n) {
                cte.clone()
            } else {
                let cte_name = SeaRc::new(Alias::new(format!("seg_r1_{}", s.query_state.global_unique)));
                s.query_state.global_unique += 1;
                let mut cte = sea_query::CommonTableExpression::new();
                let cte_table = TableRef::Table(cte_name.clone());
                let cte_col_start = ColumnRef::TableColumn(cte_name.clone(), s.query_state.ident_col_start.clone());
                let cte_col_end = ColumnRef::TableColumn(cte_name.clone(), s.query_state.ident_col_end.clone());
                let built = BuiltCte { cte: cte_table.clone() };
                s.query_state.cte_lookup.insert(n.clone(), built.clone());
                cte.table_name(cte_name.clone());
                cte.column(s.query_state.ident_col_start.clone());
                cte.column(s.query_state.ident_col_end.clone());

                // Base
                let mut sel_base = {
                    let mut chain = n_recurse.chain.iter();
                    let (mut sel_base, mut ident_prev_end) =
                        build_seg(s, chain.next().unwrap()).select(&s.query_state);
                    for next in chain {
                        ident_prev_end = build_seg(s, next).join(&mut sel_base, &ident_prev_end);
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
                        ident_prev_end = build_seg(s, next).join(&mut sel_recurse, &ident_prev_end);
                    }
                    sel_recurse.column(ident_prev_end);
                    sel_recurse
                });

                // Assemble, return
                cte.query(sel_base);
                s.query_state.ctes.push(cte);
                built
            };
            return BuildSegRes {
                table: cte.cte,
                table_as: table_as.clone(),
                clause: None,
                col_start: ColumnRef::TableColumn(table_as.clone(), s.query_state.ident_col_start.clone()),
                col_end: ColumnRef::TableColumn(table_as.clone(), s.query_state.ident_col_end.clone()),
            };
        },
    }
}

fn build_chain(query_state: &mut QueryBuildState, chain: Chain) -> BuiltChain {
    let cte_name = SeaRc::new(Alias::new(format!("chain{}", query_state.global_unique)));
    query_state.global_unique += 1;
    let mut sql_cte = sea_query::CommonTableExpression::new();
    let cte_table = TableRef::Table(cte_name.clone());
    sql_cte.table_name(cte_name.clone());
    let mut sql_sel;
    let mut selects = vec![];
    {
        let mut chain_state = ChainBuildState {
            query_state: query_state,
            unique: Default::default(),
            select: Default::default(),
        };
        let mut segments = chain.segments.into_iter();
        let mut ident_prev_end;
        (sql_sel, ident_prev_end) =
            build_seg(&mut chain_state, &segments.next().unwrap()).select(&chain_state.query_state);
        for node in segments {
            ident_prev_end = build_seg(&mut chain_state, &node).join(&mut sql_sel, &ident_prev_end);
        }
        sql_sel.expr_as(ident_prev_end.clone(), chain_state.query_state.ident_col_end.clone());
        if let Some(name) = chain.select {
            chain_state.select.insert(name, ident_prev_end.clone());
        }
        for (name, val) in chain_state.select {
            sql_sel.expr_as(val, SeaRc::new(Alias::new(format!("_{}", name))));
            selects.push(name);
        }
        for child in chain.children {
            let child = build_chain(query_state, child);
            sql_sel.join(
                sea_query::JoinType::LeftJoin,
                child.cte,
                sea_query::Expr::col(
                    ident_prev_end.clone(),
                ).eq(ColumnRef::TableColumn(child.cte_name.clone(), query_state.ident_col_start.clone())),
            );
            for name in child.selects {
                let ident_name = SeaRc::new(Alias::new(format!("_{}", name)));
                sql_sel.expr_as(ColumnRef::TableColumn(child.cte_name.clone(), ident_name.clone()), ident_name);
                selects.push(name);
            }
        }
    }
    sql_cte.query(sql_sel);
    query_state.ctes.push(sql_cte);
    return BuiltChain {
        cte_name: cte_name,
        cte: cte_table,
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

    // Build main branch
    let cte = build_chain(&mut query_state, q.chain);
    let mut sel_root = sea_query::Query::select();
    sel_root.from(cte.cte);
    if let Some(root) = q.root {
        sel_root.and_where(
            sea_query::Expr::col(
                ColumnRef::TableColumn(cte.cte_name.clone(), query_state.ident_col_start.clone()),
            ).eq(build_node(root)),
        );
    }
    for name in cte.selects {
        sel_root.expr_as(
            sea_query::ColumnRef::TableColumn(cte.cte_name.clone(), SeaRc::new(Alias::new(format!("_{}", name)))),
            SeaRc::new(Alias::new(name)),
        );
    }

    // Assemble
    let mut sel = sea_query::WithQuery::new();
    sel.recursive(true);
    sel.query(sel_root);
    for cte in query_state.ctes {
        sel.cte(cte);
    }

    // Done
    return sel.build_rusqlite(SqliteQueryBuilder);
}

fn main() {
    let (query, query_values) = build_query(Query {
        root: Some(Node::Id("sunwet/1/album".to_string())),
        chain: Chain {
            segments: vec![Seg::Move(SegMove {
                dir: MoveDirection::Up,
                predicate: "sunwet/1/is".to_string(),
            })],
            select: Some("id".to_string()),
            children: vec![
                //. .
                Chain {
                    segments: vec![
                        //. .
                        Seg::Recurse0(SegRecurse { chain: vec![Seg::Move(SegMove {
                            dir: MoveDirection::Up,
                            predicate: "sunwet/1/element".to_string(),
                        })] }),
                        Seg::Move(SegMove {
                            dir: MoveDirection::Down,
                            predicate: "sunwet/1/name".to_string(),
                        })
                    ],
                    select: Some("name".to_string()),
                    children: Default::default(),
                },
                Chain {
                    segments: vec![
                        //. .
                        Seg::Recurse0(SegRecurse { chain: vec![Seg::Move(SegMove {
                            dir: MoveDirection::Up,
                            predicate: "sunwet/1/element".to_string(),
                        })] }),
                        Seg::Move(SegMove {
                            dir: MoveDirection::Down,
                            predicate: "sunwet/1/artist".to_string(),
                        }),
                        Seg::Recurse0(SegRecurse { chain: vec![Seg::Move(SegMove {
                            dir: MoveDirection::Up,
                            predicate: "sunwet/1/element".to_string(),
                        })] }),
                        Seg::Move(SegMove {
                            dir: MoveDirection::Down,
                            predicate: "sunwet/1/name".to_string(),
                        })
                    ],
                    select: Some("artist".to_string()),
                    children: Default::default(),
                },
                Chain {
                    segments: vec![
                        //. .
                        Seg::Recurse0(SegRecurse { chain: vec![Seg::Move(SegMove {
                            dir: MoveDirection::Up,
                            predicate: "sunwet/1/element".to_string(),
                        })] }),
                        Seg::Move(SegMove {
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
    for (s, p, o) in [
        //. .
        ("a", "sunwet/1/is", "sunwet/1/album"),
        ("a", "sunwet/1/name", "a_name"),
        ("a", "sunwet/1/artist", "a_a"),
        ("a_a", "sunwet/1/name", "a_a_name"),
    ] {
        db::triple_insert(&db, &Node::Id(s.to_string()), p, &Node::Id(o.to_string())).unwrap();
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
