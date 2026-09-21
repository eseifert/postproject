//! Filesystem-facing media operations for `PostProject`.
//!
//! Candidate discovery and resolution will build on the deterministic,
//! versioned fingerprint implementation introduced here.

#![forbid(unsafe_code)]

mod fingerprint;
mod import;
mod path;
mod representation_fingerprint;
mod resolver;

pub use fingerprint::{
    FULL_FINGERPRINT_ALGORITHM, FULL_HASH_LIMIT_BYTES, FingerprintCoverage, FingerprintReport,
    REGION_SIZE_BYTES, SAMPLED_FINGERPRINT_ALGORITHM, fingerprint_file,
};
pub use import::{
    prepare_confirmed_locator, prepare_media_root, prepare_original_media,
    prepare_single_file_representation,
};
pub use path::canonical_file_uri;
pub use representation_fingerprint::{
    REPRESENTATION_FINGERPRINT_ALGORITHM, REPRESENTATION_FINGERPRINT_VERSION,
    fingerprint_representation,
};
pub use resolver::{MediaResolver, ResolverOptions};
