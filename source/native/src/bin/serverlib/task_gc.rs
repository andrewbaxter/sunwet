use {
    crate::serverlib::filesutil::file_path,
    async_walkdir::WalkDir,
    chrono::Utc,
    deadpool_sqlite::Pool,
    flowcontrol::ta_return,
    loga::{
        ea,
        DebugDisplay,
        ErrContext,
        ResultContext,
    },
    std::{
        collections::{
            BTreeMap,
            HashMap,
        },
        path::{
            Component,
            Path,
            PathBuf,
        },
        str::FromStr,
    },
    tokio::{
        fs::remove_file,
        task::spawn_blocking,
    },
    tokio_stream::StreamExt,
};

pub async fn handle_gc(log: &Log, dbc: &Pool, files_dir: &Path, generated_dir: &Path) -> Result<(), loga::Error> {
    // Clean up old triples
    spawn_blocking({
        let dbc = dbc.clone();
        move || {
            ta_res!(());
            dbc
                .run_script(include_str!("gc.cozo"), {
                    let mut m = BTreeMap::new();
                    m.insert("cutoff".to_string(), DataValue::Num(Num::Int(Utc::now().timestamp_micros())));
                    m
                }, cozo::ScriptMutability::Mutable)
                .map_err(|e| loga::err(e.dbg_str()).context("Error running gc query"))?;
            return Ok(());
        }
    }).await??;

    // Clean up unreferenced files
    async fn flush(
        log: &Log,
        dbc: &Db<SqliteStorage>,
        batch: &mut HashMap<FileHash, PathBuf>,
    ) -> Result<(), loga::Error> {
        let db_files =
            DataValue::List(
                batch
                    .keys()
                    .map(|k| DataValue::List(vec![DataValue::Str(k.to_string().into())]))
                    .collect::<Vec<_>>(),
            );
        let found = spawn_blocking({
            let dbc = dbc.clone();
            move || {
                ta_res!(Vec < FileHash >);
                let res =
                    dbc
                        .run_script(include_str!("gc_file_referenced.cozo"), {
                            let mut m = BTreeMap::new();
                            m.insert("files".to_string(), db_files);
                            m
                        }, cozo::ScriptMutability::Immutable)
                        .map_err(|e| loga::err(e.dbg_str()).context("Error running file gc query"))?;
                let mut out = vec![];
                for r in res.rows {
                    let Some(DataValue::Str(hash)) = r.get(0) else {
                        panic!("{:?}", r);
                    };
                    out.push(FileHash::from_str(hash.as_str()).unwrap());
                }
                return Ok(out);
            }
        }).await??;
        for hash in found {
            batch.remove(&hash);
        }
        for path in batch.values() {
            match remove_file(path).await {
                Ok(_) => { },
                Err(e) => {
                    log.log_err(
                        Flag::Warn,
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
            log.log(Flag::Warn, "File in files dir not in hash type directory");
            return None;
        };
        let Some(hash_hash) = components.last().and_then(|c| c.to_str()) else {
            log.log(Flag::Warn, "File in files dir has non-utf8 last path segment");
            return None;
        };
        let hash = match FileHash::from_str(&format!("{}:{}", hash_type, hash_hash)) {
            Ok(h) => h,
            Err(e) => {
                log.log_err(Flag::Warn, loga::err(e).context("Failed to determine hash for file"));
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
                log.log_err(Flag::Warn, e.context("Unable to scan file in files_dir"));
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
            flush(&log, &dbc, &mut batch).await?;
        }
    }
    if !batch.is_empty() {
        flush(&log, &dbc, &mut batch).await?;
    }

    // Clean up unreferenced generated files
    let mut walk = WalkDir::new(&generated_dir);
    while let Some(entry) = walk.next().await {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                log.log_err(Flag::Warn, e.stack_context(log, "Unable to scan file in files_dir"));
                continue;
            },
        };
        let path = entry.path();
        let log = log.fork(ea!(path = path.to_string_lossy()));
        if !entry.metadata().await.stack_context(&log, "Error reading metadata")?.is_file() {
            continue;
        }
        let Some(hash) = get_file_hash(&log, &generated_dir, &path) else {
            continue;
        };
        if !file_path(&files_dir, &hash).unwrap().exists() {
            match remove_file(&path).await {
                Ok(_) => { },
                Err(e) => {
                    log.log_err(
                        Flag::Warn,
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
