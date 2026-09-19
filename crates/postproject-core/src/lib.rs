//! Application-neutral domain types and service contracts for `PostProject`.
//!
//! This crate deliberately has no persistence, FFI, or application-framework
//! dependencies. Backends and adapters depend on this crate, never the reverse.

#![forbid(unsafe_code)]

mod content;
mod error;
mod id;
mod identifier;
mod metadata;
mod model;
mod resolution;
mod resource;
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
    ActivityId, AssetId, LocationId, LocatorId, MediaRootId, ObjectRef, ProjectId,
    RepresentationId, ResourceId, RevisionId, TransactionId,
};
pub use identifier::{
    ExternalIdentifier, IdentifierScheme, MAX_IDENTIFIER_QUALIFIER_BYTES,
    MAX_IDENTIFIER_SCHEME_BYTES, MAX_IDENTIFIER_VALUE_BYTES,
};
pub use metadata::{
    DecimalValue, MAX_LANGUAGE_TAG_BYTES, MAX_METADATA_BINARY_BYTES, MAX_METADATA_COLLECTION_ITEMS,
    MAX_METADATA_DECIMAL_SCALE, MAX_METADATA_NESTING_DEPTH, MAX_METADATA_TEXT_BYTES,
    MAX_METADATA_TOTAL_BYTES, MAX_METADATA_URI_BYTES, MAX_PROPERTY_ID_BYTES,
    MAX_VOCABULARY_ID_BYTES, MetadataAssertion, MetadataField, MetadataMatch, MetadataProperty,
    MetadataValue, MetadataValueKind, PropertyId, RationalValue, VocabularyId,
};
pub use model::{
    Asset, FileFacts, Fingerprint, Location, LocationAvailability, MediaRoot, OriginalMediaImport,
    Project, Representation, RepresentationKind, Timestamp,
};
pub use resolution::{
    Confidence, EvidenceKind, Resolution, ResolutionCandidate, ResolutionEvidence, ResolutionState,
};
pub use resource::{
    Locator, LocatorAvailability, RepresentationFingerprint, Resource, ResourceFingerprint,
};
pub use storage::{ProjectRead, ProjectStore, ProjectStoreTransaction};
pub use time::{RationalRate, RationalTime, TimeRange};
pub use transaction::{TransactionLifecycle, TransactionState};
