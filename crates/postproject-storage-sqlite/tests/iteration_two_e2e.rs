//! End-to-end coverage for compound representations on one logical asset.

use std::{fs, path::Path};

use postproject_core::{
    Activity, ActivityId, ActivityInput, ActivityKind, ActivityOutput, FrameRange,
    ImageSequencePattern, OriginalMediaImport, RationalRate, RepresentationAvailability,
    RepresentationImport, RepresentationResolution, Resource, ResourceRole,
};
use postproject_media::{
    FileResourceSource, ImageSequenceSource, MediaResolver, prepare_image_sequence_representation,
    prepare_ordered_parts_representation, prepare_original_media,
    prepare_single_file_representation,
};
use postproject_storage_sqlite::SqliteProduction;

struct Fixture {
    original: OriginalMediaImport,
    sequence: RepresentationImport,
    ordered: RepresentationImport,
    proxy: RepresentationImport,
    activity: Activity,
}

#[test]
fn one_asset_round_trips_compound_media_and_proxy_provenance() {
    let directory = tempfile::tempdir().expect("create fixture directory");
    let production_path = directory.path().join("production.pproj");
    let fixture = prepare_fixture(directory.path());
    persist_fixture(&production_path, &fixture);
    fs::remove_file(directory.path().join("plates/shot010.1003.exr"))
        .expect("remove a declared frame");
    assert_reopened(&production_path, &fixture);
}

fn prepare_fixture(root: &Path) -> Fixture {
    let source_path = root.join("source.mov");
    let proxy_path = root.join("proxy.mp4");
    fs::write(&source_path, b"camera original").expect("write original");
    fs::write(&proxy_path, b"editorial proxy").expect("write proxy");
    let original = prepare_original_media(
        &source_path,
        Some("Shot 010".to_owned()),
        Some("camera ingest".to_owned()),
    )
    .expect("prepare original");
    let asset_id = original.asset().id();
    let original_id = original.representation().id();

    let sequence_directory = root.join("plates");
    fs::create_dir(&sequence_directory).expect("create sequence directory");
    for frame in [1001, 1003, 1004] {
        fs::write(
            sequence_directory.join(format!("shot010.{frame:04}.exr")),
            format!("frame {frame}"),
        )
        .expect("write sequence frame");
    }
    let sequence = prepare_image_sequence_representation(
        asset_id,
        postproject_core::RepresentationKind::Derived,
        &ImageSequenceSource::new(
            &sequence_directory,
            ImageSequencePattern::new("shot010.", ".exr", 4).expect("valid sequence pattern"),
            FrameRange::new(1001, 1004, 1).expect("valid frame range"),
            RationalRate::new(24_000, 1_001).expect("valid rate"),
            vec![1002],
        ),
    )
    .expect("prepare sequence");

    let part_paths = [root.join("span-1.mxf"), root.join("span-2.mxf")];
    fs::write(&part_paths[0], b"span one").expect("write first span");
    fs::write(&part_paths[1], b"span two").expect("write second span");
    let part_role = ResourceRole::new("org.postproject:essence-part").expect("valid role");
    let part_sources = part_paths
        .iter()
        .map(|path| FileResourceSource::new(path, part_role.clone(), true))
        .collect::<Vec<_>>();
    let ordered = prepare_ordered_parts_representation(
        asset_id,
        postproject_core::RepresentationKind::Original,
        &part_sources,
    )
    .expect("prepare ordered parts");

    let proxy = prepare_single_file_representation(
        asset_id,
        postproject_core::RepresentationKind::Proxy,
        &proxy_path,
    )
    .expect("prepare proxy");
    let proxy_id = proxy.representation().id();
    let activity = Activity::new(
        ActivityId::new(),
        ActivityKind::new("org.postproject:transcode").expect("valid activity kind"),
        vec![ActivityInput::new(original_id, None)],
        vec![ActivityOutput::new(proxy_id, None)],
    )
    .expect("prepare activity");

    Fixture {
        original,
        sequence,
        ordered,
        proxy,
        activity,
    }
}

fn persist_fixture(production_path: &Path, fixture: &Fixture) {
    let mut production =
        SqliteProduction::create(production_path, None).expect("create production");
    {
        let mut transaction = production.begin_transaction().expect("begin import");
        transaction
            .import_original(&fixture.original)
            .expect("stage original");
        transaction.commit().expect("commit original");
    }
    {
        let mut transaction = production.begin_transaction().expect("begin additions");
        transaction
            .add_representation(&fixture.sequence)
            .expect("stage sequence");
        transaction
            .add_representation(&fixture.ordered)
            .expect("stage ordered parts");
        transaction
            .add_representation(&fixture.proxy)
            .expect("stage proxy");
        transaction
            .create_activity(&fixture.activity)
            .expect("stage provenance");
        transaction.commit().expect("commit additions");
    }
}

fn assert_reopened(production_path: &Path, fixture: &Fixture) {
    let reopened = SqliteProduction::open(production_path).expect("reopen production");
    let asset_id = fixture.original.asset().id();
    let original_id = fixture.original.representation().id();
    let sequence_id = fixture.sequence.representation().id();
    let ordered_id = fixture.ordered.representation().id();
    let proxy_id = fixture.proxy.representation().id();
    let representations = reopened
        .representations(asset_id)
        .expect("load representations");
    assert_eq!(representations.len(), 4);
    assert!(representations.iter().any(|item| item.id() == ordered_id));
    assert_eq!(
        reopened.ancestors(proxy_id).expect("load ancestry"),
        [original_id]
    );
    assert_eq!(
        reopened
            .activities_producing(proxy_id)
            .expect("load producing activity"),
        std::slice::from_ref(&fixture.activity)
    );

    let stored_sequence = representations
        .iter()
        .find(|item| item.id() == sequence_id)
        .expect("find sequence");
    assert_eq!(stored_sequence.fingerprints().len(), 1);
    let resources = reopened
        .resources(sequence_id)
        .expect("load sequence resource");
    assert_eq!(resources[0].fingerprints().len(), 1);
    let locators = reopened
        .locators(resources[0].id())
        .expect("load sequence locator");
    let resource_resolution = MediaResolver::default()
        .resolve_resource(
            &resources[0],
            stored_sequence.content_structure(),
            &locators,
            &[],
        )
        .expect("resolve sequence");
    let resolution = RepresentationResolution::aggregate(
        sequence_id,
        stored_sequence.content_structure(),
        vec![resource_resolution],
    )
    .expect("aggregate sequence availability");
    assert_eq!(
        resolution.availability(),
        RepresentationAvailability::Partial
    );
    assert_eq!(resolution.issues()[0].frames(), &[1002, 1003]);
    assert_eq!(
        reopened
            .resources(ordered_id)
            .expect("load ordered resources")
            .iter()
            .map(Resource::id)
            .collect::<Vec<_>>(),
        fixture
            .ordered
            .resources()
            .iter()
            .map(Resource::id)
            .collect::<Vec<_>>()
    );
}
