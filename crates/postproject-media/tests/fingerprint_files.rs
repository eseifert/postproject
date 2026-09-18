//! Filesystem integration tests for versioned fingerprints.

use std::{fs, io::Write};

use postproject_media::{
    FULL_HASH_LIMIT_BYTES, FingerprintCoverage, REGION_SIZE_BYTES, fingerprint_file,
};
use tempfile::NamedTempFile;

#[test]
fn threshold_selects_documented_fingerprint_algorithms() {
    let mut full = NamedTempFile::new().expect("create full-hash fixture");
    full.as_file_mut()
        .set_len(FULL_HASH_LIMIT_BYTES)
        .expect("size full-hash fixture");
    let full_report = fingerprint_file(full.path()).expect("hash threshold file");

    let mut sampled = NamedTempFile::new().expect("create sampled fixture");
    sampled
        .as_file_mut()
        .set_len(FULL_HASH_LIMIT_BYTES + 1)
        .expect("size sampled fixture");
    let sampled_report = fingerprint_file(sampled.path()).expect("hash sampled file");

    assert_eq!(full_report.coverage(), FingerprintCoverage::Full);
    assert_eq!(sampled_report.coverage(), FingerprintCoverage::Sampled);
    assert_eq!(full_report.fingerprint().version(), 1);
    assert_eq!(sampled_report.fingerprint().version(), 1);
}

#[test]
fn bytes_outside_sample_regions_are_explicitly_not_identity_evidence() {
    let size = usize::try_from(FULL_HASH_LIMIT_BYTES).expect("limit fits usize") + 1;
    let mut fixture = NamedTempFile::new().expect("create fixture");
    fixture.write_all(&vec![0_u8; size]).expect("write fixture");
    let before = fingerprint_file(fixture.path()).expect("hash baseline");

    let unsampled_offset = u64::try_from(REGION_SIZE_BYTES + 1).expect("offset fits u64");
    let file = fs::OpenOptions::new()
        .write(true)
        .open(fixture.path())
        .expect("reopen fixture");
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        file.write_at(&[1], unsampled_offset)
            .expect("change fixture");
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileExt;
        file.seek_write(&[1], unsampled_offset)
            .expect("change fixture");
    }
    drop(file);

    let after = fingerprint_file(fixture.path()).expect("hash changed fixture");
    assert_eq!(before.fingerprint(), after.fingerprint());
}
