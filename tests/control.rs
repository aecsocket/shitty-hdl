#![feature(generic_const_exprs)]
#![allow(incomplete_features, dead_code, unused_variables)]

use shitty_hdl::{
    expr::Expr,
    module::{If, if_},
    prelude::*,
    signal::SignalView,
};

// These helpers are type-checked, not executed: the HDL's elaboration and
// expression constructors still contain placeholders.
fn signal_and_value(condition: Signal<bool>, value: Signal<Uint<17>>) -> Expr<Uint<17>> {
    if_(condition, || value).else_(|| u!(0))
}

fn value_and_signal(condition: Signal<bool>, value: Signal<Uint<17>>) -> Expr<Uint<17>> {
    if_(condition, || u!(0)).else_(|| value)
}

fn expression_and_view(
    condition: Signal<bool>,
    value: Expr<Uint<17>>,
    view: SignalView<Uint<17>>,
) -> Expr<Uint<17>> {
    if_(condition, || value).else_(|| view)
}

fn chain(
    condition: Signal<bool>,
    value: Signal<Uint<17>>,
    expression: Expr<Uint<17>>,
) -> Expr<Uint<17>> {
    if_(condition, || value)
        .else_if(condition, || u!(1))
        .else_if(condition, || expression)
        .else_(|| u!(0))
}

fn unit_branches(condition: Signal<bool>) -> Expr<()> {
    if_(condition, || {}).else_if(condition, || {}).else_(|| {})
}

fn unfinished_unit(condition: Signal<bool>) -> If<()> {
    if_(condition, || {})
}

#[module]
fn expression_module(
    mut clk: Clock,
    condition: In<bool>,
    value: In<Uint<17>>,
    mut output: Out<Uint<17>>,
) {
    let condition = *condition;
    let value = *value;
    let x = if condition { value } else { u!(0) };
    let _: Expr<Uint<17>> = x;
    output <<= x;

    let nested = if condition {
        if condition { value } else { u!(1) }
    } else {
        u!(0)
    };
    let _: Expr<Uint<17>> = nested;

    comb(|| {
        if condition {
            output <<= value;
        } else {
            output <<= u!(0);
        }
    });
    clk.on(|| {
        if condition {
            output <<= value;
        } else {
            output <<= u!(0);
        }
    });

    if condition {
        output <<= value;
    } else {
        output <<= u!(0);
    }
    let _ = 0;

    if condition {
        output <<= value;
    } else {
        output <<= u!(0);
    }
}

#[test]
fn conditional_operand_types_are_accepted() {
    let _ = signal_and_value;
    let _ = value_and_signal;
    let _ = expression_and_view;
    let _ = chain;
    let _ = unit_branches;
    let _ = unfinished_unit;
    fn assert_module<T: shitty_hdl::module::Module>() {}
    assert_module::<expression_module>();
}
