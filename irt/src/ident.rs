use std::{cell::RefCell, fmt, str::FromStr, sync::LazyLock};

use derive_more::{Display, Error};
use lasso::{Rodeo, Spur};

thread_local! {
    static RODEO: LazyLock<RefCell<Rodeo>> = LazyLock::new(|| RefCell::new(Rodeo::new()));
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Ident(Spur);

#[derive(Debug, Display, Error)]
pub enum InvalidIdent {
    #[display("illegal whitespace character {ch:?} at index {at}")]
    Whitespace { at: usize, ch: char },
}

impl FromStr for Ident {
    type Err = InvalidIdent;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for (at, ch) in s.char_indices() {
            if ch.is_whitespace() {
                return Err(InvalidIdent::Whitespace { at, ch });
            }
        }

        let spur = RODEO.with(|rodeo| rodeo.borrow_mut().get_or_intern(s));
        Ok(Self(spur))
    }
}

impl fmt::Debug for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        RODEO.with(|rodeo| {
            let rodeo = rodeo.borrow();
            fmt::Debug::fmt(&rodeo.resolve(&self.0), f)
        })
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        RODEO.with(|rodeo| {
            let rodeo = rodeo.borrow();
            fmt::Display::fmt(&rodeo.resolve(&self.0), f)
        })
    }
}
