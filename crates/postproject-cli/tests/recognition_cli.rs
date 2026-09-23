//! Compound-media import coverage through the distributed CLI surface.

use std::{fs, path::Path};

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;

fn run_json(arguments: &[&str]) -> Value {
    let assertion = cargo_bin_cmd!("postproject")
        .arg("--json")
        .args(arguments)
        .assert()
        .success();
    serde_json::from_slice(&assertion.get_output().stdout).expect("command emits valid JSON")
}

#[test]
fn imports_checked_in_camera_card_as_one_package() {
    let temporary = tempfile::tempdir().expect("create temporary directory");
    let production = temporary.path().join("camera-card.pproj");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/avchd-card");
    let production = production.to_str().expect("UTF-8 production path");
    run_json(&["init", production]);

    let imported = run_json(&[
        "media",
        "add",
        production,
        fixture.to_str().expect("UTF-8 fixture path"),
    ]);
    assert_eq!(imported["resource_count"], 4);
    let shown = run_json(&[
        "media",
        "show",
        production,
        imported["asset_id"].as_str().expect("asset ID"),
    ]);
    assert_eq!(shown["representations"][0]["structure"], "package");
    assert_eq!(
        shown["representations"][0]["resources"]
            .as_array()
            .expect("resources")
            .len(),
        4
    );
}

#[test]
fn imports_a_sparse_sequence_with_an_explicit_rate() {
    let temporary = tempfile::tempdir().expect("create temporary directory");
    let production = temporary.path().join("sequence.pproj");
    let sequence = temporary.path().join("plate");
    fs::create_dir(&sequence).expect("create sequence directory");
    for name in ["plate.1001.exr", "plate.1003.exr"] {
        fs::write(sequence.join(name), name).expect("write frame");
    }
    let production = production.to_str().expect("UTF-8 production path");
    run_json(&["init", production]);

    let imported = run_json(&[
        "media",
        "add",
        production,
        sequence.to_str().expect("UTF-8 sequence path"),
        "--sequence-rate",
        "24000/1001",
    ]);
    assert_eq!(imported["resource_count"], 1);
    let shown = run_json(&[
        "media",
        "show",
        production,
        imported["asset_id"].as_str().expect("asset ID"),
    ]);
    assert_eq!(shown["representations"][0]["structure"], "image_sequence");
}
