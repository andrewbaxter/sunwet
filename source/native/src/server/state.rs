use {
    super::{
        access::Identity,
        subsystems::oidc,
    },
    crate::{
        interface::{
            self,
            config::{
                ConfigIamGrants,
                ConfigIamGrantsLimited,
                MenuItemPage,
                ServerConfigMenuItem,
                ServerConfigMenuItemDetail,
                UserConfig,
            },
        },
        server::access::AccessSourceId,
        ScopeValue,
    },
    by_address::ByAddress,
    cookie::time::ext::InstantExt,
    deadpool_sqlite::Pool,
    http::HeaderMap,
    loga::{
        ea,
        DebugDisplay,
        Log,
        ResultContext,
    },
    moka::future::Cache,
    shared::{
        interface::{
            config::{
                form::FormId,
                view::{
                    self,
                    ViewId,
                },
                MenuItemId,
            },
            iam::UserIdentityId,
            query,
            triple::FileHash,
            wire::link::WsS2L,
        },
        query_analysis::analyze_query,
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
    pub item: interface::config::Form,
}

pub struct ServerView {
    pub item: interface::config::View,
    pub query_parameters: BTreeMap<String, Vec<String>>,
}

pub struct GlobalConfig {
    pub menu: Vec<ServerConfigMenuItem>,
    pub menu_transitive_access: HashMap<MenuItemId, HashSet<AccessSourceId>>,
    pub forms: HashMap<FormId, ServerForm>,
    pub views: HashMap<ViewId, ServerView>,
    pub public_iam_grants: ConfigIamGrantsLimited,
    pub api_tokens_iam_grants: HashMap<String, ConfigIamGrants>,
}

pub fn build_global_config(config0: &interface::config::GlobalConfig) -> Result<Arc<GlobalConfig>, loga::Error> {
    let mut forms = HashMap::new();
    for (k, v) in &config0.forms {
        forms.insert(k.clone(), ServerForm { item: v.clone() });
    }
    let mut views = HashMap::new();
    for (k, v) in &config0.views {
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
                                let analysis = analyze_query(query);
                                let Some(r#struct) = analysis.r#struct else {
                                    return vec![];
                                };
                                return r#struct.inputs.into_iter().collect::<Vec<_>>();
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
                view::Widget::Icon(_) => { },
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
                    let analysis = analyze_query(query);
                    let Some(r#struct) = analysis.r#struct else {
                        return vec![];
                    };
                    return r#struct.inputs.into_iter().collect::<Vec<_>>();
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
        views.insert(k.clone(), ServerView {
            item: v.clone(),
            query_parameters: query_parameters,
        });
    }

    fn build_menu_item_access<
        'a,
    >(
        config0: &interface::config::GlobalConfig,
        menu_item_access_out: &mut HashMap<MenuItemId, HashSet<AccessSourceId>>,
        views_out: &mut HashMap<ViewId, ServerView>,
        forms_out: &mut HashMap<FormId, ServerForm>,
        seen: &mut HashSet<MenuItemId>,
        at: &interface::config::ServerConfigMenuItem,
    ) -> Result<HashSet<AccessSourceId>, loga::Error> {
        if !seen.insert(at.id.clone()) {
            return Err(loga::err(format!("Multiple menu items with id {}", at.id)));
        }
        let mut access = HashSet::new();
        match &at.detail {
            ServerConfigMenuItemDetail::Section(d) => {
                for config_child in &d.children {
                    let child_access =
                        build_menu_item_access(
                            config0,
                            menu_item_access_out,
                            views_out,
                            forms_out,
                            seen,
                            config_child,
                        )?;
                    access.extend(child_access.clone());
                }
            },
            ServerConfigMenuItemDetail::Page(d) => match d {
                MenuItemPage::View(d) => {
                    access.insert(AccessSourceId::ViewId(d.view_id.clone()));
                },
                MenuItemPage::Form(d) => {
                    access.insert(AccessSourceId::FormId(d.form_id.clone()));
                },
                MenuItemPage::History => { },
                MenuItemPage::Query => { },
                MenuItemPage::Logs => { },
            },
        }
        menu_item_access_out.insert(at.id.clone(), access.clone());
        return Ok(access);
    }

    let mut menu_item_access = HashMap::new();
    for item in &config0.menu {
        build_menu_item_access(&config0, &mut menu_item_access, &mut views, &mut forms, &mut HashSet::new(), item)?;
    }
    return Ok(Arc::new(GlobalConfig {
        menu: config0.menu.clone(),
        menu_transitive_access: menu_item_access,
        forms: forms,
        views: views,
        public_iam_grants: config0.public_iam_grants.clone(),
        api_tokens_iam_grants: config0.api_tokens.clone(),
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
    pub cache: Cache<UserIdentityId, Option<Arc<interface::config::UserConfig>>>,
}

pub struct LocalUsersState {
    pub users: HashMap<UserIdentityId, Arc<interface::config::UserConfig>>,
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

#[derive(Debug)]
pub struct IamGrantsLimited {
    pub menu_items: HashSet<MenuItemId>,
    pub views: HashSet<ViewId>,
    pub forms: HashSet<FormId>,
}

#[derive(Debug)]
pub enum IamGrants {
    Admin,
    Limited(IamGrantsLimited),
}

fn build_iam_grants_limited(
    global_config: &GlobalConfig,
    identity: &Identity,
    access: &ConfigIamGrantsLimited,
) -> Result<IamGrantsLimited, loga::Error> {
    let mut views = access.views.clone();
    let mut forms = access.forms.clone();
    for id in &access.menu_items {
        let Some(access) = global_config.menu_transitive_access.get(id) else {
            return Err(
                loga::err_with(
                    format!("Missing menu item [{}] referred to by user identity", id),
                    ea!(identity = identity.dbg_str()),
                ),
            );
        };
        for id in access {
            match id {
                AccessSourceId::FormId(id) => {
                    forms.insert(id.clone());
                },
                AccessSourceId::ViewId(id) => {
                    views.insert(id.clone());
                },
            }
        }
    }
    return Ok(IamGrantsLimited {
        menu_items: access.menu_items.clone(),
        views: views,
        forms: forms,
    });
}

pub async fn get_iam_grants(state: &State, identity: &Identity) -> Result<IamGrants, loga::Error> {
    match identity {
        Identity::Token(grants) => {
            match &grants {
                ConfigIamGrants::Admin => {
                    return Ok(IamGrants::Admin);
                },
                ConfigIamGrants::Limited(access) => {
                    let global_config = get_global_config(state).await?;
                    return Ok(IamGrants::Limited(build_iam_grants_limited(&global_config, identity, access)?));
                },
            }
        },
        Identity::User(identity1) => {
            let user_config = get_user_config(state, identity1).await?;
            match &user_config.iam_grants {
                ConfigIamGrants::Admin => {
                    return Ok(IamGrants::Admin);
                },
                ConfigIamGrants::Limited(access) => {
                    let global_config = get_global_config(state).await?;
                    return Ok(IamGrants::Limited(build_iam_grants_limited(&global_config, identity, access)?));
                },
            }
        },
        Identity::Public | Identity::Link(_) => {
            let global_config = get_global_config(state).await?;
            return Ok(
                IamGrants::Limited(
                    build_iam_grants_limited(&global_config, identity, &global_config.public_iam_grants)?,
                ),
            );
        },
    }
}
