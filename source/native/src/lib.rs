use futures::Future;
use tokio::{
    select,
    spawn,
    sync::oneshot,
};

pub mod interface;

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

trait ScopeValue_: Send + Sync { }

impl<T: 'static + Send + Sync> ScopeValue_ for T { }

pub struct ScopeValue(
    #[allow(dead_code)]
    Box<dyn ScopeValue_>,
);

struct ScopeBg {
    kill: Option<oneshot::Sender<()>>,
}

impl Drop for ScopeBg {
    fn drop(&mut self) {
        _ = self.kill.take().unwrap().send(());
    }
}

pub fn spawn_scoped<F: 'static + Send + Future<Output = ()>>(f: F) -> ScopeValue {
    let (kill, killed) = oneshot::channel();
    spawn(async move {
        select!{
            _ = killed => {
            },
            _ = f => {
            }
        }
    });
    return ScopeValue(Box::new(ScopeBg { kill: Some(kill) }));
}
