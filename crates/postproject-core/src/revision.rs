//! Durable semantic revision values for project-local change feeds.

use crate::{
    ActivityId, ActivityKind, ActivityRole, AssetId, Error, ErrorKind, ExternalIdentifier,
    LocatorId, MediaRootId, MetadataProperty, ObjectRef, RepresentationId, ResourceId, Result,
    RevisionId, Timestamp, ToolIdentity, TransactionId,
};

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

/// One semantic mutation recorded in a durable revision.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RevisionEventKind {
    /// A logical asset and its import aggregate were created.
    AssetImported {
        /// Imported logical asset.
        asset_id: AssetId,
    },
    /// A representation was attached to an asset.
    RepresentationAdded {
        /// Owning logical asset.
        asset_id: AssetId,
        /// Added representation.
        representation_id: RepresentationId,
    },
    /// A storage resource was created.
    ResourceAdded {
        /// Added resource.
        resource_id: ResourceId,
    },
    /// A resource was attached to a representation's content structure.
    RepresentationResourceAdded {
        /// Owning representation.
        representation_id: RepresentationId,
        /// Attached resource.
        resource_id: ResourceId,
        /// Stable structural position within the representation.
        position: u32,
    },
    /// A resource locator was added or explicitly confirmed.
    LocatorAdded {
        /// Located resource.
        resource_id: ResourceId,
        /// Added locator.
        locator_id: LocatorId,
    },
    /// A resolver media root was added.
    MediaRootAdded {
        /// Added media root.
        media_root_id: MediaRootId,
    },
    /// An exact external identifier attachment was added.
    ExternalIdentifierAdded {
        /// Object receiving the identifier.
        target: ObjectRef,
        /// Added external identifier.
        identifier: ExternalIdentifier,
    },
    /// An exact external identifier attachment was removed.
    ExternalIdentifierRemoved {
        /// Object losing the identifier.
        target: ObjectRef,
        /// Removed external identifier.
        identifier: ExternalIdentifier,
    },
    /// One metadata property's values were appended or replaced.
    MetadataAddedOrReplaced {
        /// Object whose metadata changed.
        target: ObjectRef,
        /// Property that consumers should re-query.
        property: MetadataProperty,
    },
    /// One metadata property was removed.
    MetadataRemoved {
        /// Object whose metadata changed.
        target: ObjectRef,
        /// Removed property.
        property: MetadataProperty,
    },
    /// A production activity was created.
    ActivityCreated {
        /// Added activity.
        activity_id: ActivityId,
        /// Extensible activity kind.
        kind: ActivityKind,
    },
    /// A production activity input edge was added.
    ActivityInputAdded {
        /// Owning activity.
        activity_id: ActivityId,
        /// Consumed representation.
        representation_id: RepresentationId,
        /// Optional semantic edge role.
        role: Option<ActivityRole>,
    },
    /// A production activity output edge was added.
    ActivityOutputAdded {
        /// Owning activity.
        activity_id: ActivityId,
        /// Produced representation.
        representation_id: RepresentationId,
        /// Optional semantic edge role.
        role: Option<ActivityRole>,
    },
}

/// One deterministically ordered semantic event within a revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionEvent {
    revision_id: RevisionId,
    position: u32,
    kind: RevisionEventKind,
}

impl RevisionEvent {
    /// Creates an event at its stable zero-based revision position.
    #[must_use]
    pub const fn new(revision_id: RevisionId, position: u32, kind: RevisionEventKind) -> Self {
        Self {
            revision_id,
            position,
            kind,
        }
    }

    /// Returns the revision that owns the event.
    #[must_use]
    pub const fn revision_id(&self) -> RevisionId {
        self.revision_id
    }

    /// Returns the zero-based stable position within the revision.
    #[must_use]
    pub const fn position(&self) -> u32 {
        self.position
    }

    /// Returns the semantic mutation payload.
    #[must_use]
    pub const fn kind(&self) -> &RevisionEventKind {
        &self.kind
    }
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

    #[test]
    fn events_identify_semantic_targets_in_stable_order() {
        let revision_id = RevisionId::new();
        let asset_id = AssetId::new();
        let representation_id = RepresentationId::new();
        let events = [
            RevisionEvent::new(
                revision_id,
                0,
                RevisionEventKind::AssetImported { asset_id },
            ),
            RevisionEvent::new(
                revision_id,
                1,
                RevisionEventKind::RepresentationAdded {
                    asset_id,
                    representation_id,
                },
            ),
        ];

        assert_eq!(events[0].revision_id(), revision_id);
        assert_eq!(events[1].position(), 1);
        assert!(matches!(
            events[1].kind(),
            RevisionEventKind::RepresentationAdded {
                representation_id: id,
                ..
            } if *id == representation_id
        ));
    }
}
