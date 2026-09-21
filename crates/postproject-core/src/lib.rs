//! Application-neutral domain types and service contracts for `PostProject`.
//!
//! This crate deliberately has no persistence, FFI, or application-framework
//! dependencies. Backends and adapters depend on this crate, never the reverse.

#![forbid(unsafe_code)]

mod content;
mod error;
mod id;
mod identifier;
mod identifier_registry;
mod metadata;
mod metadata_registry;
mod model;
mod provenance;
mod resolution;
mod resource;
mod revision;
mod storage;
mod time;
mod transaction;
mod uri;

pub use content::{
    ContentStructure, ContentStructureKind, FrameRange, ImageSequenceDescriptor,
    ImageSequencePattern, MAX_CONTENT_MEMBERS, MAX_FRAME_PADDING, MAX_RESOURCE_ROLE_BYTES,
    MAX_SEQUENCE_EXCEPTIONS, MAX_SEQUENCE_PATTERN_BYTES, ResourceMember, ResourceRole,
};
pub use error::{Error, ErrorKind, Result};
pub use id::{
    ActivityId, AssetId, HostObjectBinding, LocatorId, MediaRootId, ObjectRef, ProductionId,
    RepresentationId, ResourceId, RevisionId, TransactionId,
};
pub use identifier::{
    ExternalIdentifier, IdentifierScheme, MAX_IDENTIFIER_QUALIFIER_BYTES,
    MAX_IDENTIFIER_SCHEME_BYTES, MAX_IDENTIFIER_VALUE_BYTES,
};
pub use identifier_registry::{
    EIDR_SCHEME, IDENTIFIER_SCHEMES, ISAN_SCHEME, IdentifierSchemeDefinition,
    IdentifierValidationKind, POSTPROJECT_APPLICATION_SCHEME, SMPTE_UMID_SCHEME,
    identifier_scheme_definition, validate_known_identifier,
};
pub use metadata::{
    DecimalValue, MAX_LANGUAGE_TAG_BYTES, MAX_METADATA_BINARY_BYTES, MAX_METADATA_COLLECTION_ITEMS,
    MAX_METADATA_DECIMAL_SCALE, MAX_METADATA_NESTING_DEPTH, MAX_METADATA_TEXT_BYTES,
    MAX_METADATA_TOTAL_BYTES, MAX_METADATA_URI_BYTES, MAX_PROPERTY_ID_BYTES,
    MAX_VOCABULARY_ID_BYTES, MetadataAssertion, MetadataField, MetadataMatch, MetadataProperty,
    MetadataValue, MetadataValueKind, PropertyId, RationalValue, VocabularyId,
};
pub use metadata_registry::{
    METADATA_VOCABULARIES, MetadataCardinality, MetadataPropertyAlias, MetadataPropertyDefinition,
    MetadataVocabularyDefinition, metadata_property_definition, metadata_vocabulary_definition,
};
pub use model::{
    Asset, MediaRoot, OriginalMediaImport, Production, Representation, RepresentationKind,
    Timestamp,
};
pub use provenance::{
    Activity, ActivityInput, ActivityKind, ActivityOutput, ActivityRole, AgentIdentity,
    MAX_ACTIVITY_EDGES, MAX_ACTIVITY_KIND_BYTES, MAX_ACTIVITY_ROLE_BYTES,
    MAX_PROVENANCE_NAME_BYTES, MAX_PROVENANCE_URI_BYTES, MAX_TOOL_VERSION_BYTES, ToolIdentity,
};
pub use resolution::{
    AvailabilityIssue, AvailabilityIssueKind, Confidence, EvidenceKind, RepresentationAvailability,
    RepresentationResolution, ResolutionCandidate, ResolutionEvidence, ResourceResolution,
    ResourceResolutionState,
};
pub use resource::{
    FileFacts, Locator, LocatorAvailability, RepresentationFingerprint, Resource,
    ResourceFingerprint,
};
pub use revision::{
    MAX_REVISION_MESSAGE_BYTES, MAX_REVISION_PAGE_SIZE, OriginIdentity, Revision, RevisionContext,
    RevisionEvent, RevisionEventKind,
};
pub use storage::{ProductionRead, ProductionStore, ProductionStoreTransaction};
pub use time::{RationalRate, RationalTime, TimeRange};
pub use transaction::{TransactionLifecycle, TransactionState};
