use {
    loga::{
        ea,
        DebugDisplay,
        ErrContext,
        Log,
        ResultContext,
    },
    std::{
        future::Future,
        path::Path,
    },
    tokio::fs::{
        create_dir_all,
        read_dir,
        remove_dir_all,
        DirEntry,
    },
};

pub async fn create_dirs(path: &Path) -> Result<(), loga::Error> {
    create_dir_all(path).await.context_with("Failed to create directory", ea!(path = path.dbg_str()))?;
    return Ok(());
}

pub async fn delete_tree(path: &Path) -> Result<(), loga::Error> {
    match remove_dir_all(path).await {
        Ok(_) => {
            return Ok(());
        },
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory => {
                return Ok(());
            },
            _ => {
                return Err(e.context_with("Error removing file tree", ea!(path = path.dbg_str())));
            },
        },
    }
}

pub async fn soft_read_dir<
    T: Future<Output = Result<(), loga::Error>>,
    F: FnMut(DirEntry) -> T,
>(log: &Log, d: &Path, mut f: F) {
    let log = log.fork(ea!(dir = d.dbg_str()));
    let d = d.to_path_buf();
    let mut rd = match read_dir(&d).await {
        Ok(x) => x,
        Err(e) => {
            log.log_err(loga::WARN, e.context("Error opening directory for reading"));
            return;
        },
    };
    loop {
        let entry = match rd.next_entry().await {
            Ok(e) => e,
            Err(e) => {
                log.log_err(loga::WARN, e.context("Error reading next directory entry"));
                return;
            },
        };
        let Some(entry) = entry else {
            break;
        };
        match f(entry).await {
            Ok(_) => { },
            Err(e) => {
                log.log_err(loga::WARN, e.context("Error processing directory entry"));
            },
        }
    }
}
