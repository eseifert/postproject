//! Filesystem-facing media operations for libpostproject.
//!
//! Candidate discovery and resolution will build on the deterministic,
//! versioned fingerprint implementation introduced here.

#![forbid(unsafe_code)]

mod fingerprint;

pub use fingerprint::{
    FULL_HASH_LIMIT_BYTES, FingerprintCoverage, FingerprintReport, REGION_SIZE_BYTES,
    fingerprint_file,
};
