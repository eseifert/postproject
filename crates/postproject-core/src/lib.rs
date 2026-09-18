//! Application-neutral domain types and service contracts for libpostproject.
//!
//! This crate deliberately has no persistence, FFI, or application-framework
//! dependencies. Backends and adapters depend on this crate, never the reverse.

#![forbid(unsafe_code)]

mod error;
mod id;
mod model;
mod resolution;
mod transaction;

pub use error::{Error, ErrorKind, Result};
pub use id::{AssetId, LocationId, MediaRootId, ProjectId, RepresentationId, TransactionId};
pub use model::{
    Asset, FileFacts, Fingerprint, Location, LocationAvailability, MediaRoot, Project,
    Representation, RepresentationKind, Timestamp,
};
pub use resolution::{
    Confidence, EvidenceKind, Resolution, ResolutionCandidate, ResolutionEvidence, ResolutionState,
};
pub use transaction::{TransactionLifecycle, TransactionState};
