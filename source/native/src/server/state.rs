use {
    super::{
        access::Identity,
        handlers::handle_oidc,
    },
    crate::{
        interface::{
            self,
            config::{
                IamGrants,
                UserConfig,
            },
        },
        ScopeValue,
    },
    cookie::time::ext::InstantExt,
    deadpool_sqlite::Pool,
    htwrap::htserve::auth::AuthTokenHash,
    loga::{
        ea,
        Log,
        ResultContext,
    },
    moka::future::Cache,
    shared::interface::{
        config::{
            form::Form,
            menu::MenuItem,
            view::View,
        },
        iam::UserIdentityId,
        triple::FileHash,
        wire::link::{
            WsS2C,
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
            atomic::AtomicU8,
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
        mpsc,
        oneshot,
    },
};

pub struct WsState<M> {
    pub send: mpsc::Sender<M>,
    pub ready: Mutex<Option<oneshot::Sender<chrono::Duration>>>,
}

#[derive(Clone)]
pub struct FdapState {
    pub fdap_client: fdap::Client,
}

#[derive(Default)]
pub struct GlobalConfig {
    pub config: interface::config::GlobalConfig,
    pub admin_token: Option<AuthTokenHash>,
    pub forms: HashMap<String, Form>,
    pub views: HashMap<String, View>,
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

pub struct State {
    pub oidc_state: Option<handle_oidc::OidcState>,
    pub fdap_state: Option<FdapState>,
    pub global_state: GlobalState,
    pub users_state: UsersState,
    pub tm: TaskManager,
    pub log: Log,
    pub db: Pool,
    pub files_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub stage_dir: PathBuf,
    pub finishing_uploads: Mutex<HashSet<FileHash>>,
    // Websockets
    pub link_ids: AtomicU8,
    pub link_main: Mutex<Option<Arc<WsState<WsS2C>>>>,
    pub link_links: Mutex<HashMap<u8, Arc<WsState<WsS2L>>>>,
    pub link_bg: Mutex<Option<ScopeValue>>,
    pub link_public_files: Mutex<HashSet<FileHash>>,
    pub link_session: Mutex<Option<String>>,
}

pub fn gather_pages(config: &interface::config::GlobalConfig) -> (HashMap<String, Form>, HashMap<String, View>) {
    let mut forms = HashMap::new();
    let mut views = HashMap::new();

    fn gather_pages(forms: &mut HashMap<String, Form>, views: &mut HashMap<String, View>, m: &MenuItem) {
        match m {
            MenuItem::Section(s) => {
                for child in &s.children {
                    gather_pages(forms, views, child);
                }
            },
            MenuItem::View(v) => {
                views.insert(v.id.clone(), v.clone());
            },
            MenuItem::Form(f) => {
                forms.insert(f.id.clone(), f.clone());
            },
        }
    }

    for m in &config.menu {
        gather_pages(&mut forms, &mut views, m);
    }
    return (forms, views);
}

pub async fn get_global_config(state: &State) -> Result<Arc<GlobalConfig>, loga::Error> {
    match &state.global_state {
        GlobalState::Fdap(f) => {
            {
                let cache = f.cache.lock().unwrap();
                if let Some((stamp, config)) = (*cache).as_ref() {
                    if Instant::now().signed_duration_since(*stamp) > Duration::from_secs(5) {
                        return Ok(config.clone());
                    }
                }
            }
            let Some(json) = f.fdap.fdap_client.get(f.subpath.iter(), 100 * 1024 * 1024).await? else {
                let config = Arc::new(GlobalConfig::default());
                *f.cache.lock().unwrap() = Some((Instant::now(), config.clone()));
                return Ok(config);
            };
            let config0 =
                serde_json::from_value::<interface::config::GlobalConfig>(
                    json,
                ).context("Global config in FDAP doesn't match expected schema")?;
            let (forms, views) = gather_pages(&config0);
            let config = Arc::new(GlobalConfig {
                admin_token: config0.admin_token.as_ref().map(|t| htwrap::htserve::auth::hash_auth_token(&t)),
                config: config0,
                forms: forms,
                views: views,
            });
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
                                fdap_client.user_get(&user.0, fdap_subpath.iter(), 100 * 1024 * 1024).await? else {
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
        Identity::Admin => {
            return Ok(IamGrants::Admin);
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
        Identity::Public => {
            return Ok(IamGrants::Limited(get_global_config(state).await?.config.public_iam_grants.clone()));
        },
    }
}
