pub mod model;

#[macro_export]
macro_rules! unenum{
    ($s: expr, $m: pat => $v: expr) => {
        match $s {
            $m => Some($v),
            _ => None,
        }
    }
}

#[macro_export]
macro_rules! bb{
    ($l: lifetime _; $($t: tt) *) => {
        $l: loop {
            #[allow(unreachable_code)] break {
                $($t) *
            };
        }
    };
    ($($t: tt) *) => {
        loop {
            #[allow(unreachable_code)] break {
                $($t) *
            };
        }
    };
}
