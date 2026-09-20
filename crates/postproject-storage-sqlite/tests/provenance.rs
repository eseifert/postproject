//! Activity persistence, atomicity, and cycle-invariant integration tests.

use postproject_core::{
    Activity, ActivityId, ActivityInput, ActivityKind, ActivityOutput, ActivityRole, AgentIdentity,
    Asset, AssetId, ContentStructure, ErrorKind, ExternalIdentifier, IdentifierScheme, Locator,
    LocatorAvailability, LocatorId, MetadataProperty, MetadataValue, ObjectRef,
    OriginalMediaImport, PropertyId, Representation, RepresentationId, RepresentationKind,
    Resource, ResourceId, Timestamp, ToolIdentity, VocabularyId,
};
use postproject_storage_sqlite::SqliteProject;
use tempfile::tempdir;

fn import(label: u8) -> (OriginalMediaImport, RepresentationId) {
    let asset_id = AssetId::from_bytes([label; 16]);
    let representation_id = RepresentationId::from_bytes([label; 16]);
    let resource_id = ResourceId::from_bytes([label; 16]);
    let asset = Asset::new(
        asset_id,
        Timestamp::from_unix_micros(i64::from(label)),
        None,
        None,
    );
    let representation = Representation::new(
        representation_id,
        asset_id,
        RepresentationKind::Original,
        ContentStructure::single_resource(resource_id),
        Vec::new(),
    );
    let resource = Resource::new(resource_id, Vec::new(), None);
    let locator = Locator::new(
        LocatorId::from_bytes([label; 16]),
        resource_id,
        format!("file:///media/{label}.mov"),
        None,
        LocatorAvailability::Online,
    )
    .expect("valid locator");
    (
        OriginalMediaImport::new(asset, representation, vec![resource], vec![locator])
            .expect("valid import"),
        representation_id,
    )
}

fn activity(id: ActivityId, input: RepresentationId, output: RepresentationId) -> Activity {
    Activity::new(
        id,
        ActivityKind::new("postproject:transcode").expect("valid kind"),
        vec![ActivityInput::new(input, None)],
        vec![ActivityOutput::new(output, None)],
    )
    .expect("valid activity")
}

#[test]
fn activity_metadata_is_atomic_with_activity_creation() {
    let directory = tempdir().expect("create temporary directory");
    let path = directory.path().join("production.pproj");
    let mut project = SqliteProject::create(&path, None).expect("create project");
    let (source, source_id) = import(1);
    let (proxy, proxy_id) = import(2);
    let activity_id = ActivityId::new();
    let agent_identifier = ExternalIdentifier::new(
        IdentifierScheme::new("com.example.worker").expect("valid scheme"),
        "worker-7",
        None,
    )
    .expect("valid identifier");
    let activity = Activity::new(
        activity_id,
        ActivityKind::new("postproject:transcode").expect("valid kind"),
        vec![ActivityInput::new(
            source_id,
            Some(ActivityRole::new("postproject:input.primary").expect("valid role")),
        )],
        vec![ActivityOutput::new(
            proxy_id,
            Some(ActivityRole::new("postproject:output.proxy").expect("valid role")),
        )],
    )
    .expect("valid activity")
    .with_timing(
        Some(Timestamp::from_unix_micros(10)),
        Some(Timestamp::from_unix_micros(20)),
    )
    .expect("valid timing")
    .with_tool(
        ToolIdentity::new(
            "FFmpeg",
            Some("8.0".to_owned()),
            Some("https://ffmpeg.org".to_owned()),
        )
        .expect("valid tool"),
    )
    .with_agent(
        AgentIdentity::new(Some("Render worker".to_owned()), Some(agent_identifier))
            .expect("valid agent"),
    );
    let property = MetadataProperty::new(
        VocabularyId::new("com.example.provenance").expect("valid vocabulary"),
        PropertyId::new("preset").expect("valid property"),
    );
    let value = MetadataValue::string("editorial-proxy").expect("valid value");
    {
        let mut transaction = project.begin_transaction().expect("begin transaction");
        transaction.import_original(&source).expect("import source");
        transaction.import_original(&proxy).expect("import proxy");
        transaction
            .create_activity(&activity)
            .expect("create activity");
        transaction
            .add_metadata_value(ObjectRef::Activity(activity_id), &property, &value)
            .expect("attach activity metadata");
        transaction.commit().expect("commit provenance");
    }

    let reopened = SqliteProject::open(&path).expect("reopen project");
    assert_eq!(reopened.activities().expect("load activities"), [activity]);
    assert_eq!(
        reopened
            .metadata_values(ObjectRef::Activity(activity_id), &property)
            .expect("load activity metadata"),
        [value]
    );
}

#[test]
fn invalid_activities_leave_no_partial_rows() {
    let directory = tempdir().expect("create temporary directory");
    let path = directory.path().join("production.pproj");
    let mut project = SqliteProject::create(path, None).expect("create project");
    let (source, source_id) = import(3);
    let (proxy, proxy_id) = import(4);
    let (delivery, delivery_id) = import(5);
    {
        let mut transaction = project.begin_transaction().expect("begin transaction");
        transaction.import_original(&source).expect("import source");
        transaction.import_original(&proxy).expect("import proxy");
        transaction
            .import_original(&delivery)
            .expect("import delivery");
        transaction
            .create_activity(&activity(ActivityId::new(), source_id, proxy_id))
            .expect("create first activity");
        transaction.commit().expect("commit fixtures");
    }

    let rejected_id = ActivityId::new();
    let missing = RepresentationId::new();
    let property = MetadataProperty::new(
        VocabularyId::new("com.example.provenance").expect("valid vocabulary"),
        PropertyId::new("note").expect("valid property"),
    );
    let value = MetadataValue::string("invalid").expect("valid value");
    let mut transaction = project.begin_transaction().expect("begin transaction");
    assert_eq!(
        transaction
            .create_activity(&activity(rejected_id, source_id, missing))
            .expect_err("missing output must fail")
            .kind(),
        ErrorKind::NotFound
    );
    assert_eq!(
        transaction
            .add_metadata_value(ObjectRef::Activity(rejected_id), &property, &value)
            .expect_err("failed activity must be absent")
            .kind(),
        ErrorKind::NotFound
    );
    assert_eq!(
        transaction
            .create_activity(&activity(ActivityId::new(), proxy_id, source_id))
            .expect_err("cycle must fail")
            .kind(),
        ErrorKind::Conflict
    );
    transaction
        .create_activity(&activity(ActivityId::new(), proxy_id, delivery_id))
        .expect("transaction remains usable");
    transaction.commit().expect("commit valid activity");
}

#[test]
fn rollback_discards_activity_rows() {
    let directory = tempdir().expect("create temporary directory");
    let path = directory.path().join("production.pproj");
    let mut project = SqliteProject::create(path, None).expect("create project");
    let (source, source_id) = import(6);
    let (proxy, proxy_id) = import(7);
    let activity_id = ActivityId::new();
    {
        let mut transaction = project.begin_transaction().expect("begin transaction");
        transaction.import_original(&source).expect("import source");
        transaction.import_original(&proxy).expect("import proxy");
        transaction
            .create_activity(&activity(activity_id, source_id, proxy_id))
            .expect("create activity");
        transaction.rollback().expect("roll back provenance");
    }

    let property = MetadataProperty::new(
        VocabularyId::new("com.example.provenance").expect("valid vocabulary"),
        PropertyId::new("note").expect("valid property"),
    );
    let value = MetadataValue::string("absent").expect("valid value");
    let mut transaction = project.begin_transaction().expect("begin transaction");
    assert_eq!(
        transaction
            .add_metadata_value(ObjectRef::Activity(activity_id), &property, &value)
            .expect_err("rolled-back activity must be absent")
            .kind(),
        ErrorKind::NotFound
    );
    transaction.rollback().expect("close transaction");
}
