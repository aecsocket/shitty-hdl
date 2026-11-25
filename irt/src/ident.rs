use std::{cell::RefCell, fmt, sync::LazyLock};

use lasso::{Rodeo, Spur};

thread_local! {
    static RODEO: LazyLock<RefCell<Rodeo>> = LazyLock::new(|| RefCell::new(Rodeo::new()));
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Ident(Spur);

impl Ident {
    #[must_use]
    pub fn new(s: impl AsRef<str>) -> Self {
        let spur = RODEO.with(|rodeo| rodeo.borrow_mut().get_or_intern(s));
        Self(spur)
    }
}

impl<T: AsRef<str>> From<T> for Ident {
    fn from(s: T) -> Self {
        Self::new(s)
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
