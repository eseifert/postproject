//! Command-line demonstrator for `PostProject` domain services.

#![forbid(unsafe_code)]

use std::{path::PathBuf, process::ExitCode, str::FromStr};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use postproject_core::{
    Asset, AssetId, EvidenceKind, ExternalIdentifier, IdentifierScheme, Locator,
    LocatorAvailability, MetadataAssertion, MetadataField, MetadataProperty, MetadataValue,
    MetadataValueKind, ObjectRef, ProjectId, PropertyId, Representation, RepresentationId,
    RepresentationKind, Resolution, ResolutionEvidence, ResolutionState, Resource, VocabularyId,
};
use postproject_media::{
    MediaResolver, prepare_confirmed_locator, prepare_media_root, prepare_original_media,
};
use postproject_storage_sqlite::SqliteProject;
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(name = "postproject", version, about)]
struct Cli {
    /// Emit machine-readable JSON.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a project file.
    Init(InitArgs),
    /// Inspect and manage project media.
    Media(MediaArgs),
    /// Manage resolver search roots.
    Root(RootArgs),
    /// Manage external industry, vendor, and application identifiers.
    Identifier(IdentifierArgs),
    /// Inspect and manage standards-aware metadata assertions.
    Metadata(MetadataArgs),
}

#[derive(Debug, Args)]
struct InitArgs {
    /// Project file to create.
    project: PathBuf,
    /// Optional project display name.
    #[arg(long)]
    name: Option<String>,
}

#[derive(Debug, Args)]
struct MediaArgs {
    #[command(subcommand)]
    command: MediaCommand,
}

#[derive(Debug, Subcommand)]
enum MediaCommand {
    /// Import an original media file.
    Add(MediaAddArgs),
    /// List logical media assets.
    List(ProjectArgs),
    /// Show an asset, its representations, resources, and locators.
    Show(MediaAssetArgs),
    /// Resolve an asset under configured media roots.
    Resolve(MediaResolveArgs),
}

#[derive(Debug, Args)]
struct MediaAddArgs {
    project: PathBuf,
    file: PathBuf,
    /// Optional asset display name.
    #[arg(long)]
    name: Option<String>,
}

#[derive(Debug, Args)]
struct ProjectArgs {
    project: PathBuf,
}

#[derive(Debug, Args)]
struct MediaAssetArgs {
    project: PathBuf,
    asset_id: String,
}

#[derive(Debug, Args)]
struct MediaResolveArgs {
    project: PathBuf,
    asset_id: String,
    /// Confirm one URI returned by this resolution and persist it.
    #[arg(long, value_name = "URI")]
    confirm: Option<String>,
}

#[derive(Debug, Args)]
struct RootArgs {
    #[command(subcommand)]
    command: RootCommand,
}

#[derive(Debug, Subcommand)]
enum RootCommand {
    /// Add a directory searched during media resolution.
    Add(RootAddArgs),
}

#[derive(Debug, Args)]
struct RootAddArgs {
    project: PathBuf,
    directory: PathBuf,
    #[arg(long)]
    label: Option<String>,
    /// Lower priorities are searched first.
    #[arg(long, default_value_t = 0)]
    priority: i32,
}

#[derive(Debug, Args)]
struct IdentifierArgs {
    #[command(subcommand)]
    command: IdentifierCommand,
}

#[derive(Debug, Subcommand)]
enum IdentifierCommand {
    /// Attach an external identifier to an asset or representation.
    Add(IdentifierMutationArgs),
    /// Remove one exact external identifier attachment.
    Remove(IdentifierMutationArgs),
    /// List external identifiers attached to an object.
    List(IdentifierTargetArgs),
    /// Find objects carrying an exact scheme and value.
    Find(IdentifierFindArgs),
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum IdentifierTargetKind {
    Asset,
    Representation,
}

#[derive(Debug, Args)]
struct IdentifierTargetArgs {
    project: PathBuf,
    #[arg(value_enum)]
    target_kind: IdentifierTargetKind,
    target_id: String,
}

#[derive(Debug, Args)]
struct IdentifierMutationArgs {
    #[command(flatten)]
    target: IdentifierTargetArgs,
    scheme: String,
    value: String,
    #[arg(long)]
    qualifier: Option<String>,
}

#[derive(Debug, Args)]
struct IdentifierFindArgs {
    project: PathBuf,
    scheme: String,
    value: String,
}

#[derive(Debug, Args)]
struct MetadataArgs {
    #[command(subcommand)]
    command: MetadataCommand,
}

#[derive(Debug, Subcommand)]
enum MetadataCommand {
    /// Append a plain or language-tagged text value.
    AddText(MetadataAddTextArgs),
    /// List all metadata assertions attached to an object.
    List(MetadataTargetArgs),
    /// Remove every value of one property from an object.
    Remove(MetadataPropertyArgs),
    /// Find assertions using an exact vocabulary and property.
    Find(MetadataFindArgs),
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum MetadataTargetKind {
    Project,
    Asset,
    Representation,
}

#[derive(Debug, Args)]
struct MetadataTargetArgs {
    project: PathBuf,
    #[arg(value_enum)]
    target_kind: MetadataTargetKind,
    target_id: String,
}

#[derive(Debug, Args)]
struct MetadataPropertyArgs {
    #[command(flatten)]
    target: MetadataTargetArgs,
    vocabulary: String,
    property: String,
}

#[derive(Debug, Args)]
struct MetadataAddTextArgs {
    #[command(flatten)]
    target: MetadataTargetArgs,
    vocabulary: String,
    property: String,
    value: String,
    /// Optional BCP 47-shaped language tag.
    #[arg(long)]
    language: Option<String>,
}

#[derive(Debug, Args)]
struct MetadataFindArgs {
    project: PathBuf,
    vocabulary: String,
    property: String,
}

#[derive(Debug, Serialize)]
struct ProjectView {
    id: String,
    path: String,
    schema_version: u32,
    display_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct ImportView {
    asset_id: String,
    representation_id: String,
    resource_id: String,
    locator_id: String,
    uri: String,
}

#[derive(Debug, Serialize)]
struct AssetSummary {
    id: String,
    display_name: Option<String>,
    created_at_unix_micros: i64,
    representation_count: usize,
}

#[derive(Debug, Serialize)]
struct AssetView {
    id: String,
    display_name: Option<String>,
    import_source: Option<String>,
    created_at_unix_micros: i64,
    representations: Vec<RepresentationView>,
}

#[derive(Debug, Serialize)]
struct RepresentationView {
    id: String,
    kind: &'static str,
    structure: &'static str,
    resources: Vec<ResourceView>,
}

#[derive(Debug, Serialize)]
struct ResourceView {
    id: String,
    fingerprints: Vec<FingerprintView>,
    file_size_bytes: Option<u64>,
    locators: Vec<LocatorView>,
}

#[derive(Debug, Serialize)]
struct FingerprintView {
    algorithm: String,
    version: u16,
    value_hex: String,
}

#[derive(Debug, Serialize)]
struct LocatorView {
    id: String,
    uri: String,
    availability: &'static str,
    last_seen_unix_micros: Option<i64>,
}

#[derive(Debug, Serialize)]
struct RootView {
    id: String,
    uri: String,
    label: Option<String>,
    priority: i32,
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct ResolveView {
    asset_id: String,
    resolutions: Vec<ResolutionView>,
    confirmed_uri: Option<String>,
}

#[derive(Debug, Serialize)]
struct ResolutionView {
    representation_id: String,
    state: &'static str,
    candidates: Vec<CandidateView>,
    evidence: Vec<EvidenceView>,
}

#[derive(Debug, Serialize)]
struct CandidateView {
    uri: String,
    confidence_basis_points: u16,
    evidence: Vec<EvidenceView>,
}

#[derive(Debug, Serialize)]
struct EvidenceView {
    kind: &'static str,
    detail: Option<String>,
}

#[derive(Debug, Serialize)]
struct ExternalIdentifierView {
    target_kind: &'static str,
    target_id: String,
    scheme: String,
    value: String,
    qualifier: Option<String>,
}

#[derive(Debug, Serialize)]
struct ObjectRefView {
    kind: &'static str,
    id: String,
}

#[derive(Debug, Serialize)]
struct MetadataAssertionView {
    target_kind: &'static str,
    target_id: String,
    vocabulary: String,
    property: String,
    value: MetadataValueView,
}

#[derive(Debug, Serialize)]
struct MetadataPropertyView {
    target_kind: &'static str,
    target_id: String,
    vocabulary: String,
    property: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum MetadataValueView {
    String { value: String },
    LangString { value: String, language: String },
    I64 { value: i64 },
    U64 { value: u64 },
    Decimal { coefficient: String, scale: u32 },
    Bool { value: bool },
    Timestamp { unix_micros: i64 },
    Uri { value: String },
    Bytes { hex: String },
    Rational { numerator: i64, denominator: u64 },
    List { values: Vec<MetadataValueView> },
    Struct { fields: Vec<MetadataFieldView> },
    Reference { target: ObjectRefView },
}

#[derive(Debug, Serialize)]
struct MetadataFieldView {
    name: String,
    value: MetadataValueView,
}

fn main() -> ExitCode {
    match execute(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Init(args) => init(args, cli.json),
        Command::Media(args) => match args.command {
            MediaCommand::Add(args) => media_add(args, cli.json),
            MediaCommand::List(args) => media_list(&args, cli.json),
            MediaCommand::Show(args) => media_show(&args, cli.json),
            MediaCommand::Resolve(args) => media_resolve(args, cli.json),
        },
        Command::Root(args) => match args.command {
            RootCommand::Add(args) => root_add(args, cli.json),
        },
        Command::Identifier(args) => match args.command {
            IdentifierCommand::Add(args) => identifier_mutate(args, false, cli.json),
            IdentifierCommand::Remove(args) => identifier_mutate(args, true, cli.json),
            IdentifierCommand::List(args) => identifier_list(&args, cli.json),
            IdentifierCommand::Find(args) => identifier_find(args, cli.json),
        },
        Command::Metadata(args) => match args.command {
            MetadataCommand::AddText(args) => metadata_add_text(args, cli.json),
            MetadataCommand::List(args) => metadata_list(&args, cli.json),
            MetadataCommand::Remove(args) => metadata_remove(args, cli.json),
            MetadataCommand::Find(args) => metadata_find(args, cli.json),
        },
    }
}

fn init(args: InitArgs, json: bool) -> Result<()> {
    let project = SqliteProject::create(&args.project, args.name).context("create project")?;
    let view = ProjectView {
        id: project.project().id().to_string(),
        path: project.path().display().to_string(),
        schema_version: project.project().schema_version(),
        display_name: project.project().display_name().map(str::to_owned),
    };
    if json {
        print_json(&view)
    } else {
        println!("created project {} at {}", view.id, view.path);
        Ok(())
    }
}

fn media_add(args: MediaAddArgs, json: bool) -> Result<()> {
    let prepared =
        prepare_original_media(&args.file, args.name, Some("postproject-cli".to_owned()))
            .context("prepare media import")?;
    let view = ImportView {
        asset_id: prepared.asset().id().to_string(),
        representation_id: prepared.representation().id().to_string(),
        resource_id: prepared.resources()[0].id().to_string(),
        locator_id: prepared.locators()[0].id().to_string(),
        uri: prepared.locators()[0].uri().to_owned(),
    };
    let mut project = SqliteProject::open(&args.project).context("open project")?;
    let mut transaction = project
        .begin_transaction()
        .context("begin import transaction")?;
    transaction
        .import_original(&prepared)
        .context("stage media import")?;
    transaction.commit().context("commit media import")?;

    if json {
        print_json(&view)
    } else {
        println!("imported asset {} from {}", view.asset_id, view.uri);
        Ok(())
    }
}

fn media_list(args: &ProjectArgs, json: bool) -> Result<()> {
    let project = SqliteProject::open(&args.project).context("open project")?;
    let mut views = Vec::new();
    for asset in project.assets().context("load assets")? {
        let representation_count = project
            .representations(asset.id())
            .context("load asset representations")?
            .len();
        views.push(AssetSummary {
            id: asset.id().to_string(),
            display_name: asset.display_name().map(str::to_owned),
            created_at_unix_micros: asset.created_at().as_unix_micros(),
            representation_count,
        });
    }

    if json {
        print_json(&views)
    } else {
        for asset in &views {
            println!(
                "{}\t{}\t{} representation(s)",
                asset.id,
                asset.display_name.as_deref().unwrap_or("-"),
                asset.representation_count
            );
        }
        Ok(())
    }
}

fn media_show(args: &MediaAssetArgs, json: bool) -> Result<()> {
    let project = SqliteProject::open(&args.project).context("open project")?;
    let asset_id = parse_asset_id(&args.asset_id)?;
    let asset = find_asset(&project, asset_id)?;
    let view = asset_view(&project, &asset)?;

    if json {
        print_json(&view)
    } else {
        println!(
            "asset {} ({})",
            view.id,
            view.display_name.as_deref().unwrap_or("unnamed")
        );
        for representation in &view.representations {
            println!(
                "  {} {} ({}): {} resource(s)",
                representation.kind,
                representation.id,
                representation.structure,
                representation.resources.len()
            );
            for resource in &representation.resources {
                for locator in &resource.locators {
                    println!("    {} [{}]", locator.uri, locator.availability);
                }
            }
        }
        Ok(())
    }
}

fn root_add(args: RootAddArgs, json: bool) -> Result<()> {
    let root = prepare_media_root(&args.directory, args.label, args.priority)
        .context("prepare media root")?;
    let view = RootView {
        id: root.id().to_string(),
        uri: root.uri().to_owned(),
        label: root.label().map(str::to_owned),
        priority: root.priority(),
        enabled: root.is_enabled(),
    };
    let mut project = SqliteProject::open(&args.project).context("open project")?;
    let mut transaction = project
        .begin_transaction()
        .context("begin root transaction")?;
    transaction
        .add_media_root(root)
        .context("stage media root")?;
    transaction.commit().context("commit media root")?;

    if json {
        print_json(&view)
    } else {
        println!("added media root {} ({})", view.uri, view.id);
        Ok(())
    }
}

fn identifier_mutate(args: IdentifierMutationArgs, remove: bool, json: bool) -> Result<()> {
    let target = parse_identifier_target(args.target.target_kind, &args.target.target_id)?;
    let scheme = IdentifierScheme::new(args.scheme).context("validate identifier scheme")?;
    let identifier = ExternalIdentifier::new(scheme, args.value, args.qualifier)
        .context("validate external identifier")?;
    let view = external_identifier_view(target, &identifier);
    let mut project = SqliteProject::open(&args.target.project).context("open project")?;
    let mut transaction = project
        .begin_transaction()
        .context("begin identifier transaction")?;
    if remove {
        transaction
            .remove_external_identifier(target, &identifier)
            .context("stage external identifier removal")?;
    } else {
        transaction
            .add_external_identifier(target, &identifier)
            .context("stage external identifier attachment")?;
    }
    transaction
        .commit()
        .context("commit external identifier mutation")?;

    if json {
        print_json(&view)
    } else {
        println!(
            "{} {}:{} on {} {}",
            if remove { "removed" } else { "attached" },
            view.scheme,
            view.value,
            view.target_kind,
            view.target_id
        );
        Ok(())
    }
}

fn identifier_list(args: &IdentifierTargetArgs, json: bool) -> Result<()> {
    let target = parse_identifier_target(args.target_kind, &args.target_id)?;
    let project = SqliteProject::open(&args.project).context("open project")?;
    let views: Vec<_> = project
        .external_identifiers(target)
        .context("load external identifiers")?
        .iter()
        .map(|identifier| external_identifier_view(target, identifier))
        .collect();

    if json {
        print_json(&views)
    } else {
        for view in views {
            println!(
                "{}\t{}\t{}",
                view.scheme,
                view.value,
                view.qualifier.as_deref().unwrap_or("-")
            );
        }
        Ok(())
    }
}

fn identifier_find(args: IdentifierFindArgs, json: bool) -> Result<()> {
    let scheme = IdentifierScheme::new(args.scheme).context("validate identifier scheme")?;
    let project = SqliteProject::open(&args.project).context("open project")?;
    let views: Vec<_> = project
        .find_by_external_identifier(&scheme, &args.value)
        .context("find external identifier")?
        .into_iter()
        .map(object_ref_view)
        .collect::<Result<_>>()?;

    if json {
        print_json(&views)
    } else {
        for view in views {
            println!("{}\t{}", view.kind, view.id);
        }
        Ok(())
    }
}

fn metadata_add_text(args: MetadataAddTextArgs, json: bool) -> Result<()> {
    let target = parse_metadata_target(args.target.target_kind, &args.target.target_id)?;
    let property = parse_metadata_property(args.vocabulary, args.property)?;
    let value = match args.language {
        Some(language) => MetadataValue::language_string(args.value, language),
        None => MetadataValue::string(args.value),
    }
    .context("validate metadata text")?;
    let assertion = MetadataAssertion::new(property.clone(), value.clone());
    let view = metadata_assertion_view(target, &assertion)?;
    let mut project = SqliteProject::open(&args.target.project).context("open project")?;
    let mut transaction = project
        .begin_transaction()
        .context("begin metadata transaction")?;
    transaction
        .add_metadata_value(target, &property, &value)
        .context("stage metadata value")?;
    transaction.commit().context("commit metadata value")?;

    if json {
        print_json(&view)
    } else {
        println!(
            "added {}:{} to {} {}",
            view.vocabulary, view.property, view.target_kind, view.target_id
        );
        Ok(())
    }
}

fn metadata_list(args: &MetadataTargetArgs, json: bool) -> Result<()> {
    let target = parse_metadata_target(args.target_kind, &args.target_id)?;
    let project = SqliteProject::open(&args.project).context("open project")?;
    let views = project
        .metadata(target)
        .context("load metadata")?
        .iter()
        .map(|assertion| metadata_assertion_view(target, assertion))
        .collect::<Result<Vec<_>>>()?;

    print_metadata_assertions(&views, json)
}

fn metadata_remove(args: MetadataPropertyArgs, json: bool) -> Result<()> {
    let target = parse_metadata_target(args.target.target_kind, &args.target.target_id)?;
    let property = parse_metadata_property(args.vocabulary, args.property)?;
    let target_view = object_ref_view(target)?;
    let view = MetadataPropertyView {
        target_kind: target_view.kind,
        target_id: target_view.id,
        vocabulary: property.vocabulary().as_str().to_owned(),
        property: property.property().as_str().to_owned(),
    };
    let mut project = SqliteProject::open(&args.target.project).context("open project")?;
    let mut transaction = project
        .begin_transaction()
        .context("begin metadata transaction")?;
    transaction
        .remove_metadata_property(target, &property)
        .context("stage metadata property removal")?;
    transaction
        .commit()
        .context("commit metadata property removal")?;

    if json {
        print_json(&view)
    } else {
        println!(
            "removed {}:{} from {} {}",
            view.vocabulary, view.property, view.target_kind, view.target_id
        );
        Ok(())
    }
}

fn metadata_find(args: MetadataFindArgs, json: bool) -> Result<()> {
    let property = parse_metadata_property(args.vocabulary, args.property)?;
    let project = SqliteProject::open(&args.project).context("open project")?;
    let views = project
        .query_by_metadata_property(&property)
        .context("query metadata property")?
        .iter()
        .map(|matched| metadata_assertion_view(matched.target(), matched.assertion()))
        .collect::<Result<Vec<_>>>()?;

    print_metadata_assertions(&views, json)
}

fn print_metadata_assertions(views: &[MetadataAssertionView], json: bool) -> Result<()> {
    if json {
        print_json(&views)
    } else {
        for view in views {
            let value = serde_json::to_string(&view.value).context("format metadata value")?;
            println!(
                "{}\t{}:{}\t{}",
                view.target_id, view.vocabulary, view.property, value
            );
        }
        Ok(())
    }
}

fn media_resolve(args: MediaResolveArgs, json: bool) -> Result<()> {
    let mut project = SqliteProject::open(&args.project).context("open project")?;
    let asset_id = parse_asset_id(&args.asset_id)?;
    find_asset(&project, asset_id)?;
    let representations = project
        .representations(asset_id)
        .context("load asset representations")?;
    let resolver = MediaResolver::default();
    let mut resolutions = Vec::new();
    for representation in &representations {
        let resources = project
            .resources(representation.id())
            .context("load representation resources")?;
        if resources.len() != 1 {
            bail!("compound representation resolution is not exposed by this command yet");
        }
        let resource = &resources[0];
        let locators = project
            .locators(resource.id())
            .context("load resource locators")?;
        let resolution = resolver
            .resolve(
                representation.id(),
                resource,
                &locators,
                project.project().media_roots(),
            )
            .context("resolve representation resource")?;
        resolutions.push((resource.id(), resolution));
    }

    if let Some(uri) = args.confirm.as_deref() {
        let matching: Vec<_> = resolutions
            .iter()
            .flat_map(|(resource_id, resolution)| {
                resolution
                    .candidates()
                    .iter()
                    .filter(move |candidate| candidate.uri() == uri)
                    .map(move |_| *resource_id)
            })
            .collect();
        if matching.len() != 1 {
            bail!("confirmation URI must identify exactly one candidate from this resolution");
        }
        let locator = prepare_confirmed_locator(matching[0], uri.to_owned())
            .context("prepare confirmed locator")?;
        let mut transaction = project
            .begin_transaction()
            .context("begin confirmation transaction")?;
        transaction
            .add_locator(&locator)
            .context("stage confirmed locator")?;
        transaction.commit().context("commit confirmed locator")?;
    }

    let view = ResolveView {
        asset_id: asset_id.to_string(),
        resolutions: resolutions
            .iter()
            .map(|(_, resolution)| ResolutionView::from(resolution))
            .collect(),
        confirmed_uri: args.confirm,
    };
    if json {
        print_json(&view)
    } else {
        for resolution in &view.resolutions {
            println!("{}: {}", resolution.representation_id, resolution.state);
            for candidate in &resolution.candidates {
                println!(
                    "  {} ({} bp)",
                    candidate.uri, candidate.confidence_basis_points
                );
            }
        }
        if let Some(uri) = &view.confirmed_uri {
            println!("confirmed {uri}");
        }
        Ok(())
    }
}

fn parse_asset_id(value: &str) -> Result<AssetId> {
    AssetId::from_str(value).context("parse asset ID")
}

fn parse_identifier_target(kind: IdentifierTargetKind, value: &str) -> Result<ObjectRef> {
    match kind {
        IdentifierTargetKind::Asset => AssetId::from_str(value)
            .map(ObjectRef::Asset)
            .context("parse asset ID"),
        IdentifierTargetKind::Representation => RepresentationId::from_str(value)
            .map(ObjectRef::Representation)
            .context("parse representation ID"),
    }
}

fn parse_metadata_target(kind: MetadataTargetKind, value: &str) -> Result<ObjectRef> {
    match kind {
        MetadataTargetKind::Project => ProjectId::from_str(value)
            .map(ObjectRef::Project)
            .context("parse project ID"),
        MetadataTargetKind::Asset => AssetId::from_str(value)
            .map(ObjectRef::Asset)
            .context("parse asset ID"),
        MetadataTargetKind::Representation => RepresentationId::from_str(value)
            .map(ObjectRef::Representation)
            .context("parse representation ID"),
    }
}

fn parse_metadata_property(vocabulary: String, property: String) -> Result<MetadataProperty> {
    Ok(MetadataProperty::new(
        VocabularyId::new(vocabulary).context("validate metadata vocabulary")?,
        PropertyId::new(property).context("validate metadata property")?,
    ))
}

fn external_identifier_view(
    target: ObjectRef,
    identifier: &ExternalIdentifier,
) -> ExternalIdentifierView {
    let target = object_ref_view(target).expect("supported CLI target");
    ExternalIdentifierView {
        target_kind: target.kind,
        target_id: target.id,
        scheme: identifier.scheme().as_str().to_owned(),
        value: identifier.value().to_owned(),
        qualifier: identifier.qualifier().map(str::to_owned),
    }
}

fn object_ref_view(target: ObjectRef) -> Result<ObjectRefView> {
    match target {
        ObjectRef::Project(id) => Ok(ObjectRefView {
            kind: "project",
            id: id.to_string(),
        }),
        ObjectRef::Asset(id) => Ok(ObjectRefView {
            kind: "asset",
            id: id.to_string(),
        }),
        ObjectRef::Representation(id) => Ok(ObjectRefView {
            kind: "representation",
            id: id.to_string(),
        }),
        ObjectRef::Activity(id) => Ok(ObjectRefView {
            kind: "activity",
            id: id.to_string(),
        }),
        _ => bail!("object kind is not supported by this CLI"),
    }
}

fn metadata_assertion_view(
    target: ObjectRef,
    assertion: &MetadataAssertion,
) -> Result<MetadataAssertionView> {
    let target = object_ref_view(target)?;
    Ok(MetadataAssertionView {
        target_kind: target.kind,
        target_id: target.id,
        vocabulary: assertion.property().vocabulary().as_str().to_owned(),
        property: assertion.property().property().as_str().to_owned(),
        value: metadata_value_view(assertion.value())?,
    })
}

fn metadata_value_view(value: &MetadataValue) -> Result<MetadataValueView> {
    match value.kind() {
        MetadataValueKind::String => Ok(MetadataValueView::String {
            value: value
                .as_string()
                .context("metadata string has wrong internal type")?
                .to_owned(),
        }),
        MetadataValueKind::LangString => {
            let (text, language) = value
                .as_language_string()
                .context("metadata language string has wrong internal type")?;
            Ok(MetadataValueView::LangString {
                value: text.to_owned(),
                language: language.to_owned(),
            })
        }
        MetadataValueKind::I64 => Ok(MetadataValueView::I64 {
            value: value
                .as_i64()
                .context("metadata integer has wrong internal type")?,
        }),
        MetadataValueKind::U64 => Ok(MetadataValueView::U64 {
            value: value
                .as_u64()
                .context("metadata unsigned integer has wrong internal type")?,
        }),
        MetadataValueKind::Decimal => {
            let decimal = value
                .as_decimal()
                .context("metadata decimal has wrong internal type")?;
            Ok(MetadataValueView::Decimal {
                coefficient: decimal.coefficient().to_string(),
                scale: decimal.scale(),
            })
        }
        MetadataValueKind::Bool => Ok(MetadataValueView::Bool {
            value: value
                .as_bool()
                .context("metadata boolean has wrong internal type")?,
        }),
        MetadataValueKind::Timestamp => Ok(MetadataValueView::Timestamp {
            unix_micros: value
                .as_timestamp()
                .context("metadata timestamp has wrong internal type")?
                .as_unix_micros(),
        }),
        MetadataValueKind::Uri => Ok(MetadataValueView::Uri {
            value: value
                .as_uri()
                .context("metadata URI has wrong internal type")?
                .to_owned(),
        }),
        MetadataValueKind::Bytes => Ok(MetadataValueView::Bytes {
            hex: hex::encode(
                value
                    .as_bytes()
                    .context("metadata bytes have wrong internal type")?,
            ),
        }),
        MetadataValueKind::Rational => {
            let rational = value
                .as_rational()
                .context("metadata rational has wrong internal type")?;
            Ok(MetadataValueView::Rational {
                numerator: rational.numerator(),
                denominator: rational.denominator(),
            })
        }
        MetadataValueKind::List => Ok(MetadataValueView::List {
            values: value
                .as_list()
                .context("metadata list has wrong internal type")?
                .iter()
                .map(metadata_value_view)
                .collect::<Result<_>>()?,
        }),
        MetadataValueKind::Struct => Ok(MetadataValueView::Struct {
            fields: value
                .as_structure()
                .context("metadata structure has wrong internal type")?
                .iter()
                .map(metadata_field_view)
                .collect::<Result<_>>()?,
        }),
        MetadataValueKind::Reference => Ok(MetadataValueView::Reference {
            target: object_ref_view(
                value
                    .as_reference()
                    .context("metadata reference has wrong internal type")?,
            )?,
        }),
        _ => bail!("metadata value kind is not supported by this CLI"),
    }
}

fn metadata_field_view(field: &MetadataField) -> Result<MetadataFieldView> {
    Ok(MetadataFieldView {
        name: field.name().as_str().to_owned(),
        value: metadata_value_view(field.value())?,
    })
}

fn find_asset(project: &SqliteProject, asset_id: AssetId) -> Result<Asset> {
    project
        .assets()
        .context("load assets")?
        .into_iter()
        .find(|asset| asset.id() == asset_id)
        .with_context(|| format!("asset does not exist: {asset_id}"))
}

fn asset_view(project: &SqliteProject, asset: &Asset) -> Result<AssetView> {
    let mut representations = Vec::new();
    for representation in project
        .representations(asset.id())
        .context("load asset representations")?
    {
        let mut resources = Vec::new();
        for resource in project
            .resources(representation.id())
            .context("load representation resources")?
        {
            let locators = project
                .locators(resource.id())
                .context("load resource locators")?;
            resources.push(resource_view(&resource, &locators));
        }
        representations.push(representation_view(&representation, resources));
    }
    Ok(AssetView {
        id: asset.id().to_string(),
        display_name: asset.display_name().map(str::to_owned),
        import_source: asset.import_source().map(str::to_owned),
        created_at_unix_micros: asset.created_at().as_unix_micros(),
        representations,
    })
}

fn representation_view(
    representation: &Representation,
    resources: Vec<ResourceView>,
) -> RepresentationView {
    RepresentationView {
        id: representation.id().to_string(),
        kind: representation_kind(representation.kind()),
        structure: content_structure_kind(representation.content_structure().kind()),
        resources,
    }
}

fn resource_view(resource: &Resource, locators: &[Locator]) -> ResourceView {
    ResourceView {
        id: resource.id().to_string(),
        fingerprints: resource
            .fingerprints()
            .iter()
            .map(|fingerprint| FingerprintView {
                algorithm: fingerprint.algorithm().to_owned(),
                version: fingerprint.version(),
                value_hex: hex::encode(fingerprint.value()),
            })
            .collect(),
        file_size_bytes: resource
            .file_facts()
            .map(postproject_core::FileFacts::size_bytes),
        locators: locators.iter().map(LocatorView::from).collect(),
    }
}

impl From<&Locator> for LocatorView {
    fn from(locator: &Locator) -> Self {
        Self {
            id: locator.id().to_string(),
            uri: locator.uri().to_owned(),
            availability: locator_availability(locator.availability()),
            last_seen_unix_micros: locator
                .last_seen()
                .map(postproject_core::Timestamp::as_unix_micros),
        }
    }
}

impl From<&Resolution> for ResolutionView {
    fn from(resolution: &Resolution) -> Self {
        Self {
            representation_id: resolution.representation_id().to_string(),
            state: resolution_state(resolution.state()),
            candidates: resolution
                .candidates()
                .iter()
                .map(|candidate| CandidateView {
                    uri: candidate.uri().to_owned(),
                    confidence_basis_points: candidate.confidence().basis_points(),
                    evidence: candidate
                        .evidence()
                        .iter()
                        .map(EvidenceView::from)
                        .collect(),
                })
                .collect(),
            evidence: resolution
                .evidence()
                .iter()
                .map(EvidenceView::from)
                .collect(),
        }
    }
}

impl From<&ResolutionEvidence> for EvidenceView {
    fn from(evidence: &ResolutionEvidence) -> Self {
        Self {
            kind: evidence_kind(evidence.kind()),
            detail: evidence.detail().map(str::to_owned),
        }
    }
}

fn print_json(value: &impl Serialize) -> Result<()> {
    serde_json::to_writer_pretty(std::io::stdout().lock(), value).context("write JSON output")?;
    println!();
    Ok(())
}

const fn representation_kind(kind: RepresentationKind) -> &'static str {
    match kind {
        RepresentationKind::Original => "original",
        RepresentationKind::Proxy => "proxy",
        RepresentationKind::Optimized => "optimized",
        RepresentationKind::Derived => "derived",
        _ => "unknown",
    }
}

const fn content_structure_kind(kind: postproject_core::ContentStructureKind) -> &'static str {
    match kind {
        postproject_core::ContentStructureKind::SingleResource => "single_resource",
        postproject_core::ContentStructureKind::ImageSequence => "image_sequence",
        postproject_core::ContentStructureKind::OrderedParts => "ordered_parts",
        postproject_core::ContentStructureKind::Package => "package",
        _ => "unknown",
    }
}

const fn locator_availability(availability: LocatorAvailability) -> &'static str {
    match availability {
        LocatorAvailability::Online => "online",
        LocatorAvailability::Offline => "offline",
        _ => "unknown",
    }
}

const fn resolution_state(state: ResolutionState) -> &'static str {
    match state {
        ResolutionState::OnlineAtKnownLocation => "online_at_known_location",
        ResolutionState::ResolvedExact => "resolved_exact",
        ResolutionState::ResolvedProbable => "resolved_probable",
        ResolutionState::Missing => "missing",
        ResolutionState::Ambiguous => "ambiguous",
        ResolutionState::Error => "error",
        _ => "unknown",
    }
}

const fn evidence_kind(kind: EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::KnownLocationExists => "known_location_exists",
        EvidenceKind::ExactFingerprintMatch => "exact_fingerprint_match",
        EvidenceKind::FullHashMatch => "full_hash_match",
        EvidenceKind::PartialFingerprintMatch => "partial_fingerprint_match",
        EvidenceKind::FileSizeMatch => "file_size_match",
        EvidenceKind::FileNameMatch => "file_name_match",
        EvidenceKind::RelativePathSimilarity => "relative_path_similarity",
        EvidenceKind::MediaRootRelation => "media_root_relation",
        EvidenceKind::ConflictingCandidate => "conflicting_candidate",
        EvidenceKind::DiscoveryError => "discovery_error",
        _ => "unknown",
    }
}
