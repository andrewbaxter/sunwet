use {
    crate::js::Env,
    shared::interface::{
        triple::FileHash,
    },
};

pub fn generated_file_url(env: &Env, hash: &FileHash, gentype: &str, subpath: &str) -> String {
    let a = format!("{}file/{}.{}", env.base_url, hash.to_string(), gentype);
    if !subpath.is_empty() {
        return format!("{}/{}", a, subpath);
    } else {
        return a;
    }
}

pub fn file_url(env: &Env, hash: &FileHash) -> String {
    return format!("{}file/{}", env.base_url, hash.to_string());
}
