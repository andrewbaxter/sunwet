use {
    super::interface::{
        query::{
            Chain,
            ChainHead,
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
            SortDir,
            SortQuery,
            Step,
            StepJunction,
            StepMove,
            StepRecurse,
            StrValue,
            Value,
        },
        triple::Node,
    },
    crate::interface::query::{
        ChainTail,
        StepSpecific,
    },
    query_parser_actions::{
        ROOT,
        STEP,
    },
};

mod query_parser {
    #![allow(warnings)]

    include!(concat!(env!("OUT_DIR"), "/src/query_parser.rs"));
}

mod query_parser_actions {
    #![allow(warnings)]

    include!(concat!(env!("OUT_DIR"), "/src/query_parser_actions.rs"));
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

fn compile_value(value: query_parser_actions::VAL) -> Result<Value, String> {
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
                Value::Literal(
                    Node::Value(
                        serde_json::from_str(&v).map_err(|e| format!("Invalid JSON in value literal: {}", e))?,
                    ),
                ),
            );
        },
        query_parser_actions::VAL::param(mut v) => {
            return Ok(Value::Parameter(v.split_off(1)));
        },
    }
}

fn compile_filter_suffix(suffix: query_parser_actions::FILTER_SUFFIX) -> Result<FilterSuffix, String> {
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

fn compile_filter(filter: query_parser_actions::FILTER) -> Result<FilterExpr, String> {
    match filter {
        query_parser_actions::FILTER::FILTER_EXISTS(f) => {
            return Ok(FilterExpr::Exists(FilterExprExistance {
                type_: FilterExprExistsType::Exists,
                subchain: compile_chain_body(None, f.step0.unwrap_or_default())?,
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
                subchain: compile_chain_body(None, f.step0.unwrap_or_default())?,
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

fn compile_chain_body(body_root: Option<ROOT>, body_steps: Vec<STEP>) -> Result<ChainHead, String> {
    if body_root.is_none() && body_steps.is_empty() {
        return Err(format!("Query must have root or at least one step"));
    }
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
        let specific;
        match step.step_specific {
            query_parser_actions::STEP_SPECIFIC::STEP_MOVE_UP(step) => {
                let filter;
                if let Some(parsed_filter) = step.filteropt {
                    filter = Some(compile_filter(parsed_filter)?);
                } else {
                    filter = None;
                }
                specific = StepSpecific::Move(StepMove {
                    dir: MoveDirection::Backward,
                    predicate: compile_str_value(step.str_param_val),
                    filter: filter,
                });
            },
            query_parser_actions::STEP_SPECIFIC::STEP_MOVE_DOWN(step) => {
                let filter;
                if let Some(parsed_filter) = step.filteropt {
                    filter = Some(compile_filter(parsed_filter)?);
                } else {
                    filter = None;
                }
                specific = StepSpecific::Move(StepMove {
                    dir: MoveDirection::Forward,
                    predicate: compile_str_value(step.str_param_val),
                    filter: filter,
                });
            },
            query_parser_actions::STEP_SPECIFIC::STEP_RECURSE(step) => {
                specific =
                    StepSpecific::Recurse(
                        StepRecurse { subchain: compile_chain_body(None, step.unwrap_or_default())? },
                    );
            },
            query_parser_actions::STEP_SPECIFIC::STEP_JUNCT_AND(step) => {
                let mut subchains = vec![];
                for parsed_subchain in step {
                    subchains.push(
                        compile_chain_body(parsed_subchain.rootopt, parsed_subchain.step0.unwrap_or_default())?,
                    );
                }
                specific = StepSpecific::Junction(StepJunction {
                    type_: JunctionType::And,
                    subchains: subchains,
                });
            },
            query_parser_actions::STEP_SPECIFIC::STEP_JUNCT_OR(step) => {
                let mut subchains = vec![];
                for parsed_subchain in step {
                    subchains.push(
                        compile_chain_body(parsed_subchain.rootopt, parsed_subchain.step0.unwrap_or_default())?,
                    );
                }
                specific = StepSpecific::Junction(StepJunction {
                    type_: JunctionType::Or,
                    subchains: subchains,
                });
            },
        }
        steps.push(Step {
            specific: specific,
            sort: match step.sort_stepopt {
                Some(sort) => Some(match sort {
                    query_parser_actions::SORT_STEP::SORT_STEP_ASC(_) => SortDir::Asc,
                    query_parser_actions::SORT_STEP::SORT_STEP_DESC(_) => SortDir::Desc,
                }),
                None => None,
            },
            first: step.firstopt.is_some(),
        });
    }
    return Ok(ChainHead {
        root: root,
        steps: steps,
    })
}

fn compile_chain(chain: query_parser_actions::CHAIN) -> Result<Chain, String> {
    let mut bind_current = None;
    let mut children = vec![];
    for action in chain.chain_tail.unwrap_or_default() {
        match action {
            query_parser_actions::CHAIN_BIND::CHAIN_BIND_CURRENT(action) => {
                if bind_current.is_some() {
                    return Err(format!("You can only assign one name for a chain (select)"));
                }
                bind_current = Some(action);
            },
            query_parser_actions::CHAIN_BIND::CHAIN_BIND_SUBCHAIN(action) => {
                children.push(compile_chain(*action)?);
            },
        }
    }
    return Ok(Chain {
        head: compile_chain_body(chain.chain_head.rootopt, chain.chain_head.step0.unwrap_or_default())?,
        tail: ChainTail {
            bind: bind_current,
            subchains: children,
        },
    });
}

pub fn compile_query(query: &str) -> Result<Query, String> {
    let parse =
        rustemo::Parser::parse(&query_parser::QueryParserParser::new(), query)
            .map_err(|e| e.to_string())
            .map_err(|e| format!("Error parsing query: {}", e))?;
    return Ok(Query {
        sort: match parse.sort_queryopt {
            Some(sort) => match sort {
                query_parser_actions::SORT_QUERY::SORT_QUERY_PAIRS(sort) => {
                    Some(SortQuery::Fields(sort.into_iter().map(|x| match x {
                        query_parser_actions::SORT_QUERY_PAIR::SORT_QUERY_PAIR_ASC(x) => (SortDir::Asc, x),
                        query_parser_actions::SORT_QUERY_PAIR::SORT_QUERY_PAIR_DESC(x) => (SortDir::Desc, x),
                    }).collect()))
                },
                query_parser_actions::SORT_QUERY::kw_sort_random => {
                    Some(SortQuery::Shuffle)
                },
            },
            None => None,
        },
        chain: compile_chain(parse.chain)?,
    });
}

pub fn compile_fragment_query_head(query_frag: &str) -> Result<ChainHead, String> {
    // Hack, https://github.com/igordejanovic/rustemo/issues/24
    let parse =
        rustemo::Parser::parse(&query_parser::QueryParserParser::new(), &format!("{} {{}}", query_frag))
            .map_err(|e| e.to_string())
            .map_err(|e| format!("Error parsing query: {}", e))?;
    return Ok(compile_chain(parse.chain)?.head);
}

pub fn compile_fragment_query_tail(query_frag: &str) -> Result<ChainTail, String> {
    // Hack, https://github.com/igordejanovic/rustemo/issues/24
    let parse =
        rustemo::Parser::parse(&query_parser::QueryParserParser::new(), &format!("-> \"\" {}", query_frag))
            .map_err(|e| e.to_string())
            .map_err(|e| format!("Error parsing query: {}", e))?;
    return Ok(compile_chain(parse.chain)?.tail);
}
