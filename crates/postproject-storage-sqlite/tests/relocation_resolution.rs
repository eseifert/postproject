//! End-to-end relocation, ambiguity, confirmation, and reopen scenario.

use std::fs;

use postproject_core::ResolutionState;
use postproject_media::{
    MediaResolver, prepare_confirmed_location, prepare_media_root, prepare_original_media,
};
use postproject_storage_sqlite::SqliteProject;
use tempfile::tempdir;

#[test]
fn moved_media_resolves_and_confirmed_location_persists() {
    let directory = tempdir().expect("create temporary directory");
    let project_path = directory.path().join("production.pproj");
    let original_directory = directory.path().join("original");
    let relocated_directory = directory.path().join("relocated");
    fs::create_dir(&original_directory).expect("create original directory");
    let original_path = original_directory.join("A001.mov");
    fs::write(&original_path, b"unique fixture media").expect("write media");
    let prepared = prepare_original_media(&original_path, Some("A001".to_owned()), None)
        .expect("prepare import");
    let asset_id = prepared.asset().id();

    let mut project = SqliteProject::create(&project_path, None).expect("create project");
    let mut transaction = project.begin_transaction().expect("begin import");
    transaction
        .import_original(&prepared)
        .expect("stage import");
    transaction.commit().expect("commit import");
    drop(transaction);
    drop(project);

    fs::rename(&original_directory, &relocated_directory).expect("move media directory");
    let relocated_path = relocated_directory.join("A001.mov");
    let resolver = MediaResolver::default();
    let mut project = SqliteProject::open(&project_path).expect("reopen moved project");
    let representation = project
        .representations(asset_id)
        .expect("load representation")
        .remove(0);
    let known_locations = project
        .locations(representation.id())
        .expect("load known locations");
    let missing = resolver
        .resolve(&representation, &known_locations, &[])
        .expect("resolve without roots");
    assert_eq!(missing.state(), ResolutionState::Missing);

    let root = prepare_media_root(&relocated_directory, None, 0).expect("prepare new root");
    let mut transaction = project.begin_transaction().expect("begin root transaction");
    transaction
        .add_media_root(root)
        .expect("stage new media root");
    transaction.commit().expect("commit root");
    drop(transaction);

    let unique = resolver
        .resolve(
            &representation,
            &known_locations,
            project.project().media_roots(),
        )
        .expect("resolve unique media");
    assert_eq!(unique.state(), ResolutionState::ResolvedExact);

    let duplicate_path = relocated_directory.join("duplicate.mov");
    fs::copy(&relocated_path, &duplicate_path).expect("create duplicate");
    let ambiguous = resolver
        .resolve(
            &representation,
            &known_locations,
            project.project().media_roots(),
        )
        .expect("resolve duplicate media");
    assert_eq!(ambiguous.state(), ResolutionState::Ambiguous);
    assert_eq!(ambiguous.candidates().len(), 2);

    let chosen_uri = ambiguous
        .candidates()
        .iter()
        .find(|candidate| candidate.uri().ends_with("A001.mov"))
        .expect("find intended candidate")
        .uri()
        .to_owned();
    let confirmed = prepare_confirmed_location(representation.id(), chosen_uri)
        .expect("prepare confirmed location");
    let mut transaction = project
        .begin_transaction()
        .expect("begin confirmation transaction");
    transaction
        .add_location(&confirmed)
        .expect("stage confirmed location");
    transaction.commit().expect("commit confirmation");
    drop(transaction);
    drop(project);

    let reopened = SqliteProject::open(&project_path).expect("reopen confirmed project");
    let locations = reopened
        .locations(representation.id())
        .expect("load confirmed locations");
    assert_eq!(locations.len(), 2);
    let online = resolver
        .resolve(
            &representation,
            &locations,
            reopened.project().media_roots(),
        )
        .expect("resolve confirmed location");
    assert_eq!(online.state(), ResolutionState::OnlineAtKnownLocation);
    assert_eq!(online.candidates()[0].uri(), confirmed.uri());
}
