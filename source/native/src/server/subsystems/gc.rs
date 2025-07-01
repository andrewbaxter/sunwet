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
            fsutil::soft_read_dir,
            state::State,
        },
    },
    chrono::Utc,
    deadpool_sqlite::Pool,
    enclose::enclose,
    flowcontrol::{
        exenum,
    },
    loga::{
        ea,
        DebugDisplay,
        ErrContext,
        Log,
        ResultContext,
    },
    shared::{
        interface::triple::{
            FileHash,
            Node,
        },
        steal,
    },
    std::{
        collections::HashMap,
        path::PathBuf,
        sync::{
            Arc,
            Mutex,
        },
        time::{
            SystemTime,
        },
    },
    tokio::fs::{
        remove_dir_all,
        remove_file,
    },
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
    {
        async fn clean_batch(
            log: &Log,
            dbc: &Pool,
            mut batch: HashMap<FileHash, PathBuf>,
        ) -> Result<(), loga::Error> {
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
            return Ok(());
        }

        let batch = Arc::new(Mutex::new(HashMap::new()));
        soft_read_dir(log, &state.files_dir, enclose!((batch) | entry_hashtype | async move {
            soft_read_dir(log, &entry_hashtype.path(), enclose!((batch) | entry_hash1 | async move {
                soft_read_dir(log, &entry_hash1.path(), enclose!((batch) | entry_hash2 | async move {
                    soft_read_dir(log, &entry_hash2.path(), enclose!((batch) | entry | async move {
                        let path = entry.path();
                        let Some(hash) = get_hash_from_file_path(&log, &state.files_dir, &path) else {
                            return Ok(());
                        };
                        let consume_batch = {
                            let mut batch = batch.lock().unwrap();
                            batch.insert(hash.clone(), path);
                            if batch.len() >= 1000 {
                                Some(steal(&mut *batch))
                            } else {
                                None
                            }
                        };
                        if let Some(batch) = consume_batch {
                            clean_batch(&log, &state.db, batch).await?;
                        }
                        return Ok(());
                    })).await;
                    return Ok(());
                })).await;
                return Ok(());
            })).await;
            return Ok(());
        })).await;
        let batch = steal(&mut *batch.lock().unwrap());
        if !batch.is_empty() {
            clean_batch(&log, &state.db, batch).await?;
        }
    }

    // Clean up unreferenced generated files
    soft_read_dir(log, &state.genfiles_dir, |entry_hashtype| async move {
        soft_read_dir(log, &entry_hashtype.path(), |entry_hash1| async move {
            soft_read_dir(log, &entry_hash1.path(), |entry_hash2| async move {
                soft_read_dir(log, &entry_hash2.path(), |entry| async move {
                    let path = entry.path();
                    let Some(hash) = get_hash_from_file_path(&log, &state.genfiles_dir, &path) else {
                        return Ok(());
                    };
                    if !file_path(&state, &hash).unwrap().exists() {
                        log.log(loga::DEBUG, "Garbage collecting generated file");
                        remove_dir_all(&path).await?;
                    }
                    return Ok(());
                }).await;
                return Ok(());
            }).await;
            return Ok(());
        }).await;
        return Ok(());
    }).await;

    // Clean up stale partially-uploaded files
    soft_read_dir(log, &state.stage_dir, |entry| async move {
        let day = std::time::Duration::from_secs(60 * 60 * 24);
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
        if SystemTime::now().duration_since(modified_time).unwrap_or(day * 100) > day * 3 {
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
    }).await;

    // Don
    return Ok(());
}
