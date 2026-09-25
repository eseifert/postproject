//! Durable job request integration coverage.

use postproject_core::{
    Asset, AssetId, ContentStructure, ErrorKind, Job, JobId, JobKind, JobState, Locator,
    LocatorAvailability, LocatorId, MediaRoot, MediaRootId, MetadataProperty, MetadataValue,
    ObjectRef, OriginalMediaImport, PropertyId, Representation, RepresentationId,
    RepresentationKind, RequestedJobOutput, Resource, ResourceId, RevisionEventKind, Timestamp,
    VocabularyId,
};
use postproject_storage_sqlite::SqliteProduction;
use tempfile::tempdir;

fn source_import() -> OriginalMediaImport {
    let asset_id = AssetId::from_bytes([1; 16]);
    let representation_id = RepresentationId::from_bytes([2; 16]);
    let resource_id = ResourceId::from_bytes([3; 16]);
    OriginalMediaImport::new(
        Asset::new(
            asset_id,
            Timestamp::from_unix_micros(1),
            Some("Source".to_owned()),
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
                LocatorId::from_bytes([4; 16]),
                resource_id,
                "file:///media/source.mov",
                None,
                LocatorAvailability::Online,
            )
            .expect("valid locator"),
        ],
    )
    .expect("valid source import")
}

fn requested_job(source: &OriginalMediaImport) -> Job {
    Job::new(
        JobId::from_bytes([5; 16]),
        JobKind::new("org.postproject:generate-proxy").expect("valid job kind"),
        vec![source.representation().id()],
        RequestedJobOutput::new(
            source.asset().id(),
            RepresentationKind::Proxy,
            Some("proxies".to_owned()),
        )
        .expect("valid requested output"),
    )
    .expect("valid job")
}

#[test]
fn requested_job_and_metadata_round_trip_and_are_journaled() {
    let directory = tempdir().expect("create temporary directory");
    let path = directory.path().join("production.pproj");
    let source = source_import();
    let job = requested_job(&source);
    let property = MetadataProperty::new(
        VocabularyId::new("org.postproject.job").expect("valid vocabulary"),
        PropertyId::new("codec").expect("valid property"),
    );
    let value = MetadataValue::string("prores").expect("valid value");
    let mut production = SqliteProduction::create(&path, None).expect("create production");
    {
        let mut transaction = production.begin_transaction().expect("begin setup");
        transaction.import_original(&source).expect("import source");
        transaction
            .add_media_root(
                MediaRoot::new(
                    MediaRootId::from_bytes([6; 16]),
                    "proxies",
                    None,
                    None,
                    0,
                    true,
                )
                .expect("valid root"),
            )
            .expect("add root");
        transaction.commit().expect("commit setup");
    }
    {
        let mut transaction = production.begin_transaction().expect("begin request");
        transaction.request_job(&job).expect("request job");
        transaction
            .add_metadata_value(ObjectRef::Job(job.id()), &property, &value)
            .expect("add job parameter");
        transaction.commit().expect("commit request");
    }

    drop(production);
    let production = SqliteProduction::open(path).expect("reopen production");
    assert_eq!(production.job(job.id()).expect("load job"), job);
    assert_eq!(
        production.jobs().expect("list jobs").as_slice(),
        std::slice::from_ref(&job)
    );
    assert_eq!(
        production
            .metadata_values(ObjectRef::Job(job.id()), &property)
            .expect("load parameters"),
        [value]
    );
    let revision = production
        .latest_revision()
        .expect("load revision")
        .unwrap();
    let events = production
        .events_for_revision(revision.id())
        .expect("load events");
    assert!(matches!(
        events[0].kind(),
        RevisionEventKind::JobRequested { job_id } if *job_id == job.id()
    ));
    assert!(matches!(
        events[1].kind(),
        RevisionEventKind::MetadataAddedOrReplaced {
            target: ObjectRef::Job(job_id),
            property: event_property,
        } if *job_id == job.id() && event_property == &property
    ));
}

#[test]
fn invalid_job_references_leave_no_partial_request() {
    let directory = tempdir().expect("create temporary directory");
    let mut production = SqliteProduction::create(directory.path().join("production.pproj"), None)
        .expect("create production");
    let source = source_import();
    {
        let mut transaction = production.begin_transaction().expect("begin setup");
        transaction.import_original(&source).expect("import source");
        transaction.commit().expect("commit setup");
    }
    let missing_root = requested_job(&source);
    let error = production
        .begin_transaction()
        .expect("begin request")
        .request_job(&missing_root)
        .expect_err("missing root must fail");
    assert_eq!(error.kind(), ErrorKind::NotFound);

    let missing_input = Job::new(
        JobId::from_bytes([7; 16]),
        JobKind::new("org.postproject:generate-proxy").expect("valid kind"),
        vec![RepresentationId::from_bytes([8; 16])],
        RequestedJobOutput::new(source.asset().id(), RepresentationKind::Proxy, None)
            .expect("valid output"),
    )
    .expect("valid job");
    let mut transaction = production.begin_transaction().expect("begin request");
    let error = transaction
        .request_job(&missing_input)
        .expect_err("missing input must fail");
    assert_eq!(error.kind(), ErrorKind::NotFound);
    transaction.commit().expect("commit empty transaction");
    drop(transaction);
    assert!(production.jobs().expect("list jobs").is_empty());
    assert!(matches!(missing_root.state(), JobState::Requested));
}
