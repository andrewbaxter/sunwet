use {
    super::{
        db,
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
    shared::interface::{
        triple::{
            FileHash,
            FileHash_,
            Node,
        },
        wire::alphanumeric_only,
    },
    std::{
        io::Write,
        path::{
            Component,
            Path,
            PathBuf,
        },
        str::FromStr,
        sync::Arc,
        task::Poll,
    },
    tokio::{
        fs::File,
        io::{
            copy,
            AsyncWrite,
        },
    },
};

pub fn file_path_(root_path: &Path, hash: &FileHash) -> Result<PathBuf, loga::Error> {
    match &hash.0 {
        FileHash_::Sha256(hash) => {
            if hash.len() < 4 {
                return Err(loga::err_with("Hash is too short", ea!(length = hash.len())));
            }
            return Ok(root_path.join("sha256").join(&hash[0 .. 2]).join(&hash[2 .. 4]).join(hash));
        },
    }
}

pub fn file_path(state: &Arc<State>, hash: &FileHash) -> Result<PathBuf, loga::Error> {
    return file_path_(&state.files_dir, hash);
}

pub fn genfile_path(state: &Arc<State>, hash: &FileHash, gentype: &str) -> Result<PathBuf, loga::Error> {
    return Ok(file_path_(&state.genfiles_dir, hash)?.with_extension(alphanumeric_only(gentype)));
}

pub fn staged_file_path(state: &Arc<State>, hash: &FileHash) -> Result<PathBuf, loga::Error> {
    match &hash.0 {
        FileHash_::Sha256(hash) => {
            if hash.len() < 4 {
                return Err(loga::err_with("Hash is too short", ea!(length = hash.len())));
            }
            return Ok(state.stage_dir.join(&format!("sha256_{}", hash)));
        },
    }
}

pub async fn hash_file_sha256(log: &Log, source: &Path) -> Result<FileHash, loga::Error> {
    let mut got_file = File::open(&source).await.stack_context(&log, "Failed to open staged uploaded file")?;

    struct HashAsyncWriter {
        hash: Sha256,
    }

    impl AsyncWrite for HashAsyncWriter {
        fn poll_write(
            mut self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize, std::io::Error>> {
            return Poll::Ready(self.as_mut().hash.write_all(buf).map(|_| buf.len()));
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> Poll<Result<(), std::io::Error>> {
            return Poll::Ready(Ok(()));
        }

        fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> Poll<Result<(), std::io::Error>> {
            return Poll::Ready(Ok(()));
        }
    }

    let mut got_hash = HashAsyncWriter { hash: Sha256::new() };
    copy(&mut got_file, &mut got_hash).await.stack_context(&log, "Failed to read staged uploaded file")?;
    let got_hash = hex::encode(&got_hash.hash.finalize());
    return Ok(FileHash(FileHash_::Sha256(got_hash)));
}

pub async fn get_meta(state: &Arc<State>, hash: &FileHash) -> Result<Option<db::Metadata>, loga::Error> {
    let state = state.clone();
    let hash = hash.clone();
    let Some(meta) = tx(&state.db, move |txn| {
        return Ok(db::meta_get(txn, &DbNode(Node::File(hash)))?);
    }).await? else {
        return Ok(None);
    };
    return Ok(Some(meta));
}

pub fn get_hash_from_file_path(log: &Log, root: &Path, path: &Path) -> Option<FileHash> {
    let path = path.with_extension("");
    let components = path.strip_prefix(root).unwrap().components().filter_map(|c| match c {
        Component::Normal(c) => Some(c),
        _ => None,
    }).collect::<Vec<_>>();
    let Some(hash_type) = components.first().and_then(|c| c.to_str()) else {
        log.log(loga::WARN, "File in files dir not in hash type directory");
        return None;
    };
    let Some(hash_hash) = components.last().and_then(|c| c.to_str()) else {
        log.log(loga::WARN, "File in files dir has non-utf8 last path segment");
        return None;
    };
    let hash = match FileHash::from_str(&format!("{}:{}", hash_type, hash_hash)) {
        Ok(h) => h,
        Err(e) => {
            log.log_err(loga::WARN, loga::err(e).context("Failed to determine hash for file"));
            return None;
        },
    };
    return Some(hash);
}
