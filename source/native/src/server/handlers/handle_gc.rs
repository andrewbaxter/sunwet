use {
    crate::{
        interface::triple::DbNode,
        server::{
            db,
            dbutil::tx,
            filesutil::file_path,
        },
    },
    async_walkdir::WalkDir,
    chrono::Utc,
    deadpool_sqlite::Pool,
    flowcontrol::exenum,
    loga::{
        ea,
        ErrContext,
        Log,
        ResultContext,
    },
    shared::interface::triple::{
        FileHash,
        Node,
    },
    std::{
        collections::HashMap,
        path::{
            Component,
            Path,
            PathBuf,
        },
        str::FromStr,
    },
    tokio::fs::remove_file,
    tokio_stream::StreamExt,
};

pub async fn handle_gc(log: &Log, dbc: &Pool, files_dir: &Path, cache_dir: &Path) -> Result<(), loga::Error> {
    // Clean graph
    tx(&dbc, |txn| {
        let epoch = Utc::now() - chrono::Duration::days(365);
        db::triple_gc_deleted(txn, epoch)?;
        db::meta_gc(txn)?;
        db::commit_gc(txn)?;
        return Ok(());
    }).await?;

    // Clean up unreferenced files
    async fn clean_batch(log: &Log, dbc: &Pool, batch: &mut HashMap<FileHash, PathBuf>) -> Result<(), loga::Error> {
        let unfiltered_keys = batch.keys().map(|k| DbNode(Node::File(k.clone()))).collect::<Vec<_>>();
        let found_keys = tx(&dbc, move |txn| {
            return Ok(db::meta_filter_existing(txn, unfiltered_keys.iter().collect())?);
        }).await?;
        for key in found_keys {
            batch.remove(&exenum!(key.0, Node:: File(x) => x).unwrap());
        }
        for path in batch.values() {
            match remove_file(path).await {
                Ok(_) => { },
                Err(e) => {
                    log.log_err(
                        loga::WARN,
                        e.context_with("Failed to delete unreferenced file", ea!(path = path.display().to_string())),
                    );
                },
            };
        }
        batch.clear();
        return Ok(());
    }

    fn get_file_hash(log: &Log, root: &Path, path: &Path) -> Option<FileHash> {
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

    let mut walk = WalkDir::new(&files_dir);
    let mut batch = HashMap::new();
    while let Some(entry) = walk.next().await {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                log.log_err(loga::WARN, e.context("Unable to scan file in files_dir"));
                continue;
            },
        };
        let path = entry.path();
        let log = log.fork(ea!(path = path.to_string_lossy()));
        if !entry.metadata().await.stack_context(&log, "Error reading metadata")?.is_file() {
            continue;
        }
        let Some(hash) = get_file_hash(&log, &files_dir, &path) else {
            continue;
        };
        batch.insert(hash.clone(), path);
        if batch.len() >= 1000 {
            clean_batch(&log, &dbc, &mut batch).await?;
        }
    }
    if !batch.is_empty() {
        clean_batch(&log, &dbc, &mut batch).await?;
    }

    // Clean up unreferenced generated files
    let mut walk = WalkDir::new(&cache_dir);
    while let Some(entry) = walk.next().await {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                log.log_err(loga::WARN, e.stack_context(log, "Unable to scan file in files_dir"));
                continue;
            },
        };
        let path = entry.path();
        let log = log.fork(ea!(path = path.to_string_lossy()));
        if !entry.metadata().await.stack_context(&log, "Error reading metadata")?.is_file() {
            continue;
        }
        let Some(hash) = get_file_hash(&log, &cache_dir, &path) else {
            continue;
        };
        if !file_path(&files_dir, &hash).unwrap().exists() {
            match remove_file(&path).await {
                Ok(_) => { },
                Err(e) => {
                    log.log_err(
                        loga::WARN,
                        e.context_with(
                            "Failed to delete unreferenced generated file",
                            ea!(path = path.display().to_string()),
                        ),
                    );
                },
            };
        }
    }

    // Don
    return Ok(());
}
