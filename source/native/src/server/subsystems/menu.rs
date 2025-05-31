use {
    crate::{
        interface::config::IamGrants,
        server::{
            access::Identity,
            state::{
                get_global_config,
                get_iam_grants,
                GlobalConfig,
                State,
            },
        },
    },
    flowcontrol::exenum,
    loga::{
        ea,
        Log,
        ResultContext,
    },
    shared::interface::{
        config::{
            form::ClientForm,
            view::{
                ClientView,
                DataRowsLayout,
                QueryOrField,
                Widget,
            },
            ClientConfig,
            ClientMenuItem,
            ClientMenuSection,
        },
        query::{
            Chain,
            ChainBody,
            ChainRoot,
            FilterExpr,
            FilterSuffix,
            Query,
            StrValue,
            Value,
        },
    },
    std::{
        collections::{
            BTreeMap,
            HashMap,
            HashSet,
        },
        sync::Arc,
    },
};

pub async fn handle_get_filtered_client_config(
    state: Arc<State>,
    identity: &Identity,
) -> Result<ClientConfig, loga::Error> {
    let mut views = HashMap::new();
    let mut forms = HashMap::new();

    fn compile_visible_menu(
        log: &Log,
        views: &mut HashMap<String, ClientView>,
        forms: &mut HashMap<String, ClientForm>,
        config: &GlobalConfig,
        iam_grants: &IamGrants,
        at_id: &String,
    ) -> Option<ClientMenuItem> {
        match config.menu_items.get(at_id).unwrap() {
            crate::server::state::MenuItem::Section(at) => {
                let mut children = vec![];
                for child_id in &at.children {
                    let Some(child) = compile_visible_menu(log, views, forms, config, iam_grants, child_id) else {
                        continue;
                    };
                    children.push(child);
                }
                if children.is_empty() {
                    return None;
                }
                return Some(ClientMenuItem::Section(ClientMenuSection {
                    name: at.name.clone(),
                    children: children,
                }));
            },
            crate::server::state::MenuItem::View(at) => {
                if !iam_grants.match_set(&at.self_and_ancestors) {
                    return None;
                }
                let query_parameters = match (|| -> Result<BTreeMap<String, Vec<String>>, loga::Error> {
                    fn recurse_query_value(v: &Value, query_parameters: &mut HashSet<String>) {
                        match v {
                            Value::Literal(_) => { },
                            Value::Parameter(r) => {
                                query_parameters.insert(r.clone());
                            },
                        }
                    }

                    fn recurse_query_str_value(v: &StrValue, query_parameters: &mut HashSet<String>) {
                        match v {
                            StrValue::Literal(_) => { },
                            StrValue::Parameter(r) => {
                                query_parameters.insert(r.clone());
                            },
                        }
                    }

                    fn recurse_query_filter_expr(f: &FilterExpr, query_parameters: &mut HashSet<String>) {
                        match f {
                            FilterExpr::Exists(f) => {
                                recurse_query_chain_body(&f.subchain, query_parameters);
                                if let Some(suffix) = &f.suffix {
                                    match suffix {
                                        FilterSuffix::Simple(suffix) => {
                                            recurse_query_value(&suffix.value, query_parameters);
                                        },
                                        FilterSuffix::Like(suffix) => {
                                            recurse_query_str_value(&suffix.value, query_parameters);
                                        },
                                    }
                                }
                            },
                            FilterExpr::Junction(f) => {
                                for e in &f.subexprs {
                                    recurse_query_filter_expr(e, query_parameters);
                                }
                            },
                        }
                    }

                    fn recurse_query_chain_body(query_chain: &ChainBody, query_parameters: &mut HashSet<String>) {
                        if let Some(root) = &query_chain.root {
                            match root {
                                ChainRoot::Value(r) => match r {
                                    Value::Literal(_) => { },
                                    Value::Parameter(r) => {
                                        query_parameters.insert(r.clone());
                                    },
                                },
                                ChainRoot::Search(r) => recurse_query_str_value(r, query_parameters),
                            }
                        }
                        for step in &query_chain.steps {
                            match step {
                                shared::interface::query::Step::Move(s) => {
                                    recurse_query_str_value(&s.predicate, query_parameters);
                                    if let Some(f) = &s.filter {
                                        recurse_query_filter_expr(f, query_parameters);
                                    }
                                },
                                shared::interface::query::Step::Recurse(s) => {
                                    recurse_query_chain_body(&s.subchain, query_parameters);
                                },
                                shared::interface::query::Step::Junction(s) => {
                                    for c in &s.subchains {
                                        recurse_query_chain_body(c, query_parameters);
                                    }
                                },
                            }
                        }
                    }

                    fn recurse_query_chain(query_chain: &Chain, query_parameters: &mut HashSet<String>) {
                        recurse_query_chain_body(&query_chain.body, query_parameters);
                        for s in &query_chain.subchains {
                            recurse_query_chain(s, query_parameters);
                        }
                    }

                    fn recurse(
                        queries: &BTreeMap<String, Query>,
                        w: &Widget,
                        query_parameters: &mut BTreeMap<String, Vec<String>>,
                    ) -> Result<(), loga::Error> {
                        match w {
                            Widget::Layout(w) => {
                                for e in &w.elements {
                                    recurse(queries, e, query_parameters)?;
                                }
                            },
                            Widget::DataRows(w) => {
                                match &w.data {
                                    QueryOrField::Field(_) => { },
                                    QueryOrField::Query(q) => {
                                        let query = queries.get(q).context(format!("Missing query [{}]", q))?;
                                        query_parameters.entry(q.clone()).or_insert_with(|| {
                                            let mut params = HashSet::new();
                                            recurse_query_chain(&query.chain, &mut params);
                                            return params.into_iter().collect::<Vec<_>>();
                                        });
                                    },
                                }
                                match &w.row_widget {
                                    DataRowsLayout::Unaligned(w) => {
                                        recurse(queries, &w.widget, query_parameters)?;
                                    },
                                    DataRowsLayout::Table(w) => {
                                        for e in &w.elements {
                                            recurse(queries, e, query_parameters)?;
                                        }
                                    },
                                }
                            },
                            Widget::Text(_) => { },
                            Widget::Date(_) => { },
                            Widget::Time(_) => { },
                            Widget::Datetime(_) => { },
                            Widget::Color(_) => { },
                            Widget::Media(_) => { },
                            Widget::PlayButton(_) => { },
                            Widget::Space => { },
                        }
                        return Ok(());
                    }

                    let mut query_parameters: BTreeMap<String, Vec<String>> = Default::default();
                    match &at.item.root.data {
                        QueryOrField::Field(_) => { },
                        QueryOrField::Query(q) => {
                            let query =
                                at
                                    .item
                                    .queries
                                    .get(q)
                                    .context(format!("Missing query [{}] referred in menu item [{}]", q, at_id))?;
                            query_parameters.entry(q.clone()).or_insert_with(|| {
                                let mut params = HashSet::new();
                                recurse_query_chain(&query.chain, &mut params);
                                return params.into_iter().collect::<Vec<_>>();
                            });
                        },
                    }
                    for b in &at.item.root.row_blocks {
                        recurse(&at.item.queries, &b.widget, &mut query_parameters)?;
                    }
                    return Ok(query_parameters);
                })() {
                    Ok(q) => q,
                    Err(e) => {
                        log.log(
                            loga::WARN,
                            e.context_with("Broken view/query, skipping menu item", ea!(item = at_id)),
                        );
                        return None;
                    },
                };
                return Some(ClientMenuItem::View(ClientView {
                    id: at_id.clone(),
                    name: at.item.name.clone(),
                    root: at.item.root.clone(),
                    parameters: at.item.parameters.clone(),
                    query_parameters: query_parameters,
                }));
            },
            crate::server::state::MenuItem::Form(at) => {
                if !iam_grants.match_set(&at.self_and_ancestors) {
                    return None;
                }
                return Some(ClientMenuItem::Form(ClientForm {
                    id: at_id.clone(),
                    name: at.item.name.clone(),
                    fields: at.item.fields.clone(),
                    outputs: at.item.outputs.clone(),
                }));
            },
            crate::server::state::MenuItem::History => {
                if exenum!(iam_grants, IamGrants:: Admin =>()).is_none() {
                    return None;
                }
                return Some(ClientMenuItem::History);
            },
        }
    }

    let iam_grants = get_iam_grants(&state, identity).await?;
    let global_config = get_global_config(&state).await?;
    let mut menu = vec![];
    for root_id in &global_config.menu {
        let Some(item) =
            compile_visible_menu(&state.log, &mut views, &mut forms, &global_config, &iam_grants, root_id) else {
                continue;
            };
        menu.push(item);
    }
    return Ok(ClientConfig { menu: menu });
}
