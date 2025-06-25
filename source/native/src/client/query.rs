use {
    loga::{
        ea,
        DebugDisplay,
        ResultContext,
    },
    query_parser_actions::{
        ROOT,
        STEP,
    },
    shared::interface::{
        query::{
            Chain,
            ChainBody,
            ChainRoot,
            FilterExpr,
            FilterExprExistance,
            FilterExprExistsType,
            FilterExprJunction,
            FilterSuffix,
            FilterSuffixLike,
            FilterSuffixSimple,
            FilterSuffixSimpleOperator,
            JunctionType,
            MoveDirection,
            Query,
            QuerySort,
            QuerySortDir,
            Step,
            StepJunction,
            StepMove,
            StepRecurse,
            StrValue,
            Value,
        },
        triple::Node,
    },
    std::{
        fs::read_to_string,
        path::Path,
    },
};

mod query_parser {
    #![allow(warnings)]

    include!(concat!(env!("OUT_DIR"), "/src/client/query_parser.rs"));
}

mod query_parser_actions {
    #![allow(warnings)]

    include!(concat!(env!("OUT_DIR"), "/src/client/query_parser_actions.rs"));
}

fn unquote_string(s: impl AsRef<str>) -> String {
    return serde_json::from_str::<String>(s.as_ref()).unwrap();
}

fn compile_str_value(value: query_parser_actions::STR_PARAM_VAL) -> StrValue {
    match value {
        query_parser_actions::STR_PARAM_VAL::str_(v) => return StrValue::Literal(unquote_string(v)),
        query_parser_actions::STR_PARAM_VAL::param(v) => return StrValue::Parameter(
            v.strip_prefix("$").unwrap().to_string(),
        ),
    }
}

fn compile_value(value: query_parser_actions::VAL) -> Result<Value, loga::Error> {
    match value {
        query_parser_actions::VAL::str_(v) => {
            return Ok(Value::Literal(Node::Value(serde_json::from_str(&v).unwrap())));
        },
        query_parser_actions::VAL::num(v) => {
            return Ok(Value::Literal(Node::Value(serde_json::from_str(&v).unwrap())));
        },
        query_parser_actions::VAL::true_ => {
            return Ok(Value::Literal(Node::Value(serde_json::Value::Bool(true))));
        },
        query_parser_actions::VAL::false_ => {
            return Ok(Value::Literal(Node::Value(serde_json::Value::Bool(false))));
        },
        query_parser_actions::VAL::null => {
            return Ok(Value::Literal(Node::Value(serde_json::Value::Null)));
        },
        query_parser_actions::VAL::json(v) => {
            // Remove the quotes
            let mut v = &v[1..];
            loop {
                if v.len() < 2 {
                    break;
                }
                if v.as_bytes()[0] != b'"' {
                    break;
                }
                v = &v[1 .. v.len() - 1];
            }
            return Ok(
                Value::Literal(Node::Value(serde_json::from_str(&v).context("Invalid JSON in value literal")?)),
            );
        },
        query_parser_actions::VAL::param(mut v) => {
            return Ok(Value::Parameter(v.split_off(1)));
        },
    }
}

fn compile_filter_suffix(suffix: query_parser_actions::FILTER_SUFFIX) -> Result<FilterSuffix, loga::Error> {
    match suffix {
        query_parser_actions::FILTER_SUFFIX::FILTER_SUFFIX_SIMPLE(suffix) => {
            return Ok(FilterSuffix::Simple(FilterSuffixSimple {
                op: match suffix.filter_op {
                    query_parser_actions::FILTER_OP::sym_op_eq => FilterSuffixSimpleOperator::Eq,
                    query_parser_actions::FILTER_OP::sym_op_neq => FilterSuffixSimpleOperator::Neq,
                    query_parser_actions::FILTER_OP::sym_op_gt => FilterSuffixSimpleOperator::Gt,
                    query_parser_actions::FILTER_OP::sym_op_gte => FilterSuffixSimpleOperator::Gte,
                    query_parser_actions::FILTER_OP::sym_op_lt => FilterSuffixSimpleOperator::Lt,
                    query_parser_actions::FILTER_OP::sym_op_lte => FilterSuffixSimpleOperator::Lte,
                },
                value: compile_value(suffix.val)?,
            }));
        },
        query_parser_actions::FILTER_SUFFIX::FILTER_SUFFIX_LIKE(suffix) => {
            return Ok(FilterSuffix::Like(FilterSuffixLike { value: compile_str_value(suffix) }));
        },
    }
}

fn compile_filter(filter: query_parser_actions::FILTER) -> Result<FilterExpr, loga::Error> {
    match filter {
        query_parser_actions::FILTER::FILTER_EXISTS(f) => {
            return Ok(FilterExpr::Exists(FilterExprExistance {
                type_: FilterExprExistsType::Exists,
                subchain: compile_chain_body(f.chain_body.rootopt, f.chain_body.step0.unwrap_or_default())?,
                suffix: if let Some(parsed_suffix) = f.filter_suffixopt {
                    Some(compile_filter_suffix(parsed_suffix)?)
                } else {
                    None
                },
            }));
        },
        query_parser_actions::FILTER::FILTER_NOT_EXISTS(f) => {
            return Ok(FilterExpr::Exists(FilterExprExistance {
                type_: FilterExprExistsType::DoesntExist,
                subchain: compile_chain_body(f.chain_body.rootopt, f.chain_body.step0.unwrap_or_default())?,
                suffix: if let Some(parsed_suffix) = f.filter_suffixopt {
                    Some(compile_filter_suffix(parsed_suffix)?)
                } else {
                    None
                },
            }));
        },
        query_parser_actions::FILTER::FILTER_JUNCT_AND(f) => {
            let mut subexprs = vec![];
            for parsed_subexpr in f {
                subexprs.push(compile_filter(*parsed_subexpr)?);
            }
            return Ok(FilterExpr::Junction(FilterExprJunction {
                type_: JunctionType::And,
                subexprs: subexprs,
            }));
        },
        query_parser_actions::FILTER::FILTER_JUNCT_OR(f) => {
            let mut subexprs = vec![];
            for parsed_subexpr in f {
                subexprs.push(compile_filter(*parsed_subexpr)?);
            }
            return Ok(FilterExpr::Junction(FilterExprJunction {
                type_: JunctionType::Or,
                subexprs: subexprs,
            }));
        },
    }
}

fn compile_chain_body(body_root: Option<ROOT>, body_steps: Vec<STEP>) -> Result<ChainBody, loga::Error> {
    let root;
    match body_root {
        Some(parsed_root) => match parsed_root {
            query_parser_actions::ROOT::VAL(v) => {
                root = Some(ChainRoot::Value(compile_value(v)?));
            },
            query_parser_actions::ROOT::ROOT_SEARCH(s) => {
                root = Some(ChainRoot::Search(compile_str_value(s)));
            },
        },
        None => {
            root = None;
        },
    }
    let mut steps = vec![];
    for step in body_steps {
        match step {
            query_parser_actions::STEP::STEP_MOVE_UP(step) => {
                let filter;
                if let Some(parsed_filter) = step.filteropt {
                    filter = Some(compile_filter(parsed_filter)?);
                } else {
                    filter = None;
                }
                steps.push(Step::Move(StepMove {
                    dir: MoveDirection::Backward,
                    predicate: compile_str_value(step.str_param_val),
                    filter: filter,
                    first: step.firstopt.is_some(),
                }));
            },
            query_parser_actions::STEP::STEP_MOVE_DOWN(step) => {
                let filter;
                if let Some(parsed_filter) = step.filteropt {
                    filter = Some(compile_filter(parsed_filter)?);
                } else {
                    filter = None;
                }
                steps.push(Step::Move(StepMove {
                    dir: MoveDirection::Forward,
                    predicate: compile_str_value(step.str_param_val),
                    filter: filter,
                    first: step.firstopt.is_some(),
                }));
            },
            query_parser_actions::STEP::STEP_RECURSE(step) => {
                steps.push(Step::Recurse(StepRecurse {
                    subchain: compile_chain_body(step.chain_body.rootopt, step.chain_body.step0.unwrap_or_default())?,
                    first: step.firstopt.is_some(),
                }));
            },
            query_parser_actions::STEP::STEP_JUNCT_AND(step) => {
                let mut subchains = vec![];
                for parsed_subchain in step {
                    subchains.push(
                        compile_chain_body(parsed_subchain.rootopt, parsed_subchain.step0.unwrap_or_default())?,
                    );
                }
                steps.push(Step::Junction(StepJunction {
                    type_: JunctionType::And,
                    subchains: subchains,
                }));
            },
            query_parser_actions::STEP::STEP_JUNCT_OR(step) => {
                let mut subchains = vec![];
                for parsed_subchain in step {
                    subchains.push(
                        compile_chain_body(parsed_subchain.rootopt, parsed_subchain.step0.unwrap_or_default())?,
                    );
                }
                steps.push(Step::Junction(StepJunction {
                    type_: JunctionType::Or,
                    subchains: subchains,
                }));
            },
        }
    }
    return Ok(ChainBody {
        root: root,
        steps: steps,
    })
}

fn compile_chain(query_dir: Option<&Path>, chain: query_parser_actions::CHAIN) -> Result<Chain, loga::Error> {
    let mut bind_current = None;
    let mut children = vec![];
    let body_root = chain.chain_body.rootopt;
    let mut body_steps = chain.chain_body.step0.unwrap_or_default();
    let tail_bind;
    let mut at_tail = chain.chain_tail;
    loop {
        match at_tail {
            query_parser_actions::CHAIN_TAIL::CHAIN_TAIL_BIND(tail) => {
                tail_bind = tail.unwrap_or_default();
                break;
            },
            query_parser_actions::CHAIN_TAIL::CHAIN_TAIL_INCLUDE(tail) => {
                let tail = unquote_string(tail);
                let Some(query_dir) = query_dir else {
                    return Err(
                        loga::err_with(
                            "Query has include but query filesystem path not provided, cannot resolve",
                            ea!(include = tail),
                        ),
                    );
                };
                let built_path = query_dir.join(&tail);
                let include_query =
                    read_to_string(
                        &built_path,
                    ).context_with(
                        "Error reading include query",
                        ea!(
                            context_path = query_dir.dbg_str(),
                            include_path = tail,
                            combined_path = built_path.dbg_str()
                        ),
                    )?;
                let display_path = built_path.dbg_str();
                let parse =
                    rustemo::Parser::parse(&query_parser::QueryParserParser::new(), &include_query)
                        .map_err(loga::err)
                        .context_with("Error parsing included query", ea!(path = display_path))?;
                if parse.chain.chain_body.rootopt.is_some() {
                    return Err(
                        loga::err_with("Included query has root, cannot be used as suffix", ea!(path = display_path)),
                    );
                }
                body_steps.extend(parse.chain.chain_body.step0.unwrap_or_default());
                at_tail = parse.chain.chain_tail;
            },
        }
    }
    for action in tail_bind {
        match action {
            query_parser_actions::CHAIN_BIND::CHAIN_BIND_CURRENT(action) => {
                if bind_current.is_some() {
                    return Err(loga::err("You can only assign one name for a chain (select)"));
                }
                bind_current = Some(action);
            },
            query_parser_actions::CHAIN_BIND::CHAIN_BIND_SUBCHAIN(action) => {
                children.push(compile_chain(query_dir, *action)?);
            },
        }
    }
    return Ok(Chain {
        bind: bind_current,
        body: compile_chain_body(body_root, body_steps)?,
        subchains: children,
    });
}

pub fn compile_query(query_dir: Option<&Path>, query: &str) -> Result<Query, loga::Error> {
    let parse =
        rustemo::Parser::parse(&query_parser::QueryParserParser::new(), query)
            .map_err(loga::err)
            .context("Error parsing query")?;
    return Ok(Query {
        chain: compile_chain(query_dir, parse.chain)?,
        sort: match parse.sortopt {
            Some(sort) => match sort {
                query_parser_actions::SORT::SORT_PAIRS(sort) => {
                    Some(QuerySort::Fields(sort.into_iter().map(|x| match x {
                        query_parser_actions::SORT_PAIR::SORT_PAIR_ASC(x) => (QuerySortDir::Asc, x),
                        query_parser_actions::SORT_PAIR::SORT_PAIR_DESC(x) => (QuerySortDir::Desc, x),
                    }).collect()))
                },
                query_parser_actions::SORT::kw_sort_random => {
                    Some(QuerySort::Random)
                },
            },
            None => None,
        },
    });
}
