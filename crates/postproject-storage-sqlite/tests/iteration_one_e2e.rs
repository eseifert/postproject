//! Complete iteration-one relocation, ambiguity, and persistence scenario.

use std::{collections::BTreeMap, fs};

use postproject_core::{AssetId, RepresentationId, ResolutionState};
use postproject_media::{
    MediaResolver, prepare_confirmed_location, prepare_media_root, prepare_original_media,
};
use postproject_storage_sqlite::SqliteProject;

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "keeping the ordered acceptance scenario in one test makes its state transitions auditable"
)]
fn relocation_workflow_handles_unique_and_ambiguous_media() {
    let directory = tempfile::tempdir().expect("create test directory");
    let project_path = directory.path().join("production.pproj");
    let original_root = directory.path().join("original");
    fs::create_dir(&original_root).expect("create original root");
    let fixtures = [
        ("camera-a.mov", b"camera A unique media".as_slice()),
        ("camera-b.mov", b"camera B unique media".as_slice()),
        ("ambiguous.wav", b"ambiguous production audio".as_slice()),
    ];
    for (name, bytes) in fixtures {
        fs::write(original_root.join(name), bytes).expect("write media fixture");
    }

    let mut project = SqliteProject::create(&project_path, Some("E2E production".to_owned()))
        .expect("create project");
    let prepared: Vec<_> = fixtures
        .iter()
        .map(|(name, _)| {
            prepare_original_media(original_root.join(name), Some((*name).to_owned()), None)
                .expect("prepare media import")
        })
        .collect();
    let identities: BTreeMap<AssetId, RepresentationId> = prepared
        .iter()
        .map(|import| (import.asset().id(), import.representation().id()))
        .collect();
    let mut transaction = project
        .begin_transaction()
        .expect("begin import transaction");
    for import in &prepared {
        transaction
            .import_original(import)
            .expect("stage media import");
    }
    transaction.commit().expect("commit media imports");
    drop(transaction);
    drop(project);

    let relocated_parent = directory.path().join("relocated");
    let relocated_media = relocated_parent.join("media");
    fs::create_dir(&relocated_parent).expect("create relocated parent");
    fs::rename(&original_root, &relocated_media).expect("move entire media directory");
    let duplicate_directory = relocated_parent.join("duplicate");
    fs::create_dir(&duplicate_directory).expect("create duplicate directory");
    fs::copy(
        relocated_media.join("ambiguous.wav"),
        duplicate_directory.join("ambiguous-copy.wav"),
    )
    .expect("create ambiguous candidate");

    let mut project = SqliteProject::open(&project_path).expect("reopen moved project");
    let resolver = MediaResolver::default();
    for representation_id in identities.values() {
        let representation = project
            .representations(
                prepared
                    .iter()
                    .find(|import| import.representation().id() == *representation_id)
                    .expect("known representation")
                    .asset()
                    .id(),
            )
            .expect("load representation")
            .into_iter()
            .find(|representation| representation.id() == *representation_id)
            .expect("representation exists");
        let locations = project
            .locations(*representation_id)
            .expect("load known locations");
        let resolution = resolver
            .resolve(&representation, &locations, &[])
            .expect("resolve without roots");
        assert_eq!(resolution.state(), ResolutionState::Missing);
    }

    let root = prepare_media_root(&relocated_parent, Some("relocated".to_owned()), 0)
        .expect("prepare relocated root");
    let mut transaction = project.begin_transaction().expect("begin root transaction");
    transaction.add_media_root(root).expect("stage media root");
    transaction.commit().expect("commit media root");
    drop(transaction);

    let mut confirmations = Vec::new();
    let mut ambiguous_count = 0;
    for asset in project.assets().expect("load assets") {
        for representation in project
            .representations(asset.id())
            .expect("load representations")
        {
            let locations = project
                .locations(representation.id())
                .expect("load locations");
            let resolution = resolver
                .resolve(&representation, &locations, project.project().media_roots())
                .expect("resolve relocated media");
            match resolution.state() {
                ResolutionState::ResolvedExact => {
                    assert_eq!(resolution.candidates().len(), 1);
                }
                ResolutionState::Ambiguous => {
                    ambiguous_count += 1;
                    assert_eq!(resolution.candidates().len(), 2);
                }
                state => panic!("unexpected relocation state: {state:?}"),
            }
            let selected = resolution
                .candidates()
                .first()
                .expect("resolved candidate exists");
            confirmations.push(
                prepare_confirmed_location(representation.id(), selected.uri())
                    .expect("prepare confirmed location"),
            );
        }
    }
    assert_eq!(ambiguous_count, 1);

    let mut transaction = project
        .begin_transaction()
        .expect("begin confirmation transaction");
    for location in &confirmations {
        transaction
            .add_location(location)
            .expect("stage confirmed location");
    }
    transaction.commit().expect("commit confirmed locations");
    drop(transaction);
    drop(project);

    let project = SqliteProject::open(&project_path).expect("reopen confirmed project");
    for (asset_id, representation_id) in identities {
        let representation = project
            .representations(asset_id)
            .expect("load persisted representation")
            .into_iter()
            .find(|representation| representation.id() == representation_id)
            .expect("persisted representation exists");
        let locations = project
            .locations(representation_id)
            .expect("load persisted locations");
        assert_eq!(locations.len(), 2);
        let resolution = resolver
            .resolve(&representation, &locations, &[])
            .expect("resolve from confirmed location without roots");
        assert_eq!(resolution.state(), ResolutionState::OnlineAtKnownLocation);
    }
}
