//! End-to-end tests of the public command-line workflow.

use std::{fs, str::FromStr};

use assert_cmd::cargo::cargo_bin_cmd;
use postproject_core::{
    AssetId, MetadataField, MetadataProperty, MetadataValue, ObjectRef, PropertyId, VocabularyId,
};
use postproject_storage_sqlite::SqliteProduction;
use serde_json::Value;

const TRANSCODE_KIND: &str = "org.postproject:transcode";
const PRIMARY_INPUT_ROLE: &str = "org.postproject:input.primary-video";
const PROXY_OUTPUT_ROLE: &str = "org.postproject:output.proxy";

fn run_json(arguments: &[&str]) -> Value {
    let assertion = cargo_bin_cmd!("postproject")
        .arg("--json")
        .args(arguments)
        .assert()
        .success();
    serde_json::from_slice(&assertion.get_output().stdout).expect("command emits valid JSON")
}

fn exercise_identifiers(production: &str, asset_id: &str) {
    let identifier = run_json(&[
        "identifier",
        "add",
        production,
        "asset",
        asset_id,
        "com.example.asset",
        "asset-42",
        "--qualifier",
        "primary",
    ]);
    assert_eq!(identifier["scheme"], "com.example.asset");
    assert_eq!(identifier["value"], "asset-42");

    let identifiers = run_json(&["identifier", "list", production, "asset", asset_id]);
    assert_eq!(identifiers.as_array().expect("identifier array").len(), 1);
    assert_eq!(identifiers[0]["qualifier"], "primary");

    let found = run_json(&[
        "identifier",
        "find",
        production,
        "com.example.asset",
        "asset-42",
    ]);
    assert_eq!(found.as_array().expect("object array").len(), 1);
    assert_eq!(found[0]["kind"], "asset");
    assert_eq!(found[0]["id"], asset_id);
}

fn exercise_metadata(production_path: &str, asset_id: &str) {
    let vocabulary = "http://iptc.org/std/videometadatahub/1.0";
    for (value, language) in [("Interview", "en-US"), ("Gespräch", "de-DE")] {
        let added = run_json(&[
            "metadata",
            "add-text",
            production_path,
            "asset",
            asset_id,
            vocabulary,
            "title",
            value,
            "--language",
            language,
        ]);
        assert_eq!(added["value"]["type"], "lang_string");
        assert_eq!(added["value"]["language"], language);
    }

    let found = run_json(&["metadata", "find", production_path, vocabulary, "title"]);
    assert_eq!(found.as_array().expect("metadata matches").len(), 2);

    inject_structured_metadata(production_path, asset_id);
    let listed = run_json(&["metadata", "list", production_path, "asset", asset_id]);
    assert_eq!(listed.as_array().expect("metadata assertions").len(), 3);
    assert_eq!(listed[0]["value"]["type"], "struct");
    assert_eq!(listed[0]["value"]["fields"][0]["name"], "name");

    run_json(&[
        "metadata",
        "remove",
        production_path,
        "asset",
        asset_id,
        vocabulary,
        "title",
    ]);
    let listed = run_json(&["metadata", "list", production_path, "asset", asset_id]);
    assert_eq!(listed.as_array().expect("remaining metadata").len(), 1);
}

fn inject_structured_metadata(production_path: &str, asset_id: &str) {
    let target = ObjectRef::Asset(AssetId::from_str(asset_id).expect("parse asset ID"));
    let property = MetadataProperty::new(
        VocabularyId::new("com.example.editor/metadata").unwrap(),
        PropertyId::new("contact").unwrap(),
    );
    let value = MetadataValue::structure(vec![MetadataField::new(
        PropertyId::new("name").unwrap(),
        MetadataValue::string("Camera department").unwrap(),
    )])
    .unwrap();
    let mut production =
        SqliteProduction::open(production_path).expect("open production for test metadata");
    let mut transaction = production.begin_transaction().unwrap();
    transaction
        .add_metadata_value(target, &property, &value)
        .unwrap();
    transaction.commit().unwrap();
}

fn exercise_provenance(
    production: &str,
    input_representation_id: &str,
    directory: &std::path::Path,
) {
    let proxy = directory.join("proxy.mov");
    fs::write(&proxy, b"derived proxy fixture media").expect("write proxy fixture");
    let imported = run_json(&[
        "media",
        "add",
        production,
        proxy.to_str().expect("UTF-8 proxy path"),
        "--name",
        "Editorial proxy",
    ]);
    let output_representation_id = imported["representation_id"]
        .as_str()
        .expect("proxy representation ID");
    let input = format!("{input_representation_id}={PRIMARY_INPUT_ROLE}");
    let output = format!("{output_representation_id}={PROXY_OUTPUT_ROLE}");

    let created = run_json(&[
        "activity",
        "add",
        production,
        TRANSCODE_KIND,
        "--input",
        &input,
        "--output",
        &output,
        "--started-at-unix-micros",
        "100",
        "--finished-at-unix-micros",
        "200",
        "--tool-name",
        "FFmpeg",
        "--tool-version",
        "8.0",
        "--tool-uri",
        "https://ffmpeg.org",
        "--agent-name",
        "Render worker",
        "--agent-identifier-scheme",
        "com.example.worker",
        "--agent-identifier-value",
        "worker-42",
        "--agent-identifier-qualifier",
        "primary",
    ]);
    let activity_id = created["id"].as_str().expect("activity ID");
    assert_eq!(created["started_at_unix_micros"], 100);
    assert_eq!(created["finished_at_unix_micros"], 200);
    assert_eq!(created["tool"]["name"], "FFmpeg");
    assert_eq!(created["tool"]["version"], "8.0");
    assert_eq!(created["tool"]["uri"], "https://ffmpeg.org/");
    assert_eq!(created["agent"]["name"], "Render worker");
    let agent_identifier = &created["agent"]["identifier"];
    assert_eq!(agent_identifier["scheme"], "com.example.worker");
    assert_eq!(agent_identifier["value"], "worker-42");
    assert_eq!(agent_identifier["qualifier"], "primary");
    let created_input = &created["inputs"][0];
    assert_eq!(created_input["role"], PRIMARY_INPUT_ROLE);
    assert_eq!(created["outputs"][0]["role"], PROXY_OUTPUT_ROLE);

    let parameter = run_json(&[
        "metadata",
        "add-text",
        production,
        "activity",
        activity_id,
        "com.example.transcode",
        "preset",
        "editorial-proxy-h264",
    ]);
    assert_eq!(parameter["target_kind"], "activity");
    assert_eq!(parameter["value"]["value"], "editorial-proxy-h264");

    let listed = run_json(&["activity", "list", production]);
    assert_eq!(listed.as_array().expect("activity array").len(), 1);
    assert_eq!(listed[0]["id"], activity_id);

    let producing = run_json(&[
        "activity",
        "producing",
        production,
        output_representation_id,
    ]);
    assert_eq!(producing[0]["id"], activity_id);
    let consuming = run_json(&["activity", "consuming", production, input_representation_id]);
    assert_eq!(consuming[0]["id"], activity_id);

    let ancestors = run_json(&[
        "activity",
        "ancestors",
        production,
        output_representation_id,
    ]);
    assert_eq!(ancestors[0]["representation_id"], input_representation_id);
    let descendants = run_json(&[
        "activity",
        "descendants",
        production,
        input_representation_id,
    ]);
    assert_eq!(
        descendants[0]["representation_id"],
        output_representation_id
    );
}

#[test]
fn lifecycle_and_explicit_ambiguous_confirmation() {
    let directory = tempfile::tempdir().expect("create test directory");
    let production = directory.path().join("production.pproj");
    let original = directory.path().join("original.mov");
    fs::write(&original, b"identifiable fixture media").expect("write original fixture");

    let initialized = run_json(&[
        "init",
        production.to_str().expect("UTF-8 production path"),
        "--name",
        "CLI workflow",
    ]);
    assert_eq!(initialized["display_name"], "CLI workflow");

    let imported = run_json(&[
        "media",
        "add",
        production.to_str().expect("UTF-8 production path"),
        original.to_str().expect("UTF-8 media path"),
        "--name",
        "Camera original",
    ]);
    let asset_id = imported["asset_id"]
        .as_str()
        .expect("asset ID in import output");

    let listed = run_json(&[
        "media",
        "list",
        production.to_str().expect("UTF-8 production path"),
    ]);
    assert_eq!(listed.as_array().expect("asset array").len(), 1);
    assert_eq!(listed[0]["id"], asset_id);

    exercise_identifiers(
        production.to_str().expect("UTF-8 production path"),
        asset_id,
    );
    exercise_metadata(
        production.to_str().expect("UTF-8 production path"),
        asset_id,
    );

    let candidates = directory.path().join("candidates");
    fs::create_dir(&candidates).expect("create candidates root");
    fs::write(candidates.join("a.mov"), b"identifiable fixture media")
        .expect("write first candidate");
    fs::write(candidates.join("b.mov"), b"identifiable fixture media")
        .expect("write second candidate");
    fs::remove_file(&original).expect("make original locator unavailable");

    run_json(&[
        "root",
        "add",
        production.to_str().expect("UTF-8 production path"),
        candidates.to_str().expect("UTF-8 root path"),
        "--label",
        "Relocated",
    ]);

    let ambiguous = run_json(&[
        "media",
        "resolve",
        production.to_str().expect("UTF-8 production path"),
        asset_id,
    ]);
    assert_eq!(ambiguous["resolutions"][0]["availability"], "ambiguous");
    assert_eq!(
        ambiguous["resolutions"][0]["resources"][0]["state"],
        "ambiguous"
    );
    let confirmed_uri = ambiguous["resolutions"][0]["resources"][0]["candidates"][0]["uri"]
        .as_str()
        .expect("candidate URI");

    let confirmed = run_json(&[
        "media",
        "resolve",
        production.to_str().expect("UTF-8 production path"),
        asset_id,
        "--confirm",
        confirmed_uri,
    ]);
    assert_eq!(confirmed["confirmed_uri"], confirmed_uri);

    let resolved = run_json(&[
        "media",
        "resolve",
        production.to_str().expect("UTF-8 production path"),
        asset_id,
    ]);
    assert_eq!(resolved["resolutions"][0]["availability"], "online");
    assert_eq!(
        resolved["resolutions"][0]["resources"][0]["state"],
        "online_at_known_locator"
    );

    let shown = run_json(&[
        "media",
        "show",
        production.to_str().expect("UTF-8 production path"),
        asset_id,
    ]);
    assert_eq!(
        shown["representations"][0]["resources"][0]["locators"]
            .as_array()
            .expect("locators")
            .len(),
        2
    );
}

#[test]
fn records_and_queries_provenance() {
    let directory = tempfile::tempdir().expect("create test directory");
    let production = directory.path().join("provenance.pproj");
    let original = directory.path().join("original.mov");
    fs::write(&original, b"provenance source fixture").expect("write source fixture");

    run_json(&["init", production.to_str().expect("UTF-8 production path")]);
    let imported = run_json(&[
        "media",
        "add",
        production.to_str().expect("UTF-8 production path"),
        original.to_str().expect("UTF-8 media path"),
    ]);
    let representation_id = imported["representation_id"]
        .as_str()
        .expect("source representation ID");

    exercise_provenance(
        production.to_str().expect("UTF-8 production path"),
        representation_id,
        directory.path(),
    );
}

#[test]
fn reads_revision_pages_and_semantic_events() {
    let directory = tempfile::tempdir().expect("create test directory");
    let production = directory.path().join("revisions.pproj");
    let original = directory.path().join("original.mov");
    fs::write(&original, b"revision source fixture").expect("write source fixture");
    let production_path = production.to_str().expect("UTF-8 production path");

    run_json(&["init", production_path]);
    assert!(run_json(&["revisions", "latest", production_path]).is_null());
    run_json(&[
        "media",
        "add",
        production_path,
        original.to_str().expect("UTF-8 media path"),
    ]);

    let revisions = run_json(&[
        "revisions",
        "since",
        production_path,
        "--after",
        "0",
        "--limit",
        "1",
    ]);
    let revisions = revisions.as_array().expect("revision array");
    assert_eq!(revisions.len(), 1);
    assert_eq!(revisions[0]["sequence"], 1);
    assert!(revisions[0]["transaction_id"].is_string());
    assert_eq!(revisions[0]["origin"]["name"], "postproject-cli");
    assert!(revisions[0]["origin"]["version"].is_string());
    assert_eq!(revisions[0]["message"], "Import media");
    let revision_id = revisions[0]["id"].as_str().expect("revision ID");

    let latest = run_json(&["revisions", "latest", production_path]);
    assert_eq!(latest["id"], revision_id);
    let events = run_json(&["revisions", "events", production_path, revision_id]);
    let events = events.as_array().expect("revision event array");
    assert_eq!(events.len(), 5);
    assert_eq!(events[0]["position"], 0);
    assert_eq!(events[0]["kind"], "asset_imported");
    assert_eq!(events[1]["kind"], "representation_added");
    assert_eq!(events[2]["kind"], "resource_added");
    assert_eq!(events[3]["kind"], "representation_resource_added");
    assert_eq!(events[3]["structural_position"], 0);
    assert_eq!(events[4]["kind"], "locator_added");
}
