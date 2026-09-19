//! Integration coverage for external-identifier persistence and lookup.

use std::fs;

use postproject_core::{
    AssetId, ErrorKind, ExternalIdentifier, IdentifierScheme, ObjectRef, ProjectId,
};
use postproject_media::prepare_original_media;
use postproject_storage_sqlite::SqliteProject;
use tempfile::tempdir;

fn identifier(
    scheme: &IdentifierScheme,
    value: &str,
    qualifier: Option<&str>,
) -> ExternalIdentifier {
    ExternalIdentifier::new(scheme.clone(), value, qualifier.map(ToOwned::to_owned))
        .expect("create external identifier")
}

#[test]
fn multiple_external_identifiers_round_trip_and_support_exact_lookup() {
    let directory = tempdir().expect("create temporary directory");
    let project_path = directory.path().join("production.pproj");
    let media_path = directory.path().join("A001.mov");
    fs::write(&media_path, b"fixture media bytes").expect("write fixture media");
    let prepared = prepare_original_media(&media_path, None, None).expect("prepare import");
    let asset = ObjectRef::Asset(prepared.asset().id());
    let representation = ObjectRef::Representation(prepared.representation().id());
    let resource = ObjectRef::Resource(prepared.resources()[0].id());
    let umid_scheme = IdentifierScheme::new("urn:smpte:umid").expect("valid scheme");
    let vendor_scheme = IdentifierScheme::new("com.example.camera.serial").expect("valid scheme");
    let material_umid = identifier(
        &umid_scheme,
        "060A2B340101010501010D4313000000A1B2C3D4E5F60718293A4B5C6D7E8F90",
        Some("material"),
    );
    let instance_umid = identifier(
        &umid_scheme,
        "060A2B340101010501010D4313000000112233445566778899AABBCCDDEEFF00",
        Some("instance"),
    );
    let vendor_id = identifier(&vendor_scheme, "  Camera A / 0007  ", None);
    let storage_id = identifier(&vendor_scheme, "storage-object-7", Some("resource"));

    let mut project = SqliteProject::create(&project_path, None).expect("create project");
    {
        let mut transaction = project.begin_transaction().expect("begin transaction");
        transaction
            .import_original(&prepared)
            .expect("stage original import");
        transaction
            .add_external_identifier(asset, &material_umid)
            .expect("attach material UMID");
        transaction
            .add_external_identifier(asset, &instance_umid)
            .expect("attach instance UMID");
        transaction
            .add_external_identifier(representation, &vendor_id)
            .expect("attach vendor ID");
        transaction
            .add_external_identifier(resource, &storage_id)
            .expect("attach storage identifier");
        transaction.commit().expect("commit transaction");
    }
    drop(project);

    let reopened = SqliteProject::open(&project_path).expect("reopen project");
    let asset_identifiers = reopened
        .external_identifiers(asset)
        .expect("load asset identifiers");
    assert_eq!(asset_identifiers.len(), 2);
    assert!(asset_identifiers.contains(&material_umid));
    assert!(asset_identifiers.contains(&instance_umid));
    assert_eq!(
        reopened
            .external_identifiers(representation)
            .expect("load representation identifiers"),
        std::slice::from_ref(&vendor_id)
    );
    assert_eq!(
        reopened
            .external_identifiers(resource)
            .expect("load resource identifiers"),
        std::slice::from_ref(&storage_id)
    );
    assert_eq!(
        reopened
            .find_by_external_identifier(&vendor_scheme, vendor_id.value())
            .expect("look up exact vendor ID"),
        [representation]
    );
    assert!(
        reopened
            .find_by_external_identifier(&vendor_scheme, "Camera A / 0007")
            .expect("look up normalized-looking value")
            .is_empty(),
        "lookup must not trim or normalize opaque values"
    );
}

#[test]
fn identifier_mutations_are_atomic_and_validate_targets() {
    let directory = tempdir().expect("create temporary directory");
    let project_path = directory.path().join("production.pproj");
    let media_path = directory.path().join("clip.mov");
    fs::write(&media_path, b"fixture media bytes").expect("write fixture media");
    let prepared = prepare_original_media(&media_path, None, None).expect("prepare import");
    let target = ObjectRef::Asset(prepared.asset().id());
    let scheme = IdentifierScheme::new("com.example.asset").expect("valid scheme");
    let external_id = identifier(&scheme, "asset-42", None);
    let mut project = SqliteProject::create(&project_path, None).expect("create project");

    {
        let mut transaction = project.begin_transaction().expect("begin transaction");
        transaction
            .import_original(&prepared)
            .expect("stage original import");
        transaction.commit().expect("commit import");
    }

    {
        let mut transaction = project.begin_transaction().expect("begin transaction");
        transaction
            .add_external_identifier(target, &external_id)
            .expect("stage identifier");
        transaction.rollback().expect("roll back identifier");
    }
    assert!(
        project
            .external_identifiers(target)
            .expect("load identifiers")
            .is_empty()
    );

    {
        let mut transaction = project.begin_transaction().expect("begin transaction");
        transaction
            .add_external_identifier(target, &external_id)
            .expect("stage identifier");
        assert_eq!(
            transaction
                .add_external_identifier(target, &external_id)
                .expect_err("duplicate attachment must fail")
                .kind(),
            ErrorKind::AlreadyExists
        );
        transaction.commit().expect("commit identifier");
    }

    {
        let mut transaction = project.begin_transaction().expect("begin transaction");
        transaction
            .remove_external_identifier(target, &external_id)
            .expect("stage removal");
        transaction.rollback().expect("roll back removal");
    }
    assert_eq!(
        project
            .external_identifiers(target)
            .expect("load identifiers"),
        std::slice::from_ref(&external_id)
    );

    let mut transaction = project.begin_transaction().expect("begin transaction");
    assert_eq!(
        transaction
            .add_external_identifier(ObjectRef::Asset(AssetId::new()), &external_id)
            .expect_err("missing target must fail")
            .kind(),
        ErrorKind::NotFound
    );
    assert_eq!(
        transaction
            .add_external_identifier(ObjectRef::Project(ProjectId::new()), &external_id)
            .expect_err("unsupported target must fail")
            .kind(),
        ErrorKind::Unsupported
    );
    transaction.rollback().expect("roll back transaction");
}
