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
mod sequence_fingerprint;

pub use fingerprint::{
    FULL_FINGERPRINT_ALGORITHM, FULL_HASH_LIMIT_BYTES, FingerprintCoverage, FingerprintReport,
    REGION_SIZE_BYTES, SAMPLED_FINGERPRINT_ALGORITHM, fingerprint_file,
};
pub use import::{
    FileResourceSource, ImageSequenceSource, prepare_confirmed_locator,
    prepare_image_sequence_representation, prepare_media_root,
    prepare_ordered_parts_representation, prepare_original_media, prepare_package_representation,
    prepare_single_file_representation,
};
pub use path::canonical_file_uri;
pub use representation_fingerprint::{
    REPRESENTATION_FINGERPRINT_ALGORITHM, REPRESENTATION_FINGERPRINT_VERSION,
    fingerprint_representation,
};
pub use resolver::{MediaResolver, MediaRootMapping, ResolverOptions};
pub use sequence_fingerprint::{
    SEQUENCE_FINGERPRINT_ALGORITHM, SEQUENCE_FINGERPRINT_VERSION, SequenceFingerprintReport,
    fingerprint_image_sequence,
};
