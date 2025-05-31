use {
    crate::{
        interface::triple::DbNode,
        server::{
            db,
            dbutil::tx,
            filesutil::{
                file_path,
                get_hash_from_file_path,
            },
            state::State,
        },
    },
    async_walkdir::WalkDir,
    chrono::Utc,
    deadpool_sqlite::Pool,
    flowcontrol::{
        exenum,
        ta_return,
    },
    loga::{
        ea,
        DebugDisplay,
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
        path::PathBuf,
        sync::Arc,
        time::{
            Duration,
            SystemTime,
        },
    },
    tokio::fs::remove_file,
    tokio_stream::StreamExt,
};

pub async fn handle_gc(state: &Arc<State>, log: &Log) -> Result<(), loga::Error> {
    // Clean graph
    tx(&state.db, |txn| {
        let epoch = Utc::now() - chrono::Duration::days(365);
        db::triple_gc_deleted(txn, epoch)?;
        db::meta_gc(txn)?;
        db::commit_gc(txn)?;
        db::gen_gc(txn)?;
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
            log.log_with(loga::DEBUG, "Garbage collecting file", ea!(file = path.dbg_str()));
            remove_file(path)
                .await
                .log_with(
                    &log,
                    loga::WARN,
                    "Failed to delete unreferenced file",
                    ea!(path = path.display().to_string()),
                );
        }
        batch.clear();
        return Ok(());
    }

    let mut walk = WalkDir::new(&state.files_dir);
    let mut batch = HashMap::new();
    while let Some(entry) = walk.next().await {
        match async {
            ta_return!((), loga::Error);
            let entry = entry?;
            let path = entry.path();
            let log = log.fork(ea!(path = path.to_string_lossy()));
            let meta = entry.metadata().await.context_with("Error reading metadata", ea!(path = path.dbg_str()))?;
            if !meta.is_file() {
                return Ok(());
            }
            let Some(hash) = get_hash_from_file_path(&log, &state.files_dir, &path) else {
                return Ok(());
            };
            batch.insert(hash.clone(), path);
            if batch.len() >= 1000 {
                clean_batch(&log, &state.db, &mut batch).await?;
            }
            return Ok(());
        }.await {
            Ok(_) => { },
            Err(e) => {
                log.log_err(
                    loga::WARN,
                    e.context_with("Unable to process file in files_dir", ea!(root = state.files_dir.dbg_str())),
                );
            },
        }
    }
    if !batch.is_empty() {
        clean_batch(&log, &state.db, &mut batch).await?;
    }

    // Clean up unreferenced generated files
    let mut walk = WalkDir::new(&state.genfiles_dir);
    while let Some(entry) = walk.next().await {
        match async {
            ta_return!((), loga::Error);
            let entry = entry?;
            let path = entry.path();
            let log = log.fork(ea!(path = path.to_string_lossy()));
            if !entry.metadata().await.stack_context(&log, "Error reading metadata")?.is_file() {
                return Ok(());
            }
            let Some(hash) = get_hash_from_file_path(&log, &state.genfiles_dir, &path) else {
                return Ok(());
            };
            if !file_path(&state, &hash).unwrap().exists() {
                log.log_with(loga::DEBUG, "Garbage collecting generated file", ea!(file = path.dbg_str()));
                remove_file(&path)
                    .await
                    .log_with(
                        &log,
                        loga::WARN,
                        "Failed to delete unreferenced generated file",
                        ea!(path = path.display().to_string()),
                    );
            }
            return Ok(());
        }.await {
            Ok(_) => { },
            Err(e) => {
                log.log_err(
                    loga::WARN,
                    e.context_with(
                        "Unable to process file in genfiles_dir",
                        ea!(root = state.genfiles_dir.dbg_str()),
                    ),
                );
            },
        }
    }

    // Clean up stale partially-uploaded files
    let mut walk = WalkDir::new(&state.stage_dir);
    while let Some(entry) = walk.next().await {
        match async {
            ta_return!((), loga::Error);
            let entry = entry?;
            let path = entry.path();
            let log = log.fork(ea!(path = path.to_string_lossy()));
            let meta = entry.metadata().await.context_with("Error reading metadata", ea!(path = path.dbg_str()))?;
            if !meta.is_file() {
                return Ok(());
            }
            let modified_time = match meta.modified() {
                Ok(t) => t,
                Err(e) => {
                    log.log_err(
                        loga::WARN,
                        e.context_with(
                            "Error reading file modified time, assuming old/corrupt and removing",
                            ea!(path = path.dbg_str()),
                        ),
                    );
                    SystemTime::UNIX_EPOCH
                },
            };
            if SystemTime::now().duration_since(modified_time).unwrap_or(Duration::from_secs(60 * 60 * 24 * 100)) >
                std::time::Duration::from_secs(60 * 60 * 24 * 3) {
                log.log_with(loga::DEBUG, "Garbage collecting stale partial upload", ea!(file = path.dbg_str()));
                remove_file(&path)
                    .await
                    .log_with(
                        &log,
                        loga::WARN,
                        "Failed to delete stale partial upload file",
                        ea!(path = path.display().to_string()),
                    );
            }
            return Ok(());
        }.await {
            Ok(_) => { },
            Err(e) => {
                log.log_err(
                    loga::WARN,
                    e.context_with("Unable to process file in stage_dir", ea!(root = state.stage_dir.dbg_str())),
                );
            },
        }
    }

    // Don
    return Ok(());
}
