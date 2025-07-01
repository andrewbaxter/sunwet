use {
    super::dbutil::tx,
    deadpool_sqlite::Pool,
    flowcontrol::{
        exenum,
        superif,
    },
    htwrap::htserve::viserr::{
        ResultVisErr,
        VisErr,
    },
    loga::{
        ea,
        ResultContext,
    },
    rand::{
        seq::SliceRandom,
        thread_rng,
        Rng,
        SeedableRng,
    },
    sea_query::{
        extension::sqlite::SqliteExpr,
        Alias,
        ColumnRef,
        Expr,
        ExprTrait,
        Nullable,
        SeaRc,
        SimpleExpr,
        TableRef,
        WindowStatement,
    },
    sea_query_rusqlite::RusqliteBinder,
    shared::interface::{
        query::{
            Chain,
            ChainBody,
            ChainRoot,
            FilterExpr,
            FilterExprExistsType,
            FilterSuffix,
            FilterSuffixSimpleOperator,
            JunctionType,
            MoveDirection,
            Query,
            QuerySort,
            QuerySortDir,
            Step,
            StrValue,
            Value,
        },
        triple::Node,
        wire::{
            Pagination,
            TreeNode,
        },
    },
    std::{
        cmp::Ordering,
        collections::{
            BTreeMap,
            HashMap,
        },
    },
};

fn sql_fn(name: &str, args: Vec<SimpleExpr>) -> SimpleExpr {
    let mut f = sea_query::Func::cust(SeaRc::new(Alias::new(name)));
    for arg in args {
        f = f.arg(arg);
    }
    return sea_query::SimpleExpr::FunctionCall(f).into();
}

struct QueryBuildState {
    parameters: HashMap<String, Node>,
    // # Immutable
    ident_rowid: sea_query::DynIden,
    ident_table_primary: sea_query::DynIden,
    ident_table_prev: sea_query::DynIden,
    ident_col_start: sea_query::DynIden,
    ident_col_end: sea_query::DynIden,
    ident_col_subject: sea_query::DynIden,
    ident_col_predicate: sea_query::DynIden,
    ident_col_object: sea_query::DynIden,
    triple_exist_table: sea_query::DynIden,
    func_json_extract: sea_query::FunctionCall,
    // # Mutable
    global_unique: usize,
    ctes: Vec<sea_query::CommonTableExpression>,
    reuse_roots: HashMap<Value, BuildStepRes>,
    reuse_steps: HashMap<(Option<BuildStepRes>, Step), BuildStepRes>,
}

#[derive(Clone)]
struct BuildChainRes {
    cte_name: sea_query::DynIden,
    cte: sea_query::TableRef,
    plural: bool,
    selects: Vec<(String, bool)>,
}

#[derive(Clone, PartialEq)]
struct BuildStepRes {
    ident_table: sea_query::DynIden,
    col_start: sea_query::DynIden,
    col_end: sea_query::DynIden,
    plural: bool,
}

impl Eq for BuildStepRes { }

impl Ord for BuildStepRes {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.ident_table.to_string().cmp(&other.ident_table.to_string()) {
            core::cmp::Ordering::Equal => { },
            ord => return ord,
        }
        match self.col_start.to_string().cmp(&other.col_start.to_string()) {
            core::cmp::Ordering::Equal => { },
            ord => return ord,
        }
        match self.col_end.to_string().cmp(&other.col_end.to_string()) {
            core::cmp::Ordering::Equal => { },
            ord => return ord,
        }
        match self.plural.cmp(&other.plural) {
            core::cmp::Ordering::Equal => { },
            ord => return ord,
        }
        return std::cmp::Ordering::Equal;
    }
}

impl PartialOrd for BuildStepRes {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.ident_table.to_string().partial_cmp(&other.ident_table.to_string()) {
            Some(core::cmp::Ordering::Equal) => { },
            ord => return ord,
        }
        match self.col_start.to_string().partial_cmp(&other.col_start.to_string()) {
            Some(core::cmp::Ordering::Equal) => { },
            ord => return ord,
        }
        match self.col_end.to_string().partial_cmp(&other.col_end.to_string()) {
            Some(core::cmp::Ordering::Equal) => { },
            ord => return ord,
        }
        match self.plural.partial_cmp(&other.plural) {
            Some(core::cmp::Ordering::Equal) => { },
            ord => return ord,
        }
        return Some(std::cmp::Ordering::Equal);
    }
}

impl std::hash::Hash for BuildStepRes {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ident_table.to_string().hash(state);
        self.col_start.to_string().hash(state);
        self.col_end.to_string().hash(state);
        self.plural.hash(state);
    }
}

fn build_filter(
    query_state: &mut QueryBuildState,
    parent_end_col: &ColumnRef,
    previous: BuildStepRes,
    expr: &FilterExpr,
) -> Result<sea_query::SimpleExpr, VisErr<loga::Error>> {
    match expr {
        FilterExpr::Exists(expr) => {
            let mut sql_sel = sea_query::Query::select();
            let subchain = build_subchain(query_state, Some(previous), &expr.subchain)?;
            sql_sel.from(sea_query::TableRef::Table(subchain.ident_table.clone()));
            sql_sel.expr(sea_query::Expr::val(1));
            sql_sel.and_where(
                sea_query::Expr::col(
                    sea_query::ColumnRef::TableColumn(subchain.ident_table.clone(), subchain.col_start),
                ).eq(parent_end_col.clone()),
            );
            let primary_end_col =
                sea_query::Expr::col(sea_query::ColumnRef::TableColumn(subchain.ident_table, subchain.col_end));
            let primary_type = query_state.func_json_extract.clone().arg(primary_end_col.clone()).arg("$.t");
            let primary_value = query_state.func_json_extract.clone().arg(primary_end_col.clone()).arg("$.v");
            if let Some(filter_suffix) = &expr.suffix {
                match filter_suffix {
                    FilterSuffix::Simple(filter_suffix) => {
                        let (expr_type, expr_value) = build_split_value(query_state, &filter_suffix.value)?;
                        sql_sel.and_where(primary_type.eq(expr_type));
                        sql_sel.and_where(match filter_suffix.op {
                            FilterSuffixSimpleOperator::Eq => primary_value.eq(expr_value),
                            FilterSuffixSimpleOperator::Neq => primary_value.eq(expr_value).not(),
                            FilterSuffixSimpleOperator::Lt => primary_value.lt(expr_value),
                            FilterSuffixSimpleOperator::Gt => primary_value.gt(expr_value),
                            FilterSuffixSimpleOperator::Lte => primary_value.lte(expr_value),
                            FilterSuffixSimpleOperator::Gte => primary_value.gte(expr_value),
                        });
                    },
                    FilterSuffix::Like(filter_suffix) => {
                        sql_sel.and_where(
                            primary_value.like(&build_value_str(query_state, &filter_suffix.value)?),
                        );
                    },
                }
            }
            let sql_expr = sea_query::Expr::exists(sql_sel);
            match expr.type_ {
                FilterExprExistsType::Exists => {
                    return Ok(sql_expr);
                },
                FilterExprExistsType::DoesntExist => {
                    return Ok(sql_expr.not());
                },
            }
        },
        FilterExpr::Junction(expr) => {
            let mut out = build_filter(query_state, parent_end_col, previous.clone(), &expr.subexprs[0])?;
            for subexpr in &expr.subexprs[1..] {
                let next = build_filter(query_state, parent_end_col, previous.clone(), &subexpr)?;
                match expr.type_ {
                    JunctionType::And => {
                        out = out.and(next);
                    },
                    JunctionType::Or => {
                        out = out.or(next);
                    },
                }
            }
            return Ok(out);
        },
    }
}

fn build_step(
    query_state: &mut QueryBuildState,
    previous: Option<BuildStepRes>,
    step: &Step,
) -> Result<BuildStepRes, VisErr<loga::Error>> {
    if let Some(r) = query_state.reuse_steps.get(&(previous.clone(), step.clone())) {
        return Ok(r.clone());
    }
    let mut out;
    match step {
        Step::Move(step) => {
            let seg_name = format!("seg{}_move", query_state.global_unique);
            query_state.global_unique += 1;
            {
                let ident_cte = SeaRc::new(Alias::new(seg_name.clone()));
                let mut sql_cte = sea_query::CommonTableExpression::new();
                sql_cte.table_name(ident_cte.clone());

                // Select
                let mut sql_sel = sea_query::Query::select();
                let local_ident_table_primary = query_state.ident_table_primary.clone();
                sql_sel.from_as(query_state.triple_exist_table.clone(), local_ident_table_primary.clone());

                // Direction selection
                let from_ident_primary_start;
                let from_ident_primary_end;
                match step.dir {
                    MoveDirection::Forward => {
                        from_ident_primary_start = &query_state.ident_col_subject;
                        from_ident_primary_end = &query_state.ident_col_object;
                    },
                    MoveDirection::Backward => {
                        from_ident_primary_start = &query_state.ident_col_object;
                        from_ident_primary_end = &query_state.ident_col_subject;
                    },
                }
                let local_col_primary_start =
                    sea_query::ColumnRef::TableColumn(
                        local_ident_table_primary.clone(),
                        from_ident_primary_start.clone(),
                    );
                let local_col_primary_end =
                    sea_query::ColumnRef::TableColumn(
                        local_ident_table_primary.clone(),
                        from_ident_primary_end.clone(),
                    );

                // Movement
                sql_sel.and_where(
                    sea_query::Expr::col(
                        sea_query::ColumnRef::TableColumn(
                            local_ident_table_primary.clone(),
                            query_state.ident_col_predicate.clone(),
                        ),
                    ).eq(build_value_str(query_state, &step.predicate)?),
                );

                // Output start col - subset of previous results
                let out_col_start;
                if let Some(previous) = &previous {
                    let local_ident_table_prev = query_state.ident_table_prev.clone();
                    sql_sel.join_as(
                        sea_query::JoinType::InnerJoin,
                        previous.ident_table.clone(),
                        local_ident_table_prev.clone(),
                        sea_query::Expr::col(
                            sea_query::ColumnRef::TableColumn(
                                local_ident_table_prev.clone(),
                                previous.col_end.clone(),
                            ),
                        ).eq(local_col_primary_start.clone()),
                    );
                    out_col_start =
                        sea_query::ColumnRef::TableColumn(
                            local_ident_table_prev.clone(),
                            previous.col_start.clone(),
                        );
                } else {
                    out_col_start = local_col_primary_start.clone();
                }
                sql_cte.column(query_state.ident_col_start.clone());
                sql_sel.column(out_col_start);

                // Output rowid
                sql_cte.column(query_state.ident_rowid.clone());
                sql_sel.expr_window(sql_fn("row_number", vec![]), WindowStatement::new());

                // Output end col
                sql_cte.column(query_state.ident_col_end.clone());
                sql_sel.column(local_col_primary_end.clone());

                // Assemble
                sql_cte.query(sql_sel);
                query_state.ctes.push(sql_cte);
                out = BuildStepRes {
                    ident_table: ident_cte.clone(),
                    col_start: query_state.ident_col_start.clone(),
                    col_end: query_state.ident_col_end.clone(),
                    plural: !step.first,
                };
            }

            // Filter + limit
            if let Some(filter) = &step.filter {
                let ident_cte = SeaRc::new(Alias::new(format!("{}__filter", seg_name)));
                let mut sql_cte = sea_query::CommonTableExpression::new();
                sql_cte.table_name(ident_cte.clone());

                // Select, from previous
                let mut sql_sel = sea_query::Query::select();
                let primary_table = query_state.ident_table_primary.clone();
                let primary_col_start =
                    sea_query::ColumnRef::TableColumn(primary_table.clone(), out.col_start.clone());
                let primary_col_end = sea_query::ColumnRef::TableColumn(primary_table.clone(), out.col_end.clone());
                sql_sel.from_as(out.ident_table.clone(), primary_table.clone());

                // Apply filter
                sql_sel.and_where(build_filter(query_state, &primary_col_end, BuildStepRes {
                    ident_table: out.ident_table,
                    col_start: out.col_end.clone(),
                    col_end: out.col_end,
                    plural: out.plural,
                }, filter)?);

                // Output rowid
                sql_cte.column(query_state.ident_rowid.clone());
                sql_sel.expr_window(sql_fn("row_number", vec![]), WindowStatement::new());

                // Output start
                sql_cte.column(query_state.ident_col_start.clone());
                sql_sel.column(primary_col_start.clone());

                // Output end
                sql_cte.column(query_state.ident_col_end.clone());
                sql_sel.column(primary_col_end.clone());

                // Limit/first
                if step.first {
                    sql_sel.group_by_col(primary_col_start);
                    sql_sel.group_by_col(primary_col_end);
                    sql_cte.column(SeaRc::new(Alias::new("_unused_first")));
                    sql_sel.expr(
                        Expr::expr(
                            sea_query::ColumnRef::TableColumn(primary_table, SeaRc::new(Alias::new("rowid"))),
                        ).min(),
                    );
                }

                // Assemble
                sql_cte.query(sql_sel);
                query_state.ctes.push(sql_cte);
                out = BuildStepRes {
                    ident_table: ident_cte.clone(),
                    col_start: query_state.ident_col_start.clone(),
                    col_end: query_state.ident_col_end.clone(),
                    plural: out.plural,
                };
            }
        },
        Step::Recurse(step) => {
            let seg_name = format!("seg{}_recurse", query_state.global_unique);
            query_state.global_unique += 1;
            {
                let Some(previous) = previous.as_ref() else {
                    return Err(
                        loga::err("Recursion requires a previous step/root for the base case, but none such exists"),
                    ).err_external();
                };
                let global_ident_table_cte = SeaRc::new(Alias::new(seg_name.clone()));
                let table_cte = sea_query::TableRef::Table(global_ident_table_cte.clone());

                // Base case
                let mut sql_sel = sea_query::Query::select();
                {
                    let local_ident_table_prev = query_state.ident_table_prev.clone();
                    sql_sel.from_as(previous.ident_table.clone(), local_ident_table_prev.clone());
                    sql_sel.column(
                        sea_query::ColumnRef::TableColumn(local_ident_table_prev.clone(), previous.col_start.clone()),
                    );
                    sql_sel.column(
                        sea_query::ColumnRef::TableColumn(local_ident_table_prev.clone(), previous.col_end.clone()),
                    );
                }

                // Recursive case
                sql_sel.union(sea_query::UnionType::Distinct, {
                    let mut sql_sel = sea_query::Query::select();
                    sql_sel.from(table_cte);
                    sql_sel.column(
                        sea_query::ColumnRef::TableColumn(
                            global_ident_table_cte.clone(),
                            query_state.ident_col_start.clone(),
                        ),
                    );
                    let subchain = build_subchain(query_state, None, &step.subchain)?;
                    let local_ident_table_primary = query_state.ident_table_primary.clone();
                    sql_sel.join_as(
                        sea_query::JoinType::InnerJoin,
                        sea_query::TableRef::Table(subchain.ident_table.clone()),
                        local_ident_table_primary.clone(),
                        sea_query::Expr::col(
                            sea_query::ColumnRef::TableColumn(local_ident_table_primary.clone(), subchain.col_start),
                        ).eq(
                            sea_query::ColumnRef::TableColumn(
                                global_ident_table_cte.clone(),
                                query_state.ident_col_end.clone(),
                            ),
                        ),
                    );
                    sql_sel.column(sea_query::ColumnRef::TableColumn(local_ident_table_primary, subchain.col_end));
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
                let global_ident_table_cte = SeaRc::new(Alias::new(format!("{}_b", seg_name)));
                let ident_col_start = query_state.ident_col_start.clone();
                let ident_col_end = query_state.ident_col_end.clone();
                let mut sql_sel = sea_query::Query::select();
                sql_sel.from(sea_query::TableRef::Table(out.ident_table.clone()));
                sql_sel.column(sea_query::ColumnRef::TableColumn(out.ident_table.clone(), out.col_start));
                sql_sel.column(sea_query::ColumnRef::TableColumn(out.ident_table.clone(), out.col_end));
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
        },
        Step::Junction(step) => {
            let seg_name = format!("seg{}_recurse", query_state.global_unique);
            query_state.global_unique += 1;
            let global_ident_table_cte = SeaRc::new(Alias::new(seg_name));
            let ident_col_start = query_state.ident_col_start.clone();
            let ident_col_end = query_state.ident_col_end.clone();
            let mut build_subchain =
                |subchain: &ChainBody| -> Result<sea_query::SelectStatement, VisErr<loga::Error>> {
                    let mut sql_sel = sea_query::Query::select();
                    let subchain = build_subchain(query_state, previous.clone(), subchain)?;
                    sql_sel.from(sea_query::TableRef::Table(subchain.ident_table.clone()));
                    sql_sel.column(
                        sea_query::ColumnRef::TableColumn(subchain.ident_table.clone(), subchain.col_start),
                    );
                    sql_sel.column(sea_query::ColumnRef::TableColumn(subchain.ident_table, subchain.col_end));
                    return Ok(sql_sel);
                };
            let mut sql_sel = build_subchain(&step.subchains[0])?;
            for subchain in &step.subchains[1..] {
                sql_sel.union(match step.type_ {
                    JunctionType::And => sea_query::UnionType::Intersect,
                    JunctionType::Or => sea_query::UnionType::Distinct,
                }, build_subchain(subchain)?);
            }

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
        },
    }
    query_state.reuse_steps.insert((previous, step.clone()), out.clone());
    return Ok(out);
}

fn build_value_str(query_state: &mut QueryBuildState, param: &StrValue) -> Result<String, VisErr<loga::Error>> {
    match param {
        StrValue::Literal(r) => {
            return Ok(r.clone());
        },
        StrValue::Parameter(p) => {
            let Some(v) = query_state.parameters.get(p) else {
                return Err(loga::err_with("Missing value for parameter", ea!(parameter = p))).err_external();
            };
            let Node::Value(serde_json::Value::String(v)) = v else {
                return Err(
                    loga::err_with(
                        "Parameter used in context requiring string but supplied value was not a string",
                        ea!(parameter = p, value = serde_json::to_string(&v).unwrap()),
                    ),
                ).err_external();
            };
            return Ok(v.clone());
        },
    }
}

fn build_value_json(
    query_state: &mut QueryBuildState,
    param: &Value,
) -> Result<serde_json::Value, VisErr<loga::Error>> {
    let param = match param {
        Value::Literal(r) => r,
        Value::Parameter(p) => query_state
            .parameters
            .get(p)
            .context_with("Missing value for parameter", ea!(parameter = p))
            .err_external()?,
    };
    return Ok(serde_json::to_value(param).unwrap());
}

fn build_split_value(
    query_state: &mut QueryBuildState,
    param: &Value,
) -> Result<(sea_query::Value, sea_query::FunctionCall), VisErr<loga::Error>> {
    let mut j =
        exenum!(build_value_json(query_state, param)?, serde_json:: Value:: Object(j) => j)
            .context("Expected json object, but got another json type")
            .err_internal()?;
    let type_ = exenum!(j.remove("t").unwrap(), serde_json:: Value:: String(type_) => type_).unwrap();
    let value =
        query_state
            .func_json_extract
            .clone()
            .arg(sea_query::Value::Json(Some(Box::new(j.remove("v").unwrap()))))
            .arg("$");
    return Ok((sea_query::Value::from(type_), value));
}

fn build_value(query_state: &mut QueryBuildState, param: &Value) -> Result<sea_query::Value, VisErr<loga::Error>> {
    return Ok(sea_query::Value::Json(Some(Box::new(build_value_json(query_state, param)?))));
}

// Produces (sequence of) CTEs from steps, returning the last CTE. CTE has start
// and end fields only.
fn build_subchain(
    query_state: &mut QueryBuildState,
    mut prev_subchain_seg: Option<BuildStepRes>,
    subchain: &ChainBody,
) -> Result<BuildStepRes, VisErr<loga::Error>> {
    if let Some(root) = &subchain.root {
        let new_root_seg;
        match root {
            ChainRoot::Value(root) => {
                if let Some(root_res) = query_state.reuse_roots.get(root) {
                    new_root_seg = root_res.clone();
                } else {
                    let ident_table_root = SeaRc::new(Alias::new(format!("root{}", query_state.global_unique)));
                    query_state.global_unique += 1;
                    let mut sql_cte = sea_query::CommonTableExpression::new();
                    sql_cte.table_name(ident_table_root.clone());
                    let mut sql_sel = sea_query::Query::select();

                    // Data
                    let root_expr = build_value(query_state, root)?;

                    // Output start
                    sql_cte.column(query_state.ident_col_start.clone());
                    sql_sel.expr(root_expr.clone());

                    // Output end
                    sql_cte.column(query_state.ident_col_end.clone());
                    sql_sel.expr(root_expr.clone());

                    // Output rowid
                    sql_cte.column(query_state.ident_rowid.clone());
                    sql_sel.expr_window(sql_fn("row_number", vec![]), WindowStatement::new());

                    // Assemble
                    sql_cte.query(sql_sel);
                    query_state.ctes.push(sql_cte);
                    let root_res = BuildStepRes {
                        ident_table: ident_table_root,
                        col_start: query_state.ident_col_start.clone(),
                        col_end: query_state.ident_col_end.clone(),
                        plural: false,
                    };
                    query_state.reuse_roots.insert(root.clone(), root_res.clone());
                    new_root_seg = root_res;
                }
            },
            ChainRoot::Search(root) => {
                let ident_table_root = SeaRc::new(Alias::new(format!("root{}", query_state.global_unique)));
                query_state.global_unique += 1;
                let mut sql_cte = sea_query::CommonTableExpression::new();
                sql_cte.table_name(ident_table_root.clone());
                sql_cte.query({
                    let ident_meta = SeaRc::new(Alias::new("meta"));
                    let ident_meta_fts = SeaRc::new(Alias::new("meta_fts"));
                    let ident_rowid = SeaRc::new(Alias::new("rowid"));
                    let ident_fulltext = SeaRc::new(Alias::new("fulltext"));
                    let ident_node = SeaRc::new(Alias::new("node"));
                    let mut sql_sel = sea_query::Query::select();
                    sql_sel.from(TableRef::Table(ident_meta.clone()));
                    let node_expr =
                        Expr::col(sea_query::ColumnRef::TableColumn(ident_meta.clone(), ident_node.clone()));
                    sql_sel.expr(node_expr.clone());
                    sql_sel.expr(node_expr.clone());
                    sql_sel.and_where(
                        Expr::col(ColumnRef::TableColumn(ident_meta.clone(), ident_rowid.clone())).in_subquery({
                            let mut sql_sel = sea_query::Query::select();
                            sql_sel.from(TableRef::Table(ident_meta_fts.clone()));
                            sql_sel.column(ident_rowid.clone());
                            let raw = build_value_str(query_state, &root)?;
                            let match_str;
                            if let Some(raw) = raw.strip_prefix("raw:") {
                                match_str = raw.to_string();
                            } else {
                                let mut match_terms = vec![];
                                let mut quoted = false;
                                let mut escaped = false;
                                let mut buf = vec![];
                                for c in raw.chars() {
                                    let mut flush = false;
                                    if escaped {
                                        escaped = false;
                                        buf.push(c);
                                    } else {
                                        match c {
                                            '\\' => {
                                                escaped = true;
                                            },
                                            '"' => {
                                                flush = true;
                                                quoted = !quoted;
                                            },
                                            ' ' | '\t' | '\n' | '\r' if !quoted => {
                                                flush = true;
                                            },
                                            c => {
                                                buf.push(c);
                                            },
                                        }
                                    }
                                    if flush && !buf.is_empty() {
                                        match_terms.push(
                                            format!(
                                                "\"{}\"",
                                                buf
                                                    .split_off(0)
                                                    .into_iter()
                                                    .collect::<String>()
                                                    .replace("\"", "\"\"")
                                            ),
                                        );
                                    }
                                }
                                match_str = match_terms.join(" AND ");
                            }
                            sql_sel.and_where(
                                Expr::col(
                                    ColumnRef::TableColumn(ident_meta_fts.clone(), ident_fulltext.clone()),
                                ).matches(match_str),
                            );
                            sql_sel
                        }),
                    );
                    sql_sel
                });
                sql_cte.column(query_state.ident_col_start.clone());
                sql_cte.column(query_state.ident_col_end.clone());
                query_state.ctes.push(sql_cte);
                let root_res = BuildStepRes {
                    ident_table: ident_table_root,
                    col_start: query_state.ident_col_start.clone(),
                    col_end: query_state.ident_col_end.clone(),
                    plural: false,
                };
                new_root_seg = root_res;
            },
        }
        prev_subchain_seg = Some(new_root_seg);
    }
    for step in &subchain.steps {
        prev_subchain_seg = Some(build_step(query_state, prev_subchain_seg, step)?);
    }
    if let Some(s) = prev_subchain_seg {
        return Ok(s);
    } else {
        let ident_table_root = SeaRc::new(Alias::new(format!("root{}", query_state.global_unique)));
        query_state.global_unique += 1;
        let mut sql_cte = sea_query::CommonTableExpression::new();
        sql_cte.table_name(ident_table_root.clone());
        let mut sql_sel = sea_query::Query::select();

        // Data
        let root_expr = sea_query::Value::Int(Some(0));

        // Output start
        sql_cte.column(query_state.ident_col_start.clone());
        sql_sel.expr(root_expr.clone());

        // Output end
        sql_cte.column(query_state.ident_col_end.clone());
        sql_sel.expr(root_expr.clone());

        // Output rowid
        sql_cte.column(query_state.ident_rowid.clone());
        sql_sel.expr_window(sql_fn("row_number", vec![]), WindowStatement::new());

        // Actually, output nothing
        sql_sel.and_where(
            sea_query::SimpleExpr::Binary(
                Box::new(sea_query::Value::Int(Some(0)).into()),
                sea_query::BinOper::Equal,
                Box::new(sea_query::Value::Int(Some(1)).into()),
            ),
        );

        // Assemble
        sql_cte.query(sql_sel);
        query_state.ctes.push(sql_cte);
        let root_res = BuildStepRes {
            ident_table: ident_table_root,
            col_start: query_state.ident_col_start.clone(),
            col_end: query_state.ident_col_end.clone(),
            plural: false,
        };
        return Ok(root_res);
    }
}

/// Produces CTE with `_` selects, no aggregation.
fn build_chain(
    query_state: &mut QueryBuildState,
    prev_subchain_seg: Option<BuildStepRes>,
    chain: Chain,
) -> Result<BuildChainRes, VisErr<loga::Error>> {
    let cte_name = format!("chain{}", query_state.global_unique);
    query_state.global_unique += 1;
    let mut sql_sel = sea_query::Query::select();
    let primary_subchain = build_subchain(query_state, prev_subchain_seg, &chain.body)?;
    sql_sel.from(sea_query::TableRef::Table(primary_subchain.ident_table.clone()));
    let global_col_primary_start =
        sea_query::ColumnRef::TableColumn(primary_subchain.ident_table.clone(), primary_subchain.col_start.clone());
    let global_col_primary_end =
        sea_query::ColumnRef::TableColumn(primary_subchain.ident_table.clone(), primary_subchain.col_end.clone());
    sql_sel.expr_as(global_col_primary_start.clone(), query_state.ident_col_start.clone());
    sql_sel.group_by_col(global_col_primary_start.clone());
    sql_sel.expr_as(global_col_primary_end.clone(), query_state.ident_col_end.clone());
    sql_sel.group_by_col(global_col_primary_end.clone());

    // Add dest as selection
    let mut selects = vec![];
    if let Some(name) = chain.bind {
        sql_sel.expr_as(global_col_primary_end.clone(), SeaRc::new(Alias::new(format!("_{}", name))));
        selects.push((name, false));
    }

    // Process children
    let child_prev_subchain_seg;
    {
        let ident_table_root = SeaRc::new(Alias::new(format!("chain_child_prev{}", query_state.global_unique)));
        query_state.global_unique += 1;
        let mut sql_sel = sea_query::Query::select();
        sql_sel.from(primary_subchain.ident_table.clone());
        sql_sel.column(primary_subchain.col_end.clone());
        sql_sel.column(primary_subchain.col_end.clone());
        let mut sql_cte = sea_query::CommonTableExpression::new();
        sql_cte.table_name(ident_table_root.clone());
        sql_cte.query(sql_sel);
        sql_cte.column(query_state.ident_col_start.clone());
        sql_cte.column(query_state.ident_col_end.clone());
        query_state.ctes.push(sql_cte);
        child_prev_subchain_seg = Some(BuildStepRes {
            ident_table: ident_table_root,
            col_start: query_state.ident_col_start.clone(),
            col_end: query_state.ident_col_end.clone(),
            plural: false,
        });
    }
    for child in chain.subchains {
        let child_chain = build_chain(query_state, child_prev_subchain_seg.clone(), child)?;
        sql_sel.join(
            sea_query::JoinType::LeftJoin,
            child_chain.cte,
            sea_query::Expr::col(
                global_col_primary_end.clone(),
            ).eq(
                sea_query::ColumnRef::TableColumn(child_chain.cte_name.clone(), query_state.ident_col_start.clone()),
            ),
        );
        for (name, plural) in child_chain.selects {
            let ident_name = SeaRc::new(Alias::new(format!("_{}", name)));
            sql_sel.expr_as(
                sea_query::ColumnRef::TableColumn(child_chain.cte_name.clone(), ident_name.clone()),
                ident_name,
            );
            selects.push((name, child_chain.plural || plural));
        }
    }

    // Assemble
    let mut sql_cte = sea_query::CommonTableExpression::new();
    let ident_table_cte = SeaRc::new(Alias::new(cte_name));
    sql_cte.table_name(ident_table_cte.clone());
    sql_cte.query(sql_sel);
    query_state.ctes.push(sql_cte);
    return Ok(BuildChainRes {
        cte_name: ident_table_cte.clone(),
        cte: sea_query::TableRef::Table(ident_table_cte),
        selects: selects,
        plural: primary_subchain.plural,
    });
}

const COL_PAGE_KEY: &str = "page_key";

pub fn build_root_chain(
    root_chain: Chain,
    parameters: HashMap<String, Node>,
) -> Result<(String, sea_query_rusqlite::RusqliteValues), VisErr<loga::Error>> {
    // Prep
    let mut query_state = QueryBuildState {
        parameters: parameters,
        ident_rowid: SeaRc::new(Alias::new("rowid")),
        ident_table_primary: SeaRc::new(Alias::new("primary")),
        ident_table_prev: SeaRc::new(Alias::new("prev")),
        ident_col_start: SeaRc::new(Alias::new("start")),
        ident_col_end: SeaRc::new(Alias::new("end")),
        ident_col_subject: SeaRc::new(Alias::new("subject")),
        ident_col_predicate: SeaRc::new(Alias::new("predicate")),
        ident_col_object: SeaRc::new(Alias::new("object")),
        triple_exist_table: SeaRc::new(Alias::new("triple_exist")),
        func_json_extract: sea_query::Func::cust(SeaRc::new(Alias::new("json_extract"))),
        global_unique: Default::default(),
        ctes: Default::default(),
        reuse_roots: Default::default(),
        reuse_steps: Default::default(),
    };
    {
        // Build a cte of only currently existing triples
        let ident_cte_triple_exist0 = SeaRc::new(Alias::new("triple_exist0"));
        let ident_col_exists = SeaRc::new(Alias::new("exists"));
        {
            let ident_col_commit = SeaRc::new(Alias::new("commit_"));
            let mut sql_cte = sea_query::CommonTableExpression::new();
            sql_cte.table_name(ident_cte_triple_exist0.clone());
            let mut sql_sel = sea_query::Query::select();
            let local_ident_table_primary = query_state.ident_table_primary.clone();
            sql_sel.from_as(
                sea_query::TableRef::Table(SeaRc::new(Alias::new("triple"))),
                local_ident_table_primary.clone(),
            );
            let col_sub =
                sea_query::ColumnRef::TableColumn(
                    local_ident_table_primary.clone(),
                    query_state.ident_col_subject.clone(),
                );
            let col_pred =
                sea_query::ColumnRef::TableColumn(
                    local_ident_table_primary.clone(),
                    query_state.ident_col_predicate.clone(),
                );
            let col_obj =
                sea_query::ColumnRef::TableColumn(
                    local_ident_table_primary.clone(),
                    query_state.ident_col_object.clone(),
                );

            // Subj
            sql_cte.column(query_state.ident_col_subject.clone());
            sql_sel.column(col_sub.clone());

            // Pred
            sql_cte.column(query_state.ident_col_predicate.clone());
            sql_sel.column(col_pred.clone());

            // Obj
            sql_cte.column(query_state.ident_col_object.clone());
            sql_sel.column(col_obj.clone());

            // Exists
            sql_cte.column(ident_col_exists.clone());
            sql_sel.column(
                sea_query::ColumnRef::TableColumn(local_ident_table_primary.clone(), ident_col_exists.clone()),
            );

            // Take last commit
            sql_cte.column(SeaRc::new(Alias::new("_unused_timestamp")));
            sql_sel.expr(
                sea_query::Expr::max(
                    sea_query::Expr::col(
                        sea_query::ColumnRef::TableColumn(local_ident_table_primary.clone(), ident_col_commit.clone()),
                    ),
                ),
            );
            sql_sel.group_by_col(col_sub);
            sql_sel.group_by_col(col_pred);
            sql_sel.group_by_col(col_obj);

            // Assemble
            sql_cte.query(sql_sel);
            query_state.ctes.push(sql_cte);
        }
        {
            let ident_cte_triple_exist = query_state.triple_exist_table.clone();
            let mut sql_cte = sea_query::CommonTableExpression::new();
            sql_cte.table_name(ident_cte_triple_exist.clone());
            let mut sql_sel = sea_query::Query::select();
            let local_ident_table_primary = query_state.ident_table_primary.clone();
            sql_sel.from_as(sea_query::TableRef::Table(ident_cte_triple_exist0), local_ident_table_primary.clone());

            // Subj
            sql_cte.column(query_state.ident_col_subject.clone());
            sql_sel.column(
                sea_query::ColumnRef::TableColumn(
                    local_ident_table_primary.clone(),
                    query_state.ident_col_subject.clone(),
                ),
            );

            // Pred
            sql_cte.column(query_state.ident_col_predicate.clone());
            sql_sel.column(
                sea_query::ColumnRef::TableColumn(
                    local_ident_table_primary.clone(),
                    query_state.ident_col_predicate.clone(),
                ),
            );

            // Obj
            sql_cte.column(query_state.ident_col_object.clone());
            sql_sel.column(
                sea_query::ColumnRef::TableColumn(
                    local_ident_table_primary.clone(),
                    query_state.ident_col_object.clone(),
                ),
            );

            // Filter
            sql_sel.and_where(
                sea_query::Expr::col(
                    sea_query::ColumnRef::TableColumn(local_ident_table_primary.clone(), ident_col_exists.clone()),
                ).into(),
            );

            // Assemble
            sql_cte.query(sql_sel);
            query_state.ctes.push(sql_cte);
        }
    }

    // Build actual query now
    let cte = build_chain(&mut query_state, None, root_chain)?;
    let mut sel_root = sea_query::Query::select();
    sel_root.from(cte.cte);
    sel_root.expr_as(
        sea_query::ColumnRef::TableColumn(cte.cte_name.clone(), query_state.ident_col_end.clone()),
        SeaRc::new(Alias::new(COL_PAGE_KEY.to_string())),
    );
    sel_root.group_by_col(
        // Prevent 0-rec results from turning into 1-rec results owing to sql's vast
        // orthogonality and aggregate function myseries
        sea_query::ColumnRef::TableColumn(cte.cte_name.clone(), query_state.ident_col_end.clone()),
    );
    for (name, plural) in cte.selects {
        let user_name = SeaRc::new(Alias::new(format!("_{}", name)));
        let expr = sql_fn("json_object", vec![
            //. .
            Expr::value("scalar"),
            query_state.func_json_extract.clone().arg(sql_fn("ifnull", vec![
                //. .
                SimpleExpr::from(sea_query::ColumnRef::TableColumn(cte.cte_name.clone(), user_name.clone())),
                sql_fn(
                    "json_object",
                    vec![
                        Expr::value("t"),
                        Expr::value("v"),
                        Expr::value("v"),
                        Expr::value(<String as Nullable>::null())
                    ],
                )
            ])).arg("$").into()
        ]);
        let ident_name = user_name;
        if plural {
            sel_root.expr_as(sql_fn("json_object", vec![
                //. .
                Expr::value("array"),
                sql_fn("json_group_array", vec![expr])
            ]), ident_name);
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
    return Ok(sel.build_rusqlite(sea_query::SqliteQueryBuilder));
}

pub struct Row {
    pub value: BTreeMap<String, TreeNode>,
    pub pagination_key: Node,
}

pub fn execute_sql_query(
    db: &rusqlite::Connection,
    sql_query: String,
    sql_parameters: sea_query_rusqlite::RusqliteValues,
    sort: Option<QuerySort>,
    paginate: Option<Pagination>,
) -> Result<Vec<Row>, loga::Error> {
    let mut s = db.prepare(&sql_query)?;
    let column_names = s.column_names().into_iter().map(|k| k.to_string()).collect::<Vec<_>>();
    let mut sql_rows = s.query(&*sql_parameters.as_params()).context("Error executing query")?;
    let mut out = vec![];
    loop {
        let Some(got_row) = sql_rows.next()? else {
            break;
        };
        let mut got_row1 = BTreeMap::new();
        let mut pagination_key = None;
        for (i, name) in column_names.iter().enumerate() {
            if name.as_str() == COL_PAGE_KEY {
                pagination_key =
                    Some(
                        serde_json::from_str::<Node>(
                            &got_row.get::<usize, Option<String>>(i).unwrap().unwrap(),
                        ).unwrap(),
                    );
            } else if let Some(name) = name.strip_prefix("_") {
                got_row1.insert(
                    name.to_string(),
                    serde_json::from_str::<TreeNode>(
                        &got_row.get::<usize, Option<String>>(i).unwrap().unwrap(),
                    ).unwrap(),
                );
            } else {
                panic!("Unrecognized returned field {}", name);
            }
        }
        out.push(Row {
            value: got_row1,
            pagination_key: pagination_key.unwrap(),
        });
    }
    if let Some(sort) = sort {
        match sort {
            QuerySort::Random => {
                let seed;
                superif!({
                    let Some(pagination) = &paginate else {
                        break 'noseed;
                    };
                    let Some(seed1) = pagination.seed else {
                        break 'noseed;
                    };
                    seed = seed1;
                } 'noseed {
                    seed = thread_rng().gen();
                });
                out.shuffle(&mut rand_chacha::ChaChaRng::seed_from_u64(seed));
            },
            QuerySort::Fields(sort) => {
                out.sort_unstable_by(|a, b| {
                    for (dir, key) in &sort {
                        let res = a.value.get(key).partial_cmp(&b.value.get(key)).unwrap_or(Ordering::Equal);
                        let rev = *dir == QuerySortDir::Desc;
                        match res {
                            Ordering::Less => if rev {
                                return Ordering::Greater;
                            } else {
                                return Ordering::Less;
                            },
                            Ordering::Equal => { },
                            Ordering::Greater => if rev {
                                return Ordering::Less;
                            } else {
                                return Ordering::Greater;
                            },
                        }
                    }
                    return Ordering::Equal;
                });
            },
        }
    }
    if let Some(p) = paginate {
        if let Some(after) = p.key {
            return Ok(out.into_iter().skip_while(|x| x.pagination_key != after).skip(1).take(p.count).collect());
        } else {
            return Ok(out.into_iter().take(p.count).collect());
        }
    } else {
        return Ok(out.into_iter().collect());
    }
}

pub async fn execute_query(
    db: &Pool,
    query: Query,
    parameters: HashMap<String, Node>,
    paginate: Option<Pagination>,
) -> Result<Vec<Row>, VisErr<loga::Error>> {
    // Sorting currently happens in rust because sql does string sorting on json
    // fields, not value-based sorting (ex: numbers). Therefore pagination also has to
    // happen in rust.
    let (sql_query, sql_parameters) = build_root_chain(query.chain, parameters)?;
    return Ok(tx(&db, move |txn| {
        return Ok(execute_sql_query(txn, sql_query, sql_parameters, query.sort, paginate)?);
    }).await.err_internal()?);
}
