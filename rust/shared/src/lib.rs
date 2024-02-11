pub mod model;

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

#[macro_export]
macro_rules! unenum{
    ($s: expr, $m: pat => $v: expr) => {
        match $s {
            $m => Some($v),
            _ => None,
        }
    }
}
