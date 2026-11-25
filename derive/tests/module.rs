#![allow(dead_code, non_camel_case_types)]

// A minimal control-flow API lets these tests check the macro independently of
// the main HDL's in-progress expression implementation.
mod __ {
    pub trait Module {}

    pub fn comb(_callback: impl FnMut()) {}

    pub struct Clock;

    impl Clock {
        pub fn on(&self, _callback: impl FnMut()) {}
    }

    pub trait Shape {}
    impl Shape for () {}
    impl Shape for u8 {}
    impl Shape for u16 {}
    impl Shape for u32 {}

    pub struct Expr<T: Shape>(std::marker::PhantomData<T>);

    pub trait IntoExpr {
        type Out: Shape;
    }

    impl<T: Shape> IntoExpr for T {
        type Out = T;
    }

    impl<T: Shape> IntoExpr for Expr<T> {
        type Out = T;
    }

    pub struct If<T: Shape>(std::marker::PhantomData<T>);

    pub fn if_<R: IntoExpr>(_condition: bool, _then: impl FnOnce() -> R) -> If<R::Out> {
        If(std::marker::PhantomData)
    }

    impl<T: Shape> If<T> {
        pub fn else_if<R: IntoExpr<Out = T>>(
            self,
            _condition: bool,
            _then: impl FnOnce() -> R,
        ) -> Self {
            self
        }

        pub fn else_<R: IntoExpr<Out = T>>(self, _otherwise: impl FnOnce() -> R) -> Expr<T> {
            unreachable!("elaboration functions are only type-checked in these tests")
        }
    }
}

#[shitty_hdl_derive::module(crate = "crate")]
fn expression_values(condition: bool, other: bool) {
    let x = if condition { 1_u8 } else { 2_u8 };
    let _: __::Expr<u8> = x;

    let y = if condition {
        1_u16
    } else if other {
        2_u16
    } else {
        3_u16
    };
    let _: __::Expr<u16> = y;

    let nested = if condition {
        if other { 1_u32 } else { 2_u32 }
    } else {
        3_u32
    };
    let _: __::Expr<u32> = nested;

    let nested_else = if condition {
        1_u32
    } else {
        if other { 2_u32 } else { 3_u32 }
    };
    let _: __::Expr<u32> = nested_else;

    let value_in_argument = Some(if condition { 1_u8 } else { 2_u8 });
    let _: Option<__::Expr<u8>> = value_in_argument;

    // A bare if/else-if still yields unit, not an unfinished If<()> builder.
    let unit: () = if condition {
        let _ = 1;
    };
    let unit_chain: () = if condition {
        let _ = 1;
    } else if other {
        let _ = 2;
    };
    let _ = (unit, unit_chain);

    if condition {
        let _ = 1;
    }
    if condition {
        let _ = 1;
    } else {
        let _ = 2;
    }
}

#[shitty_hdl_derive::module(crate = "crate")]
fn tail_unit(condition: bool) {
    if condition {
    } else {
    }
}

#[shitty_hdl_derive::module(crate = "crate")]
fn statement_contexts(condition: bool, other: bool) {
    let both: __::Expr<u8> = if condition {
        if other {
        } else {
        }
        if other { 1_u8 } else { 2_u8 }
    } else {
        if other {
        } else {
        };
        if other { 3_u8 } else { 4_u8 }
    };
    let _: __::Expr<u8> = both;

    let _: __::Expr<()> = if condition {
    } else {
    };
    let _: () = {
        if condition {
        } else {
        }
        let _ = 1;
    };

    let returning = || -> __::Expr<u8> {
        return if condition { 1_u8 } else { 2_u8 };
    };
    let _: __::Expr<u8> = returning();
    let value_closure = || {
        if condition { 1_u8 } else { 2_u8 }
    };
    let _: __::Expr<u8> = value_closure();

    // Explicit semicolons still discard completed chains.
    if condition {
    } else {
    };
    // A root block's tail is a value, but the root statement discards it.
    {
        if condition {
        } else {
        }
    }
    {
        if condition {
        } else {
        }
    }
}

#[shitty_hdl_derive::module(crate = "crate")]
fn unit_callbacks(condition: bool, other: bool) {
    use __::comb;
    let clk = __::Clock;

    comb(|| {
        if condition {
        } else {
        }
    });
    __::comb(|| {
        if condition {
        } else {
        }
    });
    comb(|| -> () {
        if condition {
        } else if other {
        } else {
        }
    });
    clk.on(|| {
        if condition {
        } else {
        }
    });
    clk.on(|| {
        if condition {
        } else {
        }
    });
    clk.on(|| -> () {
        if condition {
        } else {
        }
    });

    comb(|| {
        let value = || if condition { 1_u8 } else { 2_u8 };
        let _: __::Expr<u8> = value();
        clk.on(|| {
            if other {
            } else {
            }
        });
        if condition {
        } else {
        }
    });
}

#[test]
fn modules_with_value_conditionals_compile() {
    fn assert_module<T: __::Module>() {}
    assert_module::<expression_values>();
    assert_module::<tail_unit>();
    assert_module::<statement_contexts>();
    assert_module::<unit_callbacks>();
}
