//! Durable revision journal integration coverage.

use postproject_core::{
    Asset, AssetId, ContentStructure, ErrorKind, Locator, LocatorAvailability, LocatorId,
    OriginIdentity, OriginalMediaImport, Representation, RepresentationId, RepresentationKind,
    Resource, ResourceId, RevisionContext, RevisionEventKind, Timestamp,
};
use postproject_storage_sqlite::SqliteProject;
use tempfile::tempdir;

fn import(label: u8) -> OriginalMediaImport {
    let asset_id = AssetId::from_bytes([label; 16]);
    let representation_id = RepresentationId::from_bytes([label.wrapping_add(1); 16]);
    let resource_id = ResourceId::from_bytes([label.wrapping_add(2); 16]);
    let locator_id = LocatorId::from_bytes([label.wrapping_add(3); 16]);
    OriginalMediaImport::new(
        Asset::new(
            asset_id,
            Timestamp::from_unix_micros(i64::from(label)),
            Some(format!("Asset {label}")),
            None,
        ),
        Representation::new(
            representation_id,
            asset_id,
            RepresentationKind::Original,
            ContentStructure::single_resource(resource_id),
            Vec::new(),
        ),
        vec![Resource::new(resource_id, Vec::new(), None)],
        vec![
            Locator::new(
                locator_id,
                resource_id,
                format!("file:///media/{label}.mov"),
                None,
                LocatorAvailability::Online,
            )
            .expect("valid locator"),
        ],
    )
    .expect("valid import")
}

#[test]
fn imported_media_creates_a_durable_contextual_revision() {
    let directory = tempdir().expect("create temporary directory");
    let path = directory.path().join("production.pproj");
    let media = import(10);
    let asset_id = media.asset().id();
    let representation_id = media.representation().id();
    let resource_id = media.resources()[0].id();
    let locator_id = media.locators()[0].id();
    let mut project = SqliteProject::create(&path, None).expect("create project");
    assert_eq!(project.latest_revision().expect("query journal"), None);
    assert!(project.changes_since(0, 10).expect("query feed").is_empty());

    let transaction_id;
    {
        let mut transaction = project.begin_transaction().expect("begin transaction");
        transaction_id = transaction.id();
        transaction
            .set_revision_context(
                RevisionContext::new(
                    Some(
                        OriginIdentity::new(
                            "Editorial host",
                            Some("2.4.1".to_owned()),
                            Some("https://example.com/editor".to_owned()),
                        )
                        .expect("valid origin"),
                    ),
                    Some("Import camera original".to_owned()),
                )
                .expect("valid revision context"),
            )
            .expect("set revision context");
        transaction.import_original(&media).expect("stage import");
        transaction.commit().expect("commit import");
    }

    let revision = project
        .latest_revision()
        .expect("query latest revision")
        .expect("revision exists");
    assert_eq!(revision.sequence(), 1);
    assert_eq!(revision.transaction_id(), transaction_id);
    assert_eq!(revision.origin().expect("origin").name(), "Editorial host");
    assert_eq!(revision.message(), Some("Import camera original"));
    assert_eq!(
        project.changes_since(0, 1).expect("query page"),
        [revision.clone()]
    );

    let events = project
        .events_for_revision(revision.id())
        .expect("load revision events");
    assert_eq!(events.len(), 5);
    assert!(matches!(
        events[0].kind(),
        RevisionEventKind::AssetImported { asset_id: id } if *id == asset_id
    ));
    assert!(matches!(
        events[1].kind(),
        RevisionEventKind::RepresentationAdded { asset_id: owner, representation_id: id }
            if *owner == asset_id && *id == representation_id
    ));
    assert!(matches!(
        events[2].kind(),
        RevisionEventKind::ResourceAdded { resource_id: id } if *id == resource_id
    ));
    assert!(matches!(
        events[3].kind(),
        RevisionEventKind::RepresentationResourceAdded {
            representation_id: owner,
            resource_id: id,
            position: 0,
        } if *owner == representation_id && *id == resource_id
    ));
    assert!(matches!(
        events[4].kind(),
        RevisionEventKind::LocatorAdded { resource_id: owner, locator_id: id }
            if *owner == resource_id && *id == locator_id
    ));
    assert!(
        events
            .iter()
            .enumerate()
            .all(|(position, event)| event.revision_id() == revision.id()
                && event.position() == u32::try_from(position).expect("small position"))
    );

    drop(project);
    let reopened = SqliteProject::open(path).expect("reopen project");
    assert_eq!(
        reopened.latest_revision().expect("query reopened journal"),
        Some(revision)
    );
}

#[test]
fn revision_queries_reject_invalid_bounds_and_missing_ids() {
    let directory = tempdir().expect("create temporary directory");
    let project = SqliteProject::create(directory.path().join("production.pproj"), None)
        .expect("create project");

    assert_eq!(
        project
            .changes_since(0, 0)
            .expect_err("zero page size must fail")
            .kind(),
        ErrorKind::InvalidArgument
    );
    assert_eq!(
        project
            .changes_since(0, 1_001)
            .expect_err("excessive page size must fail")
            .kind(),
        ErrorKind::InvalidArgument
    );
    assert_eq!(
        project
            .events_for_revision(postproject_core::RevisionId::new())
            .expect_err("missing revision must fail")
            .kind(),
        ErrorKind::NotFound
    );
}
