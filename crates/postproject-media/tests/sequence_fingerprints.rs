//! Sampled image-sequence fingerprint integration tests.

use std::fs;

use postproject_core::{
    FrameRange, ImageSequenceDescriptor, ImageSequencePattern, RationalRate, ResourceId,
};
use postproject_media::{
    SEQUENCE_FINGERPRINT_ALGORITHM, SEQUENCE_FINGERPRINT_VERSION, fingerprint_image_sequence,
};

fn descriptor(missing: Vec<i64>) -> ImageSequenceDescriptor {
    ImageSequenceDescriptor::new(
        ResourceId::new(),
        ImageSequencePattern::new("plate.", ".exr", 4).expect("valid pattern"),
        FrameRange::new(1, 5, 1).expect("valid range"),
        RationalRate::new(24, 1).expect("valid rate"),
        missing,
    )
    .expect("valid descriptor")
}

fn write_frames(directory: &std::path::Path) {
    for frame in 1..=5 {
        fs::write(
            directory.join(format!("plate.{frame:04}.exr")),
            format!("frame {frame}"),
        )
        .expect("write frame");
    }
}

#[test]
fn sample_coverage_is_explicit_and_content_sensitive() {
    let directory = tempfile::tempdir().expect("create directory");
    write_frames(directory.path());
    let descriptor = descriptor(Vec::new());

    let initial =
        fingerprint_image_sequence(directory.path(), &descriptor).expect("fingerprint sequence");
    assert_eq!(initial.sampled_frames(), &[1, 3, 5]);
    assert_eq!(
        initial.fingerprint().algorithm(),
        SEQUENCE_FINGERPRINT_ALGORITHM
    );
    assert_eq!(
        initial.fingerprint().version(),
        SEQUENCE_FINGERPRINT_VERSION
    );

    fs::write(directory.path().join("plate.0003.exr"), b"changed").expect("change sampled frame");
    let changed = fingerprint_image_sequence(directory.path(), &descriptor)
        .expect("fingerprint changed sequence");
    assert_ne!(initial.fingerprint(), changed.fingerprint());
}

#[test]
fn recorded_gaps_are_excluded_before_sampling() {
    let directory = tempfile::tempdir().expect("create directory");
    write_frames(directory.path());

    let report = fingerprint_image_sequence(directory.path(), &descriptor(vec![3]))
        .expect("fingerprint sparse sequence");

    assert_eq!(report.sampled_frames(), &[1, 2, 5]);
}

#[test]
fn relocation_does_not_change_collection_identity() {
    let first = tempfile::tempdir().expect("create first directory");
    let second = tempfile::tempdir().expect("create second directory");
    write_frames(first.path());
    write_frames(second.path());
    let descriptor = descriptor(Vec::new());

    let first = fingerprint_image_sequence(first.path(), &descriptor).expect("fingerprint first");
    let second =
        fingerprint_image_sequence(second.path(), &descriptor).expect("fingerprint second");

    assert_eq!(first.fingerprint(), second.fingerprint());
}
