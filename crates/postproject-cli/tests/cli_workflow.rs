//! End-to-end tests of the public command-line workflow.

use std::fs;

use assert_cmd::cargo::cargo_bin_cmd;
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

fn add_representation_from_spec(
    production: &str,
    asset_id: &str,
    kind: &str,
    spec_path: &std::path::Path,
    spec: &Value,
) -> Value {
    fs::write(
        spec_path,
        serde_json::to_vec(&spec).expect("serialize representation spec"),
    )
    .expect("write representation spec");
    run_json(&[
        "representation",
        "add",
        production,
        asset_id,
        kind,
        spec_path.to_str().expect("UTF-8 spec path"),
    ])
}

fn assert_activity_snapshots(created: &Value) {
    let created_input = &created["inputs"][0];
    assert_eq!(created_input["role"], PRIMARY_INPUT_ROLE);
    assert_eq!(created_input["snapshot"]["revision_sequence"], 3);
    assert_eq!(
        created_input["snapshot"]["fingerprints"]
            .as_array()
            .expect("input fingerprint snapshots")
            .len(),
        1
    );
    assert_eq!(created["outputs"][0]["role"], PROXY_OUTPUT_ROLE);
    assert_eq!(created["outputs"][0]["snapshot"]["revision_sequence"], 3);
}

fn exercise_artifact_evaluation(
    production: &str,
    input_asset_id: &str,
    input_representation_id: &str,
    input_path: &std::path::Path,
    output_representation_id: &str,
) {
    let current = run_json(&["artifact", "evaluate", production, output_representation_id]);
    assert_eq!(current["state"], "current");
    assert_eq!(
        current["reasons"].as_array().expect("reason array").len(),
        0
    );
    let reproducibility = run_json(&[
        "artifact",
        "reproducibility",
        production,
        output_representation_id,
    ]);
    assert_eq!(reproducibility["reproducible"], true);
    assert_eq!(
        reproducibility["activity_kind"],
        "org.postproject:transcode"
    );
    assert_eq!(
        reproducibility["issues"]
            .as_array()
            .expect("reproducibility issue array")
            .len(),
        0
    );

    fs::write(input_path, b"changed provenance source fixture").expect("replace source fixture");
    let source = run_json(&["media", "show", production, input_asset_id]);
    let resource_id = source["representations"][0]["resources"][0]["id"]
        .as_str()
        .expect("source resource ID");
    run_json(&[
        "media",
        "fingerprint",
        production,
        input_asset_id,
        input_representation_id,
        resource_id,
        input_path.to_str().expect("UTF-8 source path"),
    ]);
    let stale = run_json(&["artifact", "evaluate", production, output_representation_id]);
    assert_eq!(stale["state"], "stale");
    assert_eq!(stale["reasons"][0]["kind"], "fingerprint_changed");
    assert_eq!(
        stale["reasons"][0]["representation_id"],
        input_representation_id
    );
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
    let value_file = format!("{production_path}.metadata.json");
    fs::write(
        &value_file,
        serde_json::to_vec(&serde_json::json!({
            "type": "struct",
            "fields": [
                {
                    "name": "name",
                    "value": {"type": "string", "value": "Camera department"}
                },
                {
                    "name": "ratio",
                    "value": {"type": "rational", "numerator": 24_000, "denominator": 1_001}
                },
                {
                    "name": "confidence",
                    "value": {"type": "decimal", "coefficient": "995", "scale": 3}
                },
                {
                    "name": "payload",
                    "value": {"type": "bytes", "hex": "00ff"}
                },
                {
                    "name": "subject",
                    "value": {
                        "type": "reference",
                        "target": {"target_kind": "asset", "target_id": asset_id}
                    }
                }
            ]
        }))
        .expect("serialize typed metadata input"),
    )
    .expect("write typed metadata input");
    let added = run_json(&[
        "metadata",
        "add",
        production_path,
        "asset",
        asset_id,
        "com.example.editor/metadata",
        "contact",
        &value_file,
    ]);
    assert_eq!(added["value"]["type"], "struct");
}

#[allow(clippy::too_many_lines)]
fn exercise_provenance(
    production: &str,
    input_representation_id: &str,
    directory: &std::path::Path,
) -> String {
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
    assert_activity_snapshots(&created);

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
    assert_eq!(
        listed[0]["inputs"][0]["snapshot"],
        created["inputs"][0]["snapshot"]
    );

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

    output_representation_id.to_owned()
}

#[test]
fn adds_every_representation_shape() {
    let directory = tempfile::tempdir().expect("create test directory");
    let production = directory.path().join("production.pproj");
    let original = directory.path().join("original.mov");
    let proxy = directory.path().join("proxy.mov");
    let second_part = directory.path().join("part2.mov");
    let frame = directory.path().join("frame0001.exr");
    let sidecar = directory.path().join("clip.xml");
    fs::write(&original, b"original").expect("write original");
    fs::write(&proxy, b"proxy").expect("write proxy");
    fs::write(&second_part, b"part two").expect("write second part");
    fs::write(&frame, b"frame").expect("write frame");
    fs::write(&sidecar, b"metadata").expect("write sidecar");
    let production = production.to_str().expect("UTF-8 production path");
    run_json(&["init", production]);
    let imported = run_json(&[
        "media",
        "add",
        production,
        original.to_str().expect("UTF-8 original path"),
    ]);
    let asset_id = imported["asset_id"].as_str().expect("asset ID");
    let original_representation_id = imported["representation_id"]
        .as_str()
        .expect("representation ID");
    assert!(run_json(&["dependency", "show", production, original_representation_id,]).is_null());
    assert_eq!(
        run_json(&["dependency", "dependents", production, "asset", asset_id]),
        serde_json::json!([])
    );

    add_representation_from_spec(
        production,
        asset_id,
        "proxy",
        &directory.path().join("single.json"),
        &serde_json::json!({"structure": "single_file", "path": proxy}),
    );
    add_representation_from_spec(
        production,
        asset_id,
        "derived",
        &directory.path().join("sequence.json"),
        &serde_json::json!({
            "structure": "image_sequence",
            "directory": directory.path(),
            "prefix": "frame",
            "suffix": ".exr",
            "padding": 4,
            "start": 1,
            "end": 1,
            "step": 1,
            "rate_numerator": 24_000,
            "rate_denominator": 1_001
        }),
    );
    add_representation_from_spec(
        production,
        asset_id,
        "optimized",
        &directory.path().join("ordered.json"),
        &serde_json::json!({
            "structure": "ordered_parts",
            "members": [
                {"path": original, "role": "org.postproject:essence.first"},
                {"path": second_part, "role": "org.postproject:essence.second"}
            ]
        }),
    );
    add_representation_from_spec(
        production,
        asset_id,
        "derived",
        &directory.path().join("package.json"),
        &serde_json::json!({
            "structure": "package",
            "members": [
                {"path": proxy, "role": "org.postproject:essence"},
                {"path": sidecar, "role": "org.postproject:sidecar", "required": false}
            ]
        }),
    );

    let shown = run_json(&["media", "show", production, asset_id]);
    let representations = shown["representations"]
        .as_array()
        .expect("representation array");
    assert_eq!(representations.len(), 5);
    let structures = representations
        .iter()
        .map(|representation| representation["structure"].as_str().expect("structure"))
        .collect::<Vec<_>>();
    assert!(structures.contains(&"single_resource"));
    assert!(structures.contains(&"image_sequence"));
    assert!(structures.contains(&"ordered_parts"));
    assert!(structures.contains(&"package"));
}

#[test]
fn manages_media_root_lifecycle() {
    let directory = tempfile::tempdir().expect("create test directory");
    let production = directory.path().join("roots.pproj");
    let production = production.to_str().expect("UTF-8 production path");
    run_json(&["init", production]);

    let added = run_json(&[
        "root",
        "add",
        production,
        "rushes",
        "--label",
        "Rushes",
        "--priority",
        "7",
    ]);
    let root_id = added["id"].as_str().expect("media-root ID");

    let listed = run_json(&["root", "list", production]);
    assert_eq!(listed.as_array().expect("root array").len(), 1);
    assert_eq!(listed[0]["id"], root_id);
    assert_eq!(listed[0]["name"], "rushes");
    assert_eq!(listed[0]["label"], "Rushes");
    assert_eq!(listed[0]["priority"], 7);
    assert_eq!(listed[0]["enabled"], true);

    let disabled = run_json(&["root", "disable", production, root_id]);
    assert_eq!(disabled["id"], root_id);
    assert_eq!(disabled["enabled"], false);
    assert_eq!(run_json(&["root", "list", production])[0]["enabled"], false);

    let enabled = run_json(&["root", "enable", production, root_id]);
    assert_eq!(enabled["id"], root_id);
    assert_eq!(enabled["enabled"], true);

    let removed = run_json(&["root", "remove", production, root_id]);
    assert_eq!(removed["id"], root_id);
    assert!(
        run_json(&["root", "list", production])
            .as_array()
            .expect("root array")
            .is_empty()
    );
}

#[test]
fn retires_resource_locator() {
    let directory = tempfile::tempdir().expect("create test directory");
    let production = directory.path().join("locators.pproj");
    let media = directory.path().join("original.mov");
    fs::write(&media, b"locator lifecycle fixture").expect("write media fixture");
    let production = production.to_str().expect("UTF-8 production path");
    run_json(&["init", production]);
    let imported = run_json(&[
        "media",
        "add",
        production,
        media.to_str().expect("UTF-8 media path"),
    ]);
    let locator_id = imported["locator_id"].as_str().expect("locator ID");
    let asset_id = imported["asset_id"].as_str().expect("asset ID");

    let retired = run_json(&["locator", "retire", production, locator_id]);
    assert_eq!(retired["id"], locator_id);

    let shown = run_json(&["media", "show", production, asset_id]);
    assert!(
        shown["representations"][0]["resources"][0]["locators"]
            .as_array()
            .expect("locator array")
            .is_empty()
    );
}

fn confirm_relocated_candidate(production: &str, asset_id: &str, candidates: &std::path::Path) {
    run_json(&[
        "root",
        "add",
        production,
        "relocated",
        "--label",
        "Relocated",
    ]);
    let root_mapping = format!(
        "relocated={}",
        candidates.to_str().expect("UTF-8 root path")
    );
    let ambiguous = run_json(&[
        "media",
        "resolve",
        production,
        asset_id,
        "--root-map",
        &root_mapping,
    ]);
    assert_eq!(ambiguous["resolutions"][0]["availability"], "ambiguous");
    let resource = &ambiguous["resolutions"][0]["resources"][0];
    assert_eq!(resource["state"], "ambiguous");
    let confirmed_uri = resource["candidates"][0]["uri"]
        .as_str()
        .expect("candidate URI");
    let confirmed = run_json(&[
        "media",
        "resolve",
        production,
        asset_id,
        "--root-map",
        &root_mapping,
        "--confirm",
        confirmed_uri,
    ]);
    assert_eq!(confirmed["confirmed_uri"], confirmed_uri);
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

    confirm_relocated_candidate(
        production.to_str().expect("UTF-8 production path"),
        asset_id,
        &candidates,
    );

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
    let asset_id = imported["asset_id"].as_str().expect("source asset ID");

    let output_representation_id = exercise_provenance(
        production.to_str().expect("UTF-8 production path"),
        representation_id,
        directory.path(),
    );
    exercise_artifact_evaluation(
        production.to_str().expect("UTF-8 production path"),
        asset_id,
        representation_id,
        &original,
        &output_representation_id,
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
