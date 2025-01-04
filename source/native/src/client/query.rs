use {
    loga::ResultContext,
    shared::interface::{
        query::{
            Chain,
            ChainBody,
            ChainRoot,
            FilterExpr,
            FilterExprExists,
            FilterExprExistsType,
            FilterExprJunction,
            FilterSuffix,
            FilterSuffixLike,
            FilterSuffixSimple,
            FilterSuffixSimpleOperator,
            JunctionType,
            MoveDirection,
            Query,
            QuerySortDir,
            Step,
            StepJunction,
            StepMove,
            StepRecurse,
            Value,
        },
        triple::Node,
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
            return Ok(FilterSuffix::Like(FilterSuffixLike { value: suffix }));
        },
    }
}

fn compile_filter(filter: query_parser_actions::FILTER_BODY) -> Result<FilterExpr, loga::Error> {
    match filter {
        query_parser_actions::FILTER_BODY::FILTER_BODY_EXISTS(f) => {
            return Ok(FilterExpr::Exists(FilterExprExists {
                type_: FilterExprExistsType::Exists,
                subchain: compile_chain_body(*f.chain_body)?,
                suffix: if let Some(parsed_suffix) = f.filter_suffixopt {
                    Some(compile_filter_suffix(parsed_suffix)?)
                } else {
                    None
                },
            }));
        },
        query_parser_actions::FILTER_BODY::FILTER_BODY_NOT_EXISTS(f) => {
            return Ok(FilterExpr::Exists(FilterExprExists {
                type_: FilterExprExistsType::DoesntExist,
                subchain: compile_chain_body(*f.chain_body)?,
                suffix: if let Some(parsed_suffix) = f.filter_suffixopt {
                    Some(compile_filter_suffix(parsed_suffix)?)
                } else {
                    None
                },
            }));
        },
        query_parser_actions::FILTER_BODY::FILTER_BODY_JUNCT_AND(f) => {
            let mut subexprs = vec![];
            for parsed_subexpr in f {
                subexprs.push(compile_filter(*parsed_subexpr)?);
            }
            return Ok(FilterExpr::Junction(FilterExprJunction {
                type_: JunctionType::And,
                subexprs: subexprs,
            }));
        },
        query_parser_actions::FILTER_BODY::FILTER_BODY_JUNCT_OR(f) => {
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

fn compile_chain_body(chain_body: query_parser_actions::CHAIN_BODY) -> Result<ChainBody, loga::Error> {
    let root;
    match chain_body.rootopt {
        Some(parsed_root) => match parsed_root {
            query_parser_actions::ROOT::VAL(v) => {
                root = Some(ChainRoot::Value(compile_value(v)?));
            },
            query_parser_actions::ROOT::ROOT_SEARCH(s) => {
                root = Some(ChainRoot::Search(s));
            },
        },
        None => {
            root = None;
        },
    }
    let mut steps = vec![];
    for step in chain_body.step0.unwrap_or_default() {
        match step {
            query_parser_actions::STEP::STEP_MOVE_UP(step) => {
                let filter;
                if let Some(parsed_filter) = step.filteropt {
                    filter = Some(compile_filter(parsed_filter)?);
                } else {
                    filter = None;
                }
                steps.push(Step::Move(StepMove {
                    dir: MoveDirection::Up,
                    predicate: serde_json::from_str(&step.str_val).unwrap(),
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
                    dir: MoveDirection::Down,
                    predicate: serde_json::from_str(&step.str_val).unwrap(),
                    filter: filter,
                    first: step.firstopt.is_some(),
                }));
            },
            query_parser_actions::STEP::STEP_RECURSE(step) => {
                steps.push(Step::Recurse(StepRecurse {
                    subchain: compile_chain_body(*step.chain_body)?,
                    first: step.firstopt.is_some(),
                }));
            },
            query_parser_actions::STEP::STEP_JUNCT_AND(step) => {
                let mut subchains = vec![];
                for parsed_subchain in step {
                    subchains.push(compile_chain_body(*parsed_subchain)?);
                }
                steps.push(Step::Junction(StepJunction {
                    type_: JunctionType::And,
                    subchains: subchains,
                }));
            },
            query_parser_actions::STEP::STEP_JUNCT_OR(step) => {
                let mut subchains = vec![];
                for parsed_subchain in step {
                    subchains.push(compile_chain_body(*parsed_subchain)?);
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

fn compile_chain(chain: query_parser_actions::CHAIN) -> Result<Chain, loga::Error> {
    let mut select = None;
    let mut children = vec![];
    for action in chain.chain_action0.unwrap_or_default() {
        match action {
            query_parser_actions::CHAIN_ACTION::CHAIN_ACTION_SELECT(action) => {
                if select.is_some() {
                    return Err(loga::err("You can only assign one name for a chain (select)"));
                }
                select = Some(action);
            },
            query_parser_actions::CHAIN_ACTION::CHAIN_ACTION_SUBCHAIN(action) => {
                children.push(compile_chain(*action)?);
            },
        }
    }
    return Ok(Chain {
        select: select,
        body: compile_chain_body(chain.chain_body)?,
        subchains: children,
    });
}

pub fn compile_query(query: String) -> Result<Query, loga::Error> {
    let parse =
        rustemo::Parser::parse(&query_parser::QueryParserParser::new(), &query)
            .map_err(loga::err)
            .context("Error parsing query")?;
    return Ok(Query {
        chain: compile_chain(parse.chain)?,
        sort: parse.sort_pair0.unwrap_or_default().into_iter().map(|x| match x {
            query_parser_actions::SORT_PAIR::SORT_PAIR_ASC(x) => (QuerySortDir::Asc, x),
            query_parser_actions::SORT_PAIR::SORT_PAIR_DESC(x) => (QuerySortDir::Desc, x),
        }).collect(),
    });
}
