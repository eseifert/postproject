//! Public CLI inventory coverage.

use std::fs;

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
fn inventory_command_reports_machine_inspectable_results_and_cache_stats() {
    let temporary = tempfile::tempdir().expect("create temporary directory");
    let production = temporary.path().join("inventory.pproj");
    let media = temporary.path().join("rushes");
    let cache = temporary.path().join("cache/inventory.json");
    fs::create_dir(&media).expect("create media directory");
    let known = media.join("known.mov");
    fs::write(&known, b"known media").expect("write known media");

    run_json(&["init", production.to_str().expect("UTF-8 path")]);
    run_json(&[
        "root",
        "add",
        production.to_str().expect("UTF-8 path"),
        "rushes",
    ]);
    run_json(&[
        "media",
        "add",
        production.to_str().expect("UTF-8 path"),
        known.to_str().expect("UTF-8 path"),
    ]);
    fs::write(media.join("new.mov"), b"new media").expect("write new media");
    let mapping = format!("rushes={}", media.display());
    let arguments = [
        "media",
        "inventory",
        production.to_str().expect("UTF-8 path"),
        "--root-map",
        &mapping,
        "--cache",
        cache.to_str().expect("UTF-8 path"),
    ];

    let first = run_json(&arguments);
    let categories = first["items"]
        .as_array()
        .expect("inventory items")
        .iter()
        .map(|item| item["category"].as_str().expect("category"))
        .collect::<Vec<_>>();
    assert!(categories.contains(&"known_online"));
    assert!(categories.contains(&"new_candidate"));
    assert!(
        first["stats"]["fingerprints_computed"]
            .as_u64()
            .unwrap_or(0)
            > 0
    );

    let second = run_json(&arguments);
    assert_eq!(second["items"], first["items"]);
    assert_eq!(second["stats"]["fingerprints_computed"], 0);
    assert!(
        second["stats"]["fingerprint_cache_hits"]
            .as_u64()
            .unwrap_or(0)
            > 0
    );
}
