use {
    loga::ea,
    shared::interface::triple::FileHash,
    std::path::{
        Path,
        PathBuf,
    },
};

pub fn file_path(root_path: &Path, hash: &FileHash) -> Result<PathBuf, loga::Error> {
    match hash {
        FileHash::Sha256(hash) => {
            if hash.len() < 4 {
                return Err(loga::err_with("Hash is too short", ea!(length = hash.len())));
            }
            return Ok(root_path.join("sha256").join(&hash[0 .. 2]).join(&hash[2 .. 4]).join(hash));
        },
    }
}

pub fn generated_path(
    root_path: &Path,
    hash: &FileHash,
    mime_type: &str,
    name: &str,
) -> Result<PathBuf, loga::Error> {
    let mut suffix = String::new();
    suffix.push_str(&mime_type.replace("/", "_"));
    if !name.is_empty() {
        suffix.push_str(".");
        suffix.push_str(name);
    }
    match hash {
        FileHash::Sha256(hash) => {
            if hash.len() < 4 {
                return Err(loga::err_with("Hash is too short", ea!(length = hash.len())));
            }
            return Ok(
                root_path
                    .join("sha256")
                    .join(&hash[0 .. 2])
                    .join(&hash[2 .. 4])
                    .join(format!("{}.{}", hash, suffix)),
            );
        },
    }
}

pub fn staged_file_path(root_path: &Path, hash: &FileHash) -> Result<PathBuf, loga::Error> {
    match hash {
        FileHash::Sha256(hash) => {
            if hash.len() < 4 {
                return Err(loga::err_with("Hash is too short", ea!(length = hash.len())));
            }
            return Ok(root_path.join(&format!("sha256_{}", hash)));
        },
    }
}
