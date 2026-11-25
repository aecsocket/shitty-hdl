use std::{cell::RefCell, fmt, marker::PhantomData, rc::Rc};

use astli_syntax::{Lexer, SyntaxKind};
use derive_more::{Display, Error};
use lasso::{Rodeo, Spur};

thread_local! {
    static STRINGS: RefCell<Rodeo> = RefCell::new(Rodeo::default());
}

/// An identifier interned for the lifetime of the current thread.
///
/// Identifiers cannot cross threads, since their keys belong to a thread-local
/// interner. Names are escaped only when they are not ordinary non-keyword
/// SystemVerilog identifiers.
///
/// ```compile_fail
/// let name = verilog_gen::Ident::new("counter");
/// std::thread::spawn(move || name.to_string());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ident {
    key: Spur,
    escaped: bool,
    _thread_local: PhantomData<Rc<()>>,
}

#[derive(Debug, Display, Error, Clone, Copy, PartialEq, Eq)]
#[display("must be nonempty printable ASCII without whitespace")]
pub struct BadIdent;

impl Ident {
    /// Interns a nonempty name consisting of printable ASCII characters.
    ///
    /// # Errors
    /// Returns `BadIdent` if the name is empty or contains non-printable ASCII,
    /// whitespace, or non-ASCII characters.
    pub fn try_new(name: &str) -> Result<Self, BadIdent> {
        if name.is_empty() || !name.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(BadIdent);
        }
        let tokens = Lexer::new(name).tokenize();
        let ordinary = tokens
            .first()
            .is_some_and(|token| token.kind == SyntaxKind::IDENT && token.text(name) == name)
            && name.bytes().enumerate().all(|(index, byte)| {
                byte.is_ascii_alphabetic()
                    || byte == b'_'
                    || (index > 0 && (byte.is_ascii_digit() || byte == b'$'))
            });
        Ok(Self {
            key: STRINGS.with_borrow_mut(|strings| strings.get_or_intern(name)),
            escaped: !ordinary,
            _thread_local: PhantomData,
        })
    }

    /// Interns a name, panicking if it is invalid.
    ///
    /// # Panics
    /// Panics if `Self::try_new` rejects the name.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self::try_new(name)
            .expect("SystemVerilog identifiers must be nonempty printable ASCII without whitespace")
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Release the interner borrow before invoking a potentially reentrant writer.
        let name = STRINGS.with_borrow(|strings| strings.resolve(&self.key).to_owned());
        if self.escaped {
            // Whitespace terminates the escaped identifier, even before punctuation.
            write!(f, "\\{name} ")
        } else {
            f.write_str(&name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interns_repeated_names() {
        assert_eq!(Ident::new("count"), Ident::new("count"));
        assert_ne!(Ident::new("count"), Ident::new("clk"));
        assert_eq!(Ident::new("module").to_string(), "\\module ");
    }

    #[test]
    fn escapes_only_when_needed() {
        for name in ["counter", "count_2", "_temp", "a$b", "Module", "WIDTH"] {
            assert_eq!(Ident::new(name).to_string(), name);
        }
        for name in [
            "module",
            "logic",
            "always_ff",
            "checker",
            "soft",
            "123count",
            "$count",
            "counter.count",
            "foo-bar",
            "a\\b",
        ] {
            let rendered = Ident::new(name).to_string();
            assert_eq!(rendered, format!("\\{name} "));
            let tree = astli_parse::SyntaxTree::parse(
                "ident.sv",
                format!("module {rendered}(); endmodule"),
            );
            assert!(
                tree.diagnostics().is_empty(),
                "{name}: {:#?}",
                tree.diagnostics()
            );
        }
    }

    #[test]
    fn validates_names() {
        for name in ["", "bad name", "tab\t", "line\n", "\0", "é"] {
            assert_eq!(Ident::try_new(name), Err(BadIdent));
        }
        assert_eq!(Ident::try_new("counter").unwrap(), Ident::new("counter"));
        assert_eq!(
            BadIdent.to_string(),
            "must be nonempty printable ASCII without whitespace"
        );
        let _: &dyn std::error::Error = &BadIdent;
    }

    #[test]
    #[should_panic(expected = "SystemVerilog identifiers")]
    fn rejects_whitespace() {
        let _ = Ident::new("bad name");
    }

    #[test]
    fn independent_threads_can_format_names() {
        let name = Ident::new("parent");
        let child = std::thread::spawn(|| Ident::new("child").to_string());
        assert_eq!(child.join().unwrap(), "child");
        assert_eq!(name.to_string(), "parent");
    }
}
