use {
    crate::interface::{
        query::{
            Chain,
            FilterExpr,
            FilterExprComparisonType,
            JunctionType,
            MoveDirection,
            Query,
            Step,
            Subchain,
            Value,
        },
        triple::Node,
    },
    loga::{
        ea,
        ResultContext,
    },
    sea_query::{
        Alias,
        ColumnRef,
        ExprTrait,
        SeaRc,
    },
    sea_query_rusqlite::RusqliteBinder,
    std::collections::HashMap,
};

struct QueryBuildState {
    parameters: HashMap<String, Node>,
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
    triple_table: sea_query::TableRef,
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
) -> Result<sea_query::SimpleExpr, loga::Error> {
    match expr {
        FilterExpr::Comparison(expr) => {
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
            let (expr_type, expr_value) = build_split_value(query_state, &expr.value)?;
            sql_sel.and_where(primary_type.eq(expr_type));
            sql_sel.and_where(match expr.operator {
                crate::interface::query::FilterChainComparisonOperator::Eq => primary_value.eq(expr_value),
                crate::interface::query::FilterChainComparisonOperator::Lt => primary_value.lt(expr_value),
                crate::interface::query::FilterChainComparisonOperator::Gt => primary_value.gt(expr_value),
                crate::interface::query::FilterChainComparisonOperator::Lte => primary_value.lte(expr_value),
                crate::interface::query::FilterChainComparisonOperator::Gte => primary_value.gte(expr_value),
            });
            let sql_expr = sea_query::Expr::exists(sql_sel);
            match expr.type_ {
                FilterExprComparisonType::Exists => {
                    return Ok(sql_expr);
                },
                FilterExprComparisonType::DoesntExist => {
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
) -> Result<BuildStepRes, loga::Error> {
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
                    sea_query::ColumnRef::TableColumn(
                        local_ident_table_primary.clone(),
                        from_ident_primary_start.clone(),
                    );
                let local_col_primary_end =
                    sea_query::ColumnRef::TableColumn(
                        local_ident_table_primary.clone(),
                        from_ident_primary_end.clone(),
                    );

                // Only get latest event
                sql_sel.group_by_col(local_col_primary_start.clone());
                sql_sel.group_by_col(
                    sea_query::ColumnRef::TableColumn(
                        local_ident_table_primary.clone(),
                        query_state.ident_col_predicate.clone(),
                    ),
                );
                sql_sel.group_by_col(local_col_primary_end.clone());
                sql_sel.order_by(
                    sea_query::ColumnRef::TableColumn(
                        local_ident_table_primary.clone(),
                        query_state.ident_col_timestamp.clone(),
                    ),
                    sea_query::Order::Desc,
                );

                // Only consider elements with perm to view
                { }

                // Movement
                sql_sel.and_where(
                    sea_query::Expr::col(
                        sea_query::ColumnRef::TableColumn(
                            local_ident_table_primary.clone(),
                            query_state.ident_col_predicate.clone(),
                        ),
                    ).eq(step.predicate.clone()),
                );

                // Subset of previous results
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

                // Trim
                if step.first && step.filter.is_none() {
                    // (If filtering, apply limit during that step)
                    sql_sel.limit(1);
                }

                // Assemble
                sql_sel.column(out_col_start);
                sql_sel.column(local_col_primary_end.clone());
                sql_sel.column(
                    sea_query::ColumnRef::TableColumn(
                        local_ident_table_primary.clone(),
                        query_state.ident_col_exists.clone(),
                    ),
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

            // Exclude deleted records
            {
                let ident_cte = SeaRc::new(Alias::new(format!("{}_b", seg_name)));
                let mut sql_sel = sea_query::Query::select();
                let local_ident_table_primary = query_state.ident_table_primary.clone();
                sql_sel.from_as(out.ident_table.clone(), local_ident_table_primary.clone());
                sql_sel.column(sea_query::ColumnRef::TableColumn(local_ident_table_primary.clone(), out.col_start));
                sql_sel.column(sea_query::ColumnRef::TableColumn(local_ident_table_primary.clone(), out.col_end));
                sql_sel.and_where(
                    sea_query::Expr::col(
                        sea_query::ColumnRef::TableColumn(
                            local_ident_table_primary.clone(),
                            query_state.ident_col_exists.clone(),
                        ),
                    ).into(),
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

            // Filter + limit
            if let Some(filter) = &step.filter {
                let ident_cte = SeaRc::new(Alias::new(format!("{}_c", seg_name)));
                let mut sql_sel = sea_query::Query::select();
                let local_ident_table_primary = query_state.ident_table_primary.clone();
                sql_sel.from_as(out.ident_table.clone(), local_ident_table_primary.clone());
                sql_sel.column(
                    sea_query::ColumnRef::TableColumn(local_ident_table_primary.clone(), out.col_start.clone()),
                );
                let col_end =
                    sea_query::ColumnRef::TableColumn(local_ident_table_primary.clone(), out.col_end.clone());
                sql_sel.column(col_end.clone());
                sql_sel.and_where(build_filter(query_state, &col_end, BuildStepRes {
                    ident_table: out.ident_table,
                    col_start: out.col_end.clone(),
                    col_end: out.col_end,
                    plural: out.plural,
                }, filter)?);
                if step.first {
                    sql_sel.limit(1);
                }

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
        },
        Step::Recurse(step) => {
            let seg_name = format!("seg{}_recurse", query_state.global_unique);
            query_state.global_unique += 1;
            {
                let previous = previous.as_ref().unwrap();
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
            let mut build_subchain = |subchain: &Subchain| -> Result<sea_query::SelectStatement, loga::Error> {
                let mut sql_sel = sea_query::Query::select();
                let subchain = build_subchain(query_state, previous.clone(), subchain)?;
                sql_sel.from(sea_query::TableRef::Table(subchain.ident_table.clone()));
                sql_sel.column(sea_query::ColumnRef::TableColumn(subchain.ident_table.clone(), subchain.col_start));
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

fn build_value_json(query_state: &mut QueryBuildState, param: &Value) -> Result<serde_json::Value, loga::Error> {
    let param = match param {
        Value::Literal(r) => r,
        Value::Parameter(p) => query_state
            .parameters
            .get(p)
            .context_with("Missing value for parameter", ea!(parameter = p))?,
    };
    return Ok(serde_json::to_value(param).unwrap());
}

fn build_split_value(
    query_state: &mut QueryBuildState,
    param: &Value,
) -> Result<(sea_query::Value, sea_query::FunctionCall), loga::Error> {
    let serde_json::Value::Object(mut j) = build_value_json(query_state, param)? else {
        unreachable!()
    };
    let serde_json::Value::String(type_) = j.remove("t").unwrap() else {
        unreachable!();
    };
    let value =
        query_state
            .func_json_extract
            .clone()
            .arg(sea_query::Value::Json(Some(Box::new(j.remove("v").unwrap()))))
            .arg("$");
    return Ok((sea_query::Value::from(type_), value));
}

fn build_value(query_state: &mut QueryBuildState, param: &Value) -> Result<sea_query::Value, loga::Error> {
    return Ok(sea_query::Value::Json(Some(Box::new(build_value_json(query_state, param)?))));
}

// Produces (sequence of) CTEs from steps, returning the last CTE. CTE has start
// and end fields only.
fn build_subchain(
    query_state: &mut QueryBuildState,
    mut prev_subchain_seg: Option<BuildStepRes>,
    subchain: &Subchain,
) -> Result<BuildStepRes, loga::Error> {
    if let Some(root) = &subchain.root {
        if let Some(root_res) = query_state.reuse_roots.get(root) {
            prev_subchain_seg = Some(root_res.clone());
        } else {
            let ident_table_root = SeaRc::new(Alias::new(format!("root{}", query_state.global_unique)));
            query_state.global_unique += 1;
            let mut sql_sel = sea_query::Query::select();
            let root_expr = build_value(query_state, root)?;
            sql_sel.expr(root_expr.clone());
            sql_sel.expr(root_expr.clone());
            let mut sql_cte = sea_query::CommonTableExpression::new();
            sql_cte.table_name(ident_table_root.clone());
            sql_cte.query(sql_sel);
            sql_cte.column(query_state.ident_col_start.clone());
            sql_cte.column(query_state.ident_col_end.clone());
            query_state.ctes.push(sql_cte);
            let root_res = BuildStepRes {
                ident_table: ident_table_root,
                col_start: query_state.ident_col_start.clone(),
                col_end: query_state.ident_col_end.clone(),
                plural: false,
            };
            query_state.reuse_roots.insert(root.clone(), root_res.clone());
            prev_subchain_seg = Some(root_res);
        }
    }
    let mut prev_subchain_seg = build_step(query_state, prev_subchain_seg, &subchain.steps[0])?;
    for step in &subchain.steps[1..] {
        prev_subchain_seg = build_step(query_state, Some(prev_subchain_seg), step)?;
    }
    return Ok(prev_subchain_seg);
}

/// Produces CTE with `_` selects, no aggregation.
fn build_chain(
    query_state: &mut QueryBuildState,
    prev_subchain_seg: Option<BuildStepRes>,
    chain: Chain,
) -> Result<BuildChainRes, loga::Error> {
    let cte_name = format!("chain{}", query_state.global_unique);
    query_state.global_unique += 1;
    let mut sql_sel = sea_query::Query::select();
    let primary_subchain = build_subchain(query_state, prev_subchain_seg, &chain.subchain)?;
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
    if let Some(name) = chain.select {
        sql_sel.expr_as(global_col_primary_end.clone(), SeaRc::new(Alias::new(format!("_{}", name))));
        selects.push((name, false));
    }

    // Process children
    for child in chain.children {
        let child_chain = build_chain(query_state, Some(primary_subchain.clone()), child)?;
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

pub fn build_query(
    q: Query,
    parameters: HashMap<String, Node>,
) -> Result<(String, sea_query_rusqlite::RusqliteValues), loga::Error> {
    let mut query_state = QueryBuildState {
        parameters: parameters,
        ident_table_primary: SeaRc::new(Alias::new("primary")),
        ident_table_prev: SeaRc::new(Alias::new("prev")),
        ident_col_start: SeaRc::new(Alias::new("start")),
        ident_col_end: SeaRc::new(Alias::new("end")),
        ident_col_subject: SeaRc::new(Alias::new("subject")),
        ident_col_predicate: SeaRc::new(Alias::new("predicate")),
        ident_col_object: SeaRc::new(Alias::new("object")),
        ident_col_timestamp: SeaRc::new(Alias::new("timestamp")),
        ident_col_exists: SeaRc::new(Alias::new("exists")),
        triple_table: sea_query::TableRef::Table(SeaRc::new(Alias::new("triple"))),
        func_json_extract: sea_query::Func::cust(SeaRc::new(Alias::new("json_extract"))),
        global_unique: Default::default(),
        ctes: Default::default(),
        reuse_roots: Default::default(),
        reuse_steps: Default::default(),
    };
    let cte = build_chain(&mut query_state, None, q.chain)?;
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
    return Ok(sel.build_rusqlite(sea_query::SqliteQueryBuilder));
}
