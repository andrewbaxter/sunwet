use {
    loga::{
        ea,
        Log,
        ResultContext,
    },
    sha2::{
        Digest,
        Sha256,
    },
    shared::interface::triple::FileHash,
    std::{
        io::Write,
        path::{
            Path,
            PathBuf,
        },
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
    return Ok(FileHash::Sha256(got_hash));
}
