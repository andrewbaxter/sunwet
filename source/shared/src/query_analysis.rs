use {
    crate::interface::query::{
        self,
    },
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    std::collections::{
        HashMap,
        HashSet,
    },
    ts_rs::TS,
};

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Hash, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct QueryAnalysisOutput {
    pub plural: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct QueryAnalysis {
    pub inputs: HashSet<String>,
    pub outputs: HashMap<String, QueryAnalysisOutput>,
}

pub fn analyze_query(q: &query::Query) -> QueryAnalysis {
    struct State {
        inputs: HashSet<String>,
        outputs: HashMap<String, QueryAnalysisOutput>,
    }

    fn recurse_query_value(v: &query::Value, state: &mut State) {
        match v {
            query::Value::Literal(_) => { },
            query::Value::Parameter(r) => {
                state.inputs.insert(r.clone());
            },
        }
    }

    fn recurse_query_str_value(v: &query::StrValue, state: &mut State) {
        match v {
            query::StrValue::Literal(_) => { },
            query::StrValue::Parameter(r) => {
                state.inputs.insert(r.clone());
            },
        }
    }

    fn recurse_query_filter_expr(f: &query::FilterExpr, state: &mut State) {
        match f {
            query::FilterExpr::Exists(f) => {
                recurse_query_chain_body(&f.subchain, state);
                if let Some(suffix) = &f.suffix {
                    match suffix {
                        query::FilterSuffix::Simple(suffix) => {
                            recurse_query_value(&suffix.value, state);
                        },
                        query::FilterSuffix::Like(suffix) => {
                            recurse_query_str_value(&suffix.value, state);
                        },
                    }
                }
            },
            query::FilterExpr::Junction(f) => {
                for e in &f.subexprs {
                    recurse_query_filter_expr(e, state);
                }
            },
        }
    }

    fn recurse_query_chain_body(query_chain: &query::ChainHead, state: &mut State) -> bool {
        if let Some(root) = &query_chain.root {
            match root {
                query::ChainRoot::Value(r) => match r {
                    query::Value::Literal(_) => { },
                    query::Value::Parameter(r) => {
                        state.inputs.insert(r.clone());
                    },
                },
                query::ChainRoot::Search(r) => recurse_query_str_value(r, state),
            }
        }
        let mut plural = true;
        for step in &query_chain.steps {
            match &step.specific {
                query::StepSpecific::Move(s) => {
                    recurse_query_str_value(&s.predicate, state);
                    if let Some(f) = &s.filter {
                        recurse_query_filter_expr(f, state);
                    }
                },
                query::StepSpecific::Recurse(s) => {
                    recurse_query_chain_body(&s.subchain, state);
                },
                query::StepSpecific::Junction(s) => {
                    for c in &s.subchains {
                        recurse_query_chain_body(c, state);
                    }
                },
            }
            plural = !step.first;
        }
        return plural;
    }

    fn recurse_query_chain(query_chain: &query::Chain, state: &mut State) {
        let plural = recurse_query_chain_body(&query_chain.head, state);
        if let Some(bind) = &query_chain.tail.bind {
            state.outputs.insert(bind.clone(), QueryAnalysisOutput { plural: plural });
        }
        for s in &query_chain.tail.subchains {
            recurse_query_chain(s, state);
        }
    }

    let mut state = State {
        inputs: Default::default(),
        outputs: Default::default(),
    };
    recurse_query_chain(&q.chain, &mut state);
    return QueryAnalysis {
        inputs: state.inputs,
        outputs: state.outputs,
    };
}
