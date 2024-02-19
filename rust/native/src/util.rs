use std::{
    mem::swap,
    path::Path,
    task::Poll,
    io::Write,
};
use loga::{
    FlagStyle,
    ResultContext,
};
use sha2::{
    Sha256,
    Digest,
};
use shared::model::FileHash;
use tokio::{
    fs::File,
    io::{
        AsyncWrite,
        copy,
    },
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Flag {
    Warn,
    Info,
    Debug,
}

impl loga::Flag for Flag {
    fn style(self) -> loga::FlagStyle {
        match self {
            Flag::Debug => FlagStyle {
                body_style: loga::republish::console::Style::new().for_stderr().black().bright(),
                label_style: loga::republish::console::Style::new().for_stderr().black().bright(),
                label: "DEBUG",
            },
            Flag::Info => FlagStyle {
                body_style: loga::republish::console::Style::new().for_stderr().black(),
                label_style: loga::republish::console::Style::new().for_stderr().black(),
                label: "INFO",
            },
            Flag::Warn => FlagStyle {
                body_style: loga::republish::console::Style::new().for_stderr().black(),
                label_style: loga::republish::console::Style::new().for_stderr().yellow(),
                label: "WARN",
            },
        }
    }
}

pub type Log = loga::Log<Flag>;

pub trait VecTake<T> {
    fn take(&mut self) -> Vec<T>;
}

impl<T> VecTake<T> for Vec<T> {
    fn take(&mut self) -> Vec<T> {
        let mut out = vec![];
        swap(self, &mut out);
        return out;
    }
}

/// Explicitly capturing async closure - clones elements in the second parens into
/// the closure. Anything else will be moved.
#[macro_export]
macro_rules! cap_fn{
    (($($a: pat_param), *)($($cap: ident), *) {
        $($t: tt) *
    }) => {
        {
            $(let $cap = $cap.clone();) * move | $($a),
            *| {
                $(let $cap = $cap.clone();) * async move {
                    $($t) *
                }
            }
        }
    };
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

/// Explicitly communicate the async block return type to the compiler via
/// unexecuting code.
#[macro_export]
macro_rules! ta_res{
    ($t: ty) => {
        if false {
            fn unreachable_value<T>() -> T {
                panic!();
            }
            return std:: result:: Result::< $t,
            loga::Error > ::Ok(unreachable_value());
        }
    }
}
