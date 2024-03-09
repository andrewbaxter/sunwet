use std::{
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
};
use chrono::Duration;
use cozo::{
    Db,
    SqliteStorage,
};
use native::util::{
    Log,
    ScopeValue,
};
use shared::model::{
    link::{
        WsS2C,
        WsS2L,
    },
    FileHash,
};
use taskmanager::TaskManager;
use tokio::sync::{
    mpsc,
    oneshot,
};

pub struct WsState<M> {
    pub send: mpsc::Sender<M>,
    pub ready: Mutex<Option<oneshot::Sender<Duration>>>,
}

pub struct State {
    pub tm: TaskManager,
    pub log: Log,
    pub db: Db<SqliteStorage>,
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
