//! Subprocess-boundary tests for technical media inspection.

use std::{
    fs,
    path::{Path, PathBuf},
};

use postproject_media::{
    FfprobeInspector, InspectionOutcome, MediaInspector, TECHNICAL_INSPECTION_PROPERTY,
    TECHNICAL_METADATA_VOCABULARY,
};

fn media_fixture(directory: &Path) -> PathBuf {
    let path = directory.join("clip.mov");
    fs::write(&path, b"media fixture").expect("write media fixture");
    path
}

#[cfg(unix)]
fn fake_probe(directory: &Path, stdout: &str, exit_code: i32) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = directory.join("ffprobe-fake");
    let script = format!("#!/bin/sh\nprintf '%s' '{stdout}'\nexit {exit_code}\n");
    fs::write(&path, script).expect("write fake ffprobe");
    let mut permissions = fs::metadata(&path).expect("fake metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fake executable");
    path
}

#[cfg(windows)]
fn fake_probe(directory: &Path, stdout: &str, exit_code: i32) -> PathBuf {
    let path = directory.join("ffprobe-fake.cmd");
    let script = format!("@echo off\r\necho {stdout}\r\nexit /b {exit_code}\r\n");
    fs::write(&path, script).expect("write fake ffprobe");
    path
}

#[test]
fn converts_ffprobe_json_to_vocabulary_metadata() {
    let temporary = tempfile::tempdir().expect("create temporary directory");
    let media = media_fixture(temporary.path());
    let output = r#"{
        "format": {
            "format_name": "mov,mp4",
            "format_long_name": "QuickTime / MOV",
            "duration": "1.250000",
            "size": "42",
            "bit_rate": "800000",
            "tags": {"timecode": "01:00:00:00", "com.example.id": "A001"}
        },
        "streams": [{
            "index": 0,
            "codec_name": "prores",
            "codec_type": "video",
            "width": 1920,
            "height": 1080,
            "pix_fmt": "yuv422p10le",
            "bits_per_raw_sample": "10",
            "avg_frame_rate": "24000/1001",
            "tags": {"timecode": "01:00:00:00"}
        }]
    }"#;
    let executable = fake_probe(temporary.path(), output, 0);

    let outcome = FfprobeInspector::with_executable(executable)
        .inspect(&media)
        .expect("run inspector");
    let InspectionOutcome::Inspected(metadata) = outcome else {
        panic!("expected inspection metadata");
    };
    let [assertion] = metadata.assertions() else {
        panic!("expected one assertion");
    };
    assert_eq!(
        assertion.property().vocabulary().as_str(),
        TECHNICAL_METADATA_VOCABULARY
    );
    assert_eq!(
        assertion.property().property().as_str(),
        TECHNICAL_INSPECTION_PROPERTY
    );
    let fields = assertion.value().as_structure().expect("structured result");
    assert!(
        fields
            .iter()
            .any(|field| field.name().as_str() == "container")
    );
    let streams = fields
        .iter()
        .find(|field| field.name().as_str() == "streams")
        .expect("streams field")
        .value()
        .as_list()
        .expect("stream list");
    assert_eq!(streams.len(), 1);
}

#[test]
fn reports_missing_ffprobe_as_an_optional_capability_gap() {
    let temporary = tempfile::tempdir().expect("create temporary directory");
    let media = media_fixture(temporary.path());
    let missing = temporary.path().join("definitely-not-ffprobe");

    let outcome = FfprobeInspector::with_executable(missing)
        .inspect(&media)
        .expect("missing inspector is not an error");
    assert!(matches!(outcome, InspectionOutcome::Unavailable { .. }));
}

#[test]
fn rejects_malformed_tool_output_without_failing_the_adapter_call() {
    let temporary = tempfile::tempdir().expect("create temporary directory");
    let media = media_fixture(temporary.path());
    let executable = fake_probe(temporary.path(), "not-json", 0);

    let outcome = FfprobeInspector::with_executable(executable)
        .inspect(&media)
        .expect("malformed output is a reported outcome");
    assert!(matches!(outcome, InspectionOutcome::Failed { .. }));
}
