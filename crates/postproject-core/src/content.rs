//! Structure and membership values for compound representations.

use crate::{Error, ErrorKind, ResourceId, Result};

/// Maximum encoded length of an extensible resource-role identifier.
pub const MAX_RESOURCE_ROLE_BYTES: usize = 128;

/// A namespaced, open-world role for a resource within a representation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResourceRole(String);

impl ResourceRole {
    /// Creates a role such as `postproject:essence` or `vendor:playlist`.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when the role is empty, too long,
    /// is not namespaced, or contains unsupported bytes.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        let valid_bytes = value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'));
        let valid_namespace = value
            .split_once(':')
            .is_some_and(|(namespace, local)| !namespace.is_empty() && !local.is_empty());
        if value.len() > MAX_RESOURCE_ROLE_BYTES || !valid_bytes || !valid_namespace {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                format!(
                    "resource role must be a namespaced identifier of at most {MAX_RESOURCE_ROLE_BYTES} ASCII bytes"
                ),
            ));
        }
        Ok(Self(value))
    }

    /// Returns the exact role identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One resource's role and requiredness within a content structure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceMember {
    resource_id: ResourceId,
    role: ResourceRole,
    required: bool,
}

impl ResourceMember {
    /// Creates a resource membership.
    #[must_use]
    pub const fn new(resource_id: ResourceId, role: ResourceRole, required: bool) -> Self {
        Self {
            resource_id,
            role,
            required,
        }
    }

    /// Returns the participating resource.
    #[must_use]
    pub const fn resource_id(&self) -> ResourceId {
        self.resource_id
    }

    /// Returns the member's extensible semantic role.
    #[must_use]
    pub const fn role(&self) -> &ResourceRole {
        &self.role
    }

    /// Returns whether availability of this member is required for completeness.
    #[must_use]
    pub const fn is_required(&self) -> bool {
        self.required
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_roles_are_namespaced_and_open_world() {
        let standard = ResourceRole::new("postproject:essence").expect("valid role");
        let vendor = ResourceRole::new("example.camera:playlist-v2").expect("valid role");

        assert_eq!(standard.as_str(), "postproject:essence");
        assert_eq!(vendor.as_str(), "example.camera:playlist-v2");
        assert!(ResourceRole::new("essence").is_err());
        assert!(ResourceRole::new("vendor:").is_err());
        assert!(ResourceRole::new("vendor:side car").is_err());
    }

    #[test]
    fn membership_preserves_role_and_requiredness() {
        let resource_id = ResourceId::new();
        let role = ResourceRole::new("postproject:thumbnail").expect("valid role");
        let member = ResourceMember::new(resource_id, role, false);

        assert_eq!(member.resource_id(), resource_id);
        assert_eq!(member.role().as_str(), "postproject:thumbnail");
        assert!(!member.is_required());
    }
}
