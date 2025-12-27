/// Source location span (byte offsets)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// Trait for types that have an associated span.
///
/// Implement this for any AST node or token type that carries location information.
/// Used by error builders to automatically extract spans from nodes.
pub trait HasSpan {
    /// Returns the span for this item.
    fn span(&self) -> Span;
}

impl HasSpan for Span {
    fn span(&self) -> Span {
        *self
    }
}

impl Span {
    pub const fn new(
        start: usize,
        end: usize,
    ) -> Self {
        Self { start, end }
    }

    pub const fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    pub fn merge(
        &self,
        other: &Self,
    ) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl From<(usize, usize)> for Span {
    fn from((start, end): (usize, usize)) -> Self {
        Self::new(start, end)
    }
}

impl From<std::ops::Range<usize>> for Span {
    fn from(range: std::ops::Range<usize>) -> Self {
        Self::new(range.start, range.end)
    }
}

impl From<Span> for miette::SourceSpan {
    fn from(span: Span) -> Self {
        (span.start, span.len()).into()
    }
}

/// Source attachment for errors with file context
#[derive(Debug, Clone)]
pub struct SourceAttachment {
    pub path: std::path::PathBuf,
    pub source: std::sync::Arc<String>,
}

impl SourceAttachment {
    pub fn new(
        path: impl Into<std::path::PathBuf>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            source: std::sync::Arc::new(source.into()),
        }
    }

    pub fn from_arc(
        path: impl Into<std::path::PathBuf>,
        source: std::sync::Arc<String>,
    ) -> Self {
        Self {
            path: path.into(),
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_basics() {
        let span = Span::new(10, 20);
        assert_eq!(span.len(), 10);
        assert!(!span.is_empty());
    }

    #[test]
    fn span_merge() {
        let a = Span::new(10, 20);
        let b = Span::new(15, 30);
        let merged = a.merge(&b);
        assert_eq!(merged.start, 10);
        assert_eq!(merged.end, 30);
    }

    #[test]
    fn span_conversions() {
        let span: Span = (5, 15).into();
        assert_eq!(span.start, 5);
        assert_eq!(span.end, 15);

        let span: Span = (0..10).into();
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 10);
    }
}
