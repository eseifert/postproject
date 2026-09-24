//! Complete dependency-observation persistence and journal coverage.

use postproject_core::{
    Activity, ActivityId, ActivityInput, ActivityKind, ActivityOutput, Asset, AssetId,
    ContentStructure, Dependency, DependencyKind, DependencySetStatus, DependencyTarget, ErrorKind,
    Locator, LocatorAvailability, LocatorId, OriginalMediaImport, Representation,
    RepresentationFingerprint, RepresentationId, RepresentationKind, Resource, ResourceId,
    RevisionEventKind, Timestamp,
};
use postproject_storage_sqlite::SqliteProduction;
use rusqlite::Connection;

fn media(label: u8) -> OriginalMediaImport {
    let asset_id = AssetId::from_bytes([label; 16]);
    let representation_id = RepresentationId::from_bytes([label; 16]);
    let resource_id = ResourceId::from_bytes([label; 16]);
    OriginalMediaImport::new(
        Asset::new(
            asset_id,
            Timestamp::from_unix_micros(i64::from(label)),
            None,
            None,
        ),
        Representation::new(
            representation_id,
            asset_id,
            RepresentationKind::Original,
            ContentStructure::single_resource(resource_id),
            vec![
                RepresentationFingerprint::new("aggregate", 1, vec![label])
                    .expect("valid representation fingerprint"),
            ],
        ),
        vec![Resource::new(resource_id, Vec::new(), None)],
        vec![
            Locator::new(
                LocatorId::from_bytes([label; 16]),
                resource_id,
                format!("file:///media/{label}.mov"),
                None,
                LocatorAvailability::Online,
            )
            .expect("valid locator"),
        ],
    )
    .expect("valid media")
}

fn assert_direct_dependents(
    production: &SqliteProduction,
    source: RepresentationId,
    target_asset: AssetId,
    target_representation: RepresentationId,
) {
    assert_eq!(
        production
            .dependents(DependencyTarget::Asset(target_asset))
            .expect("query asset dependents"),
        [source]
    );
    assert_eq!(
        production
            .dependents(DependencyTarget::Representation(target_representation))
            .expect("query representation dependents"),
        [source]
    );
}

#[test]
fn complete_dependency_sets_replace_atomically_and_are_journaled() {
    let directory = tempfile::tempdir().expect("create directory");
    let path = directory.path().join("dependencies.pproj");
    let mut production = SqliteProduction::create(&path, None).expect("create production");
    let source = media(1);
    let target = media(2);
    {
        let mut transaction = production.begin_transaction().expect("begin import");
        transaction.import_original(&source).expect("import source");
        transaction.import_original(&target).expect("import target");
        transaction.commit().expect("commit import");
    }

    assert_eq!(
        production
            .dependency_set(source.representation().id())
            .expect("read absent set"),
        None
    );
    let dependencies = vec![
        Dependency::new(
            Some(source.resources()[0].id()),
            DependencyKind::new("org.openusd:reference").expect("kind"),
            DependencyTarget::Asset(target.asset().id()),
            Some(target.representation().id()),
            true,
            "../Character.usd",
        )
        .expect("asset dependency"),
        Dependency::new(
            None,
            DependencyKind::new("org.openusd:payload").expect("kind"),
            DependencyTarget::Representation(target.representation().id()),
            None,
            false,
            "payloads/optional.usd",
        )
        .expect("pinned dependency"),
    ];
    {
        let mut transaction = production.begin_transaction().expect("begin observation");
        assert!(
            transaction
                .record_dependency_set(source.representation().id(), &dependencies)
                .expect("record dependency set")
        );
        transaction.commit().expect("commit observation");
    }

    let stored = production
        .dependency_set(source.representation().id())
        .expect("read dependency set")
        .expect("recorded set");
    assert_eq!(stored.status(), DependencySetStatus::Current);
    assert_eq!(stored.recorded_at_revision(), 2);
    assert_eq!(stored.dependencies(), dependencies);
    assert_direct_dependents(
        &production,
        source.representation().id(),
        target.asset().id(),
        target.representation().id(),
    );
    let revision = production
        .latest_revision()
        .expect("read latest revision")
        .expect("latest revision");
    assert!(matches!(
        production.events_for_revision(revision.id()).expect("events")[0].kind(),
        RevisionEventKind::DependencySetRecorded { representation_id }
            if *representation_id == source.representation().id()
    ));

    {
        let mut transaction = production.begin_transaction().expect("begin no-op");
        assert!(
            !transaction
                .record_dependency_set(source.representation().id(), &dependencies)
                .expect("record identical set")
        );
        transaction.commit().expect("commit no-op");
    }
    assert_eq!(
        production
            .latest_revision()
            .expect("read latest revision")
            .expect("latest revision")
            .sequence(),
        2
    );

    {
        let mut transaction = production.begin_transaction().expect("begin empty set");
        assert!(
            transaction
                .record_dependency_set(source.representation().id(), &[])
                .expect("record known-empty set")
        );
        transaction.commit().expect("commit empty set");
    }
    let empty = production
        .dependency_set(source.representation().id())
        .expect("read empty set")
        .expect("known empty set");
    assert!(empty.dependencies().is_empty());
    assert_eq!(empty.recorded_at_revision(), 3);
}

#[test]
fn invalid_dependency_references_leave_no_partial_observation() {
    let directory = tempfile::tempdir().expect("create directory");
    let path = directory.path().join("invalid-dependency.pproj");
    let mut production = SqliteProduction::create(&path, None).expect("create production");
    let source = media(3);
    let target = media(4);
    {
        let mut transaction = production.begin_transaction().expect("begin import");
        transaction.import_original(&source).expect("import source");
        transaction.import_original(&target).expect("import target");
        transaction.commit().expect("commit import");
    }
    let invalid = Dependency::new(
        Some(target.resources()[0].id()),
        DependencyKind::new("org.openusd:reference").expect("kind"),
        DependencyTarget::Representation(target.representation().id()),
        None,
        true,
        "wrong-member.usd",
    )
    .expect("domain-valid dependency");

    let mut transaction = production.begin_transaction().expect("begin invalid set");
    let error = transaction
        .record_dependency_set(source.representation().id(), &[invalid])
        .expect_err("reject foreign source resource");
    assert_eq!(error.kind(), ErrorKind::InvalidArgument);
    transaction.commit().expect("commit unchanged transaction");
    drop(transaction);
    assert_eq!(
        production
            .dependency_set(source.representation().id())
            .expect("read absent set"),
        None
    );
    assert_eq!(
        production
            .latest_revision()
            .expect("read latest revision")
            .expect("latest revision")
            .sequence(),
        1
    );
}

#[test]
fn representation_observation_marks_dependencies_for_extraction() {
    let directory = tempfile::tempdir().expect("create directory");
    let path = directory.path().join("dependency-extraction.pproj");
    let mut production = SqliteProduction::create(&path, None).expect("create production");
    let source = media(5);
    let target = media(6);
    let dependency = Dependency::new(
        None,
        DependencyKind::new("org.postproject:requires").expect("kind"),
        DependencyTarget::Representation(target.representation().id()),
        None,
        true,
        "target.mov",
    )
    .expect("dependency");
    {
        let mut transaction = production.begin_transaction().expect("begin setup");
        transaction.import_original(&source).expect("import source");
        transaction.import_original(&target).expect("import target");
        transaction
            .record_dependency_set(
                source.representation().id(),
                std::slice::from_ref(&dependency),
            )
            .expect("record dependencies");
        transaction.commit().expect("commit setup");
    }

    let changed = RepresentationFingerprint::new("aggregate", 1, vec![9]).expect("fingerprint");
    {
        let mut transaction = production.begin_transaction().expect("begin observation");
        transaction
            .record_representation_fingerprint(source.representation().id(), &changed)
            .expect("record changed fingerprint");
        transaction.commit().expect("commit observation");
    }
    let dirty = production
        .dependency_set(source.representation().id())
        .expect("read dirty dependency set")
        .expect("dependency set");
    assert_eq!(dirty.status(), DependencySetStatus::NeedsExtraction);
    assert_eq!(dirty.recorded_at_revision(), 1);

    let mut transaction = production.begin_transaction().expect("begin extraction");
    assert!(
        transaction
            .record_dependency_set(
                source.representation().id(),
                std::slice::from_ref(&dependency)
            )
            .expect("replace extracted set")
    );
    transaction.commit().expect("commit extraction");
    drop(transaction);
    assert_eq!(
        production
            .dependency_set(source.representation().id())
            .expect("read current dependency set")
            .expect("dependency set")
            .status(),
        DependencySetStatus::Current
    );
}

#[test]
fn activity_inputs_capture_required_dependency_paths() {
    let directory = tempfile::tempdir().expect("create directory");
    let path = directory.path().join("dependency-snapshot.pproj");
    let mut production = SqliteProduction::create(&path, None).expect("create production");
    let source = media(7);
    let required = media(8);
    let optional = media(9);
    let output = media(10);
    let dependencies = [
        Dependency::new(
            None,
            DependencyKind::new("org.postproject:requires").expect("kind"),
            DependencyTarget::Representation(required.representation().id()),
            None,
            true,
            "required.mov",
        )
        .expect("required dependency"),
        Dependency::new(
            None,
            DependencyKind::new("org.openusd:payload").expect("kind"),
            DependencyTarget::Representation(optional.representation().id()),
            None,
            false,
            "optional.mov",
        )
        .expect("optional dependency"),
    ];
    let activity = Activity::new(
        ActivityId::from_bytes([11; 16]),
        ActivityKind::new("org.postproject:render").expect("kind"),
        vec![ActivityInput::new(source.representation().id(), None)],
        vec![ActivityOutput::new(output.representation().id(), None)],
    )
    .expect("activity");
    {
        let mut transaction = production.begin_transaction().expect("begin setup");
        for import in [&source, &required, &optional, &output] {
            transaction.import_original(import).expect("import media");
        }
        transaction
            .record_dependency_set(source.representation().id(), &dependencies)
            .expect("record dependencies");
        transaction
            .create_activity(&activity)
            .expect("create activity");
        transaction.commit().expect("commit setup");
    }
    drop(production);

    let connection = Connection::open(path).expect("open database");
    let marker_count: u32 = connection
        .query_row(
            "SELECT count(*) FROM activity_input_dependency_snapshots",
            [],
            |row| row.get(0),
        )
        .expect("count snapshot markers");
    let path_count: u32 = connection
        .query_row(
            "SELECT count(*) FROM activity_input_dependency_paths",
            [],
            |row| row.get(0),
        )
        .expect("count snapshot paths");
    let edge_count: u32 = connection
        .query_row(
            "SELECT count(*) FROM activity_input_dependency_path_edges",
            [],
            |row| row.get(0),
        )
        .expect("count snapshot path edges");
    let fingerprint: Vec<u8> = connection
        .query_row(
            "SELECT value FROM activity_input_dependency_fingerprint_snapshots",
            [],
            |row| row.get(0),
        )
        .expect("load dependency fingerprint");
    assert_eq!((marker_count, path_count, edge_count), (1, 1, 1));
    assert_eq!(fingerprint, [8]);
}
