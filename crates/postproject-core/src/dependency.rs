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
}
