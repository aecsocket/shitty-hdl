/// A source span's index in the `OriginMap` that created it.
///
/// Origins must be resolved against that same map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Origin(pub(crate) u32);

/// A one-based line and column in a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

/// A half-open source range. Both positions refer to the same file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    pub file: String,
    pub start: Position,
    pub end: Position,
}

/// Source spans owned by one generated design.
#[derive(Debug, Default)]
pub struct OriginMap {
    spans: Vec<Span>,
}

impl OriginMap {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a span and returns its origin handle.
    ///
    /// # Panics
    /// Panics for zero line/column coordinates, reversed ranges, or if the
    /// number of origins exceeds what a `u32` can represent.
    pub fn insert(&mut self, span: Span) -> Origin {
        assert!(
            span.start.line > 0
                && span.start.column > 0
                && span.end.line > 0
                && span.end.column > 0,
            "source positions must be one-based",
        );
        assert!(span.start <= span.end, "source span is reversed");
        let origin = Origin(self.spans.len().try_into().expect("too many origins"));
        self.spans.push(span);
        origin
    }

    #[must_use]
    pub fn get(&self, origin: Origin) -> Option<&Span> {
        self.spans.get(origin.0 as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_spans() {
        let mut origins = OriginMap::new();
        let span = Span {
            file: "src/counter.rs".into(),
            start: Position { line: 1, column: 1 },
            end: Position { line: 5, column: 2 },
        };
        let first = origins.insert(span.clone());
        let second = origins.insert(span.clone());
        assert_ne!(first, second);
        assert_eq!(origins.get(first), Some(&span));
        assert_eq!(origins.get(second), Some(&span));
        assert_eq!(origins.get(Origin(2)), None);
    }
}
