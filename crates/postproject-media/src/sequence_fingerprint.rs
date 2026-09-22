//! Deterministic sampled fingerprints for compact image sequences.

use std::path::Path;

use postproject_core::{
    Error, ErrorKind, FrameRange, ImageSequenceDescriptor, ResourceFingerprint, Result,
};

use crate::fingerprint_file;

/// Algorithm identifier for sampled image-sequence member fingerprints.
pub const SEQUENCE_FINGERPRINT_ALGORITHM: &str = "pp-blake3-sequence-sampled-members";
/// Current sampled image-sequence fingerprint strategy version.
pub const SEQUENCE_FINGERPRINT_VERSION: u16 = 1;

const CONTEXT: &str = "postproject.org image sequence sampled members v1";

/// A collection fingerprint and the exact frames that contributed to it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceFingerprintReport {
    fingerprint: ResourceFingerprint,
    sampled_frames: Vec<i64>,
}

impl SequenceFingerprintReport {
    /// Returns sampled collection identity evidence.
    #[must_use]
    pub const fn fingerprint(&self) -> &ResourceFingerprint {
        &self.fingerprint
    }

    /// Returns the deterministic sample coverage in ascending frame order.
    #[must_use]
    pub fn sampled_frames(&self) -> &[i64] {
        &self.sampled_frames
    }

    /// Splits the report into its persistable fingerprint and coverage evidence.
    #[must_use]
    pub fn into_parts(self) -> (ResourceFingerprint, Vec<i64>) {
        (self.fingerprint, self.sampled_frames)
    }
}

/// Fingerprints the first, middle, and last declared members of an image sequence.
///
/// Recorded missing frames are excluded before deterministic sample ranks are
/// selected. The digest covers the canonical descriptor, sampled filenames,
/// and each sampled member's versioned content fingerprint; it excludes the
/// directory path and internal object identity.
///
/// # Errors
///
/// Returns an invalid-argument error if the sequence declares no present
/// members, or a fingerprint/filesystem error when a sampled member cannot be
/// fingerprinted safely.
pub fn fingerprint_image_sequence(
    directory: impl AsRef<Path>,
    descriptor: &ImageSequenceDescriptor,
) -> Result<SequenceFingerprintReport> {
    let sampled_frames = sampled_frames(descriptor)?;
    let mut hasher = blake3::Hasher::new_derive_key(CONTEXT);
    hash_text(&mut hasher, descriptor.pattern().prefix());
    hash_text(&mut hasher, descriptor.pattern().suffix());
    hasher.update(&[descriptor.pattern().padding()]);
    let frames = descriptor.frames();
    hasher.update(&frames.start().to_le_bytes());
    hasher.update(&frames.end().to_le_bytes());
    hasher.update(&frames.step().to_le_bytes());
    hasher.update(&descriptor.rate().numerator().to_le_bytes());
    hasher.update(&descriptor.rate().denominator().to_le_bytes());
    hash_i64_values(&mut hasher, descriptor.known_missing_frames());
    hash_i64_values(&mut hasher, &sampled_frames);

    for frame in &sampled_frames {
        let filename = descriptor.pattern().filename(*frame);
        let report = fingerprint_file(directory.as_ref().join(&filename))?;
        let fingerprint = report.fingerprint();
        hash_text(&mut hasher, &filename);
        hash_text(&mut hasher, fingerprint.algorithm());
        hasher.update(&fingerprint.version().to_le_bytes());
        hash_bytes(&mut hasher, fingerprint.value());
    }

    let fingerprint = ResourceFingerprint::new(
        SEQUENCE_FINGERPRINT_ALGORITHM,
        SEQUENCE_FINGERPRINT_VERSION,
        hasher.finalize().as_bytes().to_vec(),
    )?;
    Ok(SequenceFingerprintReport {
        fingerprint,
        sampled_frames,
    })
}

fn sampled_frames(descriptor: &ImageSequenceDescriptor) -> Result<Vec<i64>> {
    let frames = descriptor.frames();
    let present_count = frames.frame_count().checked_sub(
        u128::try_from(descriptor.known_missing_frames().len()).map_err(|error| {
            Error::new(
                ErrorKind::Fingerprint,
                format!("sequence exception count is unsupported: {error}"),
            )
        })?,
    );
    let Some(present_count) = present_count.filter(|count| *count > 0) else {
        return Err(Error::new(
            ErrorKind::InvalidArgument,
            "cannot fingerprint an image sequence with no declared members",
        ));
    };
    let mut ranks = vec![0, (present_count - 1) / 2, present_count - 1];
    ranks.sort_unstable();
    ranks.dedup();
    ranks
        .into_iter()
        .map(|rank| nth_present_frame(frames, descriptor.known_missing_frames(), rank))
        .collect()
}

fn nth_present_frame(frames: FrameRange, missing: &[i64], rank: u128) -> Result<i64> {
    let mut low = 0_u128;
    let mut high = frames.frame_count() - 1;
    while low < high {
        let middle = low + (high - low) / 2;
        let frame = frame_at(frames, middle)?;
        let missing_through_middle = missing.partition_point(|missing| *missing <= frame) as u128;
        let present_through_middle = middle + 1 - missing_through_middle;
        if present_through_middle > rank {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    frame_at(frames, low)
}

fn frame_at(frames: FrameRange, ordinal: u128) -> Result<i64> {
    let value = i128::from(frames.start())
        + i128::try_from(ordinal).map_err(frame_number_error)? * i128::from(frames.step());
    i64::try_from(value).map_err(frame_number_error)
}

fn frame_number_error(error: impl std::fmt::Display) -> Error {
    Error::new(
        ErrorKind::Fingerprint,
        format!("sequence frame number is unsupported: {error}"),
    )
}

fn hash_i64_values(hasher: &mut blake3::Hasher, values: &[i64]) {
    hash_count(hasher, values.len());
    for value in values {
        hasher.update(&value.to_le_bytes());
    }
}

fn hash_text(hasher: &mut blake3::Hasher, value: &str) {
    hash_bytes(hasher, value.as_bytes());
}

fn hash_bytes(hasher: &mut blake3::Hasher, value: &[u8]) {
    hash_count(hasher, value.len());
    hasher.update(value);
}

fn hash_count(hasher: &mut blake3::Hasher, count: usize) {
    hasher.update(&u64::try_from(count).unwrap_or(u64::MAX).to_le_bytes());
}
