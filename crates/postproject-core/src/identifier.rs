//! External, industry, and application identifier values.

use crate::{Error, ErrorKind, Result};

/// Maximum UTF-8 byte length of an external identifier scheme.
pub const MAX_IDENTIFIER_SCHEME_BYTES: usize = 255;

/// Maximum UTF-8 byte length of an external identifier value.
pub const MAX_IDENTIFIER_VALUE_BYTES: usize = 4096;

/// Maximum UTF-8 byte length of an optional identifier qualifier.
pub const MAX_IDENTIFIER_QUALIFIER_BYTES: usize = 1024;

/// An extensible scheme name for an external identifier.
///
/// Schemes are deliberately strings rather than a closed enum. Core performs
/// only bounded, transport-safe validation; standards-specific validation is a
/// separate opt-in concern.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IdentifierScheme(String);

impl IdentifierScheme {
    /// Creates a scheme while preserving its exact spelling.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when `value` is empty, exceeds
    /// [`MAX_IDENTIFIER_SCHEME_BYTES`], or contains whitespace/control bytes.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.is_empty() || value.len() > MAX_IDENTIFIER_SCHEME_BYTES {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                format!(
                    "identifier scheme must contain 1-{MAX_IDENTIFIER_SCHEME_BYTES} UTF-8 bytes"
                ),
            ));
        }
        if value.chars().any(char::is_whitespace) || value.chars().any(char::is_control) {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "identifier scheme must not contain whitespace or control characters",
            ));
        }
        Ok(Self(value))
    }

    /// Returns the exact scheme supplied by the caller.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the scheme and returns its owned string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

/// An identifier assigned by an external standard, vendor, or application.
///
/// This value does not replace a `PostProject` object ID. It is opaque to core
/// unless a caller explicitly applies a scheme-specific validator.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ExternalIdentifier {
    scheme: IdentifierScheme,
    value: String,
    qualifier: Option<String>,
}

impl ExternalIdentifier {
    /// Creates an external identifier without normalizing its value.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when the value or qualifier is
    /// empty when present, or exceeds its documented byte limit.
    pub fn new(
        scheme: IdentifierScheme,
        value: impl Into<String>,
        qualifier: Option<String>,
    ) -> Result<Self> {
        let value = value.into();
        validate_bounded_text("identifier value", &value, MAX_IDENTIFIER_VALUE_BYTES)?;
        if let Some(qualifier) = qualifier.as_deref() {
            validate_bounded_text(
                "identifier qualifier",
                qualifier,
                MAX_IDENTIFIER_QUALIFIER_BYTES,
            )?;
        }
        Ok(Self {
            scheme,
            value,
            qualifier,
        })
    }

    /// Returns the identifier scheme.
    #[must_use]
    pub const fn scheme(&self) -> &IdentifierScheme {
        &self.scheme
    }

    /// Returns the exact, opaque identifier value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the optional exact scope or qualifier.
    #[must_use]
    pub fn qualifier(&self) -> Option<&str> {
        self.qualifier.as_deref()
    }
}

fn validate_bounded_text(label: &str, value: &str, maximum: usize) -> Result<()> {
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

    #[test]
    fn unknown_scheme_and_value_round_trip_exactly() {
        let scheme = IdentifierScheme::new("com.example.camera.serial").expect("valid scheme");
        let identifier = ExternalIdentifier::new(
            scheme,
            "  Vendor Value 01  ",
            Some("camera-body".to_owned()),
        )
        .expect("valid identifier");

        assert_eq!(identifier.scheme().as_str(), "com.example.camera.serial");
        assert_eq!(identifier.value(), "  Vendor Value 01  ");
        assert_eq!(identifier.qualifier(), Some("camera-body"));
    }

    #[test]
    fn same_scheme_can_describe_multiple_values() {
        let scheme = IdentifierScheme::new("urn:smpte:umid").expect("valid scheme");
        let first = ExternalIdentifier::new(scheme.clone(), "first", None).expect("valid id");
        let second = ExternalIdentifier::new(scheme, "second", None).expect("valid id");

        assert_ne!(first, second);
    }

    #[test]
    fn generic_validation_rejects_unbounded_or_ambiguous_scheme_text() {
        for invalid in ["", "urn:example:has space", "urn:example:\ncontrol"] {
            let error = IdentifierScheme::new(invalid).expect_err("scheme must fail");
            assert_eq!(error.kind(), ErrorKind::InvalidArgument);
        }

        let oversized = "s".repeat(MAX_IDENTIFIER_SCHEME_BYTES + 1);
        assert_eq!(
            IdentifierScheme::new(oversized)
                .expect_err("oversized scheme must fail")
                .kind(),
            ErrorKind::InvalidArgument
        );
    }

    #[test]
    fn generic_validation_bounds_values_and_qualifiers() {
        let scheme = IdentifierScheme::new("com.example.id").expect("valid scheme");
        assert_eq!(
            ExternalIdentifier::new(scheme.clone(), "", None)
                .expect_err("empty value must fail")
                .kind(),
            ErrorKind::InvalidArgument
        );
        assert_eq!(
            ExternalIdentifier::new(scheme, "value", Some(String::new()))
                .expect_err("empty qualifier must fail")
                .kind(),
            ErrorKind::InvalidArgument
        );
        let scheme = IdentifierScheme::new("com.example.id").expect("valid scheme");
        assert_eq!(
            ExternalIdentifier::new(scheme, "value\0suffix", None)
                .expect_err("NUL-containing value must fail")
                .kind(),
            ErrorKind::InvalidArgument
        );
    }
}
