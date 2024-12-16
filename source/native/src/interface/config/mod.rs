use {
    access::Access,
    identity::Identity,
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::config::menu::MenuItem,
    std::{
        net::SocketAddr,
        path::PathBuf,
    },
};

pub mod access;
pub mod identity;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub debug: bool,
    pub persistent_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub bind_addr: SocketAddr,
    pub identity: Identity,
    pub access: Access,
    pub menu: Vec<MenuItem>,
}
