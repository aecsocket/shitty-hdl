macro_rules! make {
    ($raw:ident => $( ( $wrap:ident, $width:literal ))*) => {
        $(
        #[expect(non_camel_case_types, reason = "mirrors core `u*` primitives")]
        pub struct $wrap($raw);

        impl $wrap {
            pub const ZERO: Self = Self(0);
            pub const MAX: Self = Self((1 << $width) - 1);
        }

        impl $wrap {
            #[must_use]
            pub const fn new(n: $raw) -> Option<Self> {
                if n <= Self::MAX.0 {
                    Some($wrap(n))
                } else {
                    None
                }
            }

            #[must_use]
            pub const unsafe fn new_unchecked(n: $raw) -> Self {
                $wrap(n)
            }
        }

        impl TryFrom<$raw> for $wrap {
            type Error = ();

            fn try_from(n: $raw) -> Result<$wrap, Self::Error> {
                Self::new(n).ok_or(())
            }
        }

        impl From<$wrap> for $raw {
            fn from($wrap(n): $wrap) -> $raw {
                n
            }
        }

        #[macro_export]
        macro_rules! $wrap {
            ($n:literal) => {{
                const VALUE: $crate::n::$wrap = match $crate::n::$wrap::new($n) {
                    Some(value) => value,
                    None => panic!(concat!(
                        "value ",
                        stringify!($n),
                        " out of range for ",
                        stringify!($wrap),
                    ))
                };
                VALUE
            }}
        }
        )*
    };
}

make!(u8 =>  ( u1,  1) ( u2,  2) ( u3,  3) ( u4,  4) ( u5,  5) ( u6,  6) ( u7,  7));
make!(u16 => ( u9,  9) (u10, 10) (u11, 11) (u12, 12) (u13, 13) (u14, 14) (u15, 15));
make!(u32 => (u17, 17) (u18, 18) (u19, 19) (u20, 20) (u21, 21) (u22, 22) (u23, 23) (u24, 24)
             (u25, 25) (u26, 26) (u27, 27) (u28, 28) (u29, 29) (u30, 30) (u31, 31));
make!(u64 => (u33, 33) (u34, 34) (u35, 35) (u36, 36) (u37, 37) (u38, 38) (u39, 39) (u40, 40)
             (u41, 41) (u42, 42) (u43, 43) (u44, 44) (u45, 45) (u46, 46) (u47, 47) (u48, 48)
             (u49, 49) (u50, 50) (u51, 51) (u52, 52) (u53, 53) (u54, 54) (u55, 55) (u56, 56)
             (u57, 57) (u58, 58) (u59, 59) (u60, 60) (u61, 61) (u62, 62) (u63, 63));

fn x() {
    let x = u4!(15);
}
