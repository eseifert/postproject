//! Integration coverage for durable production-file lifecycle behavior.

use postproject_core::ErrorKind;
use postproject_storage_sqlite::{CURRENT_SCHEMA_VERSION, SqliteProduction};
use tempfile::tempdir;

#[test]
fn production_identity_and_metadata_survive_reopen() {
    let directory = tempdir().expect("create temporary directory");
    let path = directory.path().join("production.pproj");
    let created =
        SqliteProduction::create(&path, Some("Documentary".to_owned())).expect("create production");
    let id = created.production().id();
    let created_at = created.production().created_at();
    assert!(created.foreign_keys_enabled().expect("query foreign keys"));
    drop(created);

    let reopened = SqliteProduction::open(&path).expect("reopen production");
    assert_eq!(reopened.path(), path);
    assert_eq!(reopened.production().id(), id);
    assert_eq!(reopened.production().created_at(), created_at);
    assert_eq!(reopened.production().display_name(), Some("Documentary"));
    assert_eq!(
        reopened.production().schema_version(),
        CURRENT_SCHEMA_VERSION
    );
    assert!(reopened.foreign_keys_enabled().expect("query foreign keys"));
}

#[test]
fn create_never_overwrites_an_existing_file() {
    let directory = tempdir().expect("create temporary directory");
    let path = directory.path().join("production.pproj");
    let first = SqliteProduction::create(&path, None).expect("create first production");
    let expected_id = first.production().id();
    drop(first);

    let error = SqliteProduction::create(&path, None).expect_err("second create must fail");
    assert_eq!(error.kind(), ErrorKind::AlreadyExists);
    assert_eq!(
        SqliteProduction::open(&path)
            .expect("original production remains valid")
            .production()
            .id(),
        expected_id
    );
}

#[test]
fn opening_a_missing_production_is_not_found() {
    let directory = tempdir().expect("create temporary directory");
    let error = SqliteProduction::open(directory.path().join("missing.pproj"))
        .expect_err("missing production must fail");
    assert_eq!(error.kind(), ErrorKind::NotFound);
}
