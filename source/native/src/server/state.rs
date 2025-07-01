use {
    super::{
        access::Identity,
        subsystems::oidc,
    },
    crate::{
        interface::{
            self,
            config::{
                IamGrants,
                IamGrantsLimited,
                MenuItemPage,
                ServerConfigMenuItem,
                ServerConfigMenuItemDetail,
                UserConfig,
            },
        },
        ScopeValue,
    },
    by_address::ByAddress,
    cookie::time::ext::InstantExt,
    deadpool_sqlite::Pool,
    http::HeaderMap,
    loga::{
        ea,
        Log,
        ResultContext,
    },
    moka::future::Cache,
    shared::interface::{
        config::view,
        iam::UserIdentityId,
        query,
        triple::FileHash,
        wire::link::WsS2L,
    },
    std::{
        collections::{
            BTreeMap,
            HashMap,
            HashSet,
        },
        path::PathBuf,
        sync::{
            Arc,
            Mutex,
        },
        time::{
            Duration,
            Instant,
        },
    },
    taskmanager::TaskManager,
    tokio::sync::{
        mpsc::{
            self,
            UnboundedSender,
        },
        oneshot,
    },
};

pub struct WsLinkState<M> {
    pub send: mpsc::Sender<M>,
    pub ready: Mutex<Option<oneshot::Sender<chrono::Duration>>>,
}

#[derive(Clone)]
pub struct FdapState {
    pub fdap_client: fdap::Client,
}

pub struct ServerForm {
    pub menu_self_and_ancestors: HashSet<String>,
    pub item: interface::config::Form,
}

pub struct ServerView {
    pub menu_self_and_ancestors: HashSet<String>,
    pub item: interface::config::View,
    pub query_parameters: BTreeMap<String, Vec<String>>,
}

pub struct GlobalConfig {
    pub public_iam_grants: IamGrantsLimited,
    pub menu: Vec<ServerConfigMenuItem>,
    pub forms: HashMap<String, ServerForm>,
    pub views: HashMap<String, ServerView>,
    pub api_tokens: HashMap<String, IamGrants>,
}

pub fn build_global_config(config0: &interface::config::GlobalConfig) -> Result<Arc<GlobalConfig>, loga::Error> {
    let mut forms = HashMap::new();
    for (k, v) in &config0.forms {
        forms.insert(k.to_string(), ServerForm {
            menu_self_and_ancestors: Default::default(),
            item: v.clone(),
        });
    }
    let mut views = HashMap::new();
    for (k, v) in &config0.views {
        fn recurse_query_value(v: &query::Value, query_parameters: &mut HashSet<String>) {
            match v {
                query::Value::Literal(_) => { },
                query::Value::Parameter(r) => {
                    query_parameters.insert(r.clone());
                },
            }
        }

        fn recurse_query_str_value(v: &query::StrValue, query_parameters: &mut HashSet<String>) {
            match v {
                query::StrValue::Literal(_) => { },
                query::StrValue::Parameter(r) => {
                    query_parameters.insert(r.clone());
                },
            }
        }

        fn recurse_query_filter_expr(f: &query::FilterExpr, query_parameters: &mut HashSet<String>) {
            match f {
                query::FilterExpr::Exists(f) => {
                    recurse_query_chain_body(&f.subchain, query_parameters);
                    if let Some(suffix) = &f.suffix {
                        match suffix {
                            query::FilterSuffix::Simple(suffix) => {
                                recurse_query_value(&suffix.value, query_parameters);
                            },
                            query::FilterSuffix::Like(suffix) => {
                                recurse_query_str_value(&suffix.value, query_parameters);
                            },
                        }
                    }
                },
                query::FilterExpr::Junction(f) => {
                    for e in &f.subexprs {
                        recurse_query_filter_expr(e, query_parameters);
                    }
                },
            }
        }

        fn recurse_query_chain_body(query_chain: &query::ChainBody, query_parameters: &mut HashSet<String>) {
            if let Some(root) = &query_chain.root {
                match root {
                    query::ChainRoot::Value(r) => match r {
                        query::Value::Literal(_) => { },
                        query::Value::Parameter(r) => {
                            query_parameters.insert(r.clone());
                        },
                    },
                    query::ChainRoot::Search(r) => recurse_query_str_value(r, query_parameters),
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

        fn recurse_query_chain(query_chain: &query::Chain, query_parameters: &mut HashSet<String>) {
            recurse_query_chain_body(&query_chain.body, query_parameters);
            for s in &query_chain.subchains {
                recurse_query_chain(s, query_parameters);
            }
        }

        fn recurse_build_query_parameters(
            queries: &BTreeMap<String, query::Query>,
            w: &view::Widget,
            query_parameters: &mut BTreeMap<String, Vec<String>>,
        ) -> Result<(), loga::Error> {
            match w {
                view::Widget::Layout(w) => {
                    for e in &w.elements {
                        recurse_build_query_parameters(queries, e, query_parameters)?;
                    }
                },
                view::Widget::DataRows(w) => {
                    match &w.data {
                        view::QueryOrField::Field(_) => { },
                        view::QueryOrField::Query(q) => {
                            let query = queries.get(q).context(format!("Missing query [{}]", q))?;
                            query_parameters.entry(q.clone()).or_insert_with(|| {
                                let mut params = HashSet::new();
                                recurse_query_chain(&query.chain, &mut params);
                                return params.into_iter().collect::<Vec<_>>();
                            });
                        },
                    }
                    match &w.row_widget {
                        view::DataRowsLayout::Unaligned(w) => {
                            recurse_build_query_parameters(queries, &w.widget, query_parameters)?;
                        },
                        view::DataRowsLayout::Table(w) => {
                            for e in &w.elements {
                                recurse_build_query_parameters(queries, e, query_parameters)?;
                            }
                        },
                    }
                },
                view::Widget::Text(_) => { },
                view::Widget::Date(_) => { },
                view::Widget::Time(_) => { },
                view::Widget::Datetime(_) => { },
                view::Widget::Color(_) => { },
                view::Widget::Media(_) => { },
                view::Widget::PlayButton(_) => { },
                view::Widget::Space => { },
            }
            return Ok(());
        }

        let mut query_parameters: BTreeMap<String, Vec<String>> = Default::default();
        match &v.display.data {
            view::QueryOrField::Field(_) => { },
            view::QueryOrField::Query(q) => {
                let query = v.queries.get(q).context(format!("Missing query [{}] referred in view [{}]", q, k))?;
                query_parameters.entry(q.clone()).or_insert_with(|| {
                    let mut params = HashSet::new();
                    recurse_query_chain(&query.chain, &mut params);
                    return params.into_iter().collect::<Vec<_>>();
                });
            },
        }
        for b in &v.display.row_blocks {
            recurse_build_query_parameters(
                &v.queries,
                &b.widget,
                &mut query_parameters,
            ).context(format!("Error extracting query parameters in view [{}]", k))?;
        }
        views.insert(k.to_string(), ServerView {
            menu_self_and_ancestors: Default::default(),
            item: v.clone(),
            query_parameters: query_parameters,
        });
    }

    fn build_menu_items(
        config0: &interface::config::GlobalConfig,
        views_out: &mut HashMap<String, ServerView>,
        forms_out: &mut HashMap<String, ServerForm>,
        ancestry: &mut Vec<String>,
        seen: &mut HashSet<String>,
        at: &interface::config::ServerConfigMenuItem,
    ) -> Result<(), loga::Error> {
        if !seen.insert(at.id.clone()) {
            return Err(loga::err(format!("Multiple menu items with id {}", at.id)));
        }
        ancestry.push(at.id.clone());
        match &at.detail {
            ServerConfigMenuItemDetail::Section(d) => {
                for child in &d.children {
                    build_menu_items(config0, views_out, forms_out, ancestry, seen, child)?;
                }
            },
            ServerConfigMenuItemDetail::Page(d) => match d {
                MenuItemPage::View(d) => {
                    let view =
                        views_out
                            .get_mut(&d.view_id)
                            .context_with(
                                "Menu item refers to nonexistent view",
                                ea!(menu_item = at.id, view = d.view_id),
                            )?;
                    view.menu_self_and_ancestors.extend(ancestry.iter().cloned());
                },
                MenuItemPage::Form(d) => {
                    let form =
                        forms_out
                            .get_mut(&d.form_id)
                            .context_with(
                                "Menu item refers to nonexistent form",
                                ea!(menu_item = at.id, form = d.form_id),
                            )?;
                    form.menu_self_and_ancestors.extend(ancestry.iter().cloned());
                },
                MenuItemPage::History => { },
                MenuItemPage::Query => { },
            },
        }
        ancestry.pop();
        return Ok(());
    }

    for item in &config0.menu {
        build_menu_items(&config0, &mut views, &mut forms, &mut vec![], &mut HashSet::new(), item)?;
    }
    return Ok(Arc::new(GlobalConfig {
        public_iam_grants: config0.public_iam_grants.clone(),
        menu: config0.menu.clone(),
        forms: forms,
        views: views,
        api_tokens: config0.api_tokens.clone(),
    }));
}

pub struct FdapGlobalState {
    pub fdap: FdapState,
    pub subpath: Vec<String>,
    pub cache: Mutex<Option<(Instant, Arc<GlobalConfig>)>>,
}

pub enum GlobalState {
    Fdap(FdapGlobalState),
    Local(Arc<GlobalConfig>),
}

pub struct FdapUsersState {
    pub fdap: FdapState,
    pub user_subpath: Vec<String>,
    pub cache: Cache<UserIdentityId, Option<Arc<UserConfig>>>,
}

pub struct LocalUsersState {
    pub users: HashMap<UserIdentityId, Arc<UserConfig>>,
}

pub enum UsersState {
    Fdap(FdapUsersState),
    Local(LocalUsersState),
}

pub struct LinkSessionState {
    pub links: Mutex<HashSet<ByAddress<Arc<WsLinkState<WsS2L>>>>>,
    pub public_files: Mutex<HashSet<FileHash>>,
}

pub struct State {
    pub oidc_state: Option<oidc::OidcState>,
    pub fdap_state: Option<FdapState>,
    pub global_state: GlobalState,
    pub users_state: UsersState,
    pub tm: TaskManager,
    pub log: Log,
    pub db: Pool,
    pub temp_dir: PathBuf,
    pub files_dir: PathBuf,
    pub genfiles_dir: PathBuf,
    pub stage_dir: PathBuf,
    pub finishing_uploads: Mutex<HashSet<FileHash>>,
    pub generate_files: UnboundedSender<Option<FileHash>>,
    pub http_resp_headers: HeaderMap,
    // Websockets
    pub link_sessions: Cache<String, Arc<LinkSessionState>>,
    pub link_bg: Mutex<Option<ScopeValue>>,
}

pub async fn get_global_config(state: &State) -> Result<Arc<GlobalConfig>, loga::Error> {
    match &state.global_state {
        GlobalState::Fdap(f) => {
            {
                let cache = f.cache.lock().unwrap();
                if let Some((stamp, config)) = (*cache).as_ref() {
                    if Instant::now().signed_duration_since(*stamp) < Duration::from_secs(5) {
                        return Ok(config.clone());
                    }
                }
            }
            let Some(json) =
                f
                    .fdap
                    .fdap_client
                    .get(f.subpath.iter(), 100 * 1024 * 1024)
                    .await
                    .context("Error making request to FDAP server")? else {
                    return Err(loga::err(format!("No config found in FDAP server at [{:?}]", f.subpath)))
                };
            let config =
                build_global_config(
                    &serde_json::from_value::<interface::config::GlobalConfig>(
                        json,
                    ).context("Global config in FDAP doesn't match expected schema")?,
                )?;
            *f.cache.lock().unwrap() = Some((Instant::now(), config.clone()));
            return Ok(config);
        },
        GlobalState::Local(l) => return Ok(l.clone()),
    }
}

pub async fn get_user_config(state: &State, user: &UserIdentityId) -> Result<Arc<UserConfig>, loga::Error> {
    match &state.users_state {
        UsersState::Fdap(f) => {
            return Ok(
                f
                    .cache
                    .try_get_with::<_, loga::Error>(user.clone(), {
                        let user = user.clone();
                        let fdap_client = f.fdap.fdap_client.clone();
                        let fdap_subpath = f.user_subpath.clone();
                        async move {
                            let Some(json) =
                                fdap_client
                                    .user_get(&user.0, fdap_subpath.iter(), 100 * 1024 * 1024)
                                    .await
                                    .context("Error making request to FDAP server")? else {
                                    return Ok(None);
                                };
                            return Ok(
                                Some(
                                    Arc::new(
                                        serde_json::from_value::<UserConfig>(
                                            json,
                                        ).context_with(
                                            "User config in FDAP doesn't match expected schema",
                                            ea!(user = user.0),
                                        )?,
                                    ),
                                ),
                            );
                        }
                    })
                    .await
                    .map_err(|e| e.as_ref().clone())?
                    .context_with("No config found in FDAP for user", ea!(user = user.0))?,
            );
        },
        UsersState::Local(l) => {
            return Ok(l.users.get(user).context_with("No config defined for user", ea!(user = user.0))?.clone());
        },
    }
}

pub async fn get_iam_grants(state: &State, identity: &Identity) -> Result<IamGrants, loga::Error> {
    match identity {
        Identity::Token(grants) => {
            match &grants {
                IamGrants::Admin => {
                    return Ok(IamGrants::Admin);
                },
                IamGrants::Limited(access) => {
                    return Ok(IamGrants::Limited(access.clone()));
                },
            }
        },
        Identity::User(identity) => {
            let user_config = get_user_config(state, identity).await?;
            match &user_config.iam_grants {
                IamGrants::Admin => {
                    return Ok(IamGrants::Admin);
                },
                IamGrants::Limited(access) => {
                    return Ok(IamGrants::Limited(access.clone()));
                },
            }
        },
        Identity::Public | Identity::Link(_) => {
            return Ok(IamGrants::Limited(get_global_config(state).await?.public_iam_grants.clone()));
        },
    }
}
