use {
    crate::js::Env,
    shared::interface::{
        triple::FileHash,
        wire::FileUrlQuery,
    },
};

pub fn generated_file_url(env: &Env, hash: &FileHash, q: FileUrlQuery) -> String {
    return format!("{}file/{}?{}", env.base_url, hash.to_string(), serde_json::to_string(&q).unwrap());
}

pub fn file_url(env: &Env, hash: &FileHash) -> String {
    return format!("{}file/{}", env.base_url, hash.to_string());
}
