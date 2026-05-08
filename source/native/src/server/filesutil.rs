use {
    super::{
        db,
        dbutil,
        dbutil::tx,
        state::State,
    },
    crate::interface::triple::DbNode,
    loga::{
        ea,
        Log,
        ResultContext,
    },
    sha2::{
        Digest,
        Sha256,
    },
    shared::interface::triple::{
        FileHash,
        Node,
    },
    std::{
        sync::Arc,
    },
    tokio::io::AsyncReadExt,
};

pub struct Metadata {
    pub mimetype: Option<String>,
}

pub async fn get_meta(state: &Arc<State>, hash: &FileHash) -> Result<Option<Metadata>, loga::Error> {
    let state = state.clone();
    let hash = hash.clone();
    let res = tx(&state.db, move |txn| {
        let mut db = dbutil::db3(txn);
        let node_str = serde_json_canonicalizer::to_string(&Node::File(hash)).unwrap();
        return Ok(
            db
                .0
                .query(r#"select
                     "mimetype"
                   from
                     "meta"
                   where
                     "node" = ?
                   "#, [node_str], |row| Ok(row.get::<_, Option<String>>(0)?))?
                .into_iter()
                .next()
                .map(|mimetype| Metadata { mimetype }),
        );
    }).await?;
    return Ok(res);
}

pub async fn get_hash(log: &Log, path: &std::path::Path) -> Option<FileHash> {
    let mut file = match tokio::fs::File::open(path).await {
        Ok(f) => f,
        Err(e) => {
            log.log_err(loga::WARN, loga::err(e).context("Failed to open file for hashing"));
            return None;
        },
    };
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        match file.read(&mut buffer).await {
            Ok(0) => break,
            Ok(n) => hasher.update(&buffer[..n]),
            Err(e) => {
                log.log_err(loga::WARN, loga::err(e).context("Failed to read file for hashing"));
                return None;
            },
        }
    }
    let hash_hash = hex::encode(hasher.finalize());
    let hash_type = "sha256";
    let hash = match serde_json::from_str(&format!("\"{}:{}\"", hash_type, hash_hash)) {
        Ok(h) => h,
        Err(e) => {
            log.log_err(loga::WARN, loga::err(e).context("Failed to determine hash for file"));
            return None;
        },
    };
    return Some(hash);
}
