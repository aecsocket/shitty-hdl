#[macro_export]
macro_rules! sim_assert {
    ($cond:expr) => {
        $crate::__::assert($cond, stringify!($cond))
    };
    ($cond:expr, $msg:literal) => {
        $crate::__::assert($cond, $msg)
    };
}
