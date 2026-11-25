use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{
    domain::Domain,
    elab,
    module::LoopDomain,
    value::{IntoValues, Shape, Value},
};

pub trait JoinOutput: Sized {
    type Domain: Domain;
    type Out: Shape;

    fn pack_join(self) -> Value<Self::Domain, Self::Out>;

    fn unpack_join(value: Value<Self::Domain, Self::Out>) -> Self;
}

impl<D: Domain, T: Shape> JoinOutput for Value<D, T> {
    type Domain = D;
    type Out = T;

    fn pack_join(self) -> Value<Self::Domain, Self::Out> {
        self
    }

    fn unpack_join(value: Value<Self::Domain, Self::Out>) -> Self {
        value
    }
}

impl JoinOutput for () {
    type Domain = LoopDomain;
    type Out = ();

    fn pack_join(self) -> Value<LoopDomain, ()> {
        elab::literal(&())
    }

    fn unpack_join(_: Value<LoopDomain, ()>) -> Self {}
}

macro_rules! impl_join_output_tuple {
    ($($name:ident),*) => {
        impl<D: Domain, $($name: Shape),*> JoinOutput for ($(Value<D, $name>,)*)
        where
            $($name: Shape<Values<D> = Value<D, $name>>),*
        {
            type Domain = D;
            type Out = ($($name,)*);

            fn pack_join(self) -> Value<D, Self::Out> {
                let values = <Self as IntoValues<D>>::into_values(self);
                <Self::Out as Shape>::pack_values::<D>(values)
            }

            fn unpack_join(value: Value<D, Self::Out>) -> Self {
                <Self::Out as Shape>::make_values::<D>(value)
            }
        }
    };
}

impl_join_output_tuple!(T0);
impl_join_output_tuple!(T0, T1);
impl_join_output_tuple!(T0, T1, T2);
impl_join_output_tuple!(T0, T1, T2, T3);
impl_join_output_tuple!(T0, T1, T2, T3, T4);
impl_join_output_tuple!(T0, T1, T2, T3, T4, T5);
impl_join_output_tuple!(T0, T1, T2, T3, T4, T5, T6);
impl_join_output_tuple!(T0, T1, T2, T3, T4, T5, T6, T7);

pub trait JoinTuple<'a> {
    type Output;

    fn join(self) -> Pin<Box<dyn Future<Output = Self::Output> + 'a>>;
}

pub fn join<'a, T: JoinTuple<'a>>(tuple: T) -> Pin<Box<dyn Future<Output = T::Output> + 'a>> {
    tuple.join()
}

impl<'a> JoinTuple<'a> for () {
    type Output = ();

    fn join(self) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        struct Unit;

        impl Future for Unit {
            type Output = ();

            fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
                Poll::Ready(())
            }
        }

        Box::pin(Unit)
    }
}

macro_rules! join_tuple {
    ($(($name:ident, $index:tt)),+) => {
        impl<'a, $($name),*> JoinTuple<'a> for ($($name,)*)
        where
            $($name: Future + 'a,)*
            $($name::Output: JoinOutput,)*
        {
            type Output = ($($name::Output,)*);

            fn join(self) -> Pin<Box<dyn Future<Output = Self::Output> + 'a>> {
                struct Slot<F> {
                    handle: elab::RegisterHandle,
                    future: PhantomData<fn() -> F>,
                }

                struct JoinFuture<$($name: Future,)*> {
                    futures: ($(Option<Pin<Box<$name>>>,)*),
                    results: ($(Slot<$name>,)*),
                    flushed: bool,
                }

                impl<$($name: Future,)*> Future for JoinFuture<$($name,)*>
                where
                    $($name::Output: JoinOutput,)*
                {
                    type Output = ($($name::Output,)*);

                    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                        let this = self.get_mut();
                        let mut ready = true;
                        $(
                            if let Some(future) = this.futures.$index.as_mut() {
                                match future.as_mut().poll(cx) {
                                    Poll::Ready(value) => {
                                        this.futures.$index = None;
                                        elab::write_register::<
                                            <$name::Output as JoinOutput>::Domain,
                                            <$name::Output as JoinOutput>::Out,
                                        >(
                                            this.results.$index.handle,
                                            value.pack_join(),
                                        );
                                    }
                                    Poll::Pending => ready = false,
                                }
                            }
                        )*
                        if ready {
                            if this.flushed {
                                Poll::Ready(($(
                                    <$name::Output as JoinOutput>::unpack_join(
                                        elab::read_register::<
                                            <$name::Output as JoinOutput>::Domain,
                                            <$name::Output as JoinOutput>::Out,
                                        >(this.results.$index.handle),
                                    ),)*
                                ))
                            } else {
                                this.flushed = true;
                                Poll::Pending
                            }
                        } else {
                            Poll::Pending
                        }
                    }
                }

                let results = ($(elab::register_default::<
                    <$name::Output as JoinOutput>::Domain,
                    <$name::Output as JoinOutput>::Out,
                >(None).0,)*);

                Box::pin(JoinFuture {
                    futures: ($(Some(Box::pin(self.$index)),)*),
                    results: ($(Slot {
                        handle: results.$index,
                        future: PhantomData,
                    },)*),
                    flushed: false,
                })
            }
        }
    };
}

join_tuple!((F0, 0));
join_tuple!((F0, 0), (F1, 1));
join_tuple!((F0, 0), (F1, 1), (F2, 2));
join_tuple!((F0, 0), (F1, 1), (F2, 2), (F3, 3));
join_tuple!((F0, 0), (F1, 1), (F2, 2), (F3, 3), (F4, 4));
join_tuple!((F0, 0), (F1, 1), (F2, 2), (F3, 3), (F4, 4), (F5, 5));
join_tuple!(
    (F0, 0),
    (F1, 1),
    (F2, 2),
    (F3, 3),
    (F4, 4),
    (F5, 5),
    (F6, 6)
);
join_tuple!(
    (F0, 0),
    (F1, 1),
    (F2, 2),
    (F3, 3),
    (F4, 4),
    (F5, 5),
    (F6, 6),
    (F7, 7)
);
