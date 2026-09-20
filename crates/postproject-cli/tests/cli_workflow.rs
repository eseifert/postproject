//! End-to-end tests of the public command-line workflow.

use std::{fs, str::FromStr};

use assert_cmd::cargo::cargo_bin_cmd;
use postproject_core::{
    AssetId, MetadataField, MetadataProperty, MetadataValue, ObjectRef, PropertyId, VocabularyId,
};
use postproject_storage_sqlite::SqliteProject;
use serde_json::Value;

fn run_json(arguments: &[&str]) -> Value {
    let assertion = cargo_bin_cmd!("postproject")
        .arg("--json")
        .args(arguments)
        .assert()
        .success();
    serde_json::from_slice(&assertion.get_output().stdout).expect("command emits valid JSON")
}

fn exercise_identifiers(project: &str, asset_id: &str) {
    let identifier = run_json(&[
        "identifier",
        "add",
        project,
        "asset",
        asset_id,
        "com.example.asset",
        "asset-42",
        "--qualifier",
        "primary",
    ]);
    assert_eq!(identifier["scheme"], "com.example.asset");
    assert_eq!(identifier["value"], "asset-42");

    let identifiers = run_json(&["identifier", "list", project, "asset", asset_id]);
    assert_eq!(identifiers.as_array().expect("identifier array").len(), 1);
    assert_eq!(identifiers[0]["qualifier"], "primary");

    let found = run_json(&[
        "identifier",
        "find",
        project,
        "com.example.asset",
        "asset-42",
    ]);
    assert_eq!(found.as_array().expect("object array").len(), 1);
    assert_eq!(found[0]["kind"], "asset");
    assert_eq!(found[0]["id"], asset_id);
}

fn exercise_metadata(project_path: &str, asset_id: &str) {
    let vocabulary = "http://iptc.org/std/videometadatahub/1.0";
    for (value, language) in [("Interview", "en-US"), ("Gespräch", "de-DE")] {
        let added = run_json(&[
            "metadata",
            "add-text",
            project_path,
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

    let found = run_json(&["metadata", "find", project_path, vocabulary, "title"]);
    assert_eq!(found.as_array().expect("metadata matches").len(), 2);

    inject_structured_metadata(project_path, asset_id);
    let listed = run_json(&["metadata", "list", project_path, "asset", asset_id]);
    assert_eq!(listed.as_array().expect("metadata assertions").len(), 3);
    assert_eq!(listed[0]["value"]["type"], "struct");
    assert_eq!(listed[0]["value"]["fields"][0]["name"], "name");

    run_json(&[
        "metadata",
        "remove",
        project_path,
        "asset",
        asset_id,
        vocabulary,
        "title",
    ]);
    let listed = run_json(&["metadata", "list", project_path, "asset", asset_id]);
    assert_eq!(listed.as_array().expect("remaining metadata").len(), 1);
}

fn inject_structured_metadata(project_path: &str, asset_id: &str) {
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
    let mut project = SqliteProject::open(project_path).expect("open project for test metadata");
    let mut transaction = project.begin_transaction().unwrap();
    transaction
        .add_metadata_value(target, &property, &value)
        .unwrap();
    transaction.commit().unwrap();
}

fn exercise_provenance(project: &str, input_representation_id: &str, directory: &std::path::Path) {
    let proxy = directory.join("proxy.mov");
    fs::write(&proxy, b"derived proxy fixture media").expect("write proxy fixture");
    let imported = run_json(&[
        "media",
        "add",
        project,
        proxy.to_str().expect("UTF-8 proxy path"),
        "--name",
        "Editorial proxy",
    ]);
    let output_representation_id = imported["representation_id"]
        .as_str()
        .expect("proxy representation ID");
    let input = format!("{input_representation_id}=postproject:input.primary-video");
    let output = format!("{output_representation_id}=postproject:output.proxy");

    let created = run_json(&[
        "activity",
        "add",
        project,
        "postproject:transcode",
        "--input",
        &input,
        "--output",
        &output,
    ]);
    let activity_id = created["id"].as_str().expect("activity ID");
    assert_eq!(
        created["inputs"][0]["role"],
        "postproject:input.primary-video"
    );
    assert_eq!(created["outputs"][0]["role"], "postproject:output.proxy");

    let listed = run_json(&["activity", "list", project]);
    assert_eq!(listed.as_array().expect("activity array").len(), 1);
    assert_eq!(listed[0]["id"], activity_id);

    let producing = run_json(&["activity", "producing", project, output_representation_id]);
    assert_eq!(producing[0]["id"], activity_id);
    let consuming = run_json(&["activity", "consuming", project, input_representation_id]);
    assert_eq!(consuming[0]["id"], activity_id);

    let ancestors = run_json(&["activity", "ancestors", project, output_representation_id]);
    assert_eq!(ancestors[0]["representation_id"], input_representation_id);
    let descendants = run_json(&["activity", "descendants", project, input_representation_id]);
    assert_eq!(
        descendants[0]["representation_id"],
        output_representation_id
    );
}

#[test]
fn lifecycle_and_explicit_ambiguous_confirmation() {
    let directory = tempfile::tempdir().expect("create test directory");
    let project = directory.path().join("production.pproj");
    let original = directory.path().join("original.mov");
    fs::write(&original, b"identifiable fixture media").expect("write original fixture");

    let initialized = run_json(&[
        "init",
        project.to_str().expect("UTF-8 project path"),
        "--name",
        "CLI workflow",
    ]);
    assert_eq!(initialized["display_name"], "CLI workflow");

    let imported = run_json(&[
        "media",
        "add",
        project.to_str().expect("UTF-8 project path"),
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
        project.to_str().expect("UTF-8 project path"),
    ]);
    assert_eq!(listed.as_array().expect("asset array").len(), 1);
    assert_eq!(listed[0]["id"], asset_id);

    exercise_identifiers(project.to_str().expect("UTF-8 project path"), asset_id);
    exercise_metadata(project.to_str().expect("UTF-8 project path"), asset_id);

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
        project.to_str().expect("UTF-8 project path"),
        candidates.to_str().expect("UTF-8 root path"),
        "--label",
        "Relocated",
    ]);

    let ambiguous = run_json(&[
        "media",
        "resolve",
        project.to_str().expect("UTF-8 project path"),
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
        project.to_str().expect("UTF-8 project path"),
        asset_id,
        "--confirm",
        confirmed_uri,
    ]);
    assert_eq!(confirmed["confirmed_uri"], confirmed_uri);

    let resolved = run_json(&[
        "media",
        "resolve",
        project.to_str().expect("UTF-8 project path"),
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
        project.to_str().expect("UTF-8 project path"),
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
    let project = directory.path().join("provenance.pproj");
    let original = directory.path().join("original.mov");
    fs::write(&original, b"provenance source fixture").expect("write source fixture");

    run_json(&["init", project.to_str().expect("UTF-8 project path")]);
    let imported = run_json(&[
        "media",
        "add",
        project.to_str().expect("UTF-8 project path"),
        original.to_str().expect("UTF-8 media path"),
    ]);
    let representation_id = imported["representation_id"]
        .as_str()
        .expect("source representation ID");

    exercise_provenance(
        project.to_str().expect("UTF-8 project path"),
        representation_id,
        directory.path(),
    );
}
