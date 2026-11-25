#![allow(dead_code)]

extern crate self as shitty_hdl;

pub mod value {
    pub type ShapeSize = u32;

    pub trait Shape {
        const WIDTH: u32;
        type View: Copy;
    }

    impl Shape for bool {
        const WIDTH: u32 = 1;
        type View = bool;
    }

    impl Shape for u8 {
        const WIDTH: u32 = 8;
        type View = u8;
    }

    impl Shape for u16 {
        const WIDTH: u32 = 16;
        type View = u16;
    }
}

use value::Shape;

// Deliberately neither Copy nor Clone: only its view may be copied.
pub struct NonCopy(String);

impl Shape for NonCopy {
    const WIDTH: u32 = 5;
    type View = u8;
}

pub struct Marker<'a, T, const N: usize>(std::marker::PhantomData<&'a T>);

impl<T, const N: usize> Shape for Marker<'_, T, N> {
    const WIDTH: u32 = N as u32;
    type View = bool;
}

#[derive(shitty_hdl_derive::Shape)]
struct Named {
    flag: bool,
    byte: u8,
    word: u16,
}

#[derive(shitty_hdl_derive::Shape)]
struct Tuple(bool, u16, NonCopy);

#[derive(shitty_hdl_derive::Shape)]
struct Unit;

#[derive(shitty_hdl_derive::Shape)]
struct Nested {
    named: Named,
    tuple: Tuple,
    unit: Unit,
}

mod public_shapes {
    use super::{Marker, NonCopy};

    // T has no explicit Shape bound; the derive must add field-type bounds.
    #[derive(shitty_hdl_derive::Shape)]
    pub struct Generic<'a, T = NonCopy, const N: usize = 3>
    where
        T: 'a + Sized,
    {
        pub payload: T,
        pub marker: Marker<'a, T, N>,
        pub enabled: bool,
    }

    #[derive(shitty_hdl_derive::Shape)]
    pub struct GenericTuple<T = NonCopy>(pub T, pub bool)
    where
        T: Sized;

    #[derive(shitty_hdl_derive::Shape)]
    pub struct PublicUnit;

    pub mod restricted {
        #[derive(shitty_hdl_derive::Shape)]
        pub(super) struct Restricted {
            pub(super) flag: bool,
        }
    }

    pub fn restricted_view_is_accessible() -> bool {
        let view = restricted::RestrictedView { flag: true };
        view.flag
    }
}

#[derive(shitty_hdl_derive::Shape)]
#[shape(crate = "crate")]
struct Override {
    payload: NonCopy,
    flag: bool,
}

fn expect_view<S: Shape>(view: S::View) -> S::View {
    view
}

fn assert_copy_clone<T: Copy + Clone>() {}

fn clone_view<T: Clone>(view: &T) -> T {
    view.clone()
}

#[test]
fn named_fields_have_their_shape_views_and_summed_width() {
    assert_eq!(Named::WIDTH, 25);
    let view = expect_view::<Named>(NamedView {
        flag: true,
        byte: 42,
        word: 1024,
    });
    assert!(view.flag);
    assert_eq!(view.byte, 42u8);
    assert_eq!(view.word, 1024u16);
    assert_copy_clone::<NamedView>();
}

#[test]
fn tuple_and_unit_views_have_the_expected_structure() {
    assert_eq!(Tuple::WIDTH, 22);
    let view = expect_view::<Tuple>(TupleView(true, 512, 7));
    assert!(view.0);
    assert_eq!(view.1, 512u16);
    assert_eq!(view.2, 7u8);
    assert_copy_clone::<TupleView>();

    assert_eq!(Unit::WIDTH, 0);
    let UnitView = expect_view::<Unit>(UnitView);
    assert_copy_clone::<UnitView>();
}

#[test]
fn nested_shapes_use_generated_views_recursively() {
    assert_eq!(Nested::WIDTH, 47);
    let view = expect_view::<Nested>(NestedView {
        named: NamedView {
            flag: true,
            byte: 3,
            word: 4,
        },
        tuple: TupleView(false, 5, 6),
        unit: UnitView,
    });
    let _: NamedView = view.named;
    let _: TupleView = view.tuple;
    let _: UnitView = view.unit;
    assert_copy_clone::<NestedView>();
}

#[test]
fn public_generics_preserve_defaults_lifetimes_consts_and_where_clauses() {
    use public_shapes::{Generic, GenericView};

    assert_eq!(<Generic<'static> as Shape>::WIDTH, 9);
    let view: GenericView<'static> = expect_view::<Generic<'static>>(GenericView {
        payload: 7,
        marker: true,
        enabled: false,
    });
    assert_copy_clone::<GenericView<'static>>();
    let copied = view;
    let cloned = clone_view(&view);
    assert_eq!(copied.payload, cloned.payload);
    assert!(view.marker);
    assert!(!view.enabled);

    assert_eq!(Generic::<'static, u16, 11>::WIDTH, 28);
    let view = expect_view::<Generic<'static, u16, 11>>(GenericView {
        payload: 1024,
        marker: false,
        enabled: true,
    });
    assert_eq!(view.payload, 1024u16);
    assert_copy_clone::<GenericView<'static, u16, 11>>();
}

#[test]
fn public_tuple_and_unit_views_are_accessible_without_copy_shape_bounds() {
    use public_shapes::{GenericTuple, GenericTupleView, PublicUnit, PublicUnitView};

    assert_eq!(GenericTuple::<NonCopy>::WIDTH, 6);
    let view: GenericTupleView = expect_view::<GenericTuple>(GenericTupleView(9, true));
    assert_eq!(view.0, 9u8);
    assert!(view.1);
    assert_copy_clone::<GenericTupleView>();
    let copied = view;
    let cloned = clone_view(&view);
    assert_eq!(copied.0, cloned.0);

    assert_eq!(PublicUnit::WIDTH, 0);
    let PublicUnitView = expect_view::<PublicUnit>(PublicUnitView);
    assert_copy_clone::<PublicUnitView>();
    assert!(public_shapes::restricted_view_is_accessible());
}

#[test]
fn quoted_crate_override_resolves_the_local_shape_trait() {
    assert_eq!(Override::WIDTH, 6);
    let view = expect_view::<Override>(OverrideView {
        payload: 3,
        flag: true,
    });
    assert_eq!(view.payload, 3u8);
    assert!(view.flag);
    assert_copy_clone::<OverrideView>();
}
