//! Integration coverage for atomic media and media-root persistence.

use std::fs;

use postproject_core::{ErrorKind, TransactionState};
use postproject_media::{prepare_media_root, prepare_original_media};
use postproject_storage_sqlite::SqliteProject;
use tempfile::tempdir;

#[test]
fn imported_original_and_root_survive_reopen() {
    let directory = tempdir().expect("create temporary directory");
    let project_path = directory.path().join("production.pproj");
    let media_directory = directory.path().join("rushes");
    fs::create_dir(&media_directory).expect("create media directory");
    let media_path = media_directory.join("A001.mov");
    fs::write(&media_path, b"fixture media bytes").expect("write fixture media");

    let prepared = prepare_original_media(
        &media_path,
        Some("A001".to_owned()),
        Some("integration-test".to_owned()),
    )
    .expect("prepare import");
    let root = prepare_media_root(&media_directory, Some("Rushes".to_owned()), 10)
        .expect("prepare media root");
    let asset_id = prepared.asset().id();
    let representation_id = prepared.representation().id();
    let location_id = prepared.location().id();
    let fingerprint = prepared
        .representation()
        .fingerprint()
        .expect("prepared fingerprint")
        .clone();

    let mut project = SqliteProject::create(&project_path, None).expect("create project");
    {
        let mut transaction = project.begin_transaction().expect("begin transaction");
        transaction
            .import_original(&prepared)
            .expect("stage original import");
        transaction
            .add_media_root(root.clone())
            .expect("stage media root");
        transaction.commit().expect("commit transaction");
        assert_eq!(transaction.state(), TransactionState::Committed);
        assert_eq!(
            transaction
                .commit()
                .expect_err("second commit must fail")
                .kind(),
            ErrorKind::Conflict
        );
    }
    assert_eq!(project.project().media_roots(), std::slice::from_ref(&root));
    drop(project);

    let reopened = SqliteProject::open(&project_path).expect("reopen project");
    let assets = reopened.assets().expect("load assets");
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].id(), asset_id);
    assert_eq!(assets[0].display_name(), Some("A001"));
    let representations = reopened
        .representations(asset_id)
        .expect("load representations");
    assert_eq!(representations.len(), 1);
    assert_eq!(representations[0].id(), representation_id);
    assert_eq!(representations[0].fingerprint(), Some(&fingerprint));
    let locations = reopened
        .locations(representation_id)
        .expect("load locations");
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].id(), location_id);
    assert_eq!(locations[0].uri(), prepared.location().uri());
    assert_eq!(reopened.project().media_roots(), [root]);
}

#[test]
fn explicit_and_implicit_rollback_leave_no_partial_import() {
    let directory = tempdir().expect("create temporary directory");
    let project_path = directory.path().join("production.pproj");
    let media_path = directory.path().join("clip.mov");
    fs::write(&media_path, b"fixture media bytes").expect("write fixture media");
    let prepared =
        prepare_original_media(&media_path, None, None).expect("prepare original import");
    let mut project = SqliteProject::create(&project_path, None).expect("create project");

    {
        let mut transaction = project.begin_transaction().expect("begin transaction");
        transaction
            .import_original(&prepared)
            .expect("stage original import");
        transaction.rollback().expect("roll back transaction");
        assert_eq!(transaction.state(), TransactionState::RolledBack);
    }
    assert!(project.assets().expect("load assets").is_empty());

    {
        let mut transaction = project.begin_transaction().expect("begin transaction");
        transaction
            .import_original(&prepared)
            .expect("stage original import");
    }
    assert!(project.assets().expect("load assets").is_empty());
}

#[test]
fn duplicate_media_root_is_explicit_and_can_be_rolled_back() {
    let directory = tempdir().expect("create temporary directory");
    let project_path = directory.path().join("production.pproj");
    let root = prepare_media_root(directory.path(), None, 0).expect("prepare root");
    let duplicate = prepare_media_root(directory.path(), None, 1).expect("prepare duplicate root");
    let mut project = SqliteProject::create(&project_path, None).expect("create project");

    let mut transaction = project.begin_transaction().expect("begin transaction");
    transaction.add_media_root(root).expect("stage first root");
    let error = transaction
        .add_media_root(duplicate)
        .expect_err("duplicate URI must fail");
    assert_eq!(error.kind(), ErrorKind::AlreadyExists);
    transaction.rollback().expect("roll back transaction");
    drop(transaction);

    assert!(project.project().media_roots().is_empty());
}
