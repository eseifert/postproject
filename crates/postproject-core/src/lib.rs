//! Application-neutral domain types and service contracts for `PostProject`.
//!
//! This crate deliberately has no persistence, FFI, or application-framework
//! dependencies. Backends and adapters depend on this crate, never the reverse.

#![forbid(unsafe_code)]

mod error;
mod id;
mod identifier;
mod model;
mod resolution;
mod storage;
mod transaction;
mod uri;

pub use error::{Error, ErrorKind, Result};
pub use id::{
    ActivityId, AssetId, LocationId, MediaRootId, ObjectRef, ProjectId, RepresentationId,
    RevisionId, TransactionId,
};
pub use identifier::{
    ExternalIdentifier, IdentifierScheme, MAX_IDENTIFIER_QUALIFIER_BYTES,
    MAX_IDENTIFIER_SCHEME_BYTES, MAX_IDENTIFIER_VALUE_BYTES,
};
pub use model::{
    Asset, FileFacts, Fingerprint, Location, LocationAvailability, MediaRoot, OriginalMediaImport,
    Project, Representation, RepresentationKind, Timestamp,
};
pub use resolution::{
    Confidence, EvidenceKind, Resolution, ResolutionCandidate, ResolutionEvidence, ResolutionState,
};
pub use storage::{ProjectRead, ProjectStore, ProjectStoreTransaction};
pub use transaction::{TransactionLifecycle, TransactionState};
