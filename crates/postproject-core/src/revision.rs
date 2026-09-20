//! Durable semantic revision values for project-local change feeds.

use crate::{Error, ErrorKind, Result, RevisionId, Timestamp, ToolIdentity, TransactionId};

/// Maximum UTF-8 byte length of a revision message.
pub const MAX_REVISION_MESSAGE_BYTES: usize = 4_096;

/// Identity of the integrating application or process that committed a revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OriginIdentity(ToolIdentity);

impl OriginIdentity {
    /// Creates a bounded origin identity with an optional version and URI.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] under the same conditions as
    /// [`ToolIdentity::new`].
    pub fn new(
        name: impl Into<String>,
        version: Option<String>,
        uri: Option<String>,
    ) -> Result<Self> {
        ToolIdentity::new(name, version, uri).map(Self)
    }

    /// Returns the integrating application or process name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.0.name()
    }

    /// Returns the optional exact application version or build.
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.0.version()
    }

    /// Returns the optional canonical application or vendor URI.
    #[must_use]
    pub fn uri(&self) -> Option<&str> {
        self.0.uri()
    }
}

/// One committed project mutation transaction in local sequence order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Revision {
    id: RevisionId,
    sequence: u64,
    transaction_id: TransactionId,
    committed_at: Timestamp,
    origin: Option<OriginIdentity>,
    message: Option<String>,
}

impl Revision {
    /// Creates a complete durable revision value.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidArgument`] for sequence zero or an empty,
    /// oversized, or NUL-containing message.
    pub fn new(
        id: RevisionId,
        sequence: u64,
        transaction_id: TransactionId,
        committed_at: Timestamp,
        origin: Option<OriginIdentity>,
        message: Option<String>,
    ) -> Result<Self> {
        if sequence == 0 {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "revision sequence must be greater than zero",
            ));
        }
        if message.as_deref().is_some_and(|message| {
            message.is_empty()
                || message.len() > MAX_REVISION_MESSAGE_BYTES
                || message.contains('\0')
        }) {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                format!(
                    "revision message must contain 1-{MAX_REVISION_MESSAGE_BYTES} UTF-8 bytes without NUL"
                ),
            ));
        }
        Ok(Self {
            id,
            sequence,
            transaction_id,
            committed_at,
            origin,
            message,
        })
    }

    /// Returns the stable revision identity.
    #[must_use]
    pub const fn id(&self) -> RevisionId {
        self.id
    }

    /// Returns the monotonically increasing project-local sequence.
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Returns the transaction that produced this revision.
    #[must_use]
    pub const fn transaction_id(&self) -> TransactionId {
        self.transaction_id
    }

    /// Returns the durable commit timestamp.
    #[must_use]
    pub const fn committed_at(&self) -> Timestamp {
        self.committed_at
    }

    /// Returns the optional integrating application/process identity.
    #[must_use]
    pub const fn origin(&self) -> Option<&OriginIdentity> {
        self.origin.as_ref()
    }

    /// Returns the optional human-facing commit message.
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revision_preserves_transaction_context() {
        let origin = OriginIdentity::new(
            "Editorial host",
            Some("2.4.1".to_owned()),
            Some("https://example.com/editor".to_owned()),
        )
        .expect("valid origin");
        let revision = Revision::new(
            RevisionId::new(),
            7,
            TransactionId::new(),
            Timestamp::from_unix_micros(42),
            Some(origin),
            Some("Import camera original".to_owned()),
        )
        .expect("valid revision");

        assert_eq!(revision.sequence(), 7);
        assert_eq!(revision.committed_at().as_unix_micros(), 42);
        assert_eq!(revision.origin().unwrap().name(), "Editorial host");
        assert_eq!(revision.message(), Some("Import camera original"));
    }

    #[test]
    fn revision_rejects_ambiguous_context() {
        let create = |sequence, message| {
            Revision::new(
                RevisionId::new(),
                sequence,
                TransactionId::new(),
                Timestamp::from_unix_micros(0),
                None,
                message,
            )
        };

        assert!(create(0, None).is_err());
        assert!(create(1, Some(String::new())).is_err());
        assert!(create(1, Some("bad\0message".to_owned())).is_err());
        assert!(create(1, Some("x".repeat(MAX_REVISION_MESSAGE_BYTES + 1))).is_err());
    }
}
