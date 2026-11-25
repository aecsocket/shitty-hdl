#![allow(dead_code)]

use shitty_hdl::{prelude::*, signal::SignalView};

#[derive(Shape)]
struct Foo {
    lo: U<17>,
    hi: U<19>,
}

#[derive(Shape)]
struct Nested {
    data: Foo,
    flag: bool,
}

#[derive(Shape)]
struct Generic<T> {
    value: T,
}

fn assert_copy<T: Copy>() {}

// These signatures check the actual HDL field types, not just the mock trait
// used by the proc-macro crate's standalone tests.
fn check_fields(view: FooView) {
    let _: SignalView<U<17>> = view.lo;
    let _: SignalView<U<19>> = view.hi;
}

fn check_nested(view: NestedView) {
    let _: FooView = view.data;
    let _: SignalView<bool> = view.flag;
}

#[test]
fn derives_actual_hdl_shapes() {
    assert_eq!(Foo::WIDTH, 36);
    assert_eq!(Nested::WIDTH, 37);
    assert_eq!(Generic::<Foo>::WIDTH, 36);
    assert_copy::<FooView>();
    assert_copy::<NestedView>();
    assert_copy::<GenericView<Foo>>();
}
