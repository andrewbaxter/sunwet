use {
    loga::{
        ea,
        DebugDisplay,
        ResultContext,
    },
    std::path::Path,
    tokio::fs::create_dir_all,
};

pub async fn create_dirs(path: &Path) -> Result<(), loga::Error> {
    create_dir_all(path).await.context_with("Failed to create directory", ea!(path = path.dbg_str()))?;
    return Ok(());
}
