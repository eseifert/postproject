//! Integration coverage for durable project-file lifecycle behavior.

use postproject_core::ErrorKind;
use postproject_storage_sqlite::{CURRENT_SCHEMA_VERSION, SqliteProject};
use tempfile::tempdir;

#[test]
fn project_identity_and_metadata_survive_reopen() {
    let directory = tempdir().expect("create temporary directory");
    let path = directory.path().join("production.pproj");
    let created =
        SqliteProject::create(&path, Some("Documentary".to_owned())).expect("create project");
    let id = created.project().id();
    let created_at = created.project().created_at();
    assert!(created.foreign_keys_enabled().expect("query foreign keys"));
    drop(created);

    let reopened = SqliteProject::open(&path).expect("reopen project");
    assert_eq!(reopened.path(), path);
    assert_eq!(reopened.project().id(), id);
    assert_eq!(reopened.project().created_at(), created_at);
    assert_eq!(reopened.project().display_name(), Some("Documentary"));
    assert_eq!(reopened.project().schema_version(), CURRENT_SCHEMA_VERSION);
    assert!(reopened.foreign_keys_enabled().expect("query foreign keys"));
}

#[test]
fn create_never_overwrites_an_existing_file() {
    let directory = tempdir().expect("create temporary directory");
    let path = directory.path().join("production.pproj");
    let first = SqliteProject::create(&path, None).expect("create first project");
    let expected_id = first.project().id();
    drop(first);

    let error = SqliteProject::create(&path, None).expect_err("second create must fail");
    assert_eq!(error.kind(), ErrorKind::AlreadyExists);
    assert_eq!(
        SqliteProject::open(&path)
            .expect("original project remains valid")
            .project()
            .id(),
        expected_id
    );
}

#[test]
fn opening_a_missing_project_is_not_found() {
    let directory = tempdir().expect("create temporary directory");
    let error = SqliteProject::open(directory.path().join("missing.pproj"))
        .expect_err("missing project must fail");
    assert_eq!(error.kind(), ErrorKind::NotFound);
}
