//! Runs every Rust listing included in the `PostProject` integrator guides.
//!
//! Each `// [name]` ... `// [/name]` region is included verbatim by the
//! documentation build, so keep regions self-contained and readable.

use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use postproject_core::{
    Activity, ActivityId, ActivityInput, ActivityKind, ActivityOutput, ActivityRole, AssetId,
    ExternalIdentifier, FrameRange, HostObjectBinding, IdentifierScheme, ImageSequencePattern,
    MediaRoot, MediaRootId, MetadataProperty, MetadataValue, ObjectRef, OriginIdentity,
    ProductionId, PropertyId, RationalRate, RepresentationId, RepresentationKind,
    RepresentationResolution, Result, RevisionContext, RevisionEvent, ToolIdentity, VocabularyId,
};
use postproject_media::{
    ImageSequenceSource, MediaResolver, MediaRootMapping, prepare_confirmed_locator,
    prepare_image_sequence_representation, prepare_original_media,
};
use postproject_storage_sqlite::SqliteProduction;

// [create-production]
fn create_production(path: &Path, media: &Path) -> Result<(SqliteProduction, AssetId)> {
    let mut production = SqliteProduction::create(path, Some("Documentary".to_owned()))?;

    let import = prepare_original_media(media, Some("Camera A".to_owned()), None)?;
    let asset_id = import.asset().id();
    {
        let mut transaction = production.begin_transaction()?;
        transaction.set_revision_context(RevisionContext::new(
            Some(OriginIdentity::new(
                "com.example.editor",
                Some("0.4.0".to_owned()),
                None,
            )?),
            Some("Import camera original".to_owned()),
        )?)?;
        transaction.import_original(&import)?;
        transaction.commit()?;
    }

    let representations = production.representations(asset_id)?;
    println!("representations: {}", representations.len());
    Ok((production, asset_id))
}
// [/create-production]

// [external-identifiers]
fn tag_camera_serial(production: &mut SqliteProduction, asset_id: AssetId) -> Result<()> {
    let target = ObjectRef::Asset(asset_id);
    let identifier = ExternalIdentifier::new(
        IdentifierScheme::new("com.example.camera.serial")?,
        "A-0007",
        None,
    )?;
    {
        let mut transaction = production.begin_transaction()?;
        transaction.add_external_identifier(target, &identifier)?;
        transaction.commit()?;
    }

    let attached = production.external_identifiers(target)?;
    let matches =
        production.find_by_external_identifier(identifier.scheme(), identifier.value())?;
    assert_eq!(attached, vec![identifier]);
    assert_eq!(matches, vec![target]);
    Ok(())
}
// [/external-identifiers]

// [metadata]
fn add_title(production: &mut SqliteProduction, asset_id: AssetId) -> Result<()> {
    let title = MetadataProperty::new(
        VocabularyId::new(
            "https://iptc.org/std/videometadatahub/recommendation/iptc-vmhub-1.7-schema.json",
        )?,
        PropertyId::new("title")?,
    );
    let target = ObjectRef::Asset(asset_id);
    {
        let mut transaction = production.begin_transaction()?;
        transaction.add_metadata_value(
            target,
            &title,
            &MetadataValue::language_string("Interview", "en-US")?,
        )?;
        transaction.commit()?;
    }

    let on_asset = production.metadata(target)?;
    let everywhere = production.query_by_metadata_property(&title)?;
    assert_eq!(on_asset.len(), 1);
    assert_eq!(everywhere.len(), 1);
    Ok(())
}
// [/metadata]

// [media-root]
fn add_rushes_root(production: &mut SqliteProduction) -> Result<()> {
    let root = MediaRoot::new(
        MediaRootId::new(),
        "rushes",
        Some("Camera originals".to_owned()),
        None,
        0,
        true,
    )?;
    let mut transaction = production.begin_transaction()?;
    transaction.add_media_root(root)?;
    transaction.commit()
}
// [/media-root]

// [resolve-asset]
fn resolve_asset(
    production: &SqliteProduction,
    asset_id: AssetId,
    rushes_directory: &Path,
) -> Result<Vec<RepresentationResolution>> {
    let mappings = [MediaRootMapping::new("rushes", rushes_directory)?];
    let roots = production.production().media_roots();
    let resolver = MediaResolver::default();

    let mut resolutions = Vec::new();
    for representation in production.representations(asset_id)? {
        let mut resources = Vec::new();
        for resource in production.resources(representation.id())? {
            let locators = production.locators(resource.id())?;
            resources.push(resolver.resolve_resource(
                &resource,
                representation.content_structure(),
                &locators,
                roots,
                &mappings,
            )?);
        }
        let resolution = RepresentationResolution::aggregate(
            representation.id(),
            representation.content_structure(),
            resources,
        )?;
        println!("availability: {:?}", resolution.availability());
        for resource in resolution.resources() {
            for candidate in resource.candidates() {
                println!("candidate: {}", candidate.uri());
            }
        }
        resolutions.push(resolution);
    }
    Ok(resolutions)
}
// [/resolve-asset]

// [confirm-locator]
fn confirm_unique_candidates(
    production: &mut SqliteProduction,
    resolutions: &[RepresentationResolution],
) -> Result<()> {
    let mut transaction = production.begin_transaction()?;
    for resolution in resolutions {
        for resource in resolution.resources() {
            // Several candidates need a person to choose; never pick one here.
            if let [candidate] = resource.candidates() {
                let locator = prepare_confirmed_locator(resource.resource_id(), candidate.uri())?;
                transaction.add_locator(&locator)?;
            }
        }
    }
    transaction.commit()
}
// [/confirm-locator]

// [image-sequence]
fn add_render_sequence(
    production: &mut SqliteProduction,
    asset_id: AssetId,
    directory: &Path,
) -> Result<RepresentationId> {
    let source = ImageSequenceSource::new(
        directory,
        ImageSequencePattern::new("shot010.", ".exr", 4)?,
        FrameRange::new(1001, 1004, 1)?,
        RationalRate::new(24000, 1001)?,
        vec![1003],
    );
    let import =
        prepare_image_sequence_representation(asset_id, RepresentationKind::Derived, &source)?;
    let sequence_id = import.representation().id();
    {
        let mut transaction = production.begin_transaction()?;
        transaction.add_representation(&import)?;
        transaction.commit()?;
    }

    let stored = production
        .representations(asset_id)?
        .into_iter()
        .find(|representation| representation.id() == sequence_id)
        .expect("committed representation");
    println!("structure: {:?}", stored.content_structure().kind());
    Ok(sequence_id)
}
// [/image-sequence]

// [provenance]
fn record_render(
    production: &mut SqliteProduction,
    source_id: RepresentationId,
    render_id: RepresentationId,
) -> Result<()> {
    let activity = Activity::new(
        ActivityId::new(),
        ActivityKind::new("org.postproject:render")?,
        vec![ActivityInput::new(
            source_id,
            Some(ActivityRole::new("org.postproject:primary")?),
        )],
        vec![ActivityOutput::new(render_id, None)],
    )?
    .with_tool(ToolIdentity::new(
        "Example Renderer",
        Some("2.1".to_owned()),
        Some("https://example.com/renderer".to_owned()),
    )?);
    {
        let mut transaction = production.begin_transaction()?;
        transaction.create_activity(&activity)?;
        transaction.commit()?;
    }

    assert_eq!(
        production.activities_consuming(source_id)?,
        vec![activity.clone()]
    );
    assert_eq!(production.activities_producing(render_id)?, vec![activity]);
    assert_eq!(production.ancestors(render_id)?, vec![source_id]);
    assert_eq!(production.descendants(source_id)?, vec![render_id]);
    Ok(())
}
// [/provenance]

fn handle_event(event: &RevisionEvent) {
    println!("event {}: {:?}", event.position(), event.kind());
}

// [revision-feed]
fn process_changes(production: &SqliteProduction, mut cursor: u64) -> Result<u64> {
    loop {
        let page = production.changes_since(cursor, 100)?;
        for revision in &page {
            for event in production.events_for_revision(revision.id())? {
                handle_event(&event);
            }
            // Persist the cursor only after the whole revision is processed.
            cursor = revision.sequence();
        }
        if page.len() < 100 {
            return Ok(cursor);
        }
    }
}
// [/revision-feed]

// [host-binding]
fn bind_representation(
    production_id: ProductionId,
    representation_id: RepresentationId,
) -> Result<String> {
    let binding =
        HostObjectBinding::new(production_id, ObjectRef::Representation(representation_id))?;
    let stored = binding.to_string();

    let reopened = HostObjectBinding::from_str(&stored)?;
    assert_eq!(reopened, binding);
    Ok(stored)
}
// [/host-binding]

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/fixtures/sample-media.dat")
}

#[test]
fn guide_examples_run_in_order() -> Result<()> {
    let work = tempfile::tempdir().expect("temporary directory");
    let rushes = work.path().join("rushes");
    let moved = work.path().join("moved");
    let renders = work.path().join("renders/shot010");
    for directory in [&rushes, &moved, &renders] {
        fs::create_dir_all(directory).expect("work directory");
    }
    fs::copy(fixture(), rushes.join("A001.mov")).expect("media fixture");
    for frame in [1001, 1002, 1004] {
        fs::write(
            renders.join(format!("shot010.{frame}.exr")),
            format!("frame {frame}"),
        )
        .expect("sequence frame");
    }

    let (mut production, asset_id) = create_production(
        &work.path().join("production.pproj"),
        &rushes.join("A001.mov"),
    )?;
    let original_id = production.representations(asset_id)?[0].id();
    tag_camera_serial(&mut production, asset_id)?;
    add_title(&mut production, asset_id)?;

    add_rushes_root(&mut production)?;
    fs::rename(rushes.join("A001.mov"), moved.join("A001.mov")).expect("move media");
    let resolutions = resolve_asset(&production, asset_id, &moved)?;
    assert_eq!(resolutions[0].resources()[0].candidates().len(), 1);
    confirm_unique_candidates(&mut production, &resolutions)?;

    let sequence_id = add_render_sequence(&mut production, asset_id, &renders)?;
    record_render(&mut production, original_id, sequence_id)?;

    let cursor = process_changes(&production, 0)?;
    assert_eq!(
        Some(cursor),
        production
            .latest_revision()?
            .map(|revision| revision.sequence())
    );

    let binding = bind_representation(production.production().id(), sequence_id)?;
    assert!(binding.starts_with("https://postproject.org/ref/v1/"));
    Ok(())
}
