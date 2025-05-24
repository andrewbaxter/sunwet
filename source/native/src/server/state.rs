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
                MenuItemId,
                UserConfig,
                View,
            },
        },
        ScopeValue,
    },
    by_address::ByAddress,
    cookie::time::ext::InstantExt,
    deadpool_sqlite::Pool,
    loga::{
        ea,
        Log,
        ResultContext,
    },
    moka::future::Cache,
    shared::interface::{
        config::form::ClientForm,
        iam::UserIdentityId,
        triple::FileHash,
        wire::link::{
            WsS2L,
        },
    },
    std::{
        collections::{
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

pub struct MenuItemSection {
    pub name: String,
    pub self_and_ancestors: HashSet<String>,
    pub children: Vec<String>,
}

pub struct MenuItemView {
    pub item: interface::config::MenuItemView,
    pub self_and_ancestors: HashSet<String>,
}

pub struct MenuItemForm {
    pub item: interface::config::MenuItemForm,
    pub self_and_ancestors: HashSet<String>,
}

pub enum MenuItem {
    Section(MenuItemSection),
    View(MenuItemView),
    Form(MenuItemForm),
}

pub struct GlobalConfig {
    pub public_iam_grants: HashSet<MenuItemId>,
    pub menu: Vec<String>,
    pub menu_items: HashMap<String, MenuItem>,
    pub views: HashMap<String, View>,
    pub forms: HashMap<String, ClientForm>,
    pub api_tokens: HashMap<String, IamGrants>,
}

pub fn build_global_config(config0: &interface::config::GlobalConfig) -> Result<Arc<GlobalConfig>, loga::Error> {
    fn build_menu_items(
        config0: &interface::config::GlobalConfig,
        menu_out: &mut Vec<String>,
        menu_items_out: &mut HashMap<String, MenuItem>,
        ancestry: &HashSet<String>,
        seen: &mut HashSet<String>,
        at: &interface::config::MenuItem,
    ) -> Result<(), loga::Error> {
        if !seen.insert(at.id.clone()) {
            return Err(loga::err(format!("Multiple menu items with id {}", at.id)));
        }
        let mut ancestry = ancestry.clone();
        ancestry.insert(at.id.clone());
        match &at.sub {
            interface::config::MenuItemSub::Section(sub) => {
                let mut out = vec![];
                for child in &sub.children {
                    build_menu_items(config0, &mut out, menu_items_out, &ancestry, seen, child)?;
                }
                menu_items_out.insert(at.id.clone(), MenuItem::Section(MenuItemSection {
                    name: sub.name.clone(),
                    children: out,
                    self_and_ancestors: ancestry,
                }));
            },
            interface::config::MenuItemSub::View(sub) => {
                if !config0.views.contains_key(&sub.view_id) {
                    return Err(
                        loga::err(format!("Menu item [{}] references missing view [{}]", at.id, sub.view_id)),
                    );
                }
                menu_items_out.insert(at.id.clone(), MenuItem::View(MenuItemView {
                    item: sub.clone(),
                    self_and_ancestors: ancestry,
                }));
            },
            interface::config::MenuItemSub::Form(sub) => {
                if !config0.forms.contains_key(&sub.form_id) {
                    return Err(
                        loga::err(format!("Menu item [{}] references missing form [{}]", at.id, sub.form_id)),
                    );
                }
                menu_items_out.insert(at.id.clone(), MenuItem::Form(MenuItemForm {
                    item: sub.clone(),
                    self_and_ancestors: ancestry,
                }));
            },
        }
        menu_out.push(at.id.clone());
        return Ok(());
    }

    let mut menu = vec![];
    let mut menu_items = HashMap::new();
    for item in &config0.menu {
        build_menu_items(&config0, &mut menu, &mut menu_items, &HashSet::new(), &mut HashSet::new(), item)?;
    }
    return Ok(Arc::new(GlobalConfig {
        public_iam_grants: config0.public_iam_grants.clone(),
        menu: menu,
        menu_items: menu_items,
        views: config0.views.clone(),
        forms: config0.forms.clone(),
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
