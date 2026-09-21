//! Optional definitions and syntax checks for well-known identifier schemes.

use crate::{Error, ErrorKind, ExternalIdentifier, IdentifierScheme, Result};

/// SMPTE UMID values represented according to SMPTE ST 2029.
pub const SMPTE_UMID_SCHEME: &str = "urn:smpte:umid";
/// International Standard Audiovisual Number URNs from RFC 4246.
pub const ISAN_SCHEME: &str = "urn:isan";
/// Entertainment Identifier Registry URNs from RFC 7972.
pub const EIDR_SCHEME: &str = "urn:eidr";
/// Application-defined identifiers whose value semantics belong to the host.
pub const POSTPROJECT_APPLICATION_SCHEME: &str = "https://postproject.org/id/application";

/// The strength of local validation supplied for a known scheme.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum IdentifierValidationKind {
    /// Only the generic bounded-text rules apply.
    Opaque,
    /// The registry checks published syntax without resolving the identifier.
    Syntax,
}

/// Documentation and optional local validation for one identifier scheme.
#[derive(Clone, Copy, Debug)]
pub struct IdentifierSchemeDefinition {
    scheme: &'static str,
    label: &'static str,
    reference: &'static str,
    validation: IdentifierValidationKind,
    validator: Option<fn(&str) -> bool>,
}

impl IdentifierSchemeDefinition {
    /// Returns the exact scheme string stored with an external identifier.
    #[must_use]
    pub const fn scheme(self) -> &'static str {
        self.scheme
    }

    /// Returns a short human-readable label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        self.label
    }

    /// Returns the authoritative syntax reference.
    #[must_use]
    pub const fn reference(self) -> &'static str {
        self.reference
    }

    /// Returns the local validation strength.
    #[must_use]
    pub const fn validation(self) -> IdentifierValidationKind {
        self.validation
    }

    /// Checks the value without normalization or any registry/network access.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] when a syntax-checked value does
    /// not match the published lexical form supported by this definition.
    pub fn validate_value(self, value: &str) -> Result<()> {
        if self.validator.is_none_or(|validator| validator(value)) {
            return Ok(());
        }
        Err(Error::new(
            ErrorKind::InvalidArgument,
            format!("identifier value does not match {} syntax", self.label),
        ))
    }
}

/// Small built-in registry. Unknown schemes remain valid opaque identifiers.
pub const IDENTIFIER_SCHEMES: &[IdentifierSchemeDefinition] = &[
    definition(
        SMPTE_UMID_SCHEME,
        "SMPTE UMID",
        "https://pub.smpte.org/doc/st2029/20090310-pub/st2029-2009.pdf",
        Some(valid_umid),
    ),
    definition(
        ISAN_SCHEME,
        "ISAN",
        "https://www.rfc-editor.org/rfc/rfc4246.html",
        Some(valid_isan),
    ),
    definition(
        EIDR_SCHEME,
        "EIDR",
        "https://www.rfc-editor.org/rfc/rfc7972.html",
        Some(valid_eidr),
    ),
    definition(
        POSTPROJECT_APPLICATION_SCHEME,
        "PostProject application identifier",
        POSTPROJECT_APPLICATION_SCHEME,
        None,
    ),
];

/// Finds a built-in definition by exact scheme spelling.
#[must_use]
pub fn identifier_scheme_definition(
    scheme: &IdentifierScheme,
) -> Option<&'static IdentifierSchemeDefinition> {
    IDENTIFIER_SCHEMES
        .iter()
        .find(|definition| definition.scheme == scheme.as_str())
}

/// Applies optional built-in validation and reports whether the scheme is known.
///
/// Unknown schemes return `Ok(false)` and remain fully preservable.
///
/// # Errors
///
/// Returns an error only when a known syntax-checked value is invalid.
pub fn validate_known_identifier(identifier: &ExternalIdentifier) -> Result<bool> {
    let Some(definition) = identifier_scheme_definition(identifier.scheme()) else {
        return Ok(false);
    };
    definition.validate_value(identifier.value())?;
    Ok(true)
}

const fn definition(
    scheme: &'static str,
    label: &'static str,
    reference: &'static str,
    validator: Option<fn(&str) -> bool>,
) -> IdentifierSchemeDefinition {
    IdentifierSchemeDefinition {
        scheme,
        label,
        reference,
        validation: if validator.is_some() {
            IdentifierValidationKind::Syntax
        } else {
            IdentifierValidationKind::Opaque
        },
        validator,
    }
}

fn valid_umid(value: &str) -> bool {
    if !value.contains('.') {
        return matches!(value.len(), 64 | 128)
            && value.bytes().all(|byte| byte.is_ascii_hexdigit());
    }
    let mut groups = value.split('.');
    let count = groups.clone().count();
    matches!(count, 8 | 16)
        && groups
            .all(|group| group.len() == 8 && group.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn valid_isan(value: &str) -> bool {
    let parts: Vec<_> = value.split('-').collect();
    if !matches!(parts.len(), 5 | 8)
        || !parts[..4].iter().all(|part| hex_group(part, 4))
        || !check_character(parts[4])
    {
        return false;
    }
    parts.len() == 5
        || (parts[5..7].iter().all(|part| hex_group(part, 4)) && check_character(parts[7]))
}

fn valid_eidr(value: &str) -> bool {
    let Some((prefix, suffix)) = value.split_once(':') else {
        return false;
    };
    if prefix.is_empty()
        || suffix.is_empty()
        || !prefix.bytes().all(eidr_character)
        || !suffix.bytes().all(eidr_character)
    {
        return false;
    }
    if prefix != "10.5240" {
        return true;
    }
    let parts: Vec<_> = suffix.split('-').collect();
    parts.len() == 6
        && parts[..5].iter().all(|part| hex_group(part, 4))
        && check_character(parts[5])
}

fn hex_group(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn check_character(value: &str) -> bool {
    value.len() == 1 && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn eidr_character(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identifier(scheme: &str, value: &str) -> ExternalIdentifier {
        ExternalIdentifier::new(
            IdentifierScheme::new(scheme).expect("valid scheme"),
            value,
            None,
        )
        .expect("generically valid identifier")
    }

    #[test]
    fn published_examples_pass_local_syntax_checks() {
        let umid = identifier(
            SMPTE_UMID_SCHEME,
            "060a2b34.01010105.01010d20.13000000.d2c9036c.8f195343.ab7014d2.d718bfda",
        );
        let isan = identifier(ISAN_SCHEME, "1881-66C7-3420-6541-9-9F3A-0245-U");
        let eidr = identifier(EIDR_SCHEME, "10.5240:7791-8534-2C23-9030-8610-5");
        assert_eq!(validate_known_identifier(&umid), Ok(true));
        assert_eq!(validate_known_identifier(&isan), Ok(true));
        assert_eq!(validate_known_identifier(&eidr), Ok(true));
    }

    #[test]
    fn known_invalid_values_fail_without_changing_unknown_schemes() {
        assert!(validate_known_identifier(&identifier(EIDR_SCHEME, "not-an-eidr")).is_err());
        assert_eq!(
            validate_known_identifier(&identifier("com.example.camera", " exact value ")),
            Ok(false)
        );
    }

    #[test]
    fn application_identifiers_are_known_and_remain_opaque() {
        let identifier = identifier(POSTPROJECT_APPLICATION_SCHEME, "editor:scene/42");
        assert_eq!(validate_known_identifier(&identifier), Ok(true));
        assert_eq!(
            identifier_scheme_definition(identifier.scheme())
                .expect("known application scheme")
                .reference(),
            POSTPROJECT_APPLICATION_SCHEME
        );
    }
}
