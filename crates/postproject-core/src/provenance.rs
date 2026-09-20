//! Extensible production-provenance vocabulary and participant identities.

use crate::{Error, ErrorKind, ExternalIdentifier, Result, uri::normalize_uri};

/// Maximum encoded length of an activity-kind identifier.
pub const MAX_ACTIVITY_KIND_BYTES: usize = 128;
/// Maximum encoded length of an activity-edge role identifier.
pub const MAX_ACTIVITY_ROLE_BYTES: usize = 128;
/// Maximum UTF-8 byte length of a tool or agent display name.
pub const MAX_PROVENANCE_NAME_BYTES: usize = 256;
/// Maximum UTF-8 byte length of an optional tool version.
pub const MAX_TOOL_VERSION_BYTES: usize = 128;

/// A namespaced, open-world production activity kind.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActivityKind(String);

impl ActivityKind {
    /// Creates a kind such as `postproject:transcode`.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when the value is not a bounded
    /// namespaced ASCII identifier.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        validate_namespaced_identifier("activity kind", value.into(), MAX_ACTIVITY_KIND_BYTES)
            .map(Self)
    }

    /// Returns the exact kind identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A namespaced, open-world role for an activity input or output.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActivityRole(String);

impl ActivityRole {
    /// Creates a role such as `postproject:input.primary-video`.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when the value is not a bounded
    /// namespaced ASCII identifier.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        validate_namespaced_identifier("activity role", value.into(), MAX_ACTIVITY_ROLE_BYTES)
            .map(Self)
    }

    /// Returns the exact role identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The software or service that performed a production activity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolIdentity {
    name: String,
    version: Option<String>,
    uri: Option<String>,
}

impl ToolIdentity {
    /// Creates a bounded tool identity with an optional absolute URI.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] for empty or oversized text, NUL
    /// bytes, or an invalid/relative URI.
    pub fn new(
        name: impl Into<String>,
        version: Option<String>,
        uri: Option<String>,
    ) -> Result<Self> {
        let name = name.into();
        validate_text("tool name", &name, MAX_PROVENANCE_NAME_BYTES)?;
        if let Some(version) = version.as_deref() {
            validate_text("tool version", version, MAX_TOOL_VERSION_BYTES)?;
        }
        let uri = uri.map(|value| normalize_uri(value, "tool")).transpose()?;
        Ok(Self { name, version, uri })
    }

    /// Returns the user-facing tool name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the optional exact version or build string.
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// Returns the optional canonical absolute tool/vendor URI.
    #[must_use]
    pub fn uri(&self) -> Option<&str> {
        self.uri.as_deref()
    }
}

/// A person, organization, application instance, or service responsible for an activity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentIdentity {
    name: Option<String>,
    identifier: Option<ExternalIdentifier>,
}

impl AgentIdentity {
    /// Creates an agent described by a name, an external identifier, or both.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when neither identity is present
    /// or the name is empty, oversized, or contains NUL.
    pub fn new(name: Option<String>, identifier: Option<ExternalIdentifier>) -> Result<Self> {
        if let Some(name) = name.as_deref() {
            validate_text("agent name", name, MAX_PROVENANCE_NAME_BYTES)?;
        }
        if name.is_none() && identifier.is_none() {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "agent identity requires a name or external identifier",
            ));
        }
        Ok(Self { name, identifier })
    }

    /// Returns the optional user-facing agent name.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the optional external agent identity.
    #[must_use]
    pub const fn identifier(&self) -> Option<&ExternalIdentifier> {
        self.identifier.as_ref()
    }
}

fn validate_namespaced_identifier(label: &str, value: String, maximum: usize) -> Result<String> {
    let valid_bytes = value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'));
    let valid_namespace = value
        .split_once(':')
        .is_some_and(|(namespace, local)| !namespace.is_empty() && !local.is_empty());
    if value.len() > maximum || !valid_bytes || !valid_namespace {
        return Err(Error::new(
            ErrorKind::InvalidArgument,
            format!("{label} must be a namespaced identifier of at most {maximum} ASCII bytes"),
        ));
    }
    Ok(value)
}

fn validate_text(label: &str, value: &str, maximum: usize) -> Result<()> {
    if value.is_empty() || value.len() > maximum || value.contains('\0') {
        return Err(Error::new(
            ErrorKind::InvalidArgument,
            format!("{label} must contain 1-{maximum} UTF-8 bytes without NUL"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IdentifierScheme;

    #[test]
    fn activity_vocabulary_is_extensible_and_namespaced() {
        let kind = ActivityKind::new("vendor.example:vfx-render").expect("valid kind");
        let role = ActivityRole::new("vendor.example:input.plate").expect("valid role");

        assert_eq!(kind.as_str(), "vendor.example:vfx-render");
        assert_eq!(role.as_str(), "vendor.example:input.plate");
        assert!(ActivityKind::new("transcode").is_err());
        assert!(ActivityRole::new("vendor.example:bad role").is_err());
    }

    #[test]
    fn tool_and_agent_identity_preserve_supplied_detail() {
        let tool = ToolIdentity::new(
            "FFmpeg",
            Some("8.0-custom".to_owned()),
            Some("https://ffmpeg.org".to_owned()),
        )
        .expect("valid tool");
        let identifier = ExternalIdentifier::new(
            IdentifierScheme::new("com.example.worker").expect("valid scheme"),
            "worker-42",
            None,
        )
        .expect("valid identifier");
        let agent = AgentIdentity::new(Some("Render worker".to_owned()), Some(identifier.clone()))
            .expect("valid agent");

        assert_eq!(tool.name(), "FFmpeg");
        assert_eq!(tool.version(), Some("8.0-custom"));
        assert_eq!(tool.uri(), Some("https://ffmpeg.org/"));
        assert_eq!(agent.name(), Some("Render worker"));
        assert_eq!(agent.identifier(), Some(&identifier));
    }

    #[test]
    fn participant_identity_is_bounded() {
        assert!(ToolIdentity::new("", None, None).is_err());
        assert!(ToolIdentity::new("tool", Some(String::new()), None).is_err());
        assert!(ToolIdentity::new("tool", None, Some("relative".to_owned())).is_err());
        assert!(AgentIdentity::new(None, None).is_err());
        assert!(AgentIdentity::new(Some("bad\0name".to_owned()), None).is_err());
    }
}
