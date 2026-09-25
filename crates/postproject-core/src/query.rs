//! Bounded keyset pagination shared by named domain queries.

use crate::{Error, ErrorKind, Result};

/// Maximum number of values returned by one query page.
pub const MAX_QUERY_PAGE_SIZE: u32 = 1_000;
/// Maximum encoded size of an opaque continuation cursor.
pub const MAX_QUERY_CURSOR_BYTES: usize = 2_048;
/// Maximum depth accepted by a dependency traversal query.
pub const MAX_DEPENDENCY_QUERY_DEPTH: u32 = 64;
/// Maximum representations visited by a dependency traversal query.
pub const MAX_DEPENDENCY_QUERY_REPRESENTATIONS: u32 = 1_000;

/// Opaque, query-scoped keyset continuation token.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct QueryCursor(String);

impl QueryCursor {
    /// Copies a cursor returned by an earlier page.
    ///
    /// # Errors
    ///
    /// Returns an invalid-argument error for an empty, oversized, or
    /// NUL-containing token.
    pub fn new(token: impl Into<String>) -> Result<Self> {
        let token = token.into();
        if token.is_empty() || token.len() > MAX_QUERY_CURSOR_BYTES || token.contains('\0') {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                format!(
                    "query cursor must contain 1-{MAX_QUERY_CURSOR_BYTES} UTF-8 bytes without NUL"
                ),
            ));
        }
        Ok(Self(token))
    }

    /// Returns the opaque token for copying to the next request.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Page size and optional continuation supplied to a named query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryPageRequest {
    limit: u32,
    cursor: Option<QueryCursor>,
}

impl QueryPageRequest {
    /// Creates a bounded page request.
    ///
    /// # Errors
    ///
    /// Returns an invalid-argument error when `limit` is zero or greater than
    /// [`MAX_QUERY_PAGE_SIZE`].
    pub fn new(limit: u32, cursor: Option<QueryCursor>) -> Result<Self> {
        if limit == 0 || limit > MAX_QUERY_PAGE_SIZE {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                format!("query page size must be between 1 and {MAX_QUERY_PAGE_SIZE}"),
            ));
        }
        Ok(Self { limit, cursor })
    }

    /// Returns the maximum number of values in the page.
    #[must_use]
    pub const fn limit(&self) -> u32 {
        self.limit
    }

    /// Returns the preceding page's continuation token, when present.
    #[must_use]
    pub const fn cursor(&self) -> Option<&QueryCursor> {
        self.cursor.as_ref()
    }
}

/// One bounded page returned by a named domain query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryPage<T> {
    items: Vec<T>,
    next_cursor: Option<QueryCursor>,
    traversal_truncated: bool,
}

impl<T> QueryPage<T> {
    /// Constructs a backend query page.
    #[doc(hidden)]
    #[must_use]
    pub fn new(items: Vec<T>, next_cursor: Option<QueryCursor>, traversal_truncated: bool) -> Self {
        Self {
            items,
            next_cursor,
            traversal_truncated,
        }
    }

    /// Returns the values in stable key order.
    #[must_use]
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// Returns the token for the next page, when another page exists.
    #[must_use]
    pub const fn next_cursor(&self) -> Option<&QueryCursor> {
        self.next_cursor.as_ref()
    }

    /// Returns whether an explicit traversal bound made the result incomplete.
    #[must_use]
    pub const fn traversal_truncated(&self) -> bool {
        self.traversal_truncated
    }

    /// Consumes the page and returns its values.
    #[must_use]
    pub fn into_items(self) -> Vec<T> {
        self.items
    }
}

/// Bounds for a direct or transitive dependency query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DependencyQueryLimits {
    max_depth: u32,
    max_representations: u32,
}

impl DependencyQueryLimits {
    /// Creates explicit graph-traversal bounds.
    ///
    /// # Errors
    ///
    /// Returns an invalid-argument error when either bound is zero or exceeds
    /// its documented maximum.
    pub fn new(max_depth: u32, max_representations: u32) -> Result<Self> {
        if max_depth == 0 || max_depth > MAX_DEPENDENCY_QUERY_DEPTH {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                format!(
                    "dependency query depth must be between 1 and {MAX_DEPENDENCY_QUERY_DEPTH}"
                ),
            ));
        }
        if max_representations == 0 || max_representations > MAX_DEPENDENCY_QUERY_REPRESENTATIONS {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                format!(
                    "dependency query representation bound must be between 1 and {MAX_DEPENDENCY_QUERY_REPRESENTATIONS}"
                ),
            ));
        }
        Ok(Self {
            max_depth,
            max_representations,
        })
    }

    /// Returns the maximum number of dependency edges followed in one path.
    #[must_use]
    pub const fn max_depth(self) -> u32 {
        self.max_depth
    }

    /// Returns the maximum number of representations traversed.
    #[must_use]
    pub const fn max_representations(self) -> u32 {
        self.max_representations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_requests_and_cursors_are_bounded() {
        assert!(QueryPageRequest::new(1, None).is_ok());
        assert!(QueryPageRequest::new(MAX_QUERY_PAGE_SIZE, None).is_ok());
        assert!(QueryPageRequest::new(0, None).is_err());
        assert!(QueryPageRequest::new(MAX_QUERY_PAGE_SIZE + 1, None).is_err());
        assert!(QueryCursor::new("").is_err());
        assert!(QueryCursor::new("x".repeat(MAX_QUERY_CURSOR_BYTES + 1)).is_err());
    }

    #[test]
    fn dependency_query_limits_are_explicit() {
        assert!(DependencyQueryLimits::new(1, 1).is_ok());
        assert!(
            DependencyQueryLimits::new(
                MAX_DEPENDENCY_QUERY_DEPTH,
                MAX_DEPENDENCY_QUERY_REPRESENTATIONS,
            )
            .is_ok()
        );
        assert!(DependencyQueryLimits::new(0, 1).is_err());
        assert!(DependencyQueryLimits::new(1, 0).is_err());
    }
}
