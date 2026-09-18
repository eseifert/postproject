//! Filesystem-facing media operations for libpostproject.
//!
//! Candidate discovery and resolution will build on the deterministic,
//! versioned fingerprint implementation introduced here.

#![forbid(unsafe_code)]

mod fingerprint;
mod import;
mod path;

pub use fingerprint::{
    FULL_HASH_LIMIT_BYTES, FingerprintCoverage, FingerprintReport, REGION_SIZE_BYTES,
    fingerprint_file,
};
pub use import::{prepare_media_root, prepare_original_media};
pub use path::canonical_file_uri;
