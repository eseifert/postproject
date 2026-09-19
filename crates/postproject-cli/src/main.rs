//! Command-line demonstrator for `PostProject` domain services.

#![forbid(unsafe_code)]

use std::{path::PathBuf, process::ExitCode, str::FromStr};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use postproject_core::{
    Asset, AssetId, EvidenceKind, ExternalIdentifier, IdentifierScheme, Location,
    LocationAvailability, ObjectRef, Representation, RepresentationId, RepresentationKind,
    Resolution, ResolutionEvidence, ResolutionState,
};
use postproject_media::{
    MediaResolver, prepare_confirmed_location, prepare_media_root, prepare_original_media,
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
    /// Show an asset, its representations, and locations.
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
    location_id: String,
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
    fingerprint: Option<FingerprintView>,
    file_size_bytes: Option<u64>,
    locations: Vec<LocationView>,
}

#[derive(Debug, Serialize)]
struct FingerprintView {
    algorithm: String,
    version: u16,
    value_hex: String,
}

#[derive(Debug, Serialize)]
struct LocationView {
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
        location_id: prepared.location().id().to_string(),
        uri: prepared.location().uri().to_owned(),
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
                "  {} {}: {} location(s)",
                representation.kind,
                representation.id,
                representation.locations.len()
            );
            for location in &representation.locations {
                println!("    {} [{}]", location.uri, location.availability);
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
        let locations = project
            .locations(representation.id())
            .context("load representation locations")?;
        resolutions.push(
            resolver
                .resolve(representation, &locations, project.project().media_roots())
                .context("resolve representation")?,
        );
    }

    if let Some(uri) = args.confirm.as_deref() {
        let matching: Vec<_> = resolutions
            .iter()
            .flat_map(|resolution| {
                resolution
                    .candidates()
                    .iter()
                    .filter(move |candidate| candidate.uri() == uri)
                    .map(move |_| resolution.representation_id())
            })
            .collect();
        if matching.len() != 1 {
            bail!("confirmation URI must identify exactly one candidate from this resolution");
        }
        let location = prepare_confirmed_location(matching[0], uri.to_owned())
            .context("prepare confirmed location")?;
        let mut transaction = project
            .begin_transaction()
            .context("begin confirmation transaction")?;
        transaction
            .add_location(&location)
            .context("stage confirmed location")?;
        transaction.commit().context("commit confirmed location")?;
    }

    let view = ResolveView {
        asset_id: asset_id.to_string(),
        resolutions: resolutions.iter().map(ResolutionView::from).collect(),
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
        let locations = project
            .locations(representation.id())
            .context("load representation locations")?;
        representations.push(representation_view(&representation, &locations));
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
    locations: &[Location],
) -> RepresentationView {
    RepresentationView {
        id: representation.id().to_string(),
        kind: representation_kind(representation.kind()),
        fingerprint: representation
            .fingerprint()
            .map(|fingerprint| FingerprintView {
                algorithm: fingerprint.algorithm().to_owned(),
                version: fingerprint.version(),
                value_hex: hex::encode(fingerprint.value()),
            }),
        file_size_bytes: representation
            .file_facts()
            .map(postproject_core::FileFacts::size_bytes),
        locations: locations.iter().map(LocationView::from).collect(),
    }
}

impl From<&Location> for LocationView {
    fn from(location: &Location) -> Self {
        Self {
            id: location.id().to_string(),
            uri: location.uri().to_owned(),
            availability: location_availability(location.availability()),
            last_seen_unix_micros: location
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

const fn location_availability(availability: LocationAvailability) -> &'static str {
    match availability {
        LocationAvailability::Online => "online",
        LocationAvailability::Offline => "offline",
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
