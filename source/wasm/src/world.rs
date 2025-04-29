use {
    shared::interface::{
        triple::FileHash,
    },
};

pub fn file_url(base_url: &String, hash: &FileHash) -> String {
    return format!("{}file/{}", base_url, hash.to_string());
}
