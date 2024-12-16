use {
    super::{
        access::{
            AccessRule,
            NonAdminAccess,
        },
        handle_oidc,
    },
    chrono::Duration,
    deadpool_sqlite::Pool,
    htwrap::htserve,
    loga::Log,
    moka::future::Cache,
    native::{
        interface::config::Config,
        ScopeValue,
    },
    shared::interface::{
        iam::{
            IamTargetId,
            IamUserGroupId,
            IdentityId,
            UserIdentityId,
        },
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
    },
    taskmanager::TaskManager,
    tokio::sync::{
        mpsc,
        oneshot,
    },
};

pub struct WsState<M> {
    pub send: mpsc::Sender<M>,
    pub ready: Mutex<Option<oneshot::Sender<Duration>>>,
}

pub struct IdentityStatePublic {
    pub admin_token: Option<htserve::auth::AuthTokenHash>,
    pub fdap_client: fdap::Client,
    pub fdap_user_subpath: String,
    pub user_group_cache: Cache<UserIdentityId, Option<Vec<IamUserGroupId>>>,
}

pub enum IdentityState {
    Admin,
    Public(IdentityStatePublic),
}

pub struct State {
    pub config: Config,
    pub oidc_state: handle_oidc::OidcState,
    pub identity_mode: IdentityState,
    pub access: HashMap<IamUserGroupId, NonAdminAccess>,
    pub tm: TaskManager,
    pub log: Log,
    pub db: Pool,
    pub files_dir: PathBuf,
    pub generated_dir: PathBuf,
    pub stage_dir: PathBuf,
    pub finishing_uploads: Mutex<HashSet<FileHash>>,
    // Websockets
    pub link_ids: AtomicU8,
    pub link_main: Mutex<Option<Arc<WsState<WsS2C>>>>,
    pub link_links: Mutex<HashMap<u8, Arc<WsState<WsS2L>>>>,
    pub link_bg: Mutex<Option<ScopeValue>>,
    pub link_public_files: HashSet<FileHash>,
    pub link_session: Mutex<Option<String>>,
}
