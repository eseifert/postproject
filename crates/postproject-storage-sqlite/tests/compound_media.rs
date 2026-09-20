//! Persistence coverage for compact and multi-resource content structures.

use postproject_core::{
    Asset, AssetId, ContentStructure, FrameRange, ImageSequenceDescriptor, ImageSequencePattern,
    Locator, LocatorAvailability, LocatorId, OriginalMediaImport, RationalRate, Representation,
    RepresentationId, RepresentationKind, Resource, ResourceId, ResourceMember, ResourceRole,
    Timestamp,
};
use postproject_storage_sqlite::SqliteProduction;
use rusqlite::Connection;

#[test]
fn sparse_image_sequence_reopens_without_per_frame_resources() {
    let directory = tempfile::tempdir().expect("create temporary directory");
    let production_path = directory.path().join("sequence.pproj");
    let now = Timestamp::from_unix_micros(1_000);
    let asset = Asset::new(AssetId::new(), now, Some("VFX plate".to_owned()), None);
    let resource_id = ResourceId::new();
    let frames = FrameRange::new(1_001, 1_100, 1).expect("valid frame range");
    let pattern = ImageSequencePattern::new("plate.", ".exr", 4).expect("valid pattern");
    let rate = RationalRate::new(24_000, 1_001).expect("valid rate");
    let sequence =
        ImageSequenceDescriptor::new(resource_id, pattern, frames, rate, vec![1_027, 1_042])
            .expect("valid sequence");
    let representation = Representation::new(
        RepresentationId::new(),
        asset.id(),
        RepresentationKind::Original,
        ContentStructure::image_sequence(sequence.clone()),
        Vec::new(),
    );
    let resource = Resource::new(resource_id, Vec::new(), None);
    let locator = Locator::new(
        LocatorId::new(),
        resource_id,
        "file:///production/plates/shot-a/",
        Some(now),
        LocatorAvailability::Online,
    )
    .expect("valid locator");
    let import = OriginalMediaImport::new(
        asset,
        representation.clone(),
        vec![resource],
        vec![locator.clone()],
    )
    .expect("valid compound import");

    let mut production =
        SqliteProduction::create(&production_path, None).expect("create production");
    let mut transaction = production.begin_transaction().expect("begin transaction");
    transaction.import_original(&import).expect("stage import");
    transaction.commit().expect("commit import");
    drop(transaction);
    drop(production);

    let reopened = SqliteProduction::open(&production_path).expect("reopen production");
    let stored = reopened
        .representations(import.asset().id())
        .expect("load representations");
    assert_eq!(stored, [representation]);
    assert_eq!(
        stored[0].content_structure().image_sequence_descriptor(),
        Some(&sequence)
    );
    assert_eq!(
        reopened
            .resources(stored[0].id())
            .expect("load sequence resources")
            .len(),
        1
    );
    assert_eq!(
        reopened.locators(resource_id).expect("load locators"),
        [locator]
    );
    drop(reopened);

    let connection = Connection::open(&production_path).expect("inspect database");
    let resource_rows: u32 = connection
        .query_row("SELECT count(*) FROM resources", [], |row| row.get(0))
        .expect("count resources");
    let exception_rows: u32 = connection
        .query_row(
            "SELECT count(*) FROM image_sequence_missing_frames",
            [],
            |row| row.get(0),
        )
        .expect("count sparse exceptions");
    assert_eq!(resource_rows, 1);
    assert_eq!(exception_rows, 2);
}

#[test]
fn ordered_parts_and_package_membership_round_trip() {
    let directory = tempfile::tempdir().expect("create temporary directory");
    let production_path = directory.path().join("compound.pproj");
    let ordered_ids = [
        ResourceId::from_bytes([1; 16]),
        ResourceId::from_bytes([2; 16]),
        ResourceId::from_bytes([3; 16]),
    ];
    let part_role = ResourceRole::new("example.camera:essence-part").expect("valid role");
    let ordered = ContentStructure::ordered_parts(
        ordered_ids
            .iter()
            .map(|id| ResourceMember::new(*id, part_role.clone(), true))
            .collect(),
    )
    .expect("valid ordered parts");
    let ordered_import = compound_import("Spanned original", ordered, &ordered_ids);

    let package_ids = [
        ResourceId::from_bytes([4; 16]),
        ResourceId::from_bytes([5; 16]),
        ResourceId::from_bytes([6; 16]),
    ];
    let package = ContentStructure::package(vec![
        ResourceMember::new(
            package_ids[0],
            ResourceRole::new("postproject:essence").expect("valid role"),
            true,
        ),
        ResourceMember::new(
            package_ids[1],
            ResourceRole::new("example.camera:playlist").expect("valid role"),
            true,
        ),
        ResourceMember::new(
            package_ids[2],
            ResourceRole::new("postproject:thumbnail").expect("valid role"),
            false,
        ),
    ])
    .expect("valid package");
    let package_import = compound_import("Camera package", package, &package_ids);

    let mut production =
        SqliteProduction::create(&production_path, None).expect("create production");
    let mut transaction = production.begin_transaction().expect("begin transaction");
    transaction
        .import_original(&ordered_import)
        .expect("stage ordered import");
    transaction
        .import_original(&package_import)
        .expect("stage package import");
    transaction.commit().expect("commit imports");
    drop(transaction);
    drop(production);

    let reopened = SqliteProduction::open(&production_path).expect("reopen production");
    for (import, expected_ids) in [
        (&ordered_import, ordered_ids.as_slice()),
        (&package_import, package_ids.as_slice()),
    ] {
        let stored = reopened
            .representations(import.asset().id())
            .expect("load representation");
        assert_eq!(stored, [import.representation().clone()]);
        assert_eq!(stored[0].content_structure().resource_ids(), expected_ids);
        let resources = reopened.resources(stored[0].id()).expect("load resources");
        assert_eq!(
            resources.iter().map(Resource::id).collect::<Vec<_>>(),
            expected_ids
        );
    }
}

fn compound_import(
    name: &str,
    structure: ContentStructure,
    resource_ids: &[ResourceId],
) -> OriginalMediaImport {
    let now = Timestamp::from_unix_micros(2_000);
    let asset = Asset::new(AssetId::new(), now, Some(name.to_owned()), None);
    let representation = Representation::new(
        RepresentationId::new(),
        asset.id(),
        RepresentationKind::Original,
        structure,
        Vec::new(),
    );
    let resources = resource_ids
        .iter()
        .map(|id| Resource::new(*id, Vec::new(), None))
        .collect();
    let locators = resource_ids
        .iter()
        .enumerate()
        .map(|(index, id)| {
            Locator::new(
                LocatorId::new(),
                *id,
                format!("file:///production/compound/member-{index}"),
                Some(now),
                LocatorAvailability::Online,
            )
            .expect("valid locator")
        })
        .collect();
    OriginalMediaImport::new(asset, representation, resources, locators)
        .expect("valid compound import")
}
