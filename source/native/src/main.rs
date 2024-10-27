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
        Deserialize,
        Serialize,
    },
    std::collections::HashMap,
};

pub mod db;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum FileHash {
    Sha256(String),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Node {
    Id(String),
    File(FileHash),
    Value(serde_json::Value),
}

const NODE_PREFIX_ID: &str = "i";
const NODE_PREFIX_FILE: &str = "f";
const NODE_PREFIX_VALUE: &str = "v";
const NODE_FILE_PREFIX_SHA256: &str = "sha256";

impl GoodOrmningCustomString<Node> for Node {
    fn to_sql<'a>(value: &'a Node) -> std::borrow::Cow<'a, str> {
        match value {
            Node::Id(v) => {
                return format!("{}:{}", NODE_PREFIX_ID, v).into();
            },
            Node::File(v) => {
                match v {
                    FileHash::Sha256(v) => {
                        return format!("{}:{}:{}", NODE_PREFIX_FILE, NODE_FILE_PREFIX_SHA256, v).into();
                    },
                }
            },
            Node::Value(v) => {
                return format!("{}:{}", v, serde_json::to_string(v).unwrap()).into();
            },
        }
    }

    fn from_sql(value: String) -> Result<Node, String> {
        let Some((prefix, suffix)) = value.split_once(":") else {
            return Err("Invalid node value, no :-separated prefix and suffix".to_string());
        };
        match prefix {
            NODE_PREFIX_ID => {
                return Ok(Node::Id(suffix.to_string()));
            },
            NODE_PREFIX_FILE => {
                let Some((prefix, suffix)) = value.split_once(":") else {
                    return Err("Invalid file node hash value, no :-separated prefix and suffix".to_string());
                };
                match prefix {
                    NODE_FILE_PREFIX_SHA256 => {
                        return Ok(Node::File(FileHash::Sha256(suffix.to_string())));
                    },
                    _ => {
                        return Err(format!("Unrecognized file node hash type prefix [{}]", prefix));
                    },
                }
            },
            NODE_PREFIX_VALUE => {
                return Ok(
                    Node::Value(
                        serde_json::from_str(
                            suffix,
                        ).map_err(|e| format!("Failed to parse json for value node: {:?}", e))?,
                    ),
                );
            },
            _ => {
                return Err(format!("Unrecognized node type prefix [{}]", prefix));
            },
        }
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
    chain: Vec<Seg>,
    select: Option<String>,
    children: Vec<Chain>,
}

struct Query {
    root: Option<Node>,
    branch: Chain,
}

struct BuildState {
    // # Immutable
    ident_col_cte_start: sea_query::DynIden,
    ident_col_cte_end: sea_query::DynIden,
    ident_col_subject: sea_query::DynIden,
    ident_col_object: sea_query::DynIden,
    triple_table: TableRef,
    triple_col_subject: ColumnRef,
    triple_col_predicate: ColumnRef,
    triple_col_object: ColumnRef,
    // # Mutable
    unique: usize,
    ctes: Vec<sea_query::CommonTableExpression>,
    cte_lookup: HashMap<Seg, BuiltCte>,
    select: HashMap<String, sea_query::ColumnRef>,
}

#[derive(Clone)]
struct BuiltCte {
    cte: sea_query::TableRef,
    cte_col_start: sea_query::ColumnRef,
    cte_col_end: sea_query::ColumnRef,
}

#[derive(Clone)]
struct BuildSegRes {
    table: sea_query::TableRef,
    table_as: sea_query::DynIden,
    clause: Option<sea_query::SimpleExpr>,
    col_start: sea_query::ColumnRef,
    col_end: sea_query::ColumnRef,
}

enum WantColStart<'a> {
    None,
    Join(&'a sea_query::TableRef, &'a sea_query::ColumnRef),
    Where(sea_query::Expr),
}

impl BuildSegRes {
    fn select(self, want_start: WantColStart) -> (sea_query::SelectStatement, sea_query::ColumnRef) {
        let mut sel = sea_query::Query::select();
        sel.from_as(self.table, self.table_as);
        match want_start {
            WantColStart::None => { },
            WantColStart::Join(join_table, join_col) => {
                sel.join(
                    sea_query::JoinType::RightJoin,
                    join_table.clone(),
                    sea_query::Expr::col(self.col_start.clone()).eq(join_col.clone()),
                );
            },
            WantColStart::Where(where_) => {
                sel.and_where(sea_query::Expr::col(self.col_start.clone()).eq(where_));
            },
        }
        if let Some(clause) = self.clause {
            sel.and_where(clause);
        }
        sel.column(self.col_start.clone());
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

fn build_seg<'x>(s: &'x mut BuildState, n: &Seg) -> BuildSegRes {
    match n {
        Seg::Move(n) => {
            let start;
            let end;
            match n.dir {
                MoveDirection::Down => {
                    start = &s.ident_col_subject;
                    end = &s.ident_col_object;
                },
                MoveDirection::Up => {
                    start = &s.ident_col_object;
                    end = &s.ident_col_subject;
                },
            }
            let table_as = SeaRc::new(Alias::new(format!("t{}", s.unique)));
            s.unique += 1;
            let col_start = ColumnRef::TableColumn(table_as.clone(), start.clone());
            let col_end = ColumnRef::TableColumn(table_as.clone(), end.clone());
            return BuildSegRes {
                table: s.triple_table.clone(),
                table_as: table_as,
                clause: Some(sea_query::Expr::col(s.triple_col_predicate.clone()).eq(n.predicate.clone())),
                col_start: col_start.clone(),
                col_end: col_end.clone(),
            };
        },
        Seg::Recurse0(n_recurse) => {
            let cte_name = SeaRc::new(Alias::new(format!("seg_r0_{}", s.unique)));
            s.unique += 1;
            let cte = if let Some(cte) = s.cte_lookup.get(n) {
                cte.clone()
            } else {
                let mut cte = sea_query::CommonTableExpression::new();
                let cte_table = TableRef::Table(cte_name.clone());
                let cte_col_start = ColumnRef::TableColumn(cte_name.clone(), s.ident_col_cte_start.clone());
                let cte_col_end = ColumnRef::TableColumn(cte_name.clone(), s.ident_col_cte_end.clone());
                let built = BuiltCte {
                    cte: cte_table.clone(),
                    cte_col_start: cte_col_start.clone(),
                    cte_col_end: cte_col_end.clone(),
                };
                s.cte_lookup.insert(n.clone(), built.clone());
                cte.table_name(cte_name.clone());
                cte.column(s.ident_col_cte_start.clone());
                cte.column(s.ident_col_cte_end.clone());

                // Base, select all subjects + predicates
                let mut sel_base = sea_query::Query::select();
                sel_base.from(s.triple_table.clone());
                sel_base.column(s.triple_col_subject.clone());
                sel_base.column(s.triple_col_subject.clone());
                sel_base.union(sea_query::UnionType::Distinct, {
                    let mut q = sea_query::Query::select();
                    q.from(s.triple_table.clone());
                    q.column(s.triple_col_object.clone());
                    q.column(s.triple_col_object.clone());
                    q
                });

                // Recurse
                sel_base.union(UnionType::Distinct, {
                    let mut sel_recurse = sea_query::Query::select();
                    sel_recurse.from(cte_name.clone());
                    let mut ident_prev_end = cte_col_end;
                    for next in &n_recurse.chain {
                        ident_prev_end = build_seg(s, next).join(&mut sel_recurse, &ident_prev_end);
                    }
                    sel_recurse.column(ident_prev_end);
                    sel_recurse
                });

                // Assemble, return
                cte.query(sel_base);
                s.ctes.push(cte);
                built
            };
            return BuildSegRes {
                table: cte.cte,
                table_as: cte_name,
                clause: None,
                col_start: cte.cte_col_start,
                col_end: cte.cte_col_end,
            };
        },
        Seg::Recurse1(n_recurse) => {
            let cte_name = SeaRc::new(Alias::new(format!("seg_r1_{}", s.unique)));
            s.unique += 1;
            let cte = if let Some(cte) = s.cte_lookup.get(n) {
                cte.clone()
            } else {
                let mut cte = sea_query::CommonTableExpression::new();
                let cte_table = TableRef::Table(cte_name.clone());
                let cte_col_start = ColumnRef::TableColumn(cte_name.clone(), s.ident_col_cte_start.clone());
                let cte_col_end = ColumnRef::TableColumn(cte_name.clone(), s.ident_col_cte_end.clone());
                let built = BuiltCte {
                    cte: cte_table.clone(),
                    cte_col_start: cte_col_start.clone(),
                    cte_col_end: cte_col_end.clone(),
                };
                s.cte_lookup.insert(n.clone(), built.clone());
                cte.table_name(cte_name.clone());
                cte.column(s.ident_col_cte_start.clone());
                cte.column(s.ident_col_cte_end.clone());

                // Base
                let mut sel_base = {
                    let mut chain = n_recurse.chain.iter();
                    let (mut sel_base, mut ident_prev_end) =
                        build_seg(s, chain.next().unwrap()).select(WantColStart::None);
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
                    let mut ident_prev_end = cte_col_end;
                    for next in &n_recurse.chain {
                        ident_prev_end = build_seg(s, next).join(&mut sel_recurse, &ident_prev_end);
                    }
                    sel_recurse.column(ident_prev_end);
                    sel_recurse
                });

                // Assemble, return
                cte.query(sel_base);
                s.ctes.push(cte);
                built
            };
            return BuildSegRes {
                table: cte.cte,
                table_as: cte_name,
                clause: None,
                col_start: cte.cte_col_start,
                col_end: cte.cte_col_end,
            };
        },
    }
}

fn build_branch_inner(s: &mut BuildState, branch: Chain) -> sea_query::SelectStatement {
    let mut chain = branch.chain.into_iter();
    let (mut sel, mut ident_prev_end) = build_seg(s, &chain.next().unwrap()).select(WantColStart::None);
    for node in chain {
        ident_prev_end = build_seg(s, &node).join(&mut sel, &ident_prev_end);
    }
    sel.column(ident_prev_end.clone());
    if let Some(name) = branch.select {
        s.select.insert(name, ident_prev_end.clone());
    }
    for child in branch.children {
        let child = build_branch(s, child);
        sel.join(
            sea_query::JoinType::LeftJoin,
            child.cte,
            sea_query::Expr::col(ident_prev_end.clone()).eq(child.cte_col_start),
        );
    }
    return sel;
}

fn build_branch(s: &mut BuildState, branch: Chain) -> BuiltCte {
    let cte_name = SeaRc::new(Alias::new(format!("branch{}", s.unique)));
    s.unique += 1;
    let mut cte = sea_query::CommonTableExpression::new();
    let cte_table = TableRef::Table(cte_name.clone());
    let cte_col_start = ColumnRef::TableColumn(cte_name.clone(), s.ident_col_cte_start.clone());
    let cte_col_end = ColumnRef::TableColumn(cte_name.clone(), s.ident_col_cte_end.clone());
    cte.table_name(cte_name.clone());
    cte.column(s.ident_col_cte_start.clone());
    cte.column(s.ident_col_cte_end.clone());
    cte.query(build_branch_inner(s, branch));
    s.ctes.push(cte);
    return BuiltCte {
        cte: cte_table,
        cte_col_start: cte_col_start,
        cte_col_end: cte_col_end,
    }
}

fn build_node(v: Node) -> sea_query::Value {
    return sea_query::Value::Json(Some(Box::new(serde_json::to_value(&v).unwrap())));
}

fn build_query(q: Query) -> (String, sea_query_rusqlite::RusqliteValues) {
    let ident_tab_triple = SeaRc::new(Alias::new("triple"));
    let ident_col_subject = SeaRc::new(Alias::new("subject"));
    let ident_col_object = SeaRc::new(Alias::new("object"));
    let mut s = BuildState {
        ident_col_cte_start: SeaRc::new(Alias::new("start")),
        ident_col_cte_end: SeaRc::new(Alias::new("end")),
        ident_col_subject: ident_col_subject.clone(),
        ident_col_object: ident_col_object.clone(),
        triple_table: TableRef::Table(ident_tab_triple.clone()),
        triple_col_subject: ColumnRef::TableColumn(ident_tab_triple.clone(), ident_col_subject),
        triple_col_predicate: ColumnRef::TableColumn(ident_tab_triple.clone(), SeaRc::new(Alias::new("predicate"))),
        triple_col_object: ColumnRef::TableColumn(ident_tab_triple.clone(), ident_col_object),
        unique: Default::default(),
        ctes: Default::default(),
        cte_lookup: Default::default(),
        select: Default::default(),
    };

    // Build main branch
    let mut chain = q.branch.chain.into_iter();
    let (mut sel_base, mut ident_prev_end) = build_seg(&mut s, &chain.next().unwrap()).select(match q.root {
        Some(root) => WantColStart::Where(sea_query::Expr::val(build_node(root))),
        None => WantColStart::None,
    });
    for node in chain {
        ident_prev_end = build_seg(&mut s, &node).join(&mut sel_base, &ident_prev_end);
    }
    if let Some(name) = q.branch.select {
        s.select.insert(name, ident_prev_end.clone());
    }
    for child in q.branch.children {
        let child = build_branch(&mut s, child);
        sel_base.join(
            sea_query::JoinType::LeftJoin,
            child.cte,
            sea_query::Expr::col(ident_prev_end.clone()).eq(child.cte_col_start),
        );
    }

    // Assemble dependencies
    for (name, val) in s.select {
        sel_base.expr_as(sea_query::Expr::col(val), SeaRc::new(Alias::new(name)));
    }
    let mut sel = sea_query::WithQuery::new();
    sel.recursive(true);
    sel.query(sel_base);
    for cte in s.ctes {
        sel.cte(cte);
    }

    // Done
    return sel.build_rusqlite(SqliteQueryBuilder);
}

fn main() {
    let (query, query_values) = build_query(Query {
        root: Some(Node::Id("sunwet/1/album".to_string())),
        branch: Chain {
            chain: vec![Seg::Move(SegMove {
                dir: MoveDirection::Up,
                predicate: "sunwet/1/is".to_string(),
            })],
            select: Some("id".to_string()),
            children: vec![
                //. .
                Chain {
                    chain: vec![
                        //. .
                        Seg::Recurse1(SegRecurse { chain: vec![Seg::Move(SegMove {
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
                    chain: vec![
                        //. .
                        Seg::Recurse1(SegRecurse { chain: vec![Seg::Move(SegMove {
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
                    chain: vec![
                        //. .
                        Seg::Recurse1(SegRecurse { chain: vec![Seg::Move(SegMove {
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
    let mut db = rusqlite::Connection::open_in_memory().unwrap();
    db::migrate(&mut db).unwrap();
    for (s, p, o) in [("a", "sunwet/1/is", "sunwet/1/album")] {
        db::triple_insert(&db, &Node::Id(s.to_string()), p, &Node::Id(o.to_string())).unwrap();
    }
    println!("Query: {}", query);
    println!("{:?}", db.execute(&query, &*query_values.as_params()).unwrap());
}
