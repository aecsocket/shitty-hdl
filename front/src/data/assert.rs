use crate::elab;

#[macro_export]
macro_rules! display {
    ($($args:tt)*) => {
        $crate::__::display_format!($crate; $($args)*)
    };
}

#[macro_export]
macro_rules! sim_assert {
    ($cond:expr) => {
        $crate::__::assert($cond, stringify!($cond))
    };
    ($cond:expr, $msg:literal) => {
        $crate::__::assert($cond, $msg)
    };
}

pub fn sim_finish() {
    elab::finish();
}
