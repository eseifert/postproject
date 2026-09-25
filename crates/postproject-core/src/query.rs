//! Bounded keyset pagination shared by named domain queries.

use crate::{
    ActivityKind, ArtifactEvaluationLimits, Error, ErrorKind, MetadataProperty, MetadataValue,
    MetadataValueKind, RepresentationId, Result, ToolIdentity,
};

/// Maximum number of values returned by one query page.
pub const MAX_QUERY_PAGE_SIZE: u32 = 1_000;
/// Maximum encoded size of an opaque continuation cursor.
pub const MAX_QUERY_CURSOR_BYTES: usize = 2_048;
/// Maximum depth accepted by a dependency traversal query.
pub const MAX_DEPENDENCY_QUERY_DEPTH: u32 = 64;
/// Maximum representations visited by a dependency traversal query.
pub const MAX_DEPENDENCY_QUERY_REPRESENTATIONS: u32 = 1_000;
/// Maximum depth accepted by a provenance traversal query.
pub const MAX_PROVENANCE_QUERY_DEPTH: u32 = 64;
/// Maximum representations visited by a provenance traversal query.
pub const MAX_PROVENANCE_QUERY_REPRESENTATIONS: u32 = 1_000;

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

/// Optional exact scalar predicate for a metadata-property query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetadataQuery {
    property: MetadataProperty,
    exact_value: Option<MetadataValue>,
}

impl MetadataQuery {
    /// Creates a property query with an optional exact scalar value.
    ///
    /// # Errors
    ///
    /// Returns an invalid-argument error when the predicate is a collection.
    pub fn new(property: MetadataProperty, exact_value: Option<MetadataValue>) -> Result<Self> {
        if exact_value.as_ref().is_some_and(|value| {
            matches!(
                value.kind(),
                MetadataValueKind::List | MetadataValueKind::Struct
            )
        }) {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "metadata query exact-value predicates must be scalar",
            ));
        }
        Ok(Self {
            property,
            exact_value,
        })
    }

    /// Returns the required metadata property.
    #[must_use]
    pub const fn property(&self) -> &MetadataProperty {
        &self.property
    }

    /// Returns the optional exact scalar value.
    #[must_use]
    pub const fn exact_value(&self) -> Option<&MetadataValue> {
        self.exact_value.as_ref()
    }
}

/// Filter selecting activity outputs by exact activity or tool identity.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ActivityOutputQuery {
    /// Outputs produced by one open-world activity kind.
    Kind(ActivityKind),
    /// Outputs produced by one exact tool identity.
    Tool(ToolIdentity),
}

/// Bounds for a provenance ancestor or descendant query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProvenanceQueryLimits {
    max_depth: u32,
    max_representations: u32,
}

impl ProvenanceQueryLimits {
    /// Creates explicit provenance traversal bounds.
    ///
    /// # Errors
    ///
    /// Returns an invalid-argument error when either bound is zero or exceeds
    /// its documented maximum.
    pub fn new(max_depth: u32, max_representations: u32) -> Result<Self> {
        if max_depth == 0 || max_depth > MAX_PROVENANCE_QUERY_DEPTH {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                format!(
                    "provenance query depth must be between 1 and {MAX_PROVENANCE_QUERY_DEPTH}"
                ),
            ));
        }
        if max_representations == 0 || max_representations > MAX_PROVENANCE_QUERY_REPRESENTATIONS {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                format!(
                    "provenance query representation bound must be between 1 and {MAX_PROVENANCE_QUERY_REPRESENTATIONS}"
                ),
            ));
        }
        Ok(Self {
            max_depth,
            max_representations,
        })
    }

    /// Returns the maximum number of activity edges followed in one path.
    #[must_use]
    pub const fn max_depth(self) -> u32 {
        self.max_depth
    }

    /// Returns the maximum number of representations visited.
    #[must_use]
    pub const fn max_representations(self) -> u32 {
        self.max_representations
    }
}

/// One representation reached by a provenance traversal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProvenanceQueryMatch {
    representation_id: RepresentationId,
    depth: u32,
}

impl ProvenanceQueryMatch {
    /// Creates one shortest-depth provenance result.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(representation_id: RepresentationId, depth: u32) -> Self {
        Self {
            representation_id,
            depth,
        }
    }

    /// Returns the reached representation.
    #[must_use]
    pub const fn representation_id(self) -> RepresentationId {
        self.representation_id
    }

    /// Returns the shortest number of provenance steps from the query root.
    #[must_use]
    pub const fn depth(self) -> u32 {
        self.depth
    }
}

/// Filters a paginated stale-artifact query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StaleArtifactQuery {
    source: Option<RepresentationId>,
    evaluation_limits: ArtifactEvaluationLimits,
}

impl StaleArtifactQuery {
    /// Creates a stale-artifact query, optionally restricted to descendants.
    #[must_use]
    pub const fn new(
        source: Option<RepresentationId>,
        evaluation_limits: ArtifactEvaluationLimits,
    ) -> Self {
        Self {
            source,
            evaluation_limits,
        }
    }

    /// Returns the optional provenance source restriction.
    #[must_use]
    pub const fn source(self) -> Option<RepresentationId> {
        self.source
    }

    /// Returns the bounds used to evaluate each candidate artifact.
    #[must_use]
    pub const fn evaluation_limits(self) -> ArtifactEvaluationLimits {
        self.evaluation_limits
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

    #[test]
    fn metadata_predicates_are_scalar() {
        let property = MetadataProperty::new(
            crate::VocabularyId::new("test").expect("vocabulary"),
            crate::PropertyId::new("value").expect("property"),
        );
        assert!(MetadataQuery::new(property.clone(), Some(MetadataValue::i64(3))).is_ok());
        assert!(
            MetadataQuery::new(
                property,
                Some(MetadataValue::list(vec![MetadataValue::i64(3)]).expect("list")),
            )
            .is_err()
        );
    }

    #[test]
    fn provenance_query_limits_are_explicit() {
        assert!(ProvenanceQueryLimits::new(1, 1).is_ok());
        assert!(ProvenanceQueryLimits::new(0, 1).is_err());
        assert!(ProvenanceQueryLimits::new(1, 0).is_err());
    }
}
