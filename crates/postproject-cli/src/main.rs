//! Command-line demonstrator for `PostProject` domain services.

#![forbid(unsafe_code)]

use std::{fs, path::PathBuf, process::ExitCode, str::FromStr};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use postproject_core::{
    Activity, ActivityId, ActivityInput, ActivityKind, ActivityOutput, ActivityRole, AgentIdentity,
    ArtifactEdgeKind, ArtifactEvaluationLimits, ArtifactKnowledgeReason, ArtifactKnowledgeState,
    ArtifactTraversalLimitKind, Asset, AssetId, AvailabilityIssue, AvailabilityIssueKind,
    DecimalValue, EvidenceKind, ExternalIdentifier, FrameRange, IdentifierScheme,
    ImageSequencePattern, Locator, LocatorAvailability, LocatorId, MediaRoot, MediaRootId,
    MetadataAssertion, MetadataField, MetadataProperty, MetadataValue, MetadataValueKind,
    ObjectRef, OriginIdentity, OriginalMediaImport, ProductionId, ProductionStoreTransaction,
    PropertyId, RationalRate, RationalValue, Representation, RepresentationAvailability,
    RepresentationId, RepresentationKind, RepresentationResolution, ResolutionEvidence, Resource,
    ResourceId, ResourceResolution, ResourceResolutionState, ResourceRole, Revision,
    RevisionContext, RevisionEvent, RevisionEventKind, RevisionId, Timestamp, ToolIdentity,
    VocabularyId,
};
use postproject_media::{
    FfprobeInspector, FileResourceSource, ImageSequenceSource, InspectionOutcome,
    InventoryCategory, InventoryReport, InventoryScanner, MediaInspector, MediaRecognizer,
    MediaResolver, MediaRootMapping, RecognizedMedia, TechnicalMetadata, VerificationMode,
    fingerprint_file, fingerprint_representation, prepare_confirmed_locator,
    prepare_image_sequence_representation, prepare_ordered_parts_representation,
    prepare_original_media, prepare_package_representation, prepare_recognized_original_media,
    prepare_single_file_representation,
};
use postproject_storage_sqlite::SqliteProduction;
use serde::{Deserialize, Serialize};

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
    /// Create a production file.
    Init(InitArgs),
    /// Inspect and manage production media.
    Media(MediaArgs),
    /// Add representations to existing assets.
    Representation(RepresentationArgs),
    /// Manage resolver search roots.
    Root(RootArgs),
    /// Manage resource locators.
    Locator(LocatorArgs),
    /// Manage external industry, vendor, and application identifiers.
    Identifier(IdentifierArgs),
    /// Inspect and manage standards-aware metadata assertions.
    Metadata(MetadataArgs),
    /// Inspect production provenance activities.
    Activity(ActivityArgs),
    /// Evaluate managed artifacts from recorded production knowledge.
    Artifact(ArtifactArgs),
    /// Inspect the durable semantic change journal.
    Revisions(RevisionsArgs),
}

#[derive(Debug, Args)]
struct InitArgs {
    /// Production file to create.
    production: PathBuf,
    /// Optional production display name.
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
    List(ProductionArgs),
    /// Show an asset, its representations, resources, and locators.
    Show(MediaAssetArgs),
    /// Resolve an asset under configured media roots.
    Resolve(MediaResolveArgs),
    /// Inventory known and unassociated media without changing the production.
    Inventory(MediaInventoryArgs),
    /// Record a freshly computed resource and representation fingerprint.
    Fingerprint(MediaFingerprintArgs),
}

#[derive(Debug, Args)]
struct MediaAddArgs {
    production: PathBuf,
    path: PathBuf,
    /// Optional asset display name.
    #[arg(long)]
    name: Option<String>,
    /// Exact rate for a recognized image sequence (NUMERATOR/DENOMINATOR).
    #[arg(long)]
    sequence_rate: Option<RationalRate>,
    /// Recognize same-stem metadata sidecars beside a regular file.
    #[arg(long)]
    recognize_companions: bool,
    /// Inspect imported media with ffprobe and record technical metadata.
    #[arg(long)]
    inspect: bool,
    /// ffprobe executable used with --inspect.
    #[arg(long, default_value = "ffprobe", requires = "inspect")]
    ffprobe: PathBuf,
}

#[derive(Debug, Args)]
struct ProductionArgs {
    production: PathBuf,
}

#[derive(Debug, Args)]
struct MediaAssetArgs {
    production: PathBuf,
    asset_id: String,
}

#[derive(Debug, Args)]
struct MediaResolveArgs {
    production: PathBuf,
    asset_id: String,
    /// Confirm one URI returned by this resolution and persist it.
    #[arg(long, value_name = "URI")]
    confirm: Option<String>,
    /// Map a production root name to this machine's directory (NAME=PATH).
    #[arg(long = "root-map", value_name = "NAME=PATH")]
    root_mappings: Vec<RootMappingArg>,
    /// Recompute stored fingerprints for content at known locators.
    #[arg(long)]
    verify: bool,
    /// ffprobe executable used to compare persisted technical metadata.
    #[arg(long, default_value = "ffprobe", requires = "verify")]
    ffprobe: PathBuf,
}

#[derive(Debug, Args)]
struct MediaInventoryArgs {
    production: PathBuf,
    /// Map a production root name to this machine's directory (NAME=PATH).
    #[arg(long = "root-map", value_name = "NAME=PATH")]
    root_mappings: Vec<RootMappingArg>,
    /// Machine-local disposable sidecar cache.
    #[arg(long, value_name = "PATH")]
    cache: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct MediaFingerprintArgs {
    production: PathBuf,
    asset_id: String,
    representation_id: String,
    resource_id: String,
    /// Regular file whose content now realizes the resource.
    path: PathBuf,
}

#[derive(Debug, Args)]
struct RepresentationArgs {
    #[command(subcommand)]
    command: RepresentationCommand,
}

#[derive(Debug, Subcommand)]
enum RepresentationCommand {
    /// Add a representation described by a JSON specification.
    Add(RepresentationAddArgs),
}

#[derive(Debug, Args)]
struct RepresentationAddArgs {
    production: PathBuf,
    asset_id: String,
    #[arg(value_enum)]
    kind: RepresentationKindArg,
    /// JSON file containing a tagged representation source.
    spec_file: PathBuf,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum RepresentationKindArg {
    Original,
    Proxy,
    Optimized,
    Derived,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "structure", rename_all = "snake_case")]
enum RepresentationSourceSpec {
    SingleFile {
        path: PathBuf,
    },
    ImageSequence {
        directory: PathBuf,
        prefix: String,
        suffix: String,
        padding: u8,
        start: i64,
        end: i64,
        step: u32,
        rate_numerator: u32,
        rate_denominator: u32,
        #[serde(default)]
        missing_frames: Vec<i64>,
    },
    OrderedParts {
        members: Vec<FileResourceSpec>,
    },
    Package {
        members: Vec<FileResourceSpec>,
    },
}

#[derive(Debug, Deserialize)]
struct FileResourceSpec {
    path: PathBuf,
    role: String,
    #[serde(default = "default_required")]
    required: bool,
}

const fn default_required() -> bool {
    true
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
    /// List configured media roots in resolver order.
    List(ProductionArgs),
    /// Include a media root in resolution.
    Enable(RootMutationArgs),
    /// Exclude a media root from resolution without removing it.
    Disable(RootMutationArgs),
    /// Remove a configured media root.
    Remove(RootMutationArgs),
}

#[derive(Debug, Args)]
struct RootAddArgs {
    production: PathBuf,
    name: String,
    #[arg(long)]
    label: Option<String>,
    /// Lower priorities are searched first.
    #[arg(long, default_value_t = 0)]
    priority: i32,
}

#[derive(Debug, Args)]
struct RootMutationArgs {
    production: PathBuf,
    root_id: String,
}

#[derive(Clone, Debug)]
struct RootMappingArg {
    name: String,
    directory: PathBuf,
}

impl FromStr for RootMappingArg {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        let (name, directory) = value
            .split_once('=')
            .ok_or_else(|| "root mapping must use NAME=PATH".to_owned())?;
        if name.is_empty() || directory.is_empty() {
            return Err("root mapping name and path must not be empty".to_owned());
        }
        Ok(Self {
            name: name.to_owned(),
            directory: PathBuf::from(directory),
        })
    }
}

#[derive(Debug, Args)]
struct LocatorArgs {
    #[command(subcommand)]
    command: LocatorCommand,
}

#[derive(Debug, Subcommand)]
enum LocatorCommand {
    /// Retire a locator that no longer identifies a useful access route.
    Retire(LocatorRetireArgs),
}

#[derive(Debug, Args)]
struct LocatorRetireArgs {
    production: PathBuf,
    locator_id: String,
}

#[derive(Debug, Args)]
struct IdentifierArgs {
    #[command(subcommand)]
    command: IdentifierCommand,
}

#[derive(Debug, Subcommand)]
enum IdentifierCommand {
    /// Attach an external identifier to an asset, representation, or resource.
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
    Resource,
}

#[derive(Debug, Args)]
struct IdentifierTargetArgs {
    production: PathBuf,
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
    production: PathBuf,
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
    /// Append a typed value read from a JSON file.
    Add(MetadataAddArgs),
    /// Append a plain or language-tagged text value.
    AddText(MetadataAddTextArgs),
    /// List all metadata assertions attached to an object.
    List(MetadataTargetArgs),
    /// Remove every value of one property from an object.
    Remove(MetadataPropertyArgs),
    /// Find assertions using an exact vocabulary and property.
    Find(MetadataFindArgs),
}

#[derive(Clone, Copy, Debug, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum MetadataTargetKind {
    Production,
    Asset,
    Representation,
    Resource,
    Activity,
}

#[derive(Debug, Args)]
struct MetadataTargetArgs {
    production: PathBuf,
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
struct MetadataAddArgs {
    #[command(flatten)]
    target: MetadataTargetArgs,
    vocabulary: String,
    property: String,
    /// JSON file containing one tagged metadata value.
    value_file: PathBuf,
}

#[derive(Debug, Args)]
struct MetadataFindArgs {
    production: PathBuf,
    vocabulary: String,
    property: String,
}

#[derive(Debug, Args)]
struct ActivityArgs {
    #[command(subcommand)]
    command: ActivityCommand,
}

#[derive(Debug, Subcommand)]
enum ActivityCommand {
    /// Record a completed production activity.
    Add(Box<ActivityAddArgs>),
    /// List production activities with their inputs and outputs.
    List(ProductionArgs),
    /// List activities that produced a representation.
    Producing(ActivityRepresentationArgs),
    /// List activities that consume a representation.
    Consuming(ActivityRepresentationArgs),
    /// List every transitive provenance ancestor of a representation.
    Ancestors(ActivityRepresentationArgs),
    /// List every transitive provenance descendant of a representation.
    Descendants(ActivityRepresentationArgs),
}

#[derive(Debug, Args)]
struct ActivityAddArgs {
    production: PathBuf,
    /// Namespaced activity kind, such as `org.postproject:transcode`.
    kind: String,
    /// Consumed representation, optionally followed by `=ROLE`.
    #[arg(long = "input", value_name = "REPRESENTATION_ID[=ROLE]")]
    inputs: Vec<ActivityEdgeArg>,
    /// Produced representation, optionally followed by `=ROLE`.
    #[arg(
        long = "output",
        value_name = "REPRESENTATION_ID[=ROLE]",
        required = true
    )]
    outputs: Vec<ActivityEdgeArg>,
    /// Optional activity start as Unix microseconds.
    #[arg(long)]
    started_at_unix_micros: Option<i64>,
    /// Optional activity finish as Unix microseconds.
    #[arg(long)]
    finished_at_unix_micros: Option<i64>,
    #[arg(long)]
    tool_name: Option<String>,
    #[arg(long)]
    tool_version: Option<String>,
    #[arg(long)]
    tool_uri: Option<String>,
    #[arg(long)]
    agent_name: Option<String>,
    #[arg(long)]
    agent_identifier_scheme: Option<String>,
    #[arg(long)]
    agent_identifier_value: Option<String>,
    #[arg(long)]
    agent_identifier_qualifier: Option<String>,
}

#[derive(Debug, Args)]
struct ActivityRepresentationArgs {
    production: PathBuf,
    representation_id: String,
}

#[derive(Debug, Args)]
struct ArtifactArgs {
    #[command(subcommand)]
    command: ArtifactCommand,
}

#[derive(Debug, Subcommand)]
enum ArtifactCommand {
    /// Evaluate whether an activity-produced representation is current.
    Evaluate(ArtifactEvaluateArgs),
}

#[derive(Debug, Args)]
struct ArtifactEvaluateArgs {
    production: PathBuf,
    representation_id: String,
    /// Maximum number of upstream activity edges to follow.
    #[arg(long, default_value_t = 64)]
    max_depth: u32,
    /// Maximum number of distinct representations to inspect.
    #[arg(long, default_value_t = 1_000)]
    max_representations: u32,
}

#[derive(Debug, Args)]
struct RevisionsArgs {
    #[command(subcommand)]
    command: RevisionsCommand,
}

#[derive(Debug, Subcommand)]
enum RevisionsCommand {
    /// Show the newest committed revision.
    Latest(ProductionArgs),
    /// List revisions after a production-local sequence cursor.
    Since(RevisionsSinceArgs),
    /// List the ordered semantic events belonging to one revision.
    Events(RevisionEventsArgs),
}

#[derive(Debug, Args)]
struct RevisionsSinceArgs {
    production: PathBuf,
    /// Return revisions with a sequence greater than this cursor.
    #[arg(long, default_value_t = 0)]
    after: u64,
    /// Maximum number of revisions to return.
    #[arg(long, default_value_t = 100)]
    limit: u32,
}

#[derive(Debug, Args)]
struct RevisionEventsArgs {
    production: PathBuf,
    revision_id: String,
}

#[derive(Clone, Debug)]
struct ActivityEdgeArg {
    representation_id: RepresentationId,
    role: Option<ActivityRole>,
}

impl FromStr for ActivityEdgeArg {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        let (representation_id, role) = value
            .split_once('=')
            .map_or((value, None), |(id, role)| (id, Some(role)));
        Ok(Self {
            representation_id: RepresentationId::from_str(representation_id)
                .map_err(|error| format!("invalid representation ID: {error}"))?,
            role: role
                .map(ActivityRole::new)
                .transpose()
                .map_err(|error| format!("invalid activity role: {error}"))?,
        })
    }
}

#[derive(Debug, Serialize)]
struct ProductionView {
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
    resource_count: usize,
    inspections: Vec<InspectionView>,
}

#[derive(Debug, Serialize)]
struct InspectionView {
    path: String,
    status: &'static str,
    reason: Option<String>,
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
struct FingerprintObservationView {
    representation_id: String,
    resource_id: String,
    resource_fingerprint: FingerprintView,
    representation_fingerprint: FingerprintView,
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
    name: String,
    label: Option<String>,
    legacy_uri: Option<String>,
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
struct InventoryView {
    items: Vec<InventoryItemView>,
    stats: InventoryStatsView,
}

#[derive(Debug, Serialize)]
struct InventoryItemView {
    category: &'static str,
    representation_id: Option<String>,
    resource_id: Option<String>,
    uri: Option<String>,
    detail: Option<String>,
}

#[derive(Debug, Serialize)]
struct InventoryStatsView {
    entries_visited: usize,
    fingerprints_computed: usize,
    fingerprint_cache_hits: usize,
    cache_rebuilt: bool,
}

#[derive(Debug, Serialize)]
struct ResolutionView {
    representation_id: String,
    availability: &'static str,
    resources: Vec<ResourceResolutionView>,
    issues: Vec<AvailabilityIssueView>,
}

#[derive(Debug, Serialize)]
struct ResourceResolutionView {
    resource_id: String,
    state: &'static str,
    candidates: Vec<CandidateView>,
    evidence: Vec<EvidenceView>,
}

#[derive(Debug, Serialize)]
struct AvailabilityIssueView {
    resource_id: String,
    required: bool,
    kind: &'static str,
    frames: Vec<i64>,
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

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum MetadataValueInput {
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
    List { values: Vec<MetadataValueInput> },
    Struct { fields: Vec<MetadataFieldInput> },
    Reference { target: MetadataReferenceInput },
}

#[derive(Debug, Deserialize)]
struct MetadataFieldInput {
    name: String,
    value: MetadataValueInput,
}

#[derive(Debug, Deserialize)]
struct MetadataReferenceInput {
    target_kind: MetadataTargetKind,
    target_id: String,
}

#[derive(Debug, Serialize)]
struct MetadataFieldView {
    name: String,
    value: MetadataValueView,
}

#[derive(Debug, Serialize)]
struct ActivityView {
    id: String,
    kind: String,
    started_at_unix_micros: Option<i64>,
    finished_at_unix_micros: Option<i64>,
    tool: Option<ToolView>,
    agent: Option<AgentView>,
    inputs: Vec<ActivityEdgeView>,
    outputs: Vec<ActivityEdgeView>,
}

#[derive(Debug, Serialize)]
struct ToolView {
    name: String,
    version: Option<String>,
    uri: Option<String>,
}

#[derive(Debug, Serialize)]
struct AgentView {
    name: Option<String>,
    identifier: Option<AgentIdentifierView>,
}

#[derive(Debug, Serialize)]
struct AgentIdentifierView {
    scheme: String,
    value: String,
    qualifier: Option<String>,
}

#[derive(Debug, Serialize)]
struct ActivityEdgeView {
    representation_id: String,
    role: Option<String>,
    snapshot: Option<ActivityEdgeSnapshotView>,
}

#[derive(Debug, Serialize)]
struct ActivityEdgeSnapshotView {
    revision_sequence: u64,
    fingerprints: Vec<FingerprintSnapshotView>,
}

#[derive(Debug, Serialize)]
struct FingerprintSnapshotView {
    algorithm: String,
    version: u16,
    value_hex: String,
    observed_revision_sequence: Option<u64>,
}

#[derive(Debug, Serialize)]
struct ArtifactEvaluationView {
    representation_id: String,
    state: &'static str,
    visited_representations: u32,
    truncated: bool,
    reasons: Vec<ArtifactReasonView>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ArtifactReasonView {
    ProducingActivityMissing {
        representation_id: String,
    },
    ProducingActivityAmbiguous {
        representation_id: String,
        activity_count: u32,
    },
    SnapshotAbsent {
        activity_id: String,
        representation_id: String,
        edge: &'static str,
    },
    FingerprintEvidenceMissing {
        activity_id: String,
        representation_id: String,
        edge: &'static str,
        fingerprint_algorithm: Option<String>,
        fingerprint_version: Option<u16>,
        snapshot_value_hex: Option<String>,
        current_value_hex: Option<String>,
    },
    FingerprintChanged {
        activity_id: String,
        representation_id: String,
        edge: &'static str,
        fingerprint_algorithm: String,
        fingerprint_version: u16,
        snapshot_value_hex: String,
        current_value_hex: String,
    },
    FingerprintRecomputationPending {
        activity_id: String,
        representation_id: String,
        edge: &'static str,
    },
    UpstreamNotCurrent {
        representation_id: String,
        upstream_state: &'static str,
    },
    TraversalTruncated {
        representation_id: String,
        traversal_limit: &'static str,
    },
}

#[derive(Debug, Serialize)]
struct RevisionView {
    id: String,
    sequence: u64,
    transaction_id: String,
    committed_at_unix_micros: i64,
    origin: Option<RevisionOriginView>,
    message: Option<String>,
}

#[derive(Debug, Serialize)]
struct RevisionOriginView {
    name: String,
    version: Option<String>,
    uri: Option<String>,
}

#[derive(Debug, Serialize)]
struct RevisionEventView {
    revision_id: String,
    position: u32,
    #[serde(flatten)]
    event: RevisionEventKindView,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum RevisionEventKindView {
    AssetImported {
        asset_id: String,
    },
    RepresentationAdded {
        asset_id: String,
        representation_id: String,
    },
    ResourceAdded {
        resource_id: String,
    },
    RepresentationResourceAdded {
        representation_id: String,
        resource_id: String,
        structural_position: u32,
    },
    LocatorAdded {
        resource_id: String,
        locator_id: String,
    },
    MediaRootAdded {
        media_root_id: String,
    },
    ExternalIdentifierAdded {
        target: ObjectRefView,
        identifier: RevisionIdentifierView,
    },
    ExternalIdentifierRemoved {
        target: ObjectRefView,
        identifier: RevisionIdentifierView,
    },
    MetadataAddedOrReplaced {
        target: ObjectRefView,
        vocabulary: String,
        property: String,
    },
    MetadataRemoved {
        target: ObjectRefView,
        vocabulary: String,
        property: String,
    },
    ActivityCreated {
        activity_id: String,
        activity_kind: String,
    },
    ActivityInputAdded {
        activity_id: String,
        representation_id: String,
        role: Option<String>,
    },
    ActivityOutputAdded {
        activity_id: String,
        representation_id: String,
        role: Option<String>,
    },
    ResourceFingerprintObserved {
        resource_id: String,
        algorithm: String,
        version: u16,
    },
    RepresentationFingerprintObserved {
        representation_id: String,
        algorithm: String,
        version: u16,
    },
}

#[derive(Debug, Serialize)]
struct RevisionIdentifierView {
    scheme: String,
    value: String,
    qualifier: Option<String>,
}

#[derive(Debug, Serialize)]
struct RepresentationRefView {
    representation_id: String,
}

#[derive(Clone, Copy)]
enum ActivityLookup {
    Producing,
    Consuming,
}

#[derive(Clone, Copy)]
enum ProvenanceDirection {
    Ancestors,
    Descendants,
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
            MediaCommand::Add(args) => media_add(&args, cli.json),
            MediaCommand::List(args) => media_list(&args, cli.json),
            MediaCommand::Show(args) => media_show(&args, cli.json),
            MediaCommand::Resolve(args) => media_resolve(args, cli.json),
            MediaCommand::Inventory(args) => media_inventory(&args, cli.json),
            MediaCommand::Fingerprint(args) => media_fingerprint(&args, cli.json),
        },
        Command::Representation(args) => match args.command {
            RepresentationCommand::Add(args) => representation_add(&args, cli.json),
        },
        Command::Root(args) => match args.command {
            RootCommand::Add(args) => root_add(args, cli.json),
            RootCommand::List(args) => root_list(&args, cli.json),
            RootCommand::Enable(args) => root_set_enabled(&args, true, cli.json),
            RootCommand::Disable(args) => root_set_enabled(&args, false, cli.json),
            RootCommand::Remove(args) => root_remove(&args, cli.json),
        },
        Command::Locator(args) => match args.command {
            LocatorCommand::Retire(args) => locator_retire(&args, cli.json),
        },
        Command::Identifier(args) => match args.command {
            IdentifierCommand::Add(args) => identifier_mutate(args, false, cli.json),
            IdentifierCommand::Remove(args) => identifier_mutate(args, true, cli.json),
            IdentifierCommand::List(args) => identifier_list(&args, cli.json),
            IdentifierCommand::Find(args) => identifier_find(args, cli.json),
        },
        Command::Metadata(args) => match args.command {
            MetadataCommand::Add(args) => metadata_add(args, cli.json),
            MetadataCommand::AddText(args) => metadata_add_text(args, cli.json),
            MetadataCommand::List(args) => metadata_list(&args, cli.json),
            MetadataCommand::Remove(args) => metadata_remove(args, cli.json),
            MetadataCommand::Find(args) => metadata_find(args, cli.json),
        },
        Command::Activity(args) => match args.command {
            ActivityCommand::Add(args) => activity_add(*args, cli.json),
            ActivityCommand::List(args) => activity_list(&args, cli.json),
            ActivityCommand::Producing(args) => {
                activity_lookup(&args, ActivityLookup::Producing, cli.json)
            }
            ActivityCommand::Consuming(args) => {
                activity_lookup(&args, ActivityLookup::Consuming, cli.json)
            }
            ActivityCommand::Ancestors(args) => {
                activity_relatives(&args, ProvenanceDirection::Ancestors, cli.json)
            }
            ActivityCommand::Descendants(args) => {
                activity_relatives(&args, ProvenanceDirection::Descendants, cli.json)
            }
        },
        Command::Artifact(args) => match args.command {
            ArtifactCommand::Evaluate(args) => artifact_evaluate(&args, cli.json),
        },
        Command::Revisions(args) => match args.command {
            RevisionsCommand::Latest(args) => revisions_latest(&args, cli.json),
            RevisionsCommand::Since(args) => revisions_since(&args, cli.json),
            RevisionsCommand::Events(args) => revisions_events(&args, cli.json),
        },
    }
}

fn init(args: InitArgs, json: bool) -> Result<()> {
    let production =
        SqliteProduction::create(&args.production, args.name).context("create production")?;
    let view = ProductionView {
        id: production.production().id().to_string(),
        path: production.path().display().to_string(),
        schema_version: production.production().schema_version(),
        display_name: production.production().display_name().map(str::to_owned),
    };
    if json {
        print_json(&view)
    } else {
        println!("created production {} at {}", view.id, view.path);
        Ok(())
    }
}

fn media_add(args: &MediaAddArgs, json: bool) -> Result<()> {
    let (prepared, inspection_paths) = prepare_cli_media(args)?;
    let (technical_metadata, inspections) = inspect_cli_media(args, &inspection_paths);
    let view = ImportView {
        asset_id: prepared.asset().id().to_string(),
        representation_id: prepared.representation().id().to_string(),
        resource_id: prepared.resources()[0].id().to_string(),
        locator_id: prepared.locators()[0].id().to_string(),
        uri: prepared.locators()[0].uri().to_owned(),
        resource_count: prepared.resources().len(),
        inspections,
    };
    let mut production = SqliteProduction::open(&args.production).context("open production")?;
    let mut transaction = production
        .begin_transaction()
        .context("begin import transaction")?;
    set_cli_revision_context(&mut transaction, "Import media")?;
    transaction
        .import_original(&prepared)
        .context("stage media import")?;
    for assertion in &technical_metadata {
        transaction
            .add_metadata_value(
                ObjectRef::Representation(prepared.representation().id()),
                assertion.property(),
                assertion.value(),
            )
            .context("stage technical metadata")?;
    }
    transaction.commit().context("commit media import")?;

    if json {
        print_json(&view)
    } else {
        println!("imported asset {} from {}", view.asset_id, view.uri);
        for inspection in &view.inspections {
            println!("inspection {}: {}", inspection.path, inspection.status);
        }
        Ok(())
    }
}

fn prepare_cli_media(args: &MediaAddArgs) -> Result<(OriginalMediaImport, Vec<PathBuf>)> {
    let recognize = args.path.is_dir() || args.recognize_companions;
    if recognize {
        let fallback_rate = RationalRate::new(24, 1).context("prepare recognition rate")?;
        let recognized = MediaRecognizer::new(args.sequence_rate.unwrap_or(fallback_rate))
            .recognize(&args.path)
            .context("recognize media")?;
        let [recognized] = recognized.as_slice() else {
            bail!(
                "media recognition returned {} candidates; supply a path identifying one layout",
                recognized.len()
            );
        };
        if matches!(recognized, RecognizedMedia::ImageSequence { .. })
            && args.sequence_rate.is_none()
        {
            bail!("recognized image sequences require --sequence-rate NUMERATOR/DENOMINATOR");
        }
        let paths = recognized_inspection_paths(recognized);
        let prepared = prepare_recognized_original_media(
            recognized,
            args.name.clone(),
            Some("postproject-cli".to_owned()),
        )
        .context("prepare recognized media import")?;
        Ok((prepared, paths))
    } else {
        let prepared = prepare_original_media(
            &args.path,
            args.name.clone(),
            Some("postproject-cli".to_owned()),
        )
        .context("prepare media import")?;
        Ok((prepared, vec![args.path.clone()]))
    }
}

fn recognized_inspection_paths(recognized: &RecognizedMedia) -> Vec<PathBuf> {
    match recognized {
        RecognizedMedia::SingleFile(path) => vec![path.clone()],
        RecognizedMedia::ImageSequence {
            directory,
            pattern,
            frames,
            missing_frames,
            ..
        } => (frames.start()..=frames.end())
            .find(|frame| !missing_frames.contains(frame))
            .map(|frame| vec![directory.join(pattern.filename(frame))])
            .unwrap_or_default(),
        RecognizedMedia::OrderedParts(members) | RecognizedMedia::Package(members) => members
            .iter()
            .filter(|member| member.is_required())
            .map(|member| member.path().to_path_buf())
            .collect(),
        _ => Vec::new(),
    }
}

fn inspect_cli_media(
    args: &MediaAddArgs,
    paths: &[PathBuf],
) -> (Vec<MetadataAssertion>, Vec<InspectionView>) {
    if !args.inspect {
        return (Vec::new(), Vec::new());
    }
    let inspector = FfprobeInspector::with_executable(&args.ffprobe);
    let mut assertions = Vec::new();
    let mut views = Vec::new();
    for path in paths {
        let (status, reason) = match inspector.inspect(path) {
            Ok(InspectionOutcome::Inspected(metadata)) => {
                assertions.extend_from_slice(metadata.assertions());
                ("recorded", None)
            }
            Ok(InspectionOutcome::Unavailable { reason }) => ("unavailable", Some(reason)),
            Ok(InspectionOutcome::Failed { reason }) => ("failed", Some(reason)),
            Err(error) => ("failed", Some(error.to_string())),
            Ok(_) => ("failed", Some("unsupported inspection outcome".to_owned())),
        };
        views.push(InspectionView {
            path: path.display().to_string(),
            status,
            reason,
        });
    }
    (assertions, views)
}

fn representation_add(args: &RepresentationAddArgs, json: bool) -> Result<()> {
    let asset_id = AssetId::from_str(&args.asset_id).context("parse asset ID")?;
    let kind = match args.kind {
        RepresentationKindArg::Original => RepresentationKind::Original,
        RepresentationKindArg::Proxy => RepresentationKind::Proxy,
        RepresentationKindArg::Optimized => RepresentationKind::Optimized,
        RepresentationKindArg::Derived => RepresentationKind::Derived,
    };
    let encoded = fs::read(&args.spec_file)
        .with_context(|| format!("read representation spec {}", args.spec_file.display()))?;
    let spec: RepresentationSourceSpec =
        serde_json::from_slice(&encoded).context("parse representation spec")?;
    let prepared = match spec {
        RepresentationSourceSpec::SingleFile { path } => {
            prepare_single_file_representation(asset_id, kind, path)
        }
        RepresentationSourceSpec::ImageSequence {
            directory,
            prefix,
            suffix,
            padding,
            start,
            end,
            step,
            rate_numerator,
            rate_denominator,
            missing_frames,
        } => prepare_image_sequence_representation(
            asset_id,
            kind,
            &ImageSequenceSource::new(
                directory,
                ImageSequencePattern::new(prefix, suffix, padding)?,
                FrameRange::new(start, end, step)?,
                RationalRate::new(rate_numerator, rate_denominator)?,
                missing_frames,
            ),
        ),
        RepresentationSourceSpec::OrderedParts { members } => {
            let sources = file_resource_sources(members)?;
            prepare_ordered_parts_representation(asset_id, kind, &sources)
        }
        RepresentationSourceSpec::Package { members } => {
            let sources = file_resource_sources(members)?;
            prepare_package_representation(asset_id, kind, &sources)
        }
    }
    .context("prepare representation")?;
    let view = RepresentationRefView {
        representation_id: prepared.representation().id().to_string(),
    };
    let mut production = SqliteProduction::open(&args.production).context("open production")?;
    let mut transaction = production
        .begin_transaction()
        .context("begin representation transaction")?;
    set_cli_revision_context(&mut transaction, "Add representation")?;
    transaction
        .add_representation(&prepared)
        .context("stage representation")?;
    transaction.commit().context("commit representation")?;

    if json {
        print_json(&view)
    } else {
        println!("added representation {}", view.representation_id);
        Ok(())
    }
}

fn media_fingerprint(args: &MediaFingerprintArgs, json: bool) -> Result<()> {
    let asset_id = AssetId::from_str(&args.asset_id).context("parse asset ID")?;
    let representation_id =
        RepresentationId::from_str(&args.representation_id).context("parse representation ID")?;
    let resource_id = ResourceId::from_str(&args.resource_id).context("parse resource ID")?;
    let report = fingerprint_file(&args.path).context("fingerprint resource content")?;
    let mut production = SqliteProduction::open(&args.production).context("open production")?;
    let representation = production
        .representations(asset_id)
        .context("load asset representations")?
        .into_iter()
        .find(|candidate| candidate.id() == representation_id)
        .with_context(|| format!("representation does not exist on asset: {representation_id}"))?;
    let mut resources = production
        .resources(representation_id)
        .context("load representation resources")?;
    let resource = resources
        .iter_mut()
        .find(|candidate| candidate.id() == resource_id)
        .with_context(|| format!("resource does not belong to representation: {resource_id}"))?;
    let fingerprint = report.fingerprint().clone();
    let mut fingerprints = resource.fingerprints().to_vec();
    fingerprints.retain(|current| {
        current.algorithm() != fingerprint.algorithm() || current.version() != fingerprint.version()
    });
    fingerprints.push(fingerprint.clone());
    *resource = Resource::new(resource.id(), fingerprints, resource.file_facts());
    let representation_fingerprint =
        fingerprint_representation(representation.content_structure(), &resources)
            .context("recompute representation fingerprint")?;

    let mut transaction = production
        .begin_transaction()
        .context("begin fingerprint transaction")?;
    set_cli_revision_context(&mut transaction, "Record fingerprint observation")?;
    transaction
        .record_resource_fingerprint(resource_id, &fingerprint)
        .context("stage resource fingerprint")?;
    transaction
        .record_representation_fingerprint(representation_id, &representation_fingerprint)
        .context("stage representation fingerprint")?;
    transaction.commit().context("commit fingerprints")?;

    let view = FingerprintObservationView {
        representation_id: representation_id.to_string(),
        resource_id: resource_id.to_string(),
        resource_fingerprint: FingerprintView {
            algorithm: fingerprint.algorithm().to_owned(),
            version: fingerprint.version(),
            value_hex: hex::encode(fingerprint.value()),
        },
        representation_fingerprint: FingerprintView {
            algorithm: representation_fingerprint.algorithm().to_owned(),
            version: representation_fingerprint.version(),
            value_hex: hex::encode(representation_fingerprint.value()),
        },
    };
    if json {
        print_json(&view)
    } else {
        println!(
            "recorded fingerprints for resource {} and representation {}",
            view.resource_id, view.representation_id
        );
        Ok(())
    }
}

fn file_resource_sources(members: Vec<FileResourceSpec>) -> Result<Vec<FileResourceSource>> {
    members
        .into_iter()
        .map(|member| {
            Ok(FileResourceSource::new(
                member.path,
                ResourceRole::new(member.role)?,
                member.required,
            ))
        })
        .collect()
}

fn media_list(args: &ProductionArgs, json: bool) -> Result<()> {
    let production = SqliteProduction::open(&args.production).context("open production")?;
    let mut views = Vec::new();
    for asset in production.assets().context("load assets")? {
        let representation_count = production
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
    let production = SqliteProduction::open(&args.production).context("open production")?;
    let asset_id = parse_asset_id(&args.asset_id)?;
    let asset = find_asset(&production, asset_id)?;
    let view = asset_view(&production, &asset)?;

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
    let root = MediaRoot::new(
        MediaRootId::new(),
        args.name,
        args.label,
        None,
        args.priority,
        true,
    )
    .context("prepare media root")?;
    let view = root_view(&root);
    let mut production = SqliteProduction::open(&args.production).context("open production")?;
    let mut transaction = production
        .begin_transaction()
        .context("begin root transaction")?;
    set_cli_revision_context(&mut transaction, "Add media root")?;
    transaction
        .add_media_root(root)
        .context("stage media root")?;
    transaction.commit().context("commit media root")?;

    if json {
        print_json(&view)
    } else {
        println!("added media root {} ({})", view.name, view.id);
        Ok(())
    }
}

fn root_list(args: &ProductionArgs, json: bool) -> Result<()> {
    let production = SqliteProduction::open(&args.production).context("open production")?;
    let roots = production
        .production()
        .media_roots()
        .iter()
        .map(root_view)
        .collect::<Vec<_>>();
    if json {
        print_json(&roots)
    } else {
        for root in roots {
            println!(
                "{}\t{}\t{}\t{}\t{}",
                root.id,
                if root.enabled { "enabled" } else { "disabled" },
                root.priority,
                root.label.as_deref().unwrap_or("-"),
                root.name
            );
        }
        Ok(())
    }
}

fn root_set_enabled(args: &RootMutationArgs, enabled: bool, json: bool) -> Result<()> {
    let root_id = MediaRootId::from_str(&args.root_id).context("parse media-root ID")?;
    let mut production = SqliteProduction::open(&args.production).context("open production")?;
    let root = production
        .production()
        .media_roots()
        .iter()
        .find(|root| root.id() == root_id)
        .context("media root does not exist")?;
    let view = RootView {
        enabled,
        ..root_view(root)
    };
    let mut transaction = production
        .begin_transaction()
        .context("begin root transaction")?;
    set_cli_revision_context(
        &mut transaction,
        if enabled {
            "Enable media root"
        } else {
            "Disable media root"
        },
    )?;
    transaction
        .set_media_root_enabled(root_id, enabled)
        .context("stage media-root state change")?;
    transaction
        .commit()
        .context("commit media-root state change")?;

    if json {
        print_json(&view)
    } else {
        println!(
            "{} media root {}",
            if enabled { "enabled" } else { "disabled" },
            view.id
        );
        Ok(())
    }
}

fn root_remove(args: &RootMutationArgs, json: bool) -> Result<()> {
    let root_id = MediaRootId::from_str(&args.root_id).context("parse media-root ID")?;
    let mut production = SqliteProduction::open(&args.production).context("open production")?;
    let root = production
        .production()
        .media_roots()
        .iter()
        .find(|root| root.id() == root_id)
        .context("media root does not exist")?;
    let view = root_view(root);
    let mut transaction = production
        .begin_transaction()
        .context("begin root transaction")?;
    set_cli_revision_context(&mut transaction, "Remove media root")?;
    transaction
        .remove_media_root(root_id)
        .context("stage media-root removal")?;
    transaction.commit().context("commit media-root removal")?;

    if json {
        print_json(&view)
    } else {
        println!("removed media root {} ({})", view.name, view.id);
        Ok(())
    }
}

fn root_view(root: &MediaRoot) -> RootView {
    RootView {
        id: root.id().to_string(),
        name: root.name().to_owned(),
        label: root.label().map(str::to_owned),
        legacy_uri: root.legacy_uri().map(str::to_owned),
        priority: root.priority(),
        enabled: root.is_enabled(),
    }
}

fn locator_retire(args: &LocatorRetireArgs, json: bool) -> Result<()> {
    let locator_id = LocatorId::from_str(&args.locator_id).context("parse locator ID")?;
    let mut production = SqliteProduction::open(&args.production).context("open production")?;
    let mut transaction = production
        .begin_transaction()
        .context("begin locator transaction")?;
    set_cli_revision_context(&mut transaction, "Retire media locator")?;
    transaction
        .retire_locator(locator_id)
        .context("stage locator retirement")?;
    transaction.commit().context("commit locator retirement")?;

    if json {
        print_json(&serde_json::json!({ "id": locator_id.to_string() }))
    } else {
        println!("retired locator {locator_id}");
        Ok(())
    }
}

fn identifier_mutate(args: IdentifierMutationArgs, remove: bool, json: bool) -> Result<()> {
    let target = parse_identifier_target(args.target.target_kind, &args.target.target_id)?;
    let scheme = IdentifierScheme::new(args.scheme).context("validate identifier scheme")?;
    let identifier = ExternalIdentifier::new(scheme, args.value, args.qualifier)
        .context("validate external identifier")?;
    let view = external_identifier_view(target, &identifier);
    let mut production =
        SqliteProduction::open(&args.target.production).context("open production")?;
    let mut transaction = production
        .begin_transaction()
        .context("begin identifier transaction")?;
    set_cli_revision_context(
        &mut transaction,
        if remove {
            "Remove external identifier"
        } else {
            "Add external identifier"
        },
    )?;
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
    let production = SqliteProduction::open(&args.production).context("open production")?;
    let views: Vec<_> = production
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
    let production = SqliteProduction::open(&args.production).context("open production")?;
    let views: Vec<_> = production
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
    let mut production =
        SqliteProduction::open(&args.target.production).context("open production")?;
    let mut transaction = production
        .begin_transaction()
        .context("begin metadata transaction")?;
    set_cli_revision_context(&mut transaction, "Add metadata")?;
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

fn metadata_add(args: MetadataAddArgs, json: bool) -> Result<()> {
    let target = parse_metadata_target(args.target.target_kind, &args.target.target_id)?;
    let property = parse_metadata_property(args.vocabulary, args.property)?;
    let encoded = fs::read_to_string(&args.value_file)
        .with_context(|| format!("read metadata value {}", args.value_file.display()))?;
    let input: MetadataValueInput =
        serde_json::from_str(&encoded).context("parse typed metadata JSON")?;
    let value = input.into_value()?;
    let assertion = MetadataAssertion::new(property.clone(), value.clone());
    let view = metadata_assertion_view(target, &assertion)?;
    let mut production =
        SqliteProduction::open(&args.target.production).context("open production")?;
    let mut transaction = production
        .begin_transaction()
        .context("begin metadata transaction")?;
    set_cli_revision_context(&mut transaction, "Add metadata")?;
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
    let production = SqliteProduction::open(&args.production).context("open production")?;
    let views = production
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
    let mut production =
        SqliteProduction::open(&args.target.production).context("open production")?;
    let mut transaction = production
        .begin_transaction()
        .context("begin metadata transaction")?;
    set_cli_revision_context(&mut transaction, "Remove metadata")?;
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
    let production = SqliteProduction::open(&args.production).context("open production")?;
    let views = production
        .query_by_metadata_property(&property)
        .context("query metadata property")?
        .iter()
        .map(|matched| metadata_assertion_view(matched.target(), matched.assertion()))
        .collect::<Result<Vec<_>>>()?;

    print_metadata_assertions(&views, json)
}

fn activity_list(args: &ProductionArgs, json: bool) -> Result<()> {
    let production = SqliteProduction::open(&args.production).context("open production")?;
    let views: Vec<_> = production
        .activities()
        .context("load activities")?
        .iter()
        .map(activity_view)
        .collect();

    print_activity_views(&views, json)
}

fn activity_lookup(
    args: &ActivityRepresentationArgs,
    lookup: ActivityLookup,
    json: bool,
) -> Result<()> {
    let representation_id = parse_representation_id(&args.representation_id)?;
    let production = SqliteProduction::open(&args.production).context("open production")?;
    let activities = match lookup {
        ActivityLookup::Producing => production.activities_producing(representation_id),
        ActivityLookup::Consuming => production.activities_consuming(representation_id),
    }
    .context("query representation activities")?;
    let views: Vec<_> = activities.iter().map(activity_view).collect();

    print_activity_views(&views, json)
}

fn print_activity_views(views: &[ActivityView], json: bool) -> Result<()> {
    if json {
        print_json(&views)
    } else {
        for view in views {
            println!(
                "{}\t{}\t{} input(s)\t{} output(s)",
                view.id,
                view.kind,
                view.inputs.len(),
                view.outputs.len()
            );
        }
        Ok(())
    }
}

fn activity_relatives(
    args: &ActivityRepresentationArgs,
    direction: ProvenanceDirection,
    json: bool,
) -> Result<()> {
    let representation_id = parse_representation_id(&args.representation_id)?;
    let production = SqliteProduction::open(&args.production).context("open production")?;
    let representation_ids = match direction {
        ProvenanceDirection::Ancestors => production.ancestors(representation_id),
        ProvenanceDirection::Descendants => production.descendants(representation_id),
    }
    .context("traverse provenance")?;
    let views: Vec<_> = representation_ids
        .into_iter()
        .map(|id| RepresentationRefView {
            representation_id: id.to_string(),
        })
        .collect();

    if json {
        print_json(&views)
    } else {
        for view in views {
            println!("{}", view.representation_id);
        }
        Ok(())
    }
}

fn activity_add(args: ActivityAddArgs, json: bool) -> Result<()> {
    let kind = ActivityKind::new(args.kind).context("validate activity kind")?;
    let inputs = args
        .inputs
        .into_iter()
        .map(|edge| ActivityInput::new(edge.representation_id, edge.role))
        .collect();
    let outputs = args
        .outputs
        .into_iter()
        .map(|edge| ActivityOutput::new(edge.representation_id, edge.role))
        .collect();
    let mut activity = Activity::new(ActivityId::new(), kind, inputs, outputs)
        .context("validate activity")?
        .with_timing(
            args.started_at_unix_micros.map(Timestamp::from_unix_micros),
            args.finished_at_unix_micros
                .map(Timestamp::from_unix_micros),
        )
        .context("validate activity timing")?;
    if let Some(name) = args.tool_name {
        let tool = ToolIdentity::new(name, args.tool_version, args.tool_uri)
            .context("validate activity tool")?;
        activity = activity.with_tool(tool);
    } else if args.tool_version.is_some() || args.tool_uri.is_some() {
        bail!("--tool-version and --tool-uri require --tool-name");
    }
    let agent_identifier = match (args.agent_identifier_scheme, args.agent_identifier_value) {
        (Some(scheme), Some(value)) => Some(
            ExternalIdentifier::new(
                IdentifierScheme::new(scheme).context("validate agent identifier scheme")?,
                value,
                args.agent_identifier_qualifier,
            )
            .context("validate agent identifier")?,
        ),
        (None, None) if args.agent_identifier_qualifier.is_none() => None,
        _ => bail!(
            "agent identifier scheme and value must be supplied together; qualifier is optional"
        ),
    };
    if args.agent_name.is_some() || agent_identifier.is_some() {
        let agent = AgentIdentity::new(args.agent_name, agent_identifier)
            .context("validate activity agent")?;
        activity = activity.with_agent(agent);
    }
    let activity_id = activity.id();
    let mut production = SqliteProduction::open(&args.production).context("open production")?;
    let mut transaction = production
        .begin_transaction()
        .context("begin activity transaction")?;
    set_cli_revision_context(&mut transaction, "Record activity")?;
    transaction
        .create_activity(&activity)
        .context("stage activity")?;
    transaction.commit().context("commit activity")?;
    drop(transaction);
    let stored_activity = production
        .activities()
        .context("reload activity snapshots")?
        .into_iter()
        .find(|candidate| candidate.id() == activity_id)
        .context("committed activity is missing")?;
    let view = activity_view(&stored_activity);

    if json {
        print_json(&view)
    } else {
        println!("recorded activity {} ({})", view.id, view.kind);
        Ok(())
    }
}

fn activity_view(activity: &Activity) -> ActivityView {
    ActivityView {
        id: activity.id().to_string(),
        kind: activity.kind().as_str().to_owned(),
        started_at_unix_micros: activity
            .started_at()
            .map(postproject_core::Timestamp::as_unix_micros),
        finished_at_unix_micros: activity
            .finished_at()
            .map(postproject_core::Timestamp::as_unix_micros),
        tool: activity.tool().map(|tool| ToolView {
            name: tool.name().to_owned(),
            version: tool.version().map(str::to_owned),
            uri: tool.uri().map(str::to_owned),
        }),
        agent: activity.agent().map(|agent| AgentView {
            name: agent.name().map(str::to_owned),
            identifier: agent.identifier().map(|identifier| AgentIdentifierView {
                scheme: identifier.scheme().as_str().to_owned(),
                value: identifier.value().to_owned(),
                qualifier: identifier.qualifier().map(str::to_owned),
            }),
        }),
        inputs: activity
            .inputs()
            .iter()
            .map(|input| ActivityEdgeView {
                representation_id: input.representation_id().to_string(),
                role: input.role().map(|role| role.as_str().to_owned()),
                snapshot: input.snapshot().map(activity_edge_snapshot_view),
            })
            .collect(),
        outputs: activity
            .outputs()
            .iter()
            .map(|output| ActivityEdgeView {
                representation_id: output.representation_id().to_string(),
                role: output.role().map(|role| role.as_str().to_owned()),
                snapshot: output.snapshot().map(activity_edge_snapshot_view),
            })
            .collect(),
    }
}

fn activity_edge_snapshot_view(
    snapshot: &postproject_core::ActivityEdgeSnapshot,
) -> ActivityEdgeSnapshotView {
    ActivityEdgeSnapshotView {
        revision_sequence: snapshot.revision_sequence(),
        fingerprints: snapshot
            .fingerprints()
            .iter()
            .map(|fingerprint| FingerprintSnapshotView {
                algorithm: fingerprint.algorithm().to_owned(),
                version: fingerprint.version(),
                value_hex: hex::encode(fingerprint.value()),
                observed_revision_sequence: fingerprint.observed_revision_sequence(),
            })
            .collect(),
    }
}

fn artifact_evaluate(args: &ArtifactEvaluateArgs, json: bool) -> Result<()> {
    let representation_id = parse_representation_id(&args.representation_id)?;
    let limits = ArtifactEvaluationLimits::new(args.max_depth, args.max_representations)
        .context("validate artifact evaluation bounds")?;
    let production = SqliteProduction::open(&args.production).context("open production")?;
    let evaluation = production
        .evaluate_artifact(representation_id, limits)
        .context("evaluate artifact")?;
    let view = ArtifactEvaluationView {
        representation_id: evaluation.representation_id().to_string(),
        state: artifact_knowledge_state_name(evaluation.state())?,
        visited_representations: evaluation.visited_representations(),
        truncated: evaluation.is_truncated(),
        reasons: evaluation
            .reasons()
            .iter()
            .map(artifact_reason_view)
            .collect::<Result<Vec<_>>>()?,
    };

    if json {
        print_json(&view)
    } else {
        println!(
            "{}\t{}\t{} representation(s) visited",
            view.representation_id, view.state, view.visited_representations
        );
        for reason in &view.reasons {
            let (kind, representation_id) = artifact_reason_summary(reason);
            println!("reason\t{kind}\t{representation_id}");
        }
        Ok(())
    }
}

fn artifact_reason_view(reason: &ArtifactKnowledgeReason) -> Result<ArtifactReasonView> {
    match reason {
        ArtifactKnowledgeReason::ProducingActivityMissing { representation_id } => {
            Ok(ArtifactReasonView::ProducingActivityMissing {
                representation_id: representation_id.to_string(),
            })
        }
        ArtifactKnowledgeReason::ProducingActivityAmbiguous {
            representation_id,
            activity_count,
        } => Ok(ArtifactReasonView::ProducingActivityAmbiguous {
            representation_id: representation_id.to_string(),
            activity_count: *activity_count,
        }),
        ArtifactKnowledgeReason::SnapshotAbsent {
            activity_id,
            representation_id,
            edge,
        } => Ok(ArtifactReasonView::SnapshotAbsent {
            activity_id: activity_id.to_string(),
            representation_id: representation_id.to_string(),
            edge: artifact_edge_name(*edge)?,
        }),
        ArtifactKnowledgeReason::FingerprintEvidenceMissing {
            activity_id,
            representation_id,
            edge,
            algorithm,
            version,
            snapshot_value,
            current_value,
        } => Ok(ArtifactReasonView::FingerprintEvidenceMissing {
            activity_id: activity_id.to_string(),
            representation_id: representation_id.to_string(),
            edge: artifact_edge_name(*edge)?,
            fingerprint_algorithm: algorithm.clone(),
            fingerprint_version: *version,
            snapshot_value_hex: snapshot_value.as_ref().map(hex::encode),
            current_value_hex: current_value.as_ref().map(hex::encode),
        }),
        ArtifactKnowledgeReason::FingerprintChanged {
            activity_id,
            representation_id,
            edge,
            algorithm,
            version,
            snapshot_value,
            current_value,
        } => Ok(ArtifactReasonView::FingerprintChanged {
            activity_id: activity_id.to_string(),
            representation_id: representation_id.to_string(),
            edge: artifact_edge_name(*edge)?,
            fingerprint_algorithm: algorithm.clone(),
            fingerprint_version: *version,
            snapshot_value_hex: hex::encode(snapshot_value),
            current_value_hex: hex::encode(current_value),
        }),
        ArtifactKnowledgeReason::FingerprintRecomputationPending {
            activity_id,
            representation_id,
            edge,
        } => Ok(ArtifactReasonView::FingerprintRecomputationPending {
            activity_id: activity_id.to_string(),
            representation_id: representation_id.to_string(),
            edge: artifact_edge_name(*edge)?,
        }),
        ArtifactKnowledgeReason::UpstreamNotCurrent {
            representation_id,
            state,
        } => Ok(ArtifactReasonView::UpstreamNotCurrent {
            representation_id: representation_id.to_string(),
            upstream_state: artifact_knowledge_state_name(*state)?,
        }),
        ArtifactKnowledgeReason::TraversalTruncated {
            limit,
            representation_id,
        } => Ok(ArtifactReasonView::TraversalTruncated {
            representation_id: representation_id.to_string(),
            traversal_limit: artifact_traversal_limit_name(*limit)?,
        }),
        _ => bail!("unsupported artifact evaluation reason"),
    }
}

fn artifact_reason_summary(reason: &ArtifactReasonView) -> (&'static str, &str) {
    match reason {
        ArtifactReasonView::ProducingActivityMissing { representation_id } => {
            ("producing_activity_missing", representation_id)
        }
        ArtifactReasonView::ProducingActivityAmbiguous {
            representation_id, ..
        } => ("producing_activity_ambiguous", representation_id),
        ArtifactReasonView::SnapshotAbsent {
            representation_id, ..
        } => ("snapshot_absent", representation_id),
        ArtifactReasonView::FingerprintEvidenceMissing {
            representation_id, ..
        } => ("fingerprint_evidence_missing", representation_id),
        ArtifactReasonView::FingerprintChanged {
            representation_id, ..
        } => ("fingerprint_changed", representation_id),
        ArtifactReasonView::FingerprintRecomputationPending {
            representation_id, ..
        } => ("fingerprint_recomputation_pending", representation_id),
        ArtifactReasonView::UpstreamNotCurrent {
            representation_id, ..
        } => ("upstream_not_current", representation_id),
        ArtifactReasonView::TraversalTruncated {
            representation_id, ..
        } => ("traversal_truncated", representation_id),
    }
}

fn artifact_knowledge_state_name(state: ArtifactKnowledgeState) -> Result<&'static str> {
    match state {
        ArtifactKnowledgeState::Current => Ok("current"),
        ArtifactKnowledgeState::Stale => Ok("stale"),
        ArtifactKnowledgeState::Indeterminate => Ok("indeterminate"),
        ArtifactKnowledgeState::Diverged => Ok("diverged"),
        _ => bail!("unsupported artifact knowledge state"),
    }
}

fn artifact_edge_name(edge: ArtifactEdgeKind) -> Result<&'static str> {
    match edge {
        ArtifactEdgeKind::Input => Ok("input"),
        ArtifactEdgeKind::Output => Ok("output"),
        _ => bail!("unsupported artifact edge kind"),
    }
}

fn artifact_traversal_limit_name(limit: ArtifactTraversalLimitKind) -> Result<&'static str> {
    match limit {
        ArtifactTraversalLimitKind::Depth => Ok("depth"),
        ArtifactTraversalLimitKind::Representations => Ok("representations"),
        _ => bail!("unsupported artifact traversal limit"),
    }
}

fn revisions_latest(args: &ProductionArgs, json: bool) -> Result<()> {
    let production = SqliteProduction::open(&args.production).context("open production")?;
    let revision = production
        .latest_revision()
        .context("load latest revision")?
        .as_ref()
        .map(revision_view);
    if json {
        print_json(&revision)
    } else if let Some(revision) = revision {
        print_revision(&revision);
        Ok(())
    } else {
        println!("no revisions");
        Ok(())
    }
}

fn revisions_since(args: &RevisionsSinceArgs, json: bool) -> Result<()> {
    let production = SqliteProduction::open(&args.production).context("open production")?;
    let revisions = production
        .changes_since(args.after, args.limit)
        .context("load revision page")?;
    let views: Vec<_> = revisions.iter().map(revision_view).collect();
    if json {
        print_json(&views)
    } else {
        for revision in &views {
            print_revision(revision);
        }
        Ok(())
    }
}

fn revisions_events(args: &RevisionEventsArgs, json: bool) -> Result<()> {
    let revision_id = RevisionId::from_str(&args.revision_id).context("parse revision ID")?;
    let production = SqliteProduction::open(&args.production).context("open production")?;
    let events = production
        .events_for_revision(revision_id)
        .context("load revision events")?;
    let views = events
        .iter()
        .map(revision_event_view)
        .collect::<Result<Vec<_>>>()?;
    if json {
        print_json(&views)
    } else {
        for event in &views {
            let payload = serde_json::to_string(&event.event).context("format revision event")?;
            println!("{}\t{payload}", event.position);
        }
        Ok(())
    }
}

fn revision_view(revision: &Revision) -> RevisionView {
    RevisionView {
        id: revision.id().to_string(),
        sequence: revision.sequence(),
        transaction_id: revision.transaction_id().to_string(),
        committed_at_unix_micros: revision.committed_at().as_unix_micros(),
        origin: revision.origin().map(|origin| RevisionOriginView {
            name: origin.name().to_owned(),
            version: origin.version().map(str::to_owned),
            uri: origin.uri().map(str::to_owned),
        }),
        message: revision.message().map(str::to_owned),
    }
}

fn print_revision(revision: &RevisionView) {
    println!(
        "{}\t{}\t{}",
        revision.sequence,
        revision.id,
        revision.message.as_deref().unwrap_or("")
    );
}

#[allow(
    clippy::too_many_lines,
    reason = "the complete semantic event catalog is clearest as one exhaustive mapping"
)]
fn revision_event_view(event: &RevisionEvent) -> Result<RevisionEventView> {
    let kind = match event.kind() {
        RevisionEventKind::AssetImported { asset_id } => RevisionEventKindView::AssetImported {
            asset_id: asset_id.to_string(),
        },
        RevisionEventKind::RepresentationAdded {
            asset_id,
            representation_id,
        } => RevisionEventKindView::RepresentationAdded {
            asset_id: asset_id.to_string(),
            representation_id: representation_id.to_string(),
        },
        RevisionEventKind::ResourceAdded { resource_id } => RevisionEventKindView::ResourceAdded {
            resource_id: resource_id.to_string(),
        },
        RevisionEventKind::RepresentationResourceAdded {
            representation_id,
            resource_id,
            position,
        } => RevisionEventKindView::RepresentationResourceAdded {
            representation_id: representation_id.to_string(),
            resource_id: resource_id.to_string(),
            structural_position: *position,
        },
        RevisionEventKind::LocatorAdded {
            resource_id,
            locator_id,
        } => RevisionEventKindView::LocatorAdded {
            resource_id: resource_id.to_string(),
            locator_id: locator_id.to_string(),
        },
        RevisionEventKind::MediaRootAdded { media_root_id } => {
            RevisionEventKindView::MediaRootAdded {
                media_root_id: media_root_id.to_string(),
            }
        }
        RevisionEventKind::ExternalIdentifierAdded { target, identifier } => {
            RevisionEventKindView::ExternalIdentifierAdded {
                target: object_ref_view(*target)?,
                identifier: revision_identifier_view(identifier),
            }
        }
        RevisionEventKind::ExternalIdentifierRemoved { target, identifier } => {
            RevisionEventKindView::ExternalIdentifierRemoved {
                target: object_ref_view(*target)?,
                identifier: revision_identifier_view(identifier),
            }
        }
        RevisionEventKind::MetadataAddedOrReplaced { target, property } => {
            RevisionEventKindView::MetadataAddedOrReplaced {
                target: object_ref_view(*target)?,
                vocabulary: property.vocabulary().as_str().to_owned(),
                property: property.property().as_str().to_owned(),
            }
        }
        RevisionEventKind::MetadataRemoved { target, property } => {
            RevisionEventKindView::MetadataRemoved {
                target: object_ref_view(*target)?,
                vocabulary: property.vocabulary().as_str().to_owned(),
                property: property.property().as_str().to_owned(),
            }
        }
        RevisionEventKind::ActivityCreated { activity_id, kind } => {
            RevisionEventKindView::ActivityCreated {
                activity_id: activity_id.to_string(),
                activity_kind: kind.as_str().to_owned(),
            }
        }
        RevisionEventKind::ActivityInputAdded {
            activity_id,
            representation_id,
            role,
        } => RevisionEventKindView::ActivityInputAdded {
            activity_id: activity_id.to_string(),
            representation_id: representation_id.to_string(),
            role: role.as_ref().map(|role| role.as_str().to_owned()),
        },
        RevisionEventKind::ActivityOutputAdded {
            activity_id,
            representation_id,
            role,
        } => RevisionEventKindView::ActivityOutputAdded {
            activity_id: activity_id.to_string(),
            representation_id: representation_id.to_string(),
            role: role.as_ref().map(|role| role.as_str().to_owned()),
        },
        RevisionEventKind::ResourceFingerprintObserved {
            resource_id,
            algorithm,
            version,
        } => RevisionEventKindView::ResourceFingerprintObserved {
            resource_id: resource_id.to_string(),
            algorithm: algorithm.clone(),
            version: *version,
        },
        RevisionEventKind::RepresentationFingerprintObserved {
            representation_id,
            algorithm,
            version,
        } => RevisionEventKindView::RepresentationFingerprintObserved {
            representation_id: representation_id.to_string(),
            algorithm: algorithm.clone(),
            version: *version,
        },
        _ => bail!("revision event kind is not supported by this CLI"),
    };
    Ok(RevisionEventView {
        revision_id: event.revision_id().to_string(),
        position: event.position(),
        event: kind,
    })
}

fn revision_identifier_view(identifier: &ExternalIdentifier) -> RevisionIdentifierView {
    RevisionIdentifierView {
        scheme: identifier.scheme().as_str().to_owned(),
        value: identifier.value().to_owned(),
        qualifier: identifier.qualifier().map(str::to_owned),
    }
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

fn media_inventory(args: &MediaInventoryArgs, json: bool) -> Result<()> {
    let root_mappings = prepare_root_mappings(&args.root_mappings)?;
    let production = SqliteProduction::open(&args.production).context("open production")?;
    let report = InventoryScanner::default()
        .scan(&production, &root_mappings, args.cache.as_deref())
        .context("inventory production media")?;
    let view = InventoryView::from(&report);
    if json {
        print_json(&view)
    } else {
        for item in &view.items {
            println!(
                "{}\t{}\t{}",
                item.category,
                item.uri.as_deref().unwrap_or("-"),
                item.detail.as_deref().unwrap_or("-")
            );
        }
        println!(
            "visited={} fingerprints={} cache_hits={}",
            view.stats.entries_visited,
            view.stats.fingerprints_computed,
            view.stats.fingerprint_cache_hits
        );
        Ok(())
    }
}

fn prepare_root_mappings(mappings: &[RootMappingArg]) -> Result<Vec<MediaRootMapping>> {
    mappings
        .iter()
        .map(|mapping| MediaRootMapping::new(&mapping.name, &mapping.directory))
        .collect::<postproject_core::Result<Vec<_>>>()
        .context("prepare root mappings")
}

fn media_resolve(args: MediaResolveArgs, json: bool) -> Result<()> {
    let root_mappings = prepare_root_mappings(&args.root_mappings)?;
    let mut production = SqliteProduction::open(&args.production).context("open production")?;
    let asset_id = parse_asset_id(&args.asset_id)?;
    find_asset(&production, asset_id)?;
    let representations = production
        .representations(asset_id)
        .context("load asset representations")?;
    let resolver = MediaResolver::default();
    let inspector = FfprobeInspector::with_executable(&args.ffprobe);
    let resolutions = representations
        .iter()
        .map(|representation| {
            resolve_representation(
                &production,
                representation,
                &resolver,
                &root_mappings,
                args.verify,
                &inspector,
            )
        })
        .collect::<Result<Vec<_>>>()?;

    if let Some(uri) = args.confirm.as_deref() {
        let matching: Vec<_> = resolutions
            .iter()
            .flat_map(RepresentationResolution::resources)
            .flat_map(|resolution| {
                resolution
                    .candidates()
                    .iter()
                    .filter(move |candidate| candidate.uri() == uri)
                    .map(move |_| resolution.resource_id())
            })
            .collect();
        if matching.len() != 1 {
            bail!("confirmation URI must identify exactly one candidate from this resolution");
        }
        let locator = prepare_confirmed_locator(matching[0], uri.to_owned())
            .context("prepare confirmed locator")?;
        let mut transaction = production
            .begin_transaction()
            .context("begin confirmation transaction")?;
        set_cli_revision_context(&mut transaction, "Confirm media locator")?;
        transaction
            .add_locator(&locator)
            .context("stage confirmed locator")?;
        transaction.commit().context("commit confirmed locator")?;
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
            println!(
                "{}: {}",
                resolution.representation_id, resolution.availability
            );
            for resource in &resolution.resources {
                println!("  {}: {}", resource.resource_id, resource.state);
                for candidate in &resource.candidates {
                    println!(
                        "    {} ({} bp)",
                        candidate.uri, candidate.confidence_basis_points
                    );
                }
            }
        }
        if let Some(uri) = &view.confirmed_uri {
            println!("confirmed {uri}");
        }
        Ok(())
    }
}

fn resolve_representation(
    production: &SqliteProduction,
    representation: &Representation,
    resolver: &MediaResolver,
    root_mappings: &[MediaRootMapping],
    verify: bool,
    inspector: &dyn MediaInspector,
) -> Result<RepresentationResolution> {
    let technical_metadata = if verify {
        let assertions = production
            .metadata(ObjectRef::Representation(representation.id()))
            .context("load representation technical metadata")?;
        TechnicalMetadata::from_assertions(&assertions)
    } else {
        None
    };
    let resources = production
        .resources(representation.id())
        .context("load representation resources")?;
    let resource_resolutions = resources
        .iter()
        .map(|resource| {
            let locators = production
                .locators(resource.id())
                .context("load resource locators")?;
            let resolution = if let Some(expected) = technical_metadata.as_ref() {
                resolver.resolve_resource_with_technical_evidence(
                    resource,
                    representation.content_structure(),
                    &locators,
                    production.production().media_roots(),
                    root_mappings,
                    (expected, inspector),
                )
            } else {
                resolver.resolve_resource_with_verification(
                    resource,
                    representation.content_structure(),
                    &locators,
                    production.production().media_roots(),
                    root_mappings,
                    if verify {
                        VerificationMode::Content
                    } else {
                        VerificationMode::Presence
                    },
                )
            };
            resolution.context("resolve representation resource")
        })
        .collect::<Result<Vec<_>>>()?;
    RepresentationResolution::aggregate(
        representation.id(),
        representation.content_structure(),
        resource_resolutions,
    )
    .context("aggregate representation availability")
}

fn parse_asset_id(value: &str) -> Result<AssetId> {
    AssetId::from_str(value).context("parse asset ID")
}

fn parse_representation_id(value: &str) -> Result<RepresentationId> {
    RepresentationId::from_str(value).context("parse representation ID")
}

fn parse_identifier_target(kind: IdentifierTargetKind, value: &str) -> Result<ObjectRef> {
    match kind {
        IdentifierTargetKind::Asset => AssetId::from_str(value)
            .map(ObjectRef::Asset)
            .context("parse asset ID"),
        IdentifierTargetKind::Representation => RepresentationId::from_str(value)
            .map(ObjectRef::Representation)
            .context("parse representation ID"),
        IdentifierTargetKind::Resource => ResourceId::from_str(value)
            .map(ObjectRef::Resource)
            .context("parse resource ID"),
    }
}

fn parse_metadata_target(kind: MetadataTargetKind, value: &str) -> Result<ObjectRef> {
    match kind {
        MetadataTargetKind::Production => ProductionId::from_str(value)
            .map(ObjectRef::Production)
            .context("parse production ID"),
        MetadataTargetKind::Asset => AssetId::from_str(value)
            .map(ObjectRef::Asset)
            .context("parse asset ID"),
        MetadataTargetKind::Representation => RepresentationId::from_str(value)
            .map(ObjectRef::Representation)
            .context("parse representation ID"),
        MetadataTargetKind::Resource => ResourceId::from_str(value)
            .map(ObjectRef::Resource)
            .context("parse resource ID"),
        MetadataTargetKind::Activity => ActivityId::from_str(value)
            .map(ObjectRef::Activity)
            .context("parse activity ID"),
    }
}

fn parse_metadata_property(vocabulary: String, property: String) -> Result<MetadataProperty> {
    Ok(MetadataProperty::new(
        VocabularyId::new(vocabulary).context("validate metadata vocabulary")?,
        PropertyId::new(property).context("validate metadata property")?,
    ))
}

impl MetadataValueInput {
    fn into_value(self) -> Result<MetadataValue> {
        match self {
            Self::String { value } => MetadataValue::string(value).context("validate string"),
            Self::LangString { value, language } => {
                MetadataValue::language_string(value, language).context("validate language string")
            }
            Self::I64 { value } => Ok(MetadataValue::i64(value)),
            Self::U64 { value } => Ok(MetadataValue::u64(value)),
            Self::Decimal { coefficient, scale } => {
                let coefficient = coefficient
                    .parse::<i128>()
                    .context("parse decimal coefficient")?;
                Ok(MetadataValue::decimal(
                    DecimalValue::new(coefficient, scale).context("validate decimal")?,
                ))
            }
            Self::Bool { value } => Ok(MetadataValue::boolean(value)),
            Self::Timestamp { unix_micros } => Ok(MetadataValue::timestamp(
                Timestamp::from_unix_micros(unix_micros),
            )),
            Self::Uri { value } => MetadataValue::uri(value).context("validate URI"),
            Self::Bytes { hex } => {
                MetadataValue::bytes(hex::decode(hex).context("decode hexadecimal metadata bytes")?)
                    .context("validate bytes")
            }
            Self::Rational {
                numerator,
                denominator,
            } => Ok(MetadataValue::rational(
                RationalValue::new(numerator, denominator).context("validate rational")?,
            )),
            Self::List { values } => MetadataValue::list(
                values
                    .into_iter()
                    .map(Self::into_value)
                    .collect::<Result<Vec<_>>>()?,
            )
            .context("validate list"),
            Self::Struct { fields } => MetadataValue::structure(
                fields
                    .into_iter()
                    .map(MetadataFieldInput::into_field)
                    .collect::<Result<Vec<_>>>()?,
            )
            .context("validate structure"),
            Self::Reference { target } => Ok(MetadataValue::reference(parse_metadata_target(
                target.target_kind,
                &target.target_id,
            )?)),
        }
    }
}

impl MetadataFieldInput {
    fn into_field(self) -> Result<MetadataField> {
        Ok(MetadataField::new(
            PropertyId::new(self.name).context("validate metadata field name")?,
            self.value.into_value()?,
        ))
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
        ObjectRef::Production(id) => Ok(ObjectRefView {
            kind: "production",
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
        ObjectRef::Resource(id) => Ok(ObjectRefView {
            kind: "resource",
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

fn find_asset(production: &SqliteProduction, asset_id: AssetId) -> Result<Asset> {
    production
        .assets()
        .context("load assets")?
        .into_iter()
        .find(|asset| asset.id() == asset_id)
        .with_context(|| format!("asset does not exist: {asset_id}"))
}

fn asset_view(production: &SqliteProduction, asset: &Asset) -> Result<AssetView> {
    let mut representations = Vec::new();
    for representation in production
        .representations(asset.id())
        .context("load asset representations")?
    {
        let mut resources = Vec::new();
        for resource in production
            .resources(representation.id())
            .context("load representation resources")?
        {
            let locators = production
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

impl From<&InventoryReport> for InventoryView {
    fn from(report: &InventoryReport) -> Self {
        let stats = report.stats();
        Self {
            items: report
                .items()
                .iter()
                .map(|item| InventoryItemView {
                    category: inventory_category(item.category()),
                    representation_id: item.representation_id().map(|id| id.to_string()),
                    resource_id: item.resource_id().map(|id| id.to_string()),
                    uri: item.uri().map(str::to_owned),
                    detail: item.detail().map(str::to_owned),
                })
                .collect(),
            stats: InventoryStatsView {
                entries_visited: stats.entries_visited,
                fingerprints_computed: stats.fingerprints_computed,
                fingerprint_cache_hits: stats.fingerprint_cache_hits,
                cache_rebuilt: stats.cache_rebuilt,
            },
        }
    }
}

impl From<&RepresentationResolution> for ResolutionView {
    fn from(resolution: &RepresentationResolution) -> Self {
        Self {
            representation_id: resolution.representation_id().to_string(),
            availability: representation_availability(resolution.availability()),
            resources: resolution
                .resources()
                .iter()
                .map(ResourceResolutionView::from)
                .collect(),
            issues: resolution
                .issues()
                .iter()
                .map(AvailabilityIssueView::from)
                .collect(),
        }
    }
}

impl From<&ResourceResolution> for ResourceResolutionView {
    fn from(resolution: &ResourceResolution) -> Self {
        Self {
            resource_id: resolution.resource_id().to_string(),
            state: resource_resolution_state(resolution.state()),
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

impl From<&AvailabilityIssue> for AvailabilityIssueView {
    fn from(issue: &AvailabilityIssue) -> Self {
        Self {
            resource_id: issue.resource_id().to_string(),
            required: issue.is_required(),
            kind: availability_issue_kind(issue.kind()),
            frames: issue.frames().to_vec(),
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

fn set_cli_revision_context(
    transaction: &mut impl ProductionStoreTransaction,
    message: &str,
) -> Result<()> {
    let origin = OriginIdentity::new(
        "postproject-cli",
        Some(env!("CARGO_PKG_VERSION").to_owned()),
        None,
    )
    .context("build CLI revision origin")?;
    let context = RevisionContext::new(Some(origin), Some(message.to_owned()))
        .context("build CLI revision context")?;
    transaction
        .set_revision_context(context)
        .context("set CLI revision context")
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

const fn inventory_category(category: InventoryCategory) -> &'static str {
    match category {
        InventoryCategory::KnownOnline => "known_online",
        InventoryCategory::Partial => "partial",
        InventoryCategory::Missing => "missing",
        InventoryCategory::NewCandidate => "new_candidate",
        InventoryCategory::Changed => "changed",
        InventoryCategory::DuplicateCandidate => "duplicate_candidate",
        InventoryCategory::AmbiguousRelinkCandidate => "ambiguous_relink_candidate",
        InventoryCategory::RootUnmapped => "root_unmapped",
        InventoryCategory::RootUnavailable => "root_unavailable",
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

const fn resource_resolution_state(state: ResourceResolutionState) -> &'static str {
    match state {
        ResourceResolutionState::OnlineAtKnownLocator => "online_at_known_locator",
        ResourceResolutionState::ResolvedExact => "resolved_exact",
        ResourceResolutionState::ResolvedProbable => "resolved_probable",
        ResourceResolutionState::Offline => "offline",
        ResourceResolutionState::Ambiguous => "ambiguous",
        ResourceResolutionState::Error => "error",
        _ => "unknown",
    }
}

const fn representation_availability(state: RepresentationAvailability) -> &'static str {
    match state {
        RepresentationAvailability::Online => "online",
        RepresentationAvailability::Partial => "partial",
        RepresentationAvailability::Offline => "offline",
        RepresentationAvailability::Ambiguous => "ambiguous",
        RepresentationAvailability::Error => "error",
        _ => "unknown",
    }
}

const fn availability_issue_kind(kind: AvailabilityIssueKind) -> &'static str {
    match kind {
        AvailabilityIssueKind::OfflineResource => "offline_resource",
        AvailabilityIssueKind::AmbiguousResource => "ambiguous_resource",
        AvailabilityIssueKind::ResourceError => "resource_error",
        AvailabilityIssueKind::MissingFrames => "missing_frames",
        _ => "unknown",
    }
}

const fn evidence_kind(kind: EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::KnownLocatorAvailable => "known_locator_available",
        EvidenceKind::ExactFingerprintMatch => "exact_fingerprint_match",
        EvidenceKind::FullHashMatch => "full_hash_match",
        EvidenceKind::PartialFingerprintMatch => "partial_fingerprint_match",
        EvidenceKind::FileSizeMatch => "file_size_match",
        EvidenceKind::FileNameMatch => "file_name_match",
        EvidenceKind::RelativePathSimilarity => "relative_path_similarity",
        EvidenceKind::MediaRootRelation => "media_root_relation",
        EvidenceKind::MediaRootUnmapped => "media_root_unmapped",
        EvidenceKind::MediaRootUnavailable => "media_root_unavailable",
        EvidenceKind::ConflictingCandidate => "conflicting_candidate",
        EvidenceKind::DiscoveryError => "discovery_error",
        EvidenceKind::FingerprintMismatch => "fingerprint_mismatch",
        _ => "unknown",
    }
}
