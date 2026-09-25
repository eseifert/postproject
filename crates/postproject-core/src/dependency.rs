//! Live relationships observed from representation content.

use crate::{AssetId, Error, ErrorKind, RepresentationId, ResourceId, Result};

/// Maximum encoded length of an open-world dependency-kind identifier.
pub const MAX_DEPENDENCY_KIND_BYTES: usize = 128;
/// Maximum UTF-8 byte length of an authored reference.
pub const MAX_AUTHORED_REFERENCE_BYTES: usize = 4_096;
/// Maximum number of edges in one observed dependency set.
pub const MAX_DEPENDENCIES_PER_SET: usize = 100_000;

/// An open-world, namespaced dependency kind.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DependencyKind(String);

impl DependencyKind {
    /// Creates a kind such as `org.openusd:reference`.
    ///
    /// # Errors
    ///
    /// Returns an invalid-argument error when the value is not a bounded,
    /// namespaced ASCII identifier.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        let valid_bytes = value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'));
        let valid_namespace = value
            .split_once(':')
            .is_some_and(|(namespace, local)| !namespace.is_empty() && !local.is_empty());
        if value.len() > MAX_DEPENDENCY_KIND_BYTES || !valid_bytes || !valid_namespace {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                format!(
                    "dependency kind must be a namespaced identifier of at most {MAX_DEPENDENCY_KIND_BYTES} ASCII bytes"
                ),
            ));
        }
        Ok(Self(value))
    }

    /// Returns the exact kind identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The production object named by an authored dependency.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum DependencyTarget {
    /// A floating reference to an asset.
    Asset(AssetId),
    /// A reference pinned to one representation.
    Representation(RepresentationId),
}

/// One object reached by a bounded dependency or dependent query.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DependencyQueryMatch {
    target: DependencyTarget,
    depth: u32,
}

impl DependencyQueryMatch {
    /// Constructs one backend-derived query match.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(target: DependencyTarget, depth: u32) -> Self {
        Self { target, depth }
    }

    /// Returns the matching asset or representation.
    #[must_use]
    pub const fn target(self) -> DependencyTarget {
        self.target
    }

    /// Returns the shortest number of dependency edges from the query root.
    #[must_use]
    pub const fn depth(self) -> u32 {
        self.depth
    }
}

/// One typed dependency occurrence in a representation's observed set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dependency {
    source_resource_id: Option<ResourceId>,
    kind: DependencyKind,
    target: DependencyTarget,
    resolved_representation_id: Option<RepresentationId>,
    required: bool,
    authored_reference: String,
}

impl Dependency {
    /// Creates one dependency occurrence.
    ///
    /// The authored reference is preserved exactly and may be empty when the
    /// source format permits an implicit reference. A resolved representation
    /// applies only to a floating asset target; a representation target is
    /// already pinned.
    ///
    /// # Errors
    ///
    /// Returns an invalid-argument error for oversized or NUL-containing
    /// reference text, or a resolved representation on a pinned target.
    pub fn new(
        source_resource_id: Option<ResourceId>,
        kind: DependencyKind,
        target: DependencyTarget,
        resolved_representation_id: Option<RepresentationId>,
        required: bool,
        authored_reference: impl Into<String>,
    ) -> Result<Self> {
        let authored_reference = authored_reference.into();
        if authored_reference.len() > MAX_AUTHORED_REFERENCE_BYTES
            || authored_reference.contains('\0')
        {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                format!(
                    "authored dependency reference must not contain NUL or exceed {MAX_AUTHORED_REFERENCE_BYTES} UTF-8 bytes"
                ),
            ));
        }
        if matches!(target, DependencyTarget::Representation(_))
            && resolved_representation_id.is_some()
        {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "a pinned dependency target must not have a separate resolved representation",
            ));
        }
        Ok(Self {
            source_resource_id,
            kind,
            target,
            resolved_representation_id,
            required,
            authored_reference,
        })
    }

    /// Returns the resource that contains this reference, when known.
    #[must_use]
    pub const fn source_resource_id(&self) -> Option<ResourceId> {
        self.source_resource_id
    }

    /// Returns the exact open-world relationship kind.
    #[must_use]
    pub const fn kind(&self) -> &DependencyKind {
        &self.kind
    }

    /// Returns the floating or pinned target.
    #[must_use]
    pub const fn target(&self) -> DependencyTarget {
        self.target
    }

    /// Returns the representation selected for a floating asset target.
    #[must_use]
    pub const fn resolved_representation_id(&self) -> Option<RepresentationId> {
        self.resolved_representation_id
    }

    /// Returns whether the target is needed whenever the source is used.
    #[must_use]
    pub const fn is_required(&self) -> bool {
        self.required
    }

    /// Returns the reference text exactly as supplied by the caller.
    #[must_use]
    pub fn authored_reference(&self) -> &str {
        &self.authored_reference
    }
}

/// Whether an observed dependency set still describes its source content.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum DependencySetStatus {
    /// The set was recorded for the source's current content observation.
    Current,
    /// The source changed and its set must be extracted again.
    NeedsExtraction,
}

/// One complete ordered observation of a representation's dependencies.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencySet {
    source_representation_id: RepresentationId,
    recorded_at_revision: u64,
    status: DependencySetStatus,
    dependencies: Vec<Dependency>,
}

impl DependencySet {
    /// Reconstructs a bounded dependency-set observation.
    ///
    /// # Errors
    ///
    /// Returns an invalid-argument error for revision zero or too many edges.
    pub fn new(
        source_representation_id: RepresentationId,
        recorded_at_revision: u64,
        status: DependencySetStatus,
        dependencies: Vec<Dependency>,
    ) -> Result<Self> {
        if recorded_at_revision == 0 {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "dependency-set revision must be greater than zero",
            ));
        }
        if dependencies.len() > MAX_DEPENDENCIES_PER_SET {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                format!("dependency set must contain at most {MAX_DEPENDENCIES_PER_SET} edges"),
            ));
        }
        Ok(Self {
            source_representation_id,
            recorded_at_revision,
            status,
            dependencies,
        })
    }

    /// Returns the representation whose content supplied the set.
    #[must_use]
    pub const fn source_representation_id(&self) -> RepresentationId {
        self.source_representation_id
    }

    /// Returns the revision that committed this complete set.
    #[must_use]
    pub const fn recorded_at_revision(&self) -> u64 {
        self.recorded_at_revision
    }

    /// Returns whether the set needs extraction after a source change.
    #[must_use]
    pub const fn status(&self) -> DependencySetStatus {
        self.status
    }

    /// Returns dependency occurrences in authored order.
    #[must_use]
    pub fn dependencies(&self) -> &[Dependency] {
        &self.dependencies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_kinds_are_open_world_namespaced_text() {
        assert!(DependencyKind::new("org.openusd:reference").is_ok());
        assert!(DependencyKind::new("studio.example:custom_link").is_ok());
        assert!(DependencyKind::new("reference").is_err());
    }

    #[test]
    fn authored_reference_is_exact_and_pinned_targets_are_unambiguous() {
        let target = RepresentationId::from_bytes([2; 16]);
        let dependency = Dependency::new(
            None,
            DependencyKind::new("org.openusd:reference").expect("kind"),
            DependencyTarget::Representation(target),
            None,
            true,
            "  ../Character.usd  ",
        )
        .expect("dependency");
        assert_eq!(dependency.authored_reference(), "  ../Character.usd  ");
        assert!(
            Dependency::new(
                None,
                DependencyKind::new("org.openusd:reference").expect("kind"),
                DependencyTarget::Representation(target),
                Some(target),
                true,
                "Character.usd",
            )
            .is_err()
        );
    }

    #[test]
    fn dependency_sets_preserve_repeated_authored_order() {
        let edge = Dependency::new(
            None,
            DependencyKind::new("org.postproject:requires").expect("kind"),
            DependencyTarget::Asset(AssetId::from_bytes([3; 16])),
            Some(RepresentationId::from_bytes([4; 16])),
            true,
            "asset://character",
        )
        .expect("dependency");
        let set = DependencySet::new(
            RepresentationId::from_bytes([1; 16]),
            7,
            DependencySetStatus::Current,
            vec![edge.clone(), edge],
        )
        .expect("set");
        assert_eq!(set.dependencies().len(), 2);
    }
}
