//! Public C ABI for `PostProject`.
//!
//! All exported calls contain Rust panics and translate domain errors into stable
//! numeric codes plus owned error objects. Native consumers should include the
//! shipped `postproject.h` rather than depending on Rust declarations.

mod metadata;
mod metadata_input;
mod provenance;
mod representations;
mod revision_events;
mod revisions;

use std::{
    any::Any,
    ffi::{CStr, CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    ptr,
    str::FromStr,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
};

use postproject_core::{
    Activity, ActivityId, ActivityInput, ActivityKind, ActivityOutput, ActivityRole, AgentIdentity,
    AssetId, AvailabilityIssue, AvailabilityIssueKind, Error, ErrorKind, EvidenceKind,
    ExternalIdentifier, HostObjectBinding, IdentifierScheme, Locator, MAX_ACTIVITY_EDGES,
    MediaRoot, MetadataProperty, MetadataValue, ObjectRef, OriginIdentity, OriginalMediaImport,
    ProductionId, PropertyId, RepresentationAvailability, RepresentationId,
    RepresentationResolution, ResolutionEvidence, ResourceId, ResourceResolutionState,
    RevisionContext, RevisionId, Timestamp, ToolIdentity, TransactionLifecycle, VocabularyId,
};
use postproject_media::{
    MediaResolver, prepare_confirmed_locator, prepare_media_root, prepare_original_media,
};
use postproject_storage_sqlite::SqliteProduction;

use metadata::AbiMetadataValue;
pub use metadata::{PpMetadataSet, PpMetadataValue};
pub use metadata_input::PpMetadataInput;
use provenance::AbiActivityEdge;
pub use provenance::PpActivitySet;
pub use representations::PpRepresentationSet;
pub use revision_events::PpRevisionEventSet;
pub use revisions::PpRevisionSet;

const PP_OK: u32 = 0;
const PP_ERROR_INVALID_ARGUMENT: u32 = 1;
const PP_ERROR_NOT_FOUND: u32 = 2;
const PP_ERROR_ALREADY_EXISTS: u32 = 3;
const PP_ERROR_IO: u32 = 4;
const PP_ERROR_STORAGE: u32 = 5;
const PP_ERROR_MIGRATION: u32 = 6;
const PP_ERROR_CONFLICT: u32 = 7;
const PP_ERROR_AMBIGUOUS_RESOLUTION: u32 = 8;
const PP_ERROR_FINGERPRINT: u32 = 9;
const PP_ERROR_UNSUPPORTED: u32 = 10;
const PP_ERROR_INTERNAL: u32 = 255;

const PP_RESOURCE_ONLINE_AT_KNOWN_LOCATOR: u32 = 1;
const PP_RESOURCE_RESOLVED_EXACT: u32 = 2;
const PP_RESOURCE_RESOLVED_PROBABLE: u32 = 3;
const PP_RESOURCE_OFFLINE: u32 = 4;
const PP_RESOURCE_AMBIGUOUS: u32 = 5;
const PP_RESOURCE_RESOLUTION_ERROR: u32 = 6;

const PP_AVAILABILITY_ONLINE: u32 = 1;
const PP_AVAILABILITY_PARTIAL: u32 = 2;
const PP_AVAILABILITY_OFFLINE: u32 = 3;
const PP_AVAILABILITY_AMBIGUOUS: u32 = 4;
const PP_AVAILABILITY_ERROR: u32 = 5;

const PP_AVAILABILITY_ISSUE_OFFLINE_RESOURCE: u32 = 1;
const PP_AVAILABILITY_ISSUE_AMBIGUOUS_RESOURCE: u32 = 2;
const PP_AVAILABILITY_ISSUE_RESOURCE_ERROR: u32 = 3;
const PP_AVAILABILITY_ISSUE_MISSING_FRAMES: u32 = 4;

const PP_EVIDENCE_KNOWN_LOCATOR_AVAILABLE: u32 = 1;
const PP_EVIDENCE_EXACT_FINGERPRINT_MATCH: u32 = 2;
const PP_EVIDENCE_FULL_HASH_MATCH: u32 = 3;
const PP_EVIDENCE_PARTIAL_FINGERPRINT_MATCH: u32 = 4;
const PP_EVIDENCE_FILE_SIZE_MATCH: u32 = 5;
const PP_EVIDENCE_FILE_NAME_MATCH: u32 = 6;
const PP_EVIDENCE_RELATIVE_PATH_SIMILARITY: u32 = 7;
const PP_EVIDENCE_MEDIA_ROOT_RELATION: u32 = 8;
const PP_EVIDENCE_CONFLICTING_CANDIDATE: u32 = 9;
const PP_EVIDENCE_DISCOVERY_ERROR: u32 = 10;

const PP_OBJECT_PRODUCTION: u32 = 1;
const PP_OBJECT_ASSET: u32 = 2;
const PP_OBJECT_REPRESENTATION: u32 = 3;
const PP_OBJECT_RESOURCE: u32 = 4;
const PP_OBJECT_ACTIVITY: u32 = 5;

const PP_REVISION_ASSET_IMPORTED: u32 = 1;
const PP_REVISION_REPRESENTATION_ADDED: u32 = 2;
const PP_REVISION_RESOURCE_ADDED: u32 = 3;
const PP_REVISION_REPRESENTATION_RESOURCE_ADDED: u32 = 4;
const PP_REVISION_LOCATOR_ADDED: u32 = 5;
const PP_REVISION_MEDIA_ROOT_ADDED: u32 = 6;
const PP_REVISION_EXTERNAL_IDENTIFIER_ADDED: u32 = 7;
const PP_REVISION_EXTERNAL_IDENTIFIER_REMOVED: u32 = 8;
const PP_REVISION_METADATA_ADDED_OR_REPLACED: u32 = 9;
const PP_REVISION_METADATA_REMOVED: u32 = 10;
const PP_REVISION_ACTIVITY_CREATED: u32 = 11;
const PP_REVISION_ACTIVITY_INPUT_ADDED: u32 = 12;
const PP_REVISION_ACTIVITY_OUTPUT_ADDED: u32 = 13;

/// Current pre-1.0 ABI version.
pub const ABI_VERSION: u32 = 10;

/// Fixed-layout UUID-compatible public identifier.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PpUuid {
    /// UUID bytes in network order.
    pub bytes: [u8; 16],
}

/// Fixed-layout typed reference to a `PostProject` object.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PpObjectRef {
    /// One of the `PP_OBJECT_*` constants from the public header.
    pub kind: u32,
    /// Stable ID whose interpretation is selected by `kind`.
    pub id: PpUuid,
}

/// Borrowed activity edge supplied by a C caller.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PpActivityEdge {
    /// Representation consumed or produced by the activity.
    pub representation_id: PpUuid,
    /// Optional NUL-terminated namespaced role.
    pub role: *const c_char,
}

/// Borrowed, fixed-layout semantic revision event.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PpRevisionEvent {
    /// One of the `PP_REVISION_*` constants from the public header.
    pub kind: u32,
    /// Stable zero-based position within the owning revision.
    pub position: u32,
    /// Event asset identity, or zero when not applicable.
    pub asset_id: PpUuid,
    /// Event representation identity, or zero when not applicable.
    pub representation_id: PpUuid,
    /// Event resource identity, or zero when not applicable.
    pub resource_id: PpUuid,
    /// Event locator identity, or zero when not applicable.
    pub locator_id: PpUuid,
    /// Event media-root identity, or zero when not applicable.
    pub media_root_id: PpUuid,
    /// Event activity identity, or zero when not applicable.
    pub activity_id: PpUuid,
    /// Metadata/identifier target, with kind zero when not applicable.
    pub target: PpObjectRef,
    /// Structural member position for representation-resource events.
    pub structural_position: u32,
    /// Borrowed identifier scheme, or null when not applicable.
    pub identifier_scheme: *const c_char,
    /// Borrowed identifier value, or null when not applicable.
    pub identifier_value: *const c_char,
    /// Borrowed optional identifier qualifier.
    pub identifier_qualifier: *const c_char,
    /// Borrowed metadata vocabulary, or null when not applicable.
    pub vocabulary: *const c_char,
    /// Borrowed metadata property, or null when not applicable.
    pub property: *const c_char,
    /// Borrowed activity kind, or null when not applicable.
    pub activity_kind: *const c_char,
    /// Borrowed activity-edge role, or null when absent/not applicable.
    pub role: *const c_char,
}

/// Opaque production handle owned by the C caller.
pub struct PpProduction {
    state: Arc<ProductionState>,
}

struct ProductionState {
    inner: Mutex<SqliteProduction>,
    transaction_open: AtomicBool,
}

/// Opaque transaction handle owned by the C caller.
pub struct PpTransaction {
    state: Arc<ProductionState>,
    lifecycle: TransactionLifecycle,
    revision_context: RevisionContext,
    mutations: Vec<StagedMutation>,
}

enum StagedMutation {
    Import(OriginalMediaImport),
    MediaRoot(MediaRoot),
    Locator(Locator),
    AddExternalIdentifier(ObjectRef, ExternalIdentifier),
    RemoveExternalIdentifier(ObjectRef, ExternalIdentifier),
    AddMetadataValue(ObjectRef, MetadataProperty, MetadataValue),
    RemoveMetadataProperty(ObjectRef, MetadataProperty),
    Activity(Activity),
}

/// Opaque immutable external-identifier result set owned by the C caller.
pub struct PpExternalIdentifierSet {
    identifiers: Vec<AbiExternalIdentifier>,
}

struct AbiExternalIdentifier {
    scheme: CString,
    value: CString,
    qualifier: Option<CString>,
}

/// Opaque immutable object-reference result set owned by the C caller.
pub struct PpObjectRefSet {
    objects: Vec<PpObjectRef>,
}

/// Opaque set of immutable media-resolution results owned by the C caller.
pub struct PpResolutionSet {
    representations: Vec<AbiRepresentationResolution>,
}

struct AbiRepresentationResolution {
    representation_id: RepresentationId,
    availability: RepresentationAvailability,
    resources: Vec<AbiResolution>,
    issues: Vec<AvailabilityIssue>,
}

struct AbiResolution {
    resource_id: ResourceId,
    state: u32,
    candidates: Vec<AbiCandidate>,
    evidence: Vec<AbiEvidence>,
}

struct AbiCandidate {
    uri: CString,
    confidence: u16,
    evidence: Vec<AbiEvidence>,
}

struct AbiEvidence {
    kind: u32,
    detail: Option<CString>,
}

/// Opaque error object owned by the C caller.
pub struct PpError {
    code: u32,
    message: CString,
}

/// Returns the ABI version implemented by this shared library.
#[unsafe(no_mangle)]
pub extern "C" fn pp_abi_version() -> u32 {
    ABI_VERSION
}

/// Formats a caller-owned portable host-object binding.
///
/// # Safety
///
/// `production_id` and `object` must be readable. `out_binding` must be
/// writable and receives a string that must be released exactly once with
/// [`pp_host_binding_release`]. `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_host_binding_format(
    production_id: *const PpUuid,
    object: *const PpObjectRef,
    out_binding: *mut *mut c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and inputs checked before dereference.
    unsafe {
        initialize_output(out_binding);
        ffi_call(out_error, || {
            let production_id = production_id
                .as_ref()
                .ok_or_else(|| invalid_argument("production_id must not be null"))?;
            let object = object
                .as_ref()
                .ok_or_else(|| invalid_argument("object must not be null"))?;
            require_output(out_binding, "out_binding")?;
            let binding = HostObjectBinding::new(
                ProductionId::from_bytes(production_id.bytes),
                object_ref_from_abi(*object)?,
            )?;
            let binding = exact_cstring(&binding.to_string(), "host binding")?;
            out_binding.write(binding.into_raw());
            Ok(())
        })
    }
}

/// Parses a portable host-object binding into caller-owned value outputs.
///
/// # Safety
///
/// `binding` must be NUL-terminated UTF-8 for this call. Both value outputs
/// must be writable. `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_host_binding_parse(
    binding: *const c_char,
    out_production_id: *mut PpUuid,
    out_object: *mut PpObjectRef,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and all pointers checked before use.
    unsafe {
        initialize_uuid(out_production_id);
        initialize_object_ref(out_object);
        ffi_call(out_error, || {
            require_output(out_production_id, "out_production_id")?;
            require_output(out_object, "out_object")?;
            let binding = HostObjectBinding::from_str(required_utf8(binding, "binding")?)?;
            out_production_id.write(uuid(binding.production_id()));
            out_object.write(object_ref_to_abi(binding.object())?);
            Ok(())
        })
    }
}

/// Releases a string returned by [`pp_host_binding_format`].
///
/// # Safety
///
/// `binding` must be null or a live pointer returned by
/// [`pp_host_binding_format`] that has not already been released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_host_binding_release(binding: *mut c_char) {
    if !binding.is_null() {
        // SAFETY: The caller contract requires the exact pointer and ownership
        // originating from `CString::into_raw` above.
        drop(unsafe { CString::from_raw(binding) });
    }
}

/// Creates a new production file.
///
/// # Safety
///
/// `path` must point to a NUL-terminated byte string for the duration of the
/// call. `display_name` may be null or must satisfy the same rule. `out_production`
/// must be a writable pointer. `out_error` may be null or writable. Successful
/// handles must be released exactly once with [`pp_production_release`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_create(
    path: *const c_char,
    display_name: *const c_char,
    out_production: *mut *mut PpProduction,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: The caller contract for each pointer is documented above. Helpers
    // validate nullability before dereferencing and borrow inputs only this call.
    unsafe {
        initialize_output(out_production);
        ffi_call(out_error, || {
            if out_production.is_null() {
                return Err(invalid_argument("out_production must not be null"));
            }
            let path = required_utf8(path, "path")?;
            if path.is_empty() {
                return Err(invalid_argument("path must not be empty"));
            }
            let display_name = optional_utf8(display_name, "display_name")?.map(str::to_owned);
            let production = SqliteProduction::create(Path::new(path), display_name)?;
            out_production.write(Box::into_raw(Box::new(production_handle(production))));
            Ok(())
        })
    }
}

/// Opens an existing production file.
///
/// # Safety
///
/// `path` must point to a NUL-terminated byte string for the duration of the
/// call. `out_production` must be writable. `out_error` may be null or writable.
/// Successful handles must be released exactly once with [`pp_production_release`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_open(
    path: *const c_char,
    out_production: *mut *mut PpProduction,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: The caller contract for each pointer is documented above. Helpers
    // validate nullability before dereferencing and borrow inputs only this call.
    unsafe {
        initialize_output(out_production);
        ffi_call(out_error, || {
            if out_production.is_null() {
                return Err(invalid_argument("out_production must not be null"));
            }
            let path = required_utf8(path, "path")?;
            if path.is_empty() {
                return Err(invalid_argument("path must not be empty"));
            }
            let production = SqliteProduction::open(Path::new(path))?;
            out_production.write(Box::into_raw(Box::new(production_handle(production))));
            Ok(())
        })
    }
}

/// Copies the stable production identity into caller-owned storage.
///
/// # Safety
///
/// `production` must be a live handle returned by this library. `out_id` must be
/// writable. `out_error` may be null or writable. The production must not be used
/// concurrently by another thread during the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_id(
    production: *const PpProduction,
    out_id: *mut PpUuid,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Null pointers are rejected before dereference; non-null pointer
    // validity and synchronization are guaranteed by the caller contract.
    unsafe {
        ffi_call(out_error, || {
            let production = production
                .as_ref()
                .ok_or_else(|| invalid_argument("production must not be null"))?;
            if out_id.is_null() {
                return Err(invalid_argument("out_id must not be null"));
            }
            let inner = lock_production(&production.state);
            out_id.write(uuid(inner.production().id()));
            Ok(())
        })
    }
}

/// Reports whether a stable asset identity exists in a production.
///
/// # Safety
///
/// `production` must be a live handle returned by this library. `asset_id` must be
/// readable and `out_exists` writable. `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_asset_exists(
    production: *const PpProduction,
    asset_id: *const PpUuid,
    out_exists: *mut u8,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Null pointers are rejected before dereference; non-null pointer
    // validity and synchronization are guaranteed by the caller contract.
    unsafe {
        if !out_exists.is_null() {
            out_exists.write(0);
        }
        ffi_call(out_error, || {
            let production = production
                .as_ref()
                .ok_or_else(|| invalid_argument("production must not be null"))?;
            let asset_id = asset_id
                .as_ref()
                .ok_or_else(|| invalid_argument("asset_id must not be null"))?;
            if out_exists.is_null() {
                return Err(invalid_argument("out_exists must not be null"));
            }
            let inner = lock_production(&production.state);
            let expected = AssetId::from_bytes(asset_id.bytes);
            let exists = inner.assets()?.iter().any(|asset| asset.id() == expected);
            out_exists.write(u8::from(exists));
            Ok(())
        })
    }
}

/// Loads external identifiers attached to one typed object reference.
///
/// Strings returned by result accessors are borrowed until the result set is
/// released.
///
/// # Safety
///
/// `production` and `target` must be readable live values. `out_identifiers` must
/// be writable. `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_external_identifiers(
    production: *const PpProduction,
    target: *const PpObjectRef,
    out_identifiers: *mut *mut PpExternalIdentifierSet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Pointers are validated before use and outputs are initialized.
    unsafe {
        initialize_output(out_identifiers);
        ffi_call(out_error, || {
            let production = production
                .as_ref()
                .ok_or_else(|| invalid_argument("production must not be null"))?;
            let target = target
                .as_ref()
                .ok_or_else(|| invalid_argument("target must not be null"))?;
            if out_identifiers.is_null() {
                return Err(invalid_argument("out_identifiers must not be null"));
            }
            let target = object_ref_from_abi(*target)?;
            let inner = lock_production(&production.state);
            let identifiers = inner
                .external_identifiers(target)?
                .into_iter()
                .map(AbiExternalIdentifier::try_from)
                .collect::<Result<Vec<_>, _>>()?;
            out_identifiers.write(Box::into_raw(Box::new(PpExternalIdentifierSet {
                identifiers,
            })));
            Ok(())
        })
    }
}

/// Finds objects carrying an exact external identifier scheme and value.
///
/// # Safety
///
/// `production` must be live; `scheme` and `value` must be borrowed NUL-terminated
/// UTF-8 strings; `out_objects` must be writable; and `out_error` may be null or
/// writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_find_by_external_identifier(
    production: *const PpProduction,
    scheme: *const c_char,
    value: *const c_char,
    out_objects: *mut *mut PpObjectRefSet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Pointers are validated before use and outputs are initialized.
    unsafe {
        initialize_output(out_objects);
        ffi_call(out_error, || {
            let production = production
                .as_ref()
                .ok_or_else(|| invalid_argument("production must not be null"))?;
            if out_objects.is_null() {
                return Err(invalid_argument("out_objects must not be null"));
            }
            let scheme = IdentifierScheme::new(required_utf8(scheme, "scheme")?)?;
            let value = required_utf8(value, "value")?;
            let inner = lock_production(&production.state);
            let objects = inner
                .find_by_external_identifier(&scheme, value)?
                .into_iter()
                .map(object_ref_to_abi)
                .collect::<Result<Vec<_>, _>>()?;
            out_objects.write(Box::into_raw(Box::new(PpObjectRefSet { objects })));
            Ok(())
        })
    }
}

/// Returns the number of values in an external-identifier result set.
/// Null input returns zero.
///
/// # Safety
///
/// `identifiers` must be null or a live handle returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_external_identifier_set_count(
    identifiers: *const PpExternalIdentifierSet,
) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: A non-null handle is live by the caller contract.
        unsafe { identifiers.as_ref() }.map_or(0, |set| {
            u64::try_from(set.identifiers.len()).unwrap_or(u64::MAX)
        })
    }))
    .unwrap_or(0)
}

/// Reads one external identifier. Returned strings are borrowed from the set.
///
/// # Safety
///
/// `identifiers` must be live; outputs must be writable; and `out_error` may be
/// null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_external_identifier_set_get(
    identifiers: *const PpExternalIdentifierSet,
    index: u64,
    out_scheme: *mut *const c_char,
    out_value: *mut *const c_char,
    out_qualifier: *mut *const c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and checked before writes.
    unsafe {
        initialize_const_output(out_scheme);
        initialize_const_output(out_value);
        initialize_const_output(out_qualifier);
        ffi_call(out_error, || {
            require_output(out_scheme, "out_scheme")?;
            require_output(out_value, "out_value")?;
            require_output(out_qualifier, "out_qualifier")?;
            let identifiers = identifiers
                .as_ref()
                .ok_or_else(|| invalid_argument("identifiers must not be null"))?;
            let identifier = item_at(&identifiers.identifiers, index, "identifier")?;
            out_scheme.write(identifier.scheme.as_ptr());
            out_value.write(identifier.value.as_ptr());
            out_qualifier.write(
                identifier
                    .qualifier
                    .as_ref()
                    .map_or(ptr::null(), |value| value.as_ptr()),
            );
            Ok(())
        })
    }
}

/// Releases an external-identifier result set. Null is a no-op.
///
/// # Safety
///
/// A non-null handle must be live and released exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_external_identifier_set_release(
    identifiers: *mut PpExternalIdentifierSet,
) {
    if identifiers.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: Ownership is transferred back exactly once by contract.
        drop(unsafe { Box::from_raw(identifiers) });
    }));
}

/// Returns the number of values in an object-reference result set.
/// Null input returns zero.
///
/// # Safety
///
/// `objects` must be null or a live handle returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_object_ref_set_count(objects: *const PpObjectRefSet) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: A non-null handle is live by the caller contract.
        unsafe { objects.as_ref() }.map_or(0, |set| {
            u64::try_from(set.objects.len()).unwrap_or(u64::MAX)
        })
    }))
    .unwrap_or(0)
}

/// Reads one typed object reference.
///
/// # Safety
///
/// `objects` must be live; `out_object` must be writable; and `out_error` may
/// be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_object_ref_set_get(
    objects: *const PpObjectRefSet,
    index: u64,
    out_object: *mut PpObjectRef,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and checked before writes.
    unsafe {
        initialize_object_ref(out_object);
        ffi_call(out_error, || {
            require_output(out_object, "out_object")?;
            let objects = objects
                .as_ref()
                .ok_or_else(|| invalid_argument("objects must not be null"))?;
            out_object.write(*item_at(&objects.objects, index, "object")?);
            Ok(())
        })
    }
}

/// Releases an object-reference result set. Null is a no-op.
///
/// # Safety
///
/// A non-null handle must be live and released exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_object_ref_set_release(objects: *mut PpObjectRefSet) {
    if objects.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: Ownership is transferred back exactly once by contract.
        drop(unsafe { Box::from_raw(objects) });
    }));
}

/// Loads typed metadata assertions attached to one object.
///
/// Returned strings and value pointers are borrowed until the result set is
/// released.
///
/// # Safety
///
/// `production` and `target` must be readable live values. `out_metadata` must be
/// writable and `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_metadata(
    production: *const PpProduction,
    target: *const PpObjectRef,
    out_metadata: *mut *mut PpMetadataSet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Inputs are validated before use and output ownership is explicit.
    unsafe {
        initialize_output(out_metadata);
        ffi_call(out_error, || {
            let production = production
                .as_ref()
                .ok_or_else(|| invalid_argument("production must not be null"))?;
            let target = target
                .as_ref()
                .ok_or_else(|| invalid_argument("target must not be null"))?;
            require_output(out_metadata, "out_metadata")?;
            let target = object_ref_from_abi(*target)?;
            let inner = lock_production(&production.state);
            let metadata = PpMetadataSet::from_assertions(target, &inner.metadata(target)?)?;
            out_metadata.write(Box::into_raw(Box::new(metadata)));
            Ok(())
        })
    }
}

/// Finds every assertion using one exact vocabulary and property.
///
/// # Safety
///
/// `production` must be live, strings must be borrowed NUL-terminated UTF-8,
/// `out_metadata` must be writable, and `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_find_metadata(
    production: *const PpProduction,
    vocabulary: *const c_char,
    property: *const c_char,
    out_metadata: *mut *mut PpMetadataSet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Inputs are validated before use and output ownership is explicit.
    unsafe {
        initialize_output(out_metadata);
        ffi_call(out_error, || {
            let production = production
                .as_ref()
                .ok_or_else(|| invalid_argument("production must not be null"))?;
            require_output(out_metadata, "out_metadata")?;
            let property = metadata_property_from_abi(vocabulary, property)?;
            let inner = lock_production(&production.state);
            let metadata =
                PpMetadataSet::from_matches(&inner.query_by_metadata_property(&property)?)?;
            out_metadata.write(Box::into_raw(Box::new(metadata)));
            Ok(())
        })
    }
}

/// Returns the number of assertions in a metadata result set. Null returns zero.
///
/// # Safety
///
/// `metadata` must be null or a live result-set handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_set_count(metadata: *const PpMetadataSet) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: A non-null handle is live by the caller contract.
        unsafe { metadata.as_ref() }.map_or(0, |set| {
            u64::try_from(set.assertions.len()).unwrap_or(u64::MAX)
        })
    }))
    .unwrap_or(0)
}

/// Reads one assertion and a borrowed pointer to its recursive value.
///
/// # Safety
///
/// `metadata` must be live. Every output must be writable and `out_error` may
/// be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_set_get(
    metadata: *const PpMetadataSet,
    index: u64,
    out_target: *mut PpObjectRef,
    out_vocabulary: *mut *const c_char,
    out_property: *mut *const c_char,
    out_value: *mut *const PpMetadataValue,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and checked before writes.
    unsafe {
        initialize_object_ref(out_target);
        initialize_const_output(out_vocabulary);
        initialize_const_output(out_property);
        initialize_const_output(out_value);
        ffi_call(out_error, || {
            require_output(out_target, "out_target")?;
            require_output(out_vocabulary, "out_vocabulary")?;
            require_output(out_property, "out_property")?;
            require_output(out_value, "out_value")?;
            let metadata = metadata
                .as_ref()
                .ok_or_else(|| invalid_argument("metadata must not be null"))?;
            let assertion = item_at(&metadata.assertions, index, "metadata assertion")?;
            out_target.write(assertion.target);
            out_vocabulary.write(assertion.vocabulary.as_ptr());
            out_property.write(assertion.property.as_ptr());
            out_value.write(ptr::from_ref(&assertion.value));
            Ok(())
        })
    }
}

/// Releases a metadata result set. Null is a no-op.
///
/// # Safety
///
/// A non-null pointer must be live and released exactly once. No borrowed value
/// or string from the set may be used after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_set_release(metadata: *mut PpMetadataSet) {
    if metadata.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: Ownership is transferred back exactly once by contract.
        drop(unsafe { Box::from_raw(metadata) });
    }));
}

/// Loads every production activity in deterministic identity order.
///
/// # Safety
///
/// `production` must be live, `out_activities` must be writable, and `out_error`
/// may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_activities(
    production: *const PpProduction,
    out_activities: *mut *mut PpActivitySet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Inputs are validated before use and output ownership is explicit.
    unsafe {
        initialize_output(out_activities);
        ffi_call(out_error, || {
            let production = production
                .as_ref()
                .ok_or_else(|| invalid_argument("production must not be null"))?;
            require_output(out_activities, "out_activities")?;
            let inner = lock_production(&production.state);
            let activities = PpActivitySet::new(&inner.activities()?)?;
            out_activities.write(Box::into_raw(Box::new(activities)));
            Ok(())
        })
    }
}

/// Loads activities that produce one representation.
///
/// # Safety
///
/// All pointers must follow the same rules as [`pp_production_activities`], and
/// `representation_id` must be readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_activities_producing(
    production: *const PpProduction,
    representation_id: *const PpUuid,
    out_activities: *mut *mut PpActivitySet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: The shared helper validates every pointer before use.
    unsafe {
        production_activities_for_representation(
            production,
            representation_id,
            ActivityRelation::Producing,
            out_activities,
            out_error,
        )
    }
}

/// Loads activities that consume one representation.
///
/// # Safety
///
/// Pointer rules match [`pp_production_activities_producing`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_activities_consuming(
    production: *const PpProduction,
    representation_id: *const PpUuid,
    out_activities: *mut *mut PpActivitySet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: The shared helper validates every pointer before use.
    unsafe {
        production_activities_for_representation(
            production,
            representation_id,
            ActivityRelation::Consuming,
            out_activities,
            out_error,
        )
    }
}

/// Loads every transitive provenance ancestor as representation references.
///
/// # Safety
///
/// `production` and `representation_id` must be readable live values,
/// `out_representations` must be writable, and `out_error` may be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_provenance_ancestors(
    production: *const PpProduction,
    representation_id: *const PpUuid,
    out_representations: *mut *mut PpObjectRefSet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: The shared helper validates every pointer before use.
    unsafe {
        production_provenance_relatives(
            production,
            representation_id,
            ProvenanceDirection::Ancestors,
            out_representations,
            out_error,
        )
    }
}

/// Loads every transitive provenance descendant as representation references.
///
/// # Safety
///
/// Pointer rules match [`pp_production_provenance_ancestors`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_provenance_descendants(
    production: *const PpProduction,
    representation_id: *const PpUuid,
    out_representations: *mut *mut PpObjectRefSet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: The shared helper validates every pointer before use.
    unsafe {
        production_provenance_relatives(
            production,
            representation_id,
            ProvenanceDirection::Descendants,
            out_representations,
            out_error,
        )
    }
}

/// Returns the number of activities in a result set. Null returns zero.
///
/// # Safety
///
/// `activities` must be null or a live result-set handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_activity_set_count(activities: *const PpActivitySet) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: A non-null handle is live by the caller contract.
        unsafe { activities.as_ref() }.map_or(0, |set| {
            u64::try_from(set.activities.len()).unwrap_or(u64::MAX)
        })
    }))
    .unwrap_or(0)
}

/// Reads one activity's identity, kind, timing, and edge counts.
///
/// Strings are borrowed until the result set is released. Optional timestamps
/// have explicit presence flags and zero values when absent.
///
/// # Safety
///
/// `activities` must be live. Every output must be writable and `out_error`
/// may be null or writable.
#[unsafe(no_mangle)]
#[allow(
    clippy::too_many_arguments,
    reason = "flat C output parameters are ABI-safe"
)]
pub unsafe extern "C" fn pp_activity_set_get(
    activities: *const PpActivitySet,
    index: u64,
    out_id: *mut PpUuid,
    out_kind: *mut *const c_char,
    out_has_started_at: *mut u8,
    out_started_at_unix_micros: *mut i64,
    out_has_finished_at: *mut u8,
    out_finished_at_unix_micros: *mut i64,
    out_input_count: *mut u64,
    out_output_count: *mut u64,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and checked before writes.
    unsafe {
        initialize_uuid(out_id);
        initialize_const_output(out_kind);
        initialize_value(out_has_started_at, 0);
        initialize_value(out_started_at_unix_micros, 0);
        initialize_value(out_has_finished_at, 0);
        initialize_value(out_finished_at_unix_micros, 0);
        initialize_value(out_input_count, 0);
        initialize_value(out_output_count, 0);
        ffi_call(out_error, || {
            require_output(out_id, "out_id")?;
            require_output(out_kind, "out_kind")?;
            require_output(out_has_started_at, "out_has_started_at")?;
            require_output(out_started_at_unix_micros, "out_started_at_unix_micros")?;
            require_output(out_has_finished_at, "out_has_finished_at")?;
            require_output(out_finished_at_unix_micros, "out_finished_at_unix_micros")?;
            require_output(out_input_count, "out_input_count")?;
            require_output(out_output_count, "out_output_count")?;
            let activities = activities
                .as_ref()
                .ok_or_else(|| invalid_argument("activities must not be null"))?;
            let activity = item_at(&activities.activities, index, "activity")?;
            out_id.write(PpUuid {
                bytes: activity.id.into_bytes(),
            });
            out_kind.write(activity.kind.as_ptr());
            if let Some(value) = activity.started_at_unix_micros {
                out_has_started_at.write(1);
                out_started_at_unix_micros.write(value);
            }
            if let Some(value) = activity.finished_at_unix_micros {
                out_has_finished_at.write(1);
                out_finished_at_unix_micros.write(value);
            }
            out_input_count.write(length_as_u64(activity.inputs.len())?);
            out_output_count.write(length_as_u64(activity.outputs.len())?);
            Ok(())
        })
    }
}

/// Reads one input edge and its optional borrowed role.
///
/// # Safety
///
/// `activities` must be live. Every output must be writable and `out_error`
/// may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_activity_set_get_input(
    activities: *const PpActivitySet,
    activity_index: u64,
    input_index: u64,
    out_representation_id: *mut PpUuid,
    out_role: *mut *const c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: The shared helper initializes and validates every output.
    unsafe {
        ffi_call(out_error, || {
            let activities = activities
                .as_ref()
                .ok_or_else(|| invalid_argument("activities must not be null"))?;
            let activity = item_at(&activities.activities, activity_index, "activity")?;
            write_activity_edge(
                &activity.inputs,
                input_index,
                out_representation_id,
                out_role,
            )
        })
    }
}

/// Reads one output edge and its optional borrowed role.
///
/// # Safety
///
/// `activities` must be live. Every output must be writable and `out_error`
/// may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_activity_set_get_output(
    activities: *const PpActivitySet,
    activity_index: u64,
    output_index: u64,
    out_representation_id: *mut PpUuid,
    out_role: *mut *const c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: The shared helper initializes and validates every output.
    unsafe {
        ffi_call(out_error, || {
            let activities = activities
                .as_ref()
                .ok_or_else(|| invalid_argument("activities must not be null"))?;
            let activity = item_at(&activities.activities, activity_index, "activity")?;
            write_activity_edge(
                &activity.outputs,
                output_index,
                out_representation_id,
                out_role,
            )
        })
    }
}

/// Reads the optional tool identity for one activity.
///
/// Every returned string is borrowed. All outputs are null when no tool was
/// recorded; version and URI may independently be null for a present tool.
///
/// # Safety
///
/// `activities` must be live. Every output must be writable and `out_error`
/// may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_activity_set_get_tool(
    activities: *const PpActivitySet,
    index: u64,
    out_name: *mut *const c_char,
    out_version: *mut *const c_char,
    out_uri: *mut *const c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and checked before writes.
    unsafe {
        initialize_const_output(out_name);
        initialize_const_output(out_version);
        initialize_const_output(out_uri);
        ffi_call(out_error, || {
            require_output(out_name, "out_name")?;
            require_output(out_version, "out_version")?;
            require_output(out_uri, "out_uri")?;
            let activities = activities
                .as_ref()
                .ok_or_else(|| invalid_argument("activities must not be null"))?;
            let activity = item_at(&activities.activities, index, "activity")?;
            if let Some(tool) = &activity.tool {
                out_name.write(tool.name.as_ptr());
                out_version.write(
                    tool.version
                        .as_ref()
                        .map_or(ptr::null(), |value| value.as_ptr()),
                );
                out_uri.write(
                    tool.uri
                        .as_ref()
                        .map_or(ptr::null(), |value| value.as_ptr()),
                );
            }
            Ok(())
        })
    }
}

/// Reads the optional agent identity for one activity.
///
/// Every returned string is borrowed and nullable. A present agent has a name,
/// an external identifier, or both.
///
/// # Safety
///
/// `activities` must be live. Every output must be writable and `out_error`
/// may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_activity_set_get_agent(
    activities: *const PpActivitySet,
    index: u64,
    out_name: *mut *const c_char,
    out_identifier_scheme: *mut *const c_char,
    out_identifier_value: *mut *const c_char,
    out_identifier_qualifier: *mut *const c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and checked before writes.
    unsafe {
        initialize_const_output(out_name);
        initialize_const_output(out_identifier_scheme);
        initialize_const_output(out_identifier_value);
        initialize_const_output(out_identifier_qualifier);
        ffi_call(out_error, || {
            require_output(out_name, "out_name")?;
            require_output(out_identifier_scheme, "out_identifier_scheme")?;
            require_output(out_identifier_value, "out_identifier_value")?;
            require_output(out_identifier_qualifier, "out_identifier_qualifier")?;
            let activities = activities
                .as_ref()
                .ok_or_else(|| invalid_argument("activities must not be null"))?;
            let activity = item_at(&activities.activities, index, "activity")?;
            if let Some(agent) = &activity.agent {
                out_name.write(
                    agent
                        .name
                        .as_ref()
                        .map_or(ptr::null(), |value| value.as_ptr()),
                );
                out_identifier_scheme.write(
                    agent
                        .identifier_scheme
                        .as_ref()
                        .map_or(ptr::null(), |value| value.as_ptr()),
                );
                out_identifier_value.write(
                    agent
                        .identifier_value
                        .as_ref()
                        .map_or(ptr::null(), |value| value.as_ptr()),
                );
                out_identifier_qualifier.write(
                    agent
                        .identifier_qualifier
                        .as_ref()
                        .map_or(ptr::null(), |value| value.as_ptr()),
                );
            }
            Ok(())
        })
    }
}

/// Releases an activity result set. Null is a no-op.
///
/// # Safety
///
/// A non-null handle must be live and released exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_activity_set_release(activities: *mut PpActivitySet) {
    if activities.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: Ownership is transferred back exactly once by contract.
        drop(unsafe { Box::from_raw(activities) });
    }));
}

/// Loads the newest revision as a zero-or-one-element owned result set.
///
/// # Safety
///
/// `production` must be live, `out_revisions` must be writable, and `out_error`
/// may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_latest_revision(
    production: *const PpProduction,
    out_revisions: *mut *mut PpRevisionSet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Inputs are validated before use and output ownership is explicit.
    unsafe {
        initialize_output(out_revisions);
        ffi_call(out_error, || {
            let production = production
                .as_ref()
                .ok_or_else(|| invalid_argument("production must not be null"))?;
            require_output(out_revisions, "out_revisions")?;
            let inner = lock_production(&production.state);
            let revisions: Vec<_> = inner.latest_revision()?.into_iter().collect();
            out_revisions.write(Box::into_raw(Box::new(PpRevisionSet::new(&revisions)?)));
            Ok(())
        })
    }
}

/// Loads an ascending, bounded revision page after `sequence`.
///
/// # Safety
///
/// Pointer rules match [`pp_production_latest_revision`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_changes_since(
    production: *const PpProduction,
    sequence: u64,
    limit: u32,
    out_revisions: *mut *mut PpRevisionSet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Inputs are validated before use and output ownership is explicit.
    unsafe {
        initialize_output(out_revisions);
        ffi_call(out_error, || {
            let production = production
                .as_ref()
                .ok_or_else(|| invalid_argument("production must not be null"))?;
            require_output(out_revisions, "out_revisions")?;
            let inner = lock_production(&production.state);
            let revisions = PpRevisionSet::new(&inner.changes_since(sequence, limit)?)?;
            out_revisions.write(Box::into_raw(Box::new(revisions)));
            Ok(())
        })
    }
}

/// Returns the number of revisions in a result set. Null returns zero.
///
/// # Safety
///
/// `revisions` must be null or a live result-set handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_revision_set_count(revisions: *const PpRevisionSet) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: A non-null handle is live by the caller contract.
        unsafe { revisions.as_ref() }.map_or(0, |set| {
            u64::try_from(set.revisions.len()).unwrap_or(u64::MAX)
        })
    }))
    .unwrap_or(0)
}

/// Reads one revision summary. Returned strings borrow the result-set lifetime.
///
/// # Safety
///
/// `revisions` must be live. Every output must be writable and `out_error` may
/// be null or writable.
#[unsafe(no_mangle)]
#[allow(
    clippy::too_many_arguments,
    reason = "flat C output parameters are ABI-safe"
)]
pub unsafe extern "C" fn pp_revision_set_get(
    revisions: *const PpRevisionSet,
    index: u64,
    out_id: *mut PpUuid,
    out_sequence: *mut u64,
    out_transaction_id: *mut PpUuid,
    out_committed_at_unix_micros: *mut i64,
    out_origin_name: *mut *const c_char,
    out_origin_version: *mut *const c_char,
    out_origin_uri: *mut *const c_char,
    out_message: *mut *const c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and checked before writes.
    unsafe {
        initialize_uuid(out_id);
        initialize_value(out_sequence, 0);
        initialize_uuid(out_transaction_id);
        initialize_value(out_committed_at_unix_micros, 0);
        initialize_const_output(out_origin_name);
        initialize_const_output(out_origin_version);
        initialize_const_output(out_origin_uri);
        initialize_const_output(out_message);
        ffi_call(out_error, || {
            require_output(out_id, "out_id")?;
            require_output(out_sequence, "out_sequence")?;
            require_output(out_transaction_id, "out_transaction_id")?;
            require_output(out_committed_at_unix_micros, "out_committed_at_unix_micros")?;
            require_output(out_origin_name, "out_origin_name")?;
            require_output(out_origin_version, "out_origin_version")?;
            require_output(out_origin_uri, "out_origin_uri")?;
            require_output(out_message, "out_message")?;
            let revisions = revisions
                .as_ref()
                .ok_or_else(|| invalid_argument("revisions must not be null"))?;
            let revision = item_at(&revisions.revisions, index, "revision")?;
            out_id.write(PpUuid {
                bytes: revision.id.into_bytes(),
            });
            out_sequence.write(revision.sequence);
            out_transaction_id.write(PpUuid {
                bytes: revision.transaction_id.into_bytes(),
            });
            out_committed_at_unix_micros.write(revision.committed_at_unix_micros);
            out_origin_name.write(
                revision
                    .origin_name
                    .as_ref()
                    .map_or(ptr::null(), |value| value.as_ptr()),
            );
            out_origin_version.write(
                revision
                    .origin_version
                    .as_ref()
                    .map_or(ptr::null(), |value| value.as_ptr()),
            );
            out_origin_uri.write(
                revision
                    .origin_uri
                    .as_ref()
                    .map_or(ptr::null(), |value| value.as_ptr()),
            );
            out_message.write(
                revision
                    .message
                    .as_ref()
                    .map_or(ptr::null(), |value| value.as_ptr()),
            );
            Ok(())
        })
    }
}

/// Releases a revision result set. Null is a no-op.
///
/// # Safety
///
/// A non-null handle must be live and released exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_revision_set_release(revisions: *mut PpRevisionSet) {
    if revisions.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: Ownership is transferred back exactly once by contract.
        drop(unsafe { Box::from_raw(revisions) });
    }));
}

/// Loads one revision's ordered semantic events.
///
/// # Safety
///
/// `production` and `revision_id` must be readable live values,
/// `out_events` must be writable, and `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_revision_events(
    production: *const PpProduction,
    revision_id: *const PpUuid,
    out_events: *mut *mut PpRevisionEventSet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Inputs are validated before use and output ownership is explicit.
    unsafe {
        initialize_output(out_events);
        ffi_call(out_error, || {
            let production = production
                .as_ref()
                .ok_or_else(|| invalid_argument("production must not be null"))?;
            let revision_id = revision_id
                .as_ref()
                .ok_or_else(|| invalid_argument("revision_id must not be null"))?;
            require_output(out_events, "out_events")?;
            let inner = lock_production(&production.state);
            let events = inner.events_for_revision(RevisionId::from_bytes(revision_id.bytes))?;
            out_events.write(Box::into_raw(Box::new(PpRevisionEventSet::new(&events)?)));
            Ok(())
        })
    }
}

/// Returns the number of events in a result set. Null returns zero.
///
/// # Safety
///
/// `events` must be null or a live result-set handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_revision_event_set_count(events: *const PpRevisionEventSet) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: A non-null handle is live by the caller contract.
        unsafe { events.as_ref() }
            .map_or(0, |set| u64::try_from(set.events.len()).unwrap_or(u64::MAX))
    }))
    .unwrap_or(0)
}

/// Reads one tagged semantic event. Borrowed strings live with the result set.
///
/// # Safety
///
/// `events` must be live, `out_event` must be writable, and `out_error` may be
/// null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_revision_event_set_get(
    events: *const PpRevisionEventSet,
    index: u64,
    out_event: *mut PpRevisionEvent,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and checked before writes.
    unsafe {
        initialize_value(out_event, empty_revision_event());
        ffi_call(out_error, || {
            require_output(out_event, "out_event")?;
            let events = events
                .as_ref()
                .ok_or_else(|| invalid_argument("events must not be null"))?;
            out_event.write(item_at(&events.events, index, "revision event")?.as_abi());
            Ok(())
        })
    }
}

/// Releases a revision-event result set. Null is a no-op.
///
/// # Safety
///
/// A non-null handle must be live and released exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_revision_event_set_release(events: *mut PpRevisionEventSet) {
    if events.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: Ownership is transferred back exactly once by contract.
        drop(unsafe { Box::from_raw(events) });
    }));
}

/// Returns the `PP_METADATA_*` kind of a borrowed value. Null returns zero.
///
/// # Safety
///
/// `value` must be null or borrowed from a live metadata result set.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_value_kind(value: *const PpMetadataValue) -> u32 {
    catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: A non-null value is live by the caller contract.
        unsafe { value.as_ref() }.map_or(0, PpMetadataValue::kind)
    }))
    .unwrap_or(0)
}

/// Reads plain or language-tagged text. Language is null for plain text.
///
/// # Safety
///
/// `value` must be borrowed and live. Outputs must be writable and `out_error`
/// may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_value_get_string(
    value: *const PpMetadataValue,
    out_text: *mut *const c_char,
    out_language: *mut *const c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    unsafe {
        initialize_const_output(out_text);
        initialize_const_output(out_language);
        ffi_call(out_error, || {
            require_output(out_text, "out_text")?;
            require_output(out_language, "out_language")?;
            match &metadata_value(value)?.inner {
                AbiMetadataValue::String { value, language } => {
                    out_text.write(value.as_ptr());
                    out_language.write(language.as_ref().map_or(ptr::null(), |tag| tag.as_ptr()));
                    Ok(())
                }
                _ => Err(metadata_type_error("string")),
            }
        })
    }
}

/// Reads a signed integer value.
///
/// # Safety
///
/// `value` must be borrowed and live. `out_value` must be writable and
/// `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_value_get_i64(
    value: *const PpMetadataValue,
    out_value: *mut i64,
    out_error: *mut *mut PpError,
) -> u32 {
    unsafe {
        initialize_value(out_value, 0);
        ffi_call(out_error, || match &metadata_value(value)?.inner {
            AbiMetadataValue::I64(stored) => write_copy(out_value, *stored, "out_value"),
            _ => Err(metadata_type_error("i64")),
        })
    }
}

/// Reads an unsigned integer value.
///
/// # Safety
///
/// Pointer rules match [`pp_metadata_value_get_i64`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_value_get_u64(
    value: *const PpMetadataValue,
    out_value: *mut u64,
    out_error: *mut *mut PpError,
) -> u32 {
    unsafe {
        initialize_value(out_value, 0);
        ffi_call(out_error, || match &metadata_value(value)?.inner {
            AbiMetadataValue::U64(stored) => write_copy(out_value, *stored, "out_value"),
            _ => Err(metadata_type_error("u64")),
        })
    }
}

/// Reads an exact decimal coefficient string and fractional scale.
///
/// # Safety
///
/// `value` must be borrowed and live. Outputs must be writable and `out_error`
/// may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_value_get_decimal(
    value: *const PpMetadataValue,
    out_coefficient: *mut *const c_char,
    out_scale: *mut u32,
    out_error: *mut *mut PpError,
) -> u32 {
    unsafe {
        initialize_const_output(out_coefficient);
        initialize_value(out_scale, 0);
        ffi_call(out_error, || {
            require_output(out_coefficient, "out_coefficient")?;
            require_output(out_scale, "out_scale")?;
            match &metadata_value(value)?.inner {
                AbiMetadataValue::Decimal { coefficient, scale } => {
                    out_coefficient.write(coefficient.as_ptr());
                    out_scale.write(*scale);
                    Ok(())
                }
                _ => Err(metadata_type_error("decimal")),
            }
        })
    }
}

/// Reads a boolean as zero or one.
///
/// # Safety
///
/// `value` must be borrowed and live. `out_value` must be writable and
/// `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_value_get_bool(
    value: *const PpMetadataValue,
    out_value: *mut u8,
    out_error: *mut *mut PpError,
) -> u32 {
    unsafe {
        initialize_value(out_value, 0);
        ffi_call(out_error, || match &metadata_value(value)?.inner {
            AbiMetadataValue::Bool(stored) => write_copy(out_value, u8::from(*stored), "out_value"),
            _ => Err(metadata_type_error("bool")),
        })
    }
}

/// Reads a signed Unix-microsecond timestamp.
///
/// # Safety
///
/// Pointer rules match [`pp_metadata_value_get_i64`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_value_get_timestamp(
    value: *const PpMetadataValue,
    out_unix_micros: *mut i64,
    out_error: *mut *mut PpError,
) -> u32 {
    unsafe {
        initialize_value(out_unix_micros, 0);
        ffi_call(out_error, || match &metadata_value(value)?.inner {
            AbiMetadataValue::Timestamp(stored) => {
                write_copy(out_unix_micros, *stored, "out_unix_micros")
            }
            _ => Err(metadata_type_error("timestamp")),
        })
    }
}

/// Reads borrowed URI text.
///
/// # Safety
///
/// `value` must be borrowed and live. `out_uri` must be writable and
/// `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_value_get_uri(
    value: *const PpMetadataValue,
    out_uri: *mut *const c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    unsafe {
        initialize_const_output(out_uri);
        ffi_call(out_error, || {
            require_output(out_uri, "out_uri")?;
            match &metadata_value(value)?.inner {
                AbiMetadataValue::Uri(stored) => {
                    out_uri.write(stored.as_ptr());
                    Ok(())
                }
                _ => Err(metadata_type_error("URI")),
            }
        })
    }
}

/// Reads borrowed opaque bytes and their length.
///
/// # Safety
///
/// `value` must be borrowed and live. Outputs must be writable and `out_error`
/// may be null or writable. The byte pointer remains valid with the result set.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_value_get_bytes(
    value: *const PpMetadataValue,
    out_bytes: *mut *const u8,
    out_length: *mut u64,
    out_error: *mut *mut PpError,
) -> u32 {
    unsafe {
        initialize_const_output(out_bytes);
        initialize_value(out_length, 0);
        ffi_call(out_error, || {
            require_output(out_bytes, "out_bytes")?;
            require_output(out_length, "out_length")?;
            match &metadata_value(value)?.inner {
                AbiMetadataValue::Bytes(stored) => {
                    out_bytes.write(stored.as_ptr());
                    out_length.write(length_as_u64(stored.len())?);
                    Ok(())
                }
                _ => Err(metadata_type_error("bytes")),
            }
        })
    }
}

/// Reads an exact rational numerator and positive denominator.
///
/// # Safety
///
/// `value` must be borrowed and live. Outputs must be writable and `out_error`
/// may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_value_get_rational(
    value: *const PpMetadataValue,
    out_numerator: *mut i64,
    out_denominator: *mut u64,
    out_error: *mut *mut PpError,
) -> u32 {
    unsafe {
        initialize_value(out_numerator, 0);
        initialize_value(out_denominator, 0);
        ffi_call(out_error, || {
            require_output(out_numerator, "out_numerator")?;
            require_output(out_denominator, "out_denominator")?;
            match &metadata_value(value)?.inner {
                AbiMetadataValue::Rational {
                    numerator,
                    denominator,
                } => {
                    out_numerator.write(*numerator);
                    out_denominator.write(*denominator);
                    Ok(())
                }
                _ => Err(metadata_type_error("rational")),
            }
        })
    }
}

/// Returns the number of list items. Null or a non-list value returns zero.
///
/// # Safety
///
/// `value` must be null or borrowed from a live metadata result set.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_value_list_count(value: *const PpMetadataValue) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: A non-null value is live by the caller contract.
        unsafe { value.as_ref() }.map_or(0, |value| match &value.inner {
            AbiMetadataValue::List(items) => u64::try_from(items.len()).unwrap_or(u64::MAX),
            _ => 0,
        })
    }))
    .unwrap_or(0)
}

/// Reads one borrowed child from a list value.
///
/// # Safety
///
/// `value` must be borrowed and live. `out_item` must be writable and
/// `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_value_list_get(
    value: *const PpMetadataValue,
    index: u64,
    out_item: *mut *const PpMetadataValue,
    out_error: *mut *mut PpError,
) -> u32 {
    unsafe {
        initialize_const_output(out_item);
        ffi_call(out_error, || {
            require_output(out_item, "out_item")?;
            match &metadata_value(value)?.inner {
                AbiMetadataValue::List(items) => {
                    out_item.write(ptr::from_ref(item_at(items, index, "metadata list item")?));
                    Ok(())
                }
                _ => Err(metadata_type_error("list")),
            }
        })
    }
}

/// Returns the number of fields. Null or a non-structure value returns zero.
///
/// # Safety
///
/// `value` must be null or borrowed from a live metadata result set.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_value_struct_count(value: *const PpMetadataValue) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: A non-null value is live by the caller contract.
        unsafe { value.as_ref() }.map_or(0, |value| match &value.inner {
            AbiMetadataValue::Struct(fields) => u64::try_from(fields.len()).unwrap_or(u64::MAX),
            _ => 0,
        })
    }))
    .unwrap_or(0)
}

/// Reads one borrowed name/value pair from a structured value.
///
/// # Safety
///
/// `value` must be borrowed and live. Outputs must be writable and `out_error`
/// may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_value_struct_get(
    value: *const PpMetadataValue,
    index: u64,
    out_name: *mut *const c_char,
    out_field_value: *mut *const PpMetadataValue,
    out_error: *mut *mut PpError,
) -> u32 {
    unsafe {
        initialize_const_output(out_name);
        initialize_const_output(out_field_value);
        ffi_call(out_error, || {
            require_output(out_name, "out_name")?;
            require_output(out_field_value, "out_field_value")?;
            match &metadata_value(value)?.inner {
                AbiMetadataValue::Struct(fields) => {
                    let field = item_at(fields, index, "metadata structure field")?;
                    out_name.write(field.name.as_ptr());
                    out_field_value.write(ptr::from_ref(&field.value));
                    Ok(())
                }
                _ => Err(metadata_type_error("structure")),
            }
        })
    }
}

/// Reads a typed `PostProject` object reference.
///
/// # Safety
///
/// `value` must be borrowed and live. `out_reference` must be writable and
/// `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_value_get_reference(
    value: *const PpMetadataValue,
    out_reference: *mut PpObjectRef,
    out_error: *mut *mut PpError,
) -> u32 {
    unsafe {
        initialize_object_ref(out_reference);
        ffi_call(out_error, || match &metadata_value(value)?.inner {
            AbiMetadataValue::Reference(reference) => {
                write_copy(out_reference, *reference, "out_reference")
            }
            _ => Err(metadata_type_error("reference")),
        })
    }
}

/// Resolves every representation belonging to an asset without mutating the production.
///
/// The returned immutable result set owns all candidate URI and evidence-detail
/// strings exposed by its accessors.
///
/// # Safety
///
/// `production` must be a live handle, `asset_id` must be readable, and
/// `out_resolutions` must be writable. `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_resolve_asset(
    production: *const PpProduction,
    asset_id: *const PpUuid,
    out_resolutions: *mut *mut PpResolutionSet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Null pointers are rejected before dereference; remaining pointer
    // validity and synchronization are guaranteed by the caller contract.
    unsafe {
        initialize_output(out_resolutions);
        ffi_call(out_error, || {
            let production = production
                .as_ref()
                .ok_or_else(|| invalid_argument("production must not be null"))?;
            let asset_id = asset_id
                .as_ref()
                .ok_or_else(|| invalid_argument("asset_id must not be null"))?;
            if out_resolutions.is_null() {
                return Err(invalid_argument("out_resolutions must not be null"));
            }

            let asset_id = AssetId::from_bytes(asset_id.bytes);
            let (media_roots, work) = {
                let inner = lock_production(&production.state);
                if !inner.assets()?.iter().any(|asset| asset.id() == asset_id) {
                    return Err(Error::new(
                        ErrorKind::NotFound,
                        format!("asset {asset_id} does not exist"),
                    ));
                }
                let mut work = Vec::new();
                for representation in inner.representations(asset_id)? {
                    let mut resources = Vec::new();
                    for resource in inner.resources(representation.id())? {
                        let locators = inner.locators(resource.id())?;
                        resources.push((resource, locators));
                    }
                    work.push((representation, resources));
                }
                (inner.production().media_roots().to_vec(), work)
            };

            let resolver = MediaResolver::default();
            let mut resolutions = Vec::new();
            for (representation, resources) in work {
                let mut resource_resolutions = Vec::new();
                for (resource, locators) in resources {
                    let resolution = resolver.resolve_resource(
                        &resource,
                        representation.content_structure(),
                        &locators,
                        &media_roots,
                    )?;
                    resource_resolutions.push(resolution);
                }
                resolutions.push(RepresentationResolution::aggregate(
                    representation.id(),
                    representation.content_structure(),
                    resource_resolutions,
                )?);
            }
            out_resolutions.write(Box::into_raw(Box::new(PpResolutionSet::new(resolutions))));
            Ok(())
        })
    }
}

/// Returns the number of representation results in a resolution set.
/// Null input returns zero.
///
/// # Safety
///
/// `resolutions` must be null or a live handle returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_resolution_set_representation_count(
    resolutions: *const PpResolutionSet,
) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: A non-null pointer is live for this call by the caller contract.
        unsafe { resolutions.as_ref() }.map_or(0, |set| {
            u64::try_from(set.representations.len()).unwrap_or(u64::MAX)
        })
    }))
    .unwrap_or(0)
}

/// Reads one representation-level availability result.
///
/// # Safety
///
/// `resolutions` must be live. Every output must be writable, and `out_error`
/// may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_resolution_set_get_representation(
    resolutions: *const PpResolutionSet,
    representation_index: u64,
    out_representation_id: *mut PpUuid,
    out_availability: *mut u32,
    out_resource_count: *mut u64,
    out_issue_count: *mut u64,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and checked before writes.
    unsafe {
        initialize_uuid(out_representation_id);
        initialize_value(out_availability, 0);
        initialize_value(out_resource_count, 0);
        initialize_value(out_issue_count, 0);
        ffi_call(out_error, || {
            require_output(out_representation_id, "out_representation_id")?;
            require_output(out_availability, "out_availability")?;
            require_output(out_resource_count, "out_resource_count")?;
            require_output(out_issue_count, "out_issue_count")?;
            let resolution = representation_resolution_at(resolutions, representation_index)?;
            out_representation_id.write(PpUuid {
                bytes: resolution.representation_id.into_bytes(),
            });
            out_availability.write(representation_availability(resolution.availability));
            out_resource_count.write(length_as_u64(resolution.resources.len())?);
            out_issue_count.write(length_as_u64(resolution.issues.len())?);
            Ok(())
        })
    }
}

/// Reads one resource result nested under a representation result.
///
/// # Safety
///
/// `resolutions` must be live. Every output must be writable, and `out_error`
/// may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_resolution_set_get_resource(
    resolutions: *const PpResolutionSet,
    representation_index: u64,
    resource_index: u64,
    out_resource_id: *mut PpUuid,
    out_state: *mut u32,
    out_candidate_count: *mut u64,
    out_evidence_count: *mut u64,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and checked before writes.
    unsafe {
        initialize_uuid(out_resource_id);
        initialize_value(out_state, 0);
        initialize_value(out_candidate_count, 0);
        initialize_value(out_evidence_count, 0);
        ffi_call(out_error, || {
            require_output(out_resource_id, "out_resource_id")?;
            require_output(out_state, "out_state")?;
            require_output(out_candidate_count, "out_candidate_count")?;
            require_output(out_evidence_count, "out_evidence_count")?;
            let resolution =
                resource_resolution_at(resolutions, representation_index, resource_index)?;
            out_resource_id.write(PpUuid {
                bytes: resolution.resource_id.into_bytes(),
            });
            out_state.write(resolution.state);
            out_candidate_count.write(length_as_u64(resolution.candidates.len())?);
            out_evidence_count.write(length_as_u64(resolution.evidence.len())?);
            Ok(())
        })
    }
}

/// Reads one representation availability issue.
///
/// # Safety
///
/// `resolutions` must be live. Every output must be writable, and `out_error`
/// may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_resolution_set_get_issue(
    resolutions: *const PpResolutionSet,
    representation_index: u64,
    issue_index: u64,
    out_resource_id: *mut PpUuid,
    out_required: *mut u8,
    out_kind: *mut u32,
    out_frame_count: *mut u64,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and checked before writes.
    unsafe {
        initialize_uuid(out_resource_id);
        initialize_value(out_required, 0);
        initialize_value(out_kind, 0);
        initialize_value(out_frame_count, 0);
        ffi_call(out_error, || {
            require_output(out_resource_id, "out_resource_id")?;
            require_output(out_required, "out_required")?;
            require_output(out_kind, "out_kind")?;
            require_output(out_frame_count, "out_frame_count")?;
            let representation = representation_resolution_at(resolutions, representation_index)?;
            let issue = item_at(&representation.issues, issue_index, "availability issue")?;
            out_resource_id.write(PpUuid {
                bytes: issue.resource_id().into_bytes(),
            });
            out_required.write(u8::from(issue.is_required()));
            out_kind.write(availability_issue_kind(issue.kind()));
            out_frame_count.write(length_as_u64(issue.frames().len())?);
            Ok(())
        })
    }
}

/// Reads one missing-frame value from an availability issue.
///
/// # Safety
///
/// `resolutions` must be live. `out_frame` must be writable, and `out_error`
/// may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_resolution_set_get_issue_frame(
    resolutions: *const PpResolutionSet,
    representation_index: u64,
    issue_index: u64,
    frame_index: u64,
    out_frame: *mut i64,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Output is initialized and checked before writes.
    unsafe {
        initialize_value(out_frame, 0);
        ffi_call(out_error, || {
            require_output(out_frame, "out_frame")?;
            let representation = representation_resolution_at(resolutions, representation_index)?;
            let issue = item_at(&representation.issues, issue_index, "availability issue")?;
            let frame = item_at(issue.frames(), frame_index, "missing frame")?;
            out_frame.write(*frame);
            Ok(())
        })
    }
}

/// Reads one candidate nested under a resource result.
///
/// The URI is borrowed until the resolution set is released.
///
/// # Safety
///
/// `resolutions` must be live. Every output must be writable, and `out_error`
/// may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_resolution_set_get_candidate(
    resolutions: *const PpResolutionSet,
    representation_index: u64,
    resource_index: u64,
    candidate_index: u64,
    out_uri: *mut *const c_char,
    out_confidence_basis_points: *mut u16,
    out_evidence_count: *mut u64,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and checked before writes.
    unsafe {
        initialize_const_output(out_uri);
        initialize_value(out_confidence_basis_points, 0);
        initialize_value(out_evidence_count, 0);
        ffi_call(out_error, || {
            require_output(out_uri, "out_uri")?;
            require_output(out_confidence_basis_points, "out_confidence_basis_points")?;
            require_output(out_evidence_count, "out_evidence_count")?;
            let resource =
                resource_resolution_at(resolutions, representation_index, resource_index)?;
            let candidate = item_at(&resource.candidates, candidate_index, "candidate")?;
            out_uri.write(candidate.uri.as_ptr());
            out_confidence_basis_points.write(candidate.confidence);
            out_evidence_count.write(length_as_u64(candidate.evidence.len())?);
            Ok(())
        })
    }
}

/// Reads resource-level evidence.
///
/// # Safety
///
/// `resolutions` must be live. Outputs must be writable, and `out_error` may be
/// null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_resolution_set_get_resource_evidence(
    resolutions: *const PpResolutionSet,
    representation_index: u64,
    resource_index: u64,
    evidence_index: u64,
    out_kind: *mut u32,
    out_detail: *mut *const c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized before delegating to checked helpers.
    unsafe {
        initialize_value(out_kind, 0);
        initialize_const_output(out_detail);
        ffi_call(out_error, || {
            let resource =
                resource_resolution_at(resolutions, representation_index, resource_index)?;
            write_evidence(&resource.evidence, evidence_index, out_kind, out_detail)
        })
    }
}

/// Reads candidate-level evidence.
///
/// # Safety
///
/// `resolutions` must be live. Outputs must be writable, and `out_error` may be
/// null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_resolution_set_get_candidate_evidence(
    resolutions: *const PpResolutionSet,
    representation_index: u64,
    resource_index: u64,
    candidate_index: u64,
    evidence_index: u64,
    out_kind: *mut u32,
    out_detail: *mut *const c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized before delegating to checked helpers.
    unsafe {
        initialize_value(out_kind, 0);
        initialize_const_output(out_detail);
        ffi_call(out_error, || {
            let resource =
                resource_resolution_at(resolutions, representation_index, resource_index)?;
            let candidate = item_at(&resource.candidates, candidate_index, "candidate")?;
            write_evidence(&candidate.evidence, evidence_index, out_kind, out_detail)
        })
    }
}

/// Releases a resolution set. Passing null is a no-op.
///
/// # Safety
///
/// A non-null pointer must have been returned by this library and not previously
/// released. No borrowed strings may be used after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_resolution_set_release(resolutions: *mut PpResolutionSet) {
    if resolutions.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: Ownership of a live allocation is required by this function's
        // contract and is reconstructed exactly once here.
        drop(unsafe { Box::from_raw(resolutions) });
    }));
}

/// Begins an explicit transaction that stages mutations until commit.
///
/// At most one transaction may be open for a production state. The returned handle
/// keeps that state alive even if the original production handle is released.
///
/// # Safety
///
/// `production` must be a live handle returned by this library. `out_transaction`
/// must be writable. `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_begin_transaction(
    production: *mut PpProduction,
    out_transaction: *mut *mut PpTransaction,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: The caller contract for each pointer is documented above. Outputs
    // are initialized before validation and the production is borrowed only here.
    unsafe {
        initialize_output(out_transaction);
        ffi_call(out_error, || {
            let production = production
                .as_ref()
                .ok_or_else(|| invalid_argument("production must not be null"))?;
            if out_transaction.is_null() {
                return Err(invalid_argument("out_transaction must not be null"));
            }
            if production
                .state
                .transaction_open
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                return Err(Error::new(
                    ErrorKind::Conflict,
                    "production already has an open transaction",
                ));
            }
            out_transaction.write(Box::into_raw(Box::new(PpTransaction {
                state: Arc::clone(&production.state),
                lifecycle: TransactionLifecycle::new(),
                revision_context: RevisionContext::default(),
                mutations: Vec::new(),
            })));
            Ok(())
        })
    }
}

/// Sets the origin and message attached to this transaction's future revision.
///
/// `origin_name` and `message` may be null. Origin version and URI may be null,
/// but require a non-null origin name when supplied.
///
/// # Safety
///
/// `transaction` must be live. Every non-null string must be NUL-terminated
/// UTF-8 for the duration of the call. `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_transaction_set_revision_context(
    transaction: *mut PpTransaction,
    origin_name: *const c_char,
    origin_version: *const c_char,
    origin_uri: *const c_char,
    message: *const c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Inputs are validated and copied before this call returns.
    unsafe {
        ffi_call(out_error, || {
            let transaction = transaction
                .as_mut()
                .ok_or_else(|| invalid_argument("transaction must not be null"))?;
            transaction.lifecycle.ensure_open()?;
            let origin_name = optional_utf8(origin_name, "origin_name")?;
            let origin_version = optional_utf8(origin_version, "origin_version")?;
            let origin_uri = optional_utf8(origin_uri, "origin_uri")?;
            let origin = match origin_name {
                Some(name) => Some(OriginIdentity::new(
                    name,
                    origin_version.map(str::to_owned),
                    origin_uri.map(str::to_owned),
                )?),
                None if origin_version.is_none() && origin_uri.is_none() => None,
                None => {
                    return Err(invalid_argument(
                        "origin_version and origin_uri require origin_name",
                    ));
                }
            };
            transaction.revision_context = RevisionContext::new(
                origin,
                optional_utf8(message, "message")?.map(str::to_owned),
            )?;
            Ok(())
        })
    }
}

/// Stages an original-media import and returns its stable asset identity.
///
/// # Safety
///
/// `transaction` must be a live transaction handle. `path` must be a borrowed
/// NUL-terminated UTF-8 string; `display_name` may be null or satisfy the same
/// rule. `out_asset_id` must be writable. `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_transaction_import_media(
    transaction: *mut PpTransaction,
    path: *const c_char,
    display_name: *const c_char,
    out_asset_id: *mut PpUuid,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Null pointers are rejected before dereference and string inputs
    // follow the documented borrowed NUL-terminated contract.
    unsafe {
        initialize_uuid(out_asset_id);
        ffi_call(out_error, || {
            let transaction = transaction
                .as_mut()
                .ok_or_else(|| invalid_argument("transaction must not be null"))?;
            transaction.lifecycle.ensure_open()?;
            if out_asset_id.is_null() {
                return Err(invalid_argument("out_asset_id must not be null"));
            }
            let path = required_utf8(path, "path")?;
            if path.is_empty() {
                return Err(invalid_argument("path must not be empty"));
            }
            let display_name = optional_utf8(display_name, "display_name")?.map(str::to_owned);
            let import = prepare_original_media(Path::new(path), display_name, None)?;
            out_asset_id.write(PpUuid {
                bytes: import.asset().id().into_bytes(),
            });
            transaction.mutations.push(StagedMutation::Import(import));
            Ok(())
        })
    }
}

/// Stages a filesystem media root and returns its stable identity.
///
/// # Safety
///
/// `transaction` must be a live transaction handle. `path` must be a borrowed
/// NUL-terminated UTF-8 string; `label` may be null or satisfy the same rule.
/// `out_root_id` must be writable. `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_transaction_add_media_root(
    transaction: *mut PpTransaction,
    path: *const c_char,
    label: *const c_char,
    priority: i32,
    out_root_id: *mut PpUuid,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Null pointers are rejected before dereference and string inputs
    // follow the documented borrowed NUL-terminated contract.
    unsafe {
        initialize_uuid(out_root_id);
        ffi_call(out_error, || {
            let transaction = transaction
                .as_mut()
                .ok_or_else(|| invalid_argument("transaction must not be null"))?;
            transaction.lifecycle.ensure_open()?;
            if out_root_id.is_null() {
                return Err(invalid_argument("out_root_id must not be null"));
            }
            let path = required_utf8(path, "path")?;
            if path.is_empty() {
                return Err(invalid_argument("path must not be empty"));
            }
            let label = optional_utf8(label, "label")?.map(str::to_owned);
            let root = prepare_media_root(Path::new(path), label, priority)?;
            out_root_id.write(PpUuid {
                bytes: root.id().into_bytes(),
            });
            transaction.mutations.push(StagedMutation::MediaRoot(root));
            Ok(())
        })
    }
}

/// Stages an explicitly confirmed URI for a resource.
///
/// The URI is borrowed UTF-8 without embedded NUL and must be absolute.
/// Confirmation is not durable until the transaction commits.
///
/// # Safety
///
/// `transaction` must be a live transaction handle, `resource_id` must be
/// readable, `uri` must be a NUL-terminated string, and `out_error` may be null
/// or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_transaction_confirm_locator(
    transaction: *mut PpTransaction,
    resource_id: *const PpUuid,
    uri: *const c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Null pointers are rejected before dereference and the URI follows
    // the documented borrowed NUL-terminated contract.
    unsafe {
        ffi_call(out_error, || {
            let transaction = transaction
                .as_mut()
                .ok_or_else(|| invalid_argument("transaction must not be null"))?;
            transaction.lifecycle.ensure_open()?;
            let resource_id = resource_id
                .as_ref()
                .ok_or_else(|| invalid_argument("resource_id must not be null"))?;
            let uri = required_utf8(uri, "uri")?;
            if uri.is_empty() {
                return Err(invalid_argument("uri must not be empty"));
            }
            let locator = prepare_confirmed_locator(
                ResourceId::from_bytes(resource_id.bytes),
                uri.to_owned(),
            )?;
            transaction.mutations.push(StagedMutation::Locator(locator));
            Ok(())
        })
    }
}

/// Stages an external identifier attachment.
///
/// `qualifier` may be null; other string inputs are required borrowed
/// NUL-terminated UTF-8. The target is validated when the transaction commits.
///
/// # Safety
///
/// `transaction` must be live, `target` readable, string pointers must satisfy
/// the rules above, and `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_transaction_add_external_identifier(
    transaction: *mut PpTransaction,
    target: *const PpObjectRef,
    scheme: *const c_char,
    value: *const c_char,
    qualifier: *const c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Inputs are checked before dereference and borrowed only for this call.
    unsafe {
        ffi_call(out_error, || {
            let transaction = transaction
                .as_mut()
                .ok_or_else(|| invalid_argument("transaction must not be null"))?;
            transaction.lifecycle.ensure_open()?;
            let target = target
                .as_ref()
                .ok_or_else(|| invalid_argument("target must not be null"))?;
            let target = object_ref_from_abi(*target)?;
            let identifier = external_identifier_from_abi(scheme, value, qualifier)?;
            transaction
                .mutations
                .push(StagedMutation::AddExternalIdentifier(target, identifier));
            Ok(())
        })
    }
}

/// Stages removal of one exact external identifier attachment.
///
/// # Safety
///
/// The pointer and UTF-8 contracts are identical to
/// [`pp_transaction_add_external_identifier`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_transaction_remove_external_identifier(
    transaction: *mut PpTransaction,
    target: *const PpObjectRef,
    scheme: *const c_char,
    value: *const c_char,
    qualifier: *const c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Inputs are checked before dereference and borrowed only for this call.
    unsafe {
        ffi_call(out_error, || {
            let transaction = transaction
                .as_mut()
                .ok_or_else(|| invalid_argument("transaction must not be null"))?;
            transaction.lifecycle.ensure_open()?;
            let target = target
                .as_ref()
                .ok_or_else(|| invalid_argument("target must not be null"))?;
            let target = object_ref_from_abi(*target)?;
            let identifier = external_identifier_from_abi(scheme, value, qualifier)?;
            transaction
                .mutations
                .push(StagedMutation::RemoveExternalIdentifier(target, identifier));
            Ok(())
        })
    }
}

/// Stages one plain or language-tagged metadata text assertion.
///
/// `language` may be null for plain text. Other strings are required borrowed
/// NUL-terminated UTF-8. The target is validated at commit.
///
/// # Safety
///
/// `transaction` must be live, `target` readable, string pointers must satisfy
/// the rules above, and `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_transaction_add_metadata_text(
    transaction: *mut PpTransaction,
    target: *const PpObjectRef,
    vocabulary: *const c_char,
    property: *const c_char,
    value: *const c_char,
    language: *const c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Inputs are checked before dereference and borrowed only this call.
    unsafe {
        ffi_call(out_error, || {
            let transaction = transaction
                .as_mut()
                .ok_or_else(|| invalid_argument("transaction must not be null"))?;
            transaction.lifecycle.ensure_open()?;
            let target = target
                .as_ref()
                .ok_or_else(|| invalid_argument("target must not be null"))?;
            let target = object_ref_from_abi(*target)?;
            let property = metadata_property_from_abi(vocabulary, property)?;
            let value = required_utf8(value, "value")?;
            let language = optional_utf8(language, "language")?;
            let value = language.map_or_else(
                || MetadataValue::string(value),
                |language| MetadataValue::language_string(value, language),
            )?;
            transaction
                .mutations
                .push(StagedMutation::AddMetadataValue(target, property, value));
            Ok(())
        })
    }
}

/// Stages one typed metadata assertion from an owned metadata input.
///
/// The input remains owned by the caller and may be released immediately after
/// this call. The target is validated at commit.
///
/// # Safety
///
/// `transaction`, `target`, and `input` must be live; vocabulary/property must
/// be borrowed NUL-terminated UTF-8; and `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_transaction_add_metadata_value(
    transaction: *mut PpTransaction,
    target: *const PpObjectRef,
    vocabulary: *const c_char,
    property: *const c_char,
    input: *const PpMetadataInput,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Inputs are checked before dereference and borrowed only this call.
    unsafe {
        ffi_call(out_error, || {
            let transaction = transaction
                .as_mut()
                .ok_or_else(|| invalid_argument("transaction must not be null"))?;
            transaction.lifecycle.ensure_open()?;
            let target = target
                .as_ref()
                .ok_or_else(|| invalid_argument("target must not be null"))?;
            let input = input
                .as_ref()
                .ok_or_else(|| invalid_argument("input must not be null"))?;
            let target = object_ref_from_abi(*target)?;
            let property = metadata_property_from_abi(vocabulary, property)?;
            transaction.mutations.push(StagedMutation::AddMetadataValue(
                target,
                property,
                input.value.clone(),
            ));
            Ok(())
        })
    }
}

/// Stages removal of every value of one metadata property.
///
/// # Safety
///
/// `transaction` must be live, `target` readable, vocabulary/property must be
/// borrowed NUL-terminated UTF-8, and `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_transaction_remove_metadata_property(
    transaction: *mut PpTransaction,
    target: *const PpObjectRef,
    vocabulary: *const c_char,
    property: *const c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Inputs are checked before dereference and borrowed only this call.
    unsafe {
        ffi_call(out_error, || {
            let transaction = transaction
                .as_mut()
                .ok_or_else(|| invalid_argument("transaction must not be null"))?;
            transaction.lifecycle.ensure_open()?;
            let target = target
                .as_ref()
                .ok_or_else(|| invalid_argument("target must not be null"))?;
            let target = object_ref_from_abi(*target)?;
            let property = metadata_property_from_abi(vocabulary, property)?;
            transaction
                .mutations
                .push(StagedMutation::RemoveMetadataProperty(target, property));
            Ok(())
        })
    }
}

/// Stages one complete production activity.
///
/// Input and output arrays are borrowed only for this call. Edge roles, kind,
/// tool fields, and agent fields are NUL-terminated UTF-8. Optional values may
/// be null. Agent identifier scheme and value must be supplied together.
///
/// # Safety
///
/// `transaction` must be live, `kind` and non-empty edge arrays must be
/// readable, `out_activity_id` must be writable, and every non-null string must
/// remain valid for the call. `out_error` may be null or writable.
#[unsafe(no_mangle)]
#[allow(
    clippy::too_many_arguments,
    reason = "the C ABI keeps optional provenance fields explicit"
)]
pub unsafe extern "C" fn pp_transaction_create_activity(
    transaction: *mut PpTransaction,
    kind: *const c_char,
    inputs: *const PpActivityEdge,
    input_count: u64,
    outputs: *const PpActivityEdge,
    output_count: u64,
    started_at_unix_micros: *const i64,
    finished_at_unix_micros: *const i64,
    tool_name: *const c_char,
    tool_version: *const c_char,
    tool_uri: *const c_char,
    agent_name: *const c_char,
    agent_identifier_scheme: *const c_char,
    agent_identifier_value: *const c_char,
    agent_identifier_qualifier: *const c_char,
    out_activity_id: *mut PpUuid,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Inputs are checked and converted to owned domain values before return.
    unsafe {
        initialize_uuid(out_activity_id);
        ffi_call(out_error, || {
            let transaction = transaction
                .as_mut()
                .ok_or_else(|| invalid_argument("transaction must not be null"))?;
            transaction.lifecycle.ensure_open()?;
            require_output(out_activity_id, "out_activity_id")?;
            let kind = ActivityKind::new(required_utf8(kind, "kind")?)?;
            let inputs = activity_edges_from_abi(inputs, input_count, "input")?
                .into_iter()
                .map(|(representation_id, role)| ActivityInput::new(representation_id, role))
                .collect();
            let outputs = activity_edges_from_abi(outputs, output_count, "output")?
                .into_iter()
                .map(|(representation_id, role)| ActivityOutput::new(representation_id, role))
                .collect();
            let mut activity = Activity::new(ActivityId::new(), kind, inputs, outputs)?
                .with_timing(
                    started_at_unix_micros
                        .as_ref()
                        .copied()
                        .map(Timestamp::from_unix_micros),
                    finished_at_unix_micros
                        .as_ref()
                        .copied()
                        .map(Timestamp::from_unix_micros),
                )?;
            let tool_name = optional_utf8(tool_name, "tool_name")?;
            let tool_version = optional_utf8(tool_version, "tool_version")?;
            let tool_uri = optional_utf8(tool_uri, "tool_uri")?;
            if let Some(name) = tool_name {
                activity = activity.with_tool(ToolIdentity::new(
                    name,
                    tool_version.map(str::to_owned),
                    tool_uri.map(str::to_owned),
                )?);
            } else if tool_version.is_some() || tool_uri.is_some() {
                return Err(invalid_argument("tool version and URI require a tool name"));
            }
            let agent_name = optional_utf8(agent_name, "agent_name")?;
            let agent_scheme = optional_utf8(agent_identifier_scheme, "agent_identifier_scheme")?;
            let agent_value = optional_utf8(agent_identifier_value, "agent_identifier_value")?;
            let agent_qualifier =
                optional_utf8(agent_identifier_qualifier, "agent_identifier_qualifier")?;
            let agent_identifier = match (agent_scheme, agent_value) {
                (Some(scheme), Some(value)) => Some(ExternalIdentifier::new(
                    IdentifierScheme::new(scheme)?,
                    value,
                    agent_qualifier.map(str::to_owned),
                )?),
                (None, None) if agent_qualifier.is_none() => None,
                _ => {
                    return Err(invalid_argument(
                        "agent identifier scheme and value must be supplied together",
                    ));
                }
            };
            if agent_name.is_some() || agent_identifier.is_some() {
                activity = activity.with_agent(AgentIdentity::new(
                    agent_name.map(str::to_owned),
                    agent_identifier,
                )?);
            }
            out_activity_id.write(PpUuid {
                bytes: activity.id().into_bytes(),
            });
            transaction
                .mutations
                .push(StagedMutation::Activity(activity));
            Ok(())
        })
    }
}

/// Atomically commits all staged transaction mutations.
///
/// # Safety
///
/// `transaction` must be a live transaction handle and `out_error` may be null
/// or writable. A closed transaction remains valid only for release.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_transaction_commit(
    transaction: *mut PpTransaction,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: The non-null transaction is required to be live and exclusively
    // accessed by the caller for this operation.
    unsafe {
        ffi_call(out_error, || {
            let transaction = transaction
                .as_mut()
                .ok_or_else(|| invalid_argument("transaction must not be null"))?;
            transaction.commit()
        })
    }
}

/// Discards all staged transaction mutations.
///
/// # Safety
///
/// `transaction` must be a live transaction handle and `out_error` may be null
/// or writable. A closed transaction remains valid only for release.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_transaction_rollback(
    transaction: *mut PpTransaction,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: The non-null transaction is required to be live and exclusively
    // accessed by the caller for this operation.
    unsafe {
        ffi_call(out_error, || {
            let transaction = transaction
                .as_mut()
                .ok_or_else(|| invalid_argument("transaction must not be null"))?;
            transaction.rollback()
        })
    }
}

/// Releases a transaction, implicitly discarding staged work when still open.
/// Passing null is a no-op.
///
/// # Safety
///
/// A non-null pointer must have been returned by this library and not previously
/// released. No other thread may use it during or after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_transaction_release(transaction: *mut PpTransaction) {
    if transaction.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: Ownership of a live allocation is required by this function's
        // contract and is reconstructed exactly once here.
        drop(unsafe { Box::from_raw(transaction) });
    }));
}

/// Releases a production handle. Passing null is a no-op.
///
/// # Safety
///
/// A non-null pointer must have been returned by this library and not previously
/// released. No other thread may use it during or after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_production_release(production: *mut PpProduction) {
    if production.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: Ownership of a live allocation is required by this function's
        // contract and is reconstructed exactly once here.
        drop(unsafe { Box::from_raw(production) });
    }));
}

/// Returns an error object's stable numeric code.
///
/// # Safety
///
/// `error` must be null or a live error returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_error_code(error: *const PpError) -> u32 {
    // SAFETY: A non-null pointer is required to reference a live error by the
    // caller contract and is only borrowed for this call.
    unsafe { error.as_ref().map_or(PP_OK, |error| error.code) }
}

/// Returns a borrowed NUL-terminated UTF-8 error message.
///
/// The pointer remains valid until the error is released. Null input returns
/// null.
///
/// # Safety
///
/// `error` must be null or a live error returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_error_message(error: *const PpError) -> *const c_char {
    // SAFETY: A non-null pointer is required to reference a live error by the
    // caller contract and is only borrowed for this call.
    unsafe {
        error
            .as_ref()
            .map_or(ptr::null(), |error| error.message.as_ptr())
    }
}

/// Releases an error object. Passing null is a no-op.
///
/// # Safety
///
/// A non-null pointer must have been returned by this library and not previously
/// released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_error_release(error: *mut PpError) {
    if error.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: Ownership of a live allocation is required by this function's
        // contract and is reconstructed exactly once here.
        drop(unsafe { Box::from_raw(error) });
    }));
}

unsafe fn ffi_call(
    out_error: *mut *mut PpError,
    operation: impl FnOnce() -> Result<(), Error>,
) -> u32 {
    // SAFETY: The caller of this helper passes through the exported function's
    // documented optional writable error pointer.
    unsafe { initialize_output(out_error) };
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => PP_OK,
        Ok(Err(error)) => {
            let code = error_code(error.kind());
            let message = error.to_string();
            // SAFETY: Same output-pointer contract as above.
            unsafe { write_error(out_error, code, &message) };
            code
        }
        Err(payload) => {
            let message = panic_message(payload.as_ref());
            // SAFETY: Same output-pointer contract as above.
            unsafe { write_error(out_error, PP_ERROR_INTERNAL, &message) };
            PP_ERROR_INTERNAL
        }
    }
}

#[derive(Clone, Copy)]
enum ActivityRelation {
    Producing,
    Consuming,
}

unsafe fn production_activities_for_representation(
    production: *const PpProduction,
    representation_id: *const PpUuid,
    relation: ActivityRelation,
    out_activities: *mut *mut PpActivitySet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and all pointers checked before use.
    unsafe {
        initialize_output(out_activities);
        ffi_call(out_error, || {
            let production = production
                .as_ref()
                .ok_or_else(|| invalid_argument("production must not be null"))?;
            let representation_id = representation_id
                .as_ref()
                .ok_or_else(|| invalid_argument("representation_id must not be null"))?;
            require_output(out_activities, "out_activities")?;
            let inner = lock_production(&production.state);
            let representation_id = RepresentationId::from_bytes(representation_id.bytes);
            let activities = match relation {
                ActivityRelation::Producing => inner.activities_producing(representation_id),
                ActivityRelation::Consuming => inner.activities_consuming(representation_id),
            }?;
            out_activities.write(Box::into_raw(Box::new(PpActivitySet::new(&activities)?)));
            Ok(())
        })
    }
}

#[derive(Clone, Copy)]
enum ProvenanceDirection {
    Ancestors,
    Descendants,
}

unsafe fn production_provenance_relatives(
    production: *const PpProduction,
    representation_id: *const PpUuid,
    direction: ProvenanceDirection,
    out_representations: *mut *mut PpObjectRefSet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and all pointers checked before use.
    unsafe {
        initialize_output(out_representations);
        ffi_call(out_error, || {
            let production = production
                .as_ref()
                .ok_or_else(|| invalid_argument("production must not be null"))?;
            let representation_id = representation_id
                .as_ref()
                .ok_or_else(|| invalid_argument("representation_id must not be null"))?;
            require_output(out_representations, "out_representations")?;
            let inner = lock_production(&production.state);
            let representation_id = RepresentationId::from_bytes(representation_id.bytes);
            let related = match direction {
                ProvenanceDirection::Ancestors => inner.ancestors(representation_id),
                ProvenanceDirection::Descendants => inner.descendants(representation_id),
            }?;
            let objects = related
                .into_iter()
                .map(|id| PpObjectRef {
                    kind: PP_OBJECT_REPRESENTATION,
                    id: PpUuid {
                        bytes: id.into_bytes(),
                    },
                })
                .collect();
            out_representations.write(Box::into_raw(Box::new(PpObjectRefSet { objects })));
            Ok(())
        })
    }
}

unsafe fn initialize_output<T>(output: *mut *mut T) {
    if !output.is_null() {
        // SAFETY: Non-null output pointers are required to be writable by every
        // exported caller contract using this helper.
        unsafe { output.write(ptr::null_mut()) };
    }
}

unsafe fn initialize_uuid(output: *mut PpUuid) {
    if !output.is_null() {
        // SAFETY: Non-null UUID output pointers are required to be writable by
        // every exported caller contract using this helper.
        unsafe { output.write(PpUuid { bytes: [0; 16] }) };
    }
}

unsafe fn initialize_object_ref(output: *mut PpObjectRef) {
    if !output.is_null() {
        // SAFETY: Non-null outputs are writable by the exported caller contract.
        unsafe {
            output.write(PpObjectRef {
                kind: 0,
                id: PpUuid { bytes: [0; 16] },
            });
        }
    }
}

unsafe fn initialize_value<T: Copy>(output: *mut T, value: T) {
    if !output.is_null() {
        // SAFETY: Non-null output pointers are required to be writable by every
        // exported caller contract using this helper.
        unsafe { output.write(value) };
    }
}

unsafe fn initialize_const_output<T>(output: *mut *const T) {
    if !output.is_null() {
        // SAFETY: Non-null output pointers are required to be writable by every
        // exported caller contract using this helper.
        unsafe { output.write(ptr::null()) };
    }
}

const fn empty_revision_event() -> PpRevisionEvent {
    PpRevisionEvent {
        kind: 0,
        position: 0,
        asset_id: PpUuid { bytes: [0; 16] },
        representation_id: PpUuid { bytes: [0; 16] },
        resource_id: PpUuid { bytes: [0; 16] },
        locator_id: PpUuid { bytes: [0; 16] },
        media_root_id: PpUuid { bytes: [0; 16] },
        activity_id: PpUuid { bytes: [0; 16] },
        target: PpObjectRef {
            kind: 0,
            id: PpUuid { bytes: [0; 16] },
        },
        structural_position: 0,
        identifier_scheme: ptr::null(),
        identifier_value: ptr::null(),
        identifier_qualifier: ptr::null(),
        vocabulary: ptr::null(),
        property: ptr::null(),
        activity_kind: ptr::null(),
        role: ptr::null(),
    }
}

fn require_output<T>(output: *mut T, label: &str) -> Result<(), Error> {
    if output.is_null() {
        Err(invalid_argument(format!("{label} must not be null")))
    } else {
        Ok(())
    }
}

unsafe fn write_evidence(
    evidence: &[AbiEvidence],
    evidence_index: u64,
    out_kind: *mut u32,
    out_detail: *mut *const c_char,
) -> Result<(), Error> {
    // SAFETY: Output validity is checked before either pointer is written.
    unsafe {
        initialize_value(out_kind, 0);
        initialize_const_output(out_detail);
        require_output(out_kind, "out_kind")?;
        require_output(out_detail, "out_detail")?;
        let evidence = item_at(evidence, evidence_index, "evidence")?;
        out_kind.write(evidence.kind);
        out_detail.write(
            evidence
                .detail
                .as_ref()
                .map_or(ptr::null(), |value| value.as_ptr()),
        );
        Ok(())
    }
}

unsafe fn write_activity_edge(
    edges: &[AbiActivityEdge],
    index: u64,
    out_representation_id: *mut PpUuid,
    out_role: *mut *const c_char,
) -> Result<(), Error> {
    // SAFETY: Output validity is checked before either pointer is written.
    unsafe {
        initialize_uuid(out_representation_id);
        initialize_const_output(out_role);
        require_output(out_representation_id, "out_representation_id")?;
        require_output(out_role, "out_role")?;
        let edge = item_at(edges, index, "activity edge")?;
        out_representation_id.write(PpUuid {
            bytes: edge.representation_id.into_bytes(),
        });
        out_role.write(
            edge.role
                .as_ref()
                .map_or(ptr::null(), |value| value.as_ptr()),
        );
        Ok(())
    }
}

unsafe fn write_error(output: *mut *mut PpError, code: u32, message: &str) {
    if output.is_null() {
        return;
    }
    let message = CString::new(message.replace('\0', "�")).unwrap_or_default();
    let error = Box::new(PpError { code, message });
    // SAFETY: Non-null output pointers are required to be writable by every
    // exported caller contract using this helper.
    unsafe { output.write(Box::into_raw(error)) };
}

unsafe fn required_utf8<'a>(value: *const c_char, label: &str) -> Result<&'a str, Error> {
    if value.is_null() {
        return Err(invalid_argument(format!("{label} must not be null")));
    }
    // SAFETY: The exported caller contract requires a NUL-terminated input that
    // remains valid throughout the call.
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map_err(|error| invalid_argument(format!("{label} must contain valid UTF-8: {error}")))
}

unsafe fn optional_utf8<'a>(value: *const c_char, label: &str) -> Result<Option<&'a str>, Error> {
    if value.is_null() {
        Ok(None)
    } else {
        // SAFETY: Non-null input has the same contract as `required_utf8`.
        unsafe { required_utf8(value, label) }.map(Some)
    }
}

unsafe fn activity_edges_from_abi(
    edges: *const PpActivityEdge,
    count: u64,
    label: &str,
) -> Result<Vec<(RepresentationId, Option<ActivityRole>)>, Error> {
    let count = usize::try_from(count)
        .map_err(|_| invalid_argument(format!("activity {label} count is too large")))?;
    if count > MAX_ACTIVITY_EDGES {
        return Err(invalid_argument(format!(
            "activity {label} count must not exceed {MAX_ACTIVITY_EDGES}"
        )));
    }
    if count == 0 {
        return Ok(Vec::new());
    }
    if edges.is_null() {
        return Err(invalid_argument(format!(
            "activity {label} array must not be null when count is nonzero"
        )));
    }
    // SAFETY: The caller guarantees `count` readable contiguous edge values.
    unsafe { std::slice::from_raw_parts(edges, count) }
        .iter()
        .map(|edge| {
            // SAFETY: Each optional role follows the exported string contract.
            let role = unsafe { optional_utf8(edge.role, "activity edge role") }?
                .map(ActivityRole::new)
                .transpose()?;
            Ok((
                RepresentationId::from_bytes(edge.representation_id.bytes),
                role,
            ))
        })
        .collect()
}

fn invalid_argument(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidArgument, message)
}

unsafe fn external_identifier_from_abi(
    scheme: *const c_char,
    value: *const c_char,
    qualifier: *const c_char,
) -> Result<ExternalIdentifier, Error> {
    // SAFETY: The exported caller guarantees live NUL-terminated strings.
    let scheme = IdentifierScheme::new(unsafe { required_utf8(scheme, "scheme") }?)?;
    // SAFETY: Same contract as above.
    let value = unsafe { required_utf8(value, "value") }?;
    // SAFETY: Null is accepted for the optional qualifier.
    let qualifier = unsafe { optional_utf8(qualifier, "qualifier") }?.map(str::to_owned);
    ExternalIdentifier::new(scheme, value, qualifier)
}

unsafe fn metadata_property_from_abi(
    vocabulary: *const c_char,
    property: *const c_char,
) -> Result<MetadataProperty, Error> {
    // SAFETY: Exported callers guarantee live NUL-terminated strings.
    let vocabulary = VocabularyId::new(unsafe { required_utf8(vocabulary, "vocabulary") }?)?;
    // SAFETY: Same contract as above.
    let property = PropertyId::new(unsafe { required_utf8(property, "property") }?)?;
    Ok(MetadataProperty::new(vocabulary, property))
}

fn object_ref_from_abi(value: PpObjectRef) -> Result<ObjectRef, Error> {
    match value.kind {
        PP_OBJECT_PRODUCTION => Ok(ObjectRef::Production(ProductionId::from_bytes(
            value.id.bytes,
        ))),
        PP_OBJECT_ASSET => Ok(ObjectRef::Asset(AssetId::from_bytes(value.id.bytes))),
        PP_OBJECT_REPRESENTATION => Ok(ObjectRef::Representation(RepresentationId::from_bytes(
            value.id.bytes,
        ))),
        PP_OBJECT_RESOURCE => Ok(ObjectRef::Resource(ResourceId::from_bytes(value.id.bytes))),
        PP_OBJECT_ACTIVITY => Ok(ObjectRef::Activity(
            postproject_core::ActivityId::from_bytes(value.id.bytes),
        )),
        kind => Err(invalid_argument(format!(
            "object kind {kind} is not recognized"
        ))),
    }
}

pub(crate) fn object_ref_to_abi(value: ObjectRef) -> Result<PpObjectRef, Error> {
    let (kind, bytes) = match value {
        ObjectRef::Production(id) => (PP_OBJECT_PRODUCTION, id.into_bytes()),
        ObjectRef::Asset(id) => (PP_OBJECT_ASSET, id.into_bytes()),
        ObjectRef::Representation(id) => (PP_OBJECT_REPRESENTATION, id.into_bytes()),
        ObjectRef::Resource(id) => (PP_OBJECT_RESOURCE, id.into_bytes()),
        ObjectRef::Activity(id) => (PP_OBJECT_ACTIVITY, id.into_bytes()),
        _ => {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "object kind is not supported by this ABI",
            ));
        }
    };
    Ok(PpObjectRef {
        kind,
        id: PpUuid { bytes },
    })
}

const fn error_code(kind: ErrorKind) -> u32 {
    match kind {
        ErrorKind::InvalidArgument => PP_ERROR_INVALID_ARGUMENT,
        ErrorKind::NotFound => PP_ERROR_NOT_FOUND,
        ErrorKind::AlreadyExists => PP_ERROR_ALREADY_EXISTS,
        ErrorKind::Io => PP_ERROR_IO,
        ErrorKind::Storage => PP_ERROR_STORAGE,
        ErrorKind::Migration => PP_ERROR_MIGRATION,
        ErrorKind::Conflict => PP_ERROR_CONFLICT,
        ErrorKind::AmbiguousResolution => PP_ERROR_AMBIGUOUS_RESOLUTION,
        ErrorKind::Fingerprint => PP_ERROR_FINGERPRINT,
        ErrorKind::Unsupported => PP_ERROR_UNSUPPORTED,
        _ => PP_ERROR_INTERNAL,
    }
}

fn uuid(id: ProductionId) -> PpUuid {
    PpUuid {
        bytes: id.into_bytes(),
    }
}

fn production_handle(production: SqliteProduction) -> PpProduction {
    PpProduction {
        state: Arc::new(ProductionState {
            inner: Mutex::new(production),
            transaction_open: AtomicBool::new(false),
        }),
    }
}

fn lock_production(state: &ProductionState) -> MutexGuard<'_, SqliteProduction> {
    state
        .inner
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

unsafe fn representation_resolution_at<'a>(
    resolutions: *const PpResolutionSet,
    index: u64,
) -> Result<&'a AbiRepresentationResolution, Error> {
    // SAFETY: Exported callers guarantee a non-null handle remains live for the
    // complete call. The reference never escapes an exported operation.
    let resolutions = unsafe { resolutions.as_ref() }
        .ok_or_else(|| invalid_argument("resolutions must not be null"))?;
    item_at(
        &resolutions.representations,
        index,
        "representation resolution",
    )
}

unsafe fn resource_resolution_at<'a>(
    resolutions: *const PpResolutionSet,
    representation_index: u64,
    resource_index: u64,
) -> Result<&'a AbiResolution, Error> {
    // SAFETY: The caller upholds the live-handle contract for this complete call.
    let representation =
        unsafe { representation_resolution_at(resolutions, representation_index) }?;
    item_at(
        &representation.resources,
        resource_index,
        "resource resolution",
    )
}

fn item_at<'a, T>(items: &'a [T], index: u64, label: &str) -> Result<&'a T, Error> {
    let index = usize::try_from(index)
        .map_err(|_| invalid_argument(format!("{label} index is out of range")))?;
    items
        .get(index)
        .ok_or_else(|| invalid_argument(format!("{label} index {index} is out of range")))
}

fn length_as_u64(length: usize) -> Result<u64, Error> {
    u64::try_from(length).map_err(|_| Error::new(ErrorKind::Internal, "result is too large"))
}

unsafe fn metadata_value<'a>(value: *const PpMetadataValue) -> Result<&'a PpMetadataValue, Error> {
    // SAFETY: The exported caller guarantees the pointer is borrowed from a
    // live result set for the duration of the call.
    unsafe { value.as_ref() }.ok_or_else(|| invalid_argument("value must not be null"))
}

fn metadata_type_error(expected: &str) -> Error {
    invalid_argument(format!("metadata value is not {expected}"))
}

unsafe fn write_copy<T: Copy>(output: *mut T, value: T, label: &str) -> Result<(), Error> {
    require_output(output, label)?;
    // SAFETY: The caller contract requires this checked non-null output to be writable.
    unsafe { output.write(value) };
    Ok(())
}

impl PpResolutionSet {
    fn new(resolutions: Vec<RepresentationResolution>) -> Self {
        Self {
            representations: resolutions
                .into_iter()
                .map(|resolution| AbiRepresentationResolution {
                    representation_id: resolution.representation_id(),
                    availability: resolution.availability(),
                    resources: resolution
                        .resources()
                        .iter()
                        .map(|resource| AbiResolution {
                            resource_id: resource.resource_id(),
                            state: resolution_state(resource.state()),
                            candidates: resource
                                .candidates()
                                .iter()
                                .map(|candidate| AbiCandidate {
                                    uri: sanitized_cstring(candidate.uri()),
                                    confidence: candidate.confidence().basis_points(),
                                    evidence: candidate
                                        .evidence()
                                        .iter()
                                        .map(AbiEvidence::from)
                                        .collect(),
                                })
                                .collect(),
                            evidence: resource.evidence().iter().map(AbiEvidence::from).collect(),
                        })
                        .collect(),
                    issues: resolution.issues().to_vec(),
                })
                .collect(),
        }
    }
}

impl From<&ResolutionEvidence> for AbiEvidence {
    fn from(evidence: &ResolutionEvidence) -> Self {
        Self {
            kind: evidence_kind(evidence.kind()),
            detail: evidence.detail().map(sanitized_cstring),
        }
    }
}

fn sanitized_cstring(value: &str) -> CString {
    CString::new(value.replace('\0', "�")).unwrap_or_default()
}

pub(crate) fn exact_cstring(value: &str, label: &str) -> Result<CString, Error> {
    CString::new(value).map_err(|_| {
        Error::new(
            ErrorKind::Internal,
            format!("validated {label} unexpectedly contains NUL"),
        )
    })
}

impl TryFrom<ExternalIdentifier> for AbiExternalIdentifier {
    type Error = Error;

    fn try_from(identifier: ExternalIdentifier) -> Result<Self, Self::Error> {
        Ok(Self {
            scheme: exact_cstring(identifier.scheme().as_str(), "identifier scheme")?,
            value: exact_cstring(identifier.value(), "identifier value")?,
            qualifier: identifier
                .qualifier()
                .map(|value| exact_cstring(value, "identifier qualifier"))
                .transpose()?,
        })
    }
}

const fn resolution_state(state: ResourceResolutionState) -> u32 {
    match state {
        ResourceResolutionState::OnlineAtKnownLocator => PP_RESOURCE_ONLINE_AT_KNOWN_LOCATOR,
        ResourceResolutionState::ResolvedExact => PP_RESOURCE_RESOLVED_EXACT,
        ResourceResolutionState::ResolvedProbable => PP_RESOURCE_RESOLVED_PROBABLE,
        ResourceResolutionState::Offline => PP_RESOURCE_OFFLINE,
        ResourceResolutionState::Ambiguous => PP_RESOURCE_AMBIGUOUS,
        ResourceResolutionState::Error => PP_RESOURCE_RESOLUTION_ERROR,
        _ => 0,
    }
}

const fn representation_availability(availability: RepresentationAvailability) -> u32 {
    match availability {
        RepresentationAvailability::Online => PP_AVAILABILITY_ONLINE,
        RepresentationAvailability::Partial => PP_AVAILABILITY_PARTIAL,
        RepresentationAvailability::Offline => PP_AVAILABILITY_OFFLINE,
        RepresentationAvailability::Ambiguous => PP_AVAILABILITY_AMBIGUOUS,
        RepresentationAvailability::Error => PP_AVAILABILITY_ERROR,
        _ => 0,
    }
}

const fn availability_issue_kind(kind: AvailabilityIssueKind) -> u32 {
    match kind {
        AvailabilityIssueKind::OfflineResource => PP_AVAILABILITY_ISSUE_OFFLINE_RESOURCE,
        AvailabilityIssueKind::AmbiguousResource => PP_AVAILABILITY_ISSUE_AMBIGUOUS_RESOURCE,
        AvailabilityIssueKind::ResourceError => PP_AVAILABILITY_ISSUE_RESOURCE_ERROR,
        AvailabilityIssueKind::MissingFrames => PP_AVAILABILITY_ISSUE_MISSING_FRAMES,
        _ => 0,
    }
}

const fn evidence_kind(kind: EvidenceKind) -> u32 {
    match kind {
        EvidenceKind::KnownLocatorAvailable => PP_EVIDENCE_KNOWN_LOCATOR_AVAILABLE,
        EvidenceKind::ExactFingerprintMatch => PP_EVIDENCE_EXACT_FINGERPRINT_MATCH,
        EvidenceKind::FullHashMatch => PP_EVIDENCE_FULL_HASH_MATCH,
        EvidenceKind::PartialFingerprintMatch => PP_EVIDENCE_PARTIAL_FINGERPRINT_MATCH,
        EvidenceKind::FileSizeMatch => PP_EVIDENCE_FILE_SIZE_MATCH,
        EvidenceKind::FileNameMatch => PP_EVIDENCE_FILE_NAME_MATCH,
        EvidenceKind::RelativePathSimilarity => PP_EVIDENCE_RELATIVE_PATH_SIMILARITY,
        EvidenceKind::MediaRootRelation => PP_EVIDENCE_MEDIA_ROOT_RELATION,
        EvidenceKind::ConflictingCandidate => PP_EVIDENCE_CONFLICTING_CANDIDATE,
        EvidenceKind::DiscoveryError => PP_EVIDENCE_DISCOVERY_ERROR,
        _ => 0,
    }
}

impl PpTransaction {
    fn commit(&mut self) -> Result<(), Error> {
        self.lifecycle.ensure_open()?;
        let result = (|| {
            let mut production = lock_production(&self.state);
            let mut transaction = production.begin_transaction()?;
            transaction.set_revision_context(self.revision_context.clone())?;
            for mutation in &self.mutations {
                match mutation {
                    StagedMutation::Import(import) => transaction.import_original(import)?,
                    StagedMutation::MediaRoot(root) => transaction.add_media_root(root.clone())?,
                    StagedMutation::Locator(locator) => transaction.add_locator(locator)?,
                    StagedMutation::AddExternalIdentifier(target, identifier) => {
                        transaction.add_external_identifier(*target, identifier)?;
                    }
                    StagedMutation::RemoveExternalIdentifier(target, identifier) => {
                        transaction.remove_external_identifier(*target, identifier)?;
                    }
                    StagedMutation::AddMetadataValue(target, property, value) => {
                        transaction.add_metadata_value(*target, property, value)?;
                    }
                    StagedMutation::RemoveMetadataProperty(target, property) => {
                        transaction.remove_metadata_property(*target, property)?;
                    }
                    StagedMutation::Activity(activity) => {
                        transaction.create_activity(activity)?;
                    }
                }
            }
            transaction.commit()
        })();

        self.state.transaction_open.store(false, Ordering::Release);
        if result.is_ok() {
            self.lifecycle.mark_committed()?;
            self.mutations.clear();
        } else {
            let state_result = self.lifecycle.mark_rolled_back();
            debug_assert!(state_result.is_ok());
        }
        result
    }

    fn rollback(&mut self) -> Result<(), Error> {
        self.lifecycle.mark_rolled_back()?;
        self.mutations.clear();
        self.state.transaction_open.store(false, Ordering::Release);
        Ok(())
    }
}

impl Drop for PpTransaction {
    fn drop(&mut self) {
        self.state.transaction_open.store(false, Ordering::Release);
    }
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    payload.downcast_ref::<&str>().map_or_else(
        || {
            payload.downcast_ref::<String>().map_or_else(
                || "panic contained at the C ABI boundary".to_owned(),
                |message| format!("panic contained at the C ABI boundary: {message}"),
            )
        },
        |message| format!("panic contained at the C ABI boundary: {message}"),
    )
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, thread, time::Duration};

    use super::*;
    use postproject_core::{
        Confidence, ContentStructure, EvidenceKind, FrameRange, ImageSequenceDescriptor,
        ImageSequencePattern, MetadataAssertion, MetadataField, MetadataProperty, PropertyId,
        RationalRate, ResolutionCandidate, ResourceResolution, VocabularyId,
    };

    #[test]
    fn creates_reads_and_releases_production_handle() {
        let directory = tempfile::tempdir().expect("create directory");
        let path = CString::new(
            directory
                .path()
                .join("production.pproj")
                .to_string_lossy()
                .as_bytes(),
        )
        .expect("path has no NUL");
        let mut production = ptr::null_mut();
        let mut error = ptr::null_mut();

        // SAFETY: Test inputs and outputs follow the documented ABI contract.
        let status = unsafe {
            pp_production_create(
                path.as_ptr(),
                ptr::null(),
                &raw mut production,
                &raw mut error,
            )
        };
        assert_eq!(status, PP_OK);
        assert!(!production.is_null());
        assert!(error.is_null());

        let mut id = PpUuid { bytes: [0; 16] };
        // SAFETY: `production` is live and outputs are writable.
        assert_eq!(
            unsafe { pp_production_id(production, &raw mut id, &raw mut error) },
            PP_OK
        );
        assert_ne!(id.bytes, [0; 16]);
        // SAFETY: The live handle is released exactly once.
        unsafe { pp_production_release(production) };
    }

    #[test]
    fn production_handles_are_send_sync_and_block_on_concurrent_access() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PpProduction>();

        let directory = tempfile::tempdir().expect("create directory");
        let production = SqliteProduction::create(directory.path().join("production.pproj"), None)
            .expect("create production");
        let handle = Arc::new(production_handle(production));
        let guard = lock_production(&handle.state);
        let worker_handle = Arc::clone(&handle);
        let (sender, receiver) = mpsc::channel();
        let worker = thread::spawn(move || {
            let mut id = PpUuid { bytes: [0; 16] };
            let mut error = ptr::null_mut();
            // SAFETY: The Arc keeps the handle live, the outputs are local, and
            // no thread releases the handle while this call runs.
            let status = unsafe {
                pp_production_id(Arc::as_ptr(&worker_handle), &raw mut id, &raw mut error)
            };
            sender.send((status, id, error.is_null())).unwrap();
        });

        assert!(matches!(
            receiver.recv_timeout(Duration::from_millis(50)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        drop(guard);
        let (status, id, error_is_null) = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("concurrent read completes after unlock");
        worker.join().expect("join reader");
        assert_eq!(status, PP_OK);
        assert_ne!(id.bytes, [0; 16]);
        assert!(error_is_null);
    }

    #[test]
    fn poisoned_production_lock_is_recovered() {
        let directory = tempfile::tempdir().expect("create directory");
        let production = SqliteProduction::create(directory.path().join("production.pproj"), None)
            .expect("create production");
        let handle = Arc::new(production_handle(production));
        let state = Arc::clone(&handle.state);
        assert!(
            thread::spawn(move || {
                let _guard = lock_production(&state);
                panic!("poison production lock");
            })
            .join()
            .is_err()
        );

        let mut id = PpUuid { bytes: [0; 16] };
        let mut error = ptr::null_mut();
        // SAFETY: The Arc keeps the handle live and outputs are writable.
        let status = unsafe { pp_production_id(Arc::as_ptr(&handle), &raw mut id, &raw mut error) };
        assert_eq!(status, PP_OK);
        assert_ne!(id.bytes, [0; 16]);
        assert!(error.is_null());
    }

    #[test]
    fn invalid_arguments_return_owned_error() {
        let mut production = ptr::null_mut();
        let mut error = ptr::null_mut();

        // SAFETY: Null is intentionally supplied where the API validates it.
        let status =
            unsafe { pp_production_open(ptr::null(), &raw mut production, &raw mut error) };
        assert_eq!(status, PP_ERROR_INVALID_ARGUMENT);
        assert!(production.is_null());
        assert!(!error.is_null());
        // SAFETY: `error` is a live library-owned error handle.
        assert_eq!(unsafe { pp_error_code(error) }, status);
        // SAFETY: The message is borrowed while `error` remains live.
        assert!(!unsafe { pp_error_message(error) }.is_null());
        // SAFETY: The live error is released exactly once.
        unsafe { pp_error_release(error) };
    }

    #[test]
    fn transaction_retains_production_state_and_commits_import() {
        let directory = tempfile::tempdir().expect("create directory");
        let production_path = directory.path().join("production.pproj");
        let media_path = directory.path().join("clip.mov");
        std::fs::write(&media_path, b"FFI transaction media").expect("write media");
        let production_path = CString::new(production_path.to_string_lossy().as_bytes())
            .expect("production path has no NUL");
        let media_path =
            CString::new(media_path.to_string_lossy().as_bytes()).expect("media path has no NUL");
        let mut production = ptr::null_mut();
        let mut transaction = ptr::null_mut();
        let mut error = ptr::null_mut();

        // SAFETY: Test inputs and outputs follow the documented ABI contract.
        assert_eq!(
            unsafe {
                pp_production_create(
                    production_path.as_ptr(),
                    ptr::null(),
                    &raw mut production,
                    &raw mut error,
                )
            },
            PP_OK
        );
        // SAFETY: `production` is live and the transaction output is writable.
        assert_eq!(
            unsafe {
                pp_production_begin_transaction(production, &raw mut transaction, &raw mut error)
            },
            PP_OK
        );
        // SAFETY: The transaction retains shared ownership of the state.
        unsafe { pp_production_release(production) };

        let mut asset_id = PpUuid { bytes: [0; 16] };
        // SAFETY: `transaction` is live and inputs/outputs satisfy the contract.
        assert_eq!(
            unsafe {
                pp_transaction_import_media(
                    transaction,
                    media_path.as_ptr(),
                    ptr::null(),
                    &raw mut asset_id,
                    &raw mut error,
                )
            },
            PP_OK
        );
        // SAFETY: `transaction` remains live and exclusively accessed.
        assert_eq!(
            unsafe { pp_transaction_commit(transaction, &raw mut error) },
            PP_OK
        );
        // SAFETY: The live transaction is released exactly once.
        unsafe { pp_transaction_release(transaction) };

        let reopened = SqliteProduction::open(Path::new(
            production_path.to_str().expect("production path is UTF-8"),
        ))
        .expect("reopen production");
        let assets = reopened.assets().expect("load committed assets");
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].id().into_bytes(), asset_id.bytes);
        assert!(error.is_null());
    }

    #[test]
    fn resolution_accessors_reject_out_of_range_indices() {
        let resolutions = Box::into_raw(Box::new(PpResolutionSet::new(Vec::new())));
        let mut representation_id = PpUuid { bytes: [9; 16] };
        let mut availability = 99;
        let mut resource_count = 99;
        let mut issue_count = 99;
        let mut error = ptr::null_mut();

        // SAFETY: The handle is live and every output is writable.
        let status = unsafe {
            pp_resolution_set_get_representation(
                resolutions,
                0,
                &raw mut representation_id,
                &raw mut availability,
                &raw mut resource_count,
                &raw mut issue_count,
                &raw mut error,
            )
        };
        assert_eq!(status, PP_ERROR_INVALID_ARGUMENT);
        assert_eq!(representation_id.bytes, [0; 16]);
        assert_eq!(availability, 0);
        assert_eq!(resource_count, 0);
        assert_eq!(issue_count, 0);
        assert!(!error.is_null());

        // SAFETY: Both handles are live and released exactly once.
        unsafe {
            pp_error_release(error);
            pp_resolution_set_release(resolutions);
        }
        // SAFETY: Null is explicitly accepted by the count accessor.
        assert_eq!(
            unsafe { pp_resolution_set_representation_count(ptr::null()) },
            0
        );
    }

    #[test]
    fn nested_resolution_accessors_expose_missing_sequence_frames() {
        let representation_id = RepresentationId::new();
        let resource_id = ResourceId::new();
        let descriptor = ImageSequenceDescriptor::new(
            resource_id,
            ImageSequencePattern::new("plate.", ".exr", 4).expect("valid pattern"),
            FrameRange::new(1001, 1003, 1).expect("valid frames"),
            RationalRate::new(24, 1).expect("valid rate"),
            vec![1002],
        )
        .expect("valid sequence");
        let candidate = ResolutionCandidate::new(
            "file:///plates/",
            Confidence::CERTAIN,
            vec![ResolutionEvidence::new(
                EvidenceKind::KnownLocatorAvailable,
                None,
            )],
        )
        .expect("valid candidate");
        let resource = ResourceResolution::new(
            resource_id,
            ResourceResolutionState::OnlineAtKnownLocator,
            vec![candidate],
            Vec::new(),
        )
        .expect("valid resource result");
        let representation = RepresentationResolution::aggregate(
            representation_id,
            &ContentStructure::image_sequence(descriptor),
            vec![resource],
        )
        .expect("valid aggregate");
        let resolutions = Box::into_raw(Box::new(PpResolutionSet::new(vec![representation])));
        let mut id = PpUuid { bytes: [0; 16] };
        let mut availability = 0;
        let mut resource_count = 0;
        let mut issue_count = 0;
        let mut error = ptr::null_mut();

        // SAFETY: The handle is live and all outputs are writable.
        assert_eq!(
            unsafe {
                pp_resolution_set_get_representation(
                    resolutions,
                    0,
                    &raw mut id,
                    &raw mut availability,
                    &raw mut resource_count,
                    &raw mut issue_count,
                    &raw mut error,
                )
            },
            PP_OK
        );
        assert_eq!(id.bytes, representation_id.into_bytes());
        assert_eq!(availability, PP_AVAILABILITY_PARTIAL);
        assert_eq!(resource_count, 1);
        assert_eq!(issue_count, 1);

        let mut required = 0;
        let mut kind = 0;
        let mut frame_count = 0;
        // SAFETY: The handle remains live and all outputs are writable.
        assert_eq!(
            unsafe {
                pp_resolution_set_get_issue(
                    resolutions,
                    0,
                    0,
                    &raw mut id,
                    &raw mut required,
                    &raw mut kind,
                    &raw mut frame_count,
                    &raw mut error,
                )
            },
            PP_OK
        );
        assert_eq!(id.bytes, resource_id.into_bytes());
        assert_eq!(required, 1);
        assert_eq!(kind, PP_AVAILABILITY_ISSUE_MISSING_FRAMES);
        assert_eq!(frame_count, 1);

        let mut frame = 0;
        // SAFETY: The handle remains live and the output is writable.
        assert_eq!(
            unsafe {
                pp_resolution_set_get_issue_frame(
                    resolutions,
                    0,
                    0,
                    0,
                    &raw mut frame,
                    &raw mut error,
                )
            },
            PP_OK
        );
        assert_eq!(frame, 1002);
        assert!(error.is_null());
        // SAFETY: The live handle is released exactly once.
        unsafe { pp_resolution_set_release(resolutions) };
    }

    #[test]
    fn metadata_accessors_traverse_recursive_values() {
        let asset = ObjectRef::Asset(AssetId::from_bytes([7; 16]));
        let value = MetadataValue::structure(vec![
            MetadataField::new(
                PropertyId::new("labels").unwrap(),
                MetadataValue::list(vec![
                    MetadataValue::language_string("Interview", "en-US").unwrap(),
                ])
                .unwrap(),
            ),
            MetadataField::new(
                PropertyId::new("source").unwrap(),
                MetadataValue::reference(asset),
            ),
        ])
        .unwrap();
        let assertion = MetadataAssertion::new(
            MetadataProperty::new(
                VocabularyId::new("com.example.metadata").unwrap(),
                PropertyId::new("contact").unwrap(),
            ),
            value,
        );
        let metadata = Box::into_raw(Box::new(
            PpMetadataSet::from_assertions(asset, &[assertion]).unwrap(),
        ));
        let mut target = PpObjectRef {
            kind: 0,
            id: PpUuid { bytes: [0; 16] },
        };
        let mut vocabulary = ptr::null();
        let mut property = ptr::null();
        let mut root = ptr::null();
        let mut error = ptr::null_mut();

        // SAFETY: The result set is live and every output is writable.
        assert_eq!(
            unsafe {
                pp_metadata_set_get(
                    metadata,
                    0,
                    &raw mut target,
                    &raw mut vocabulary,
                    &raw mut property,
                    &raw mut root,
                    &raw mut error,
                )
            },
            PP_OK
        );
        assert_eq!(target.kind, PP_OBJECT_ASSET);
        assert_eq!(
            unsafe { pp_metadata_value_kind(root) },
            metadata::PP_METADATA_STRUCT
        );
        assert_eq!(unsafe { pp_metadata_value_struct_count(root) }, 2);

        let mut field_name = ptr::null();
        let mut list = ptr::null();
        assert_eq!(
            unsafe {
                pp_metadata_value_struct_get(
                    root,
                    0,
                    &raw mut field_name,
                    &raw mut list,
                    &raw mut error,
                )
            },
            PP_OK
        );
        assert_eq!(unsafe { CStr::from_ptr(field_name) }.to_bytes(), b"labels");
        assert_eq!(unsafe { pp_metadata_value_list_count(list) }, 1);
        let mut text_value = ptr::null();
        assert_eq!(
            unsafe { pp_metadata_value_list_get(list, 0, &raw mut text_value, &raw mut error) },
            PP_OK
        );
        let mut text = ptr::null();
        let mut language = ptr::null();
        assert_eq!(
            unsafe {
                pp_metadata_value_get_string(
                    text_value,
                    &raw mut text,
                    &raw mut language,
                    &raw mut error,
                )
            },
            PP_OK
        );
        assert_eq!(unsafe { CStr::from_ptr(text) }.to_bytes(), b"Interview");
        assert_eq!(unsafe { CStr::from_ptr(language) }.to_bytes(), b"en-US");

        // SAFETY: The live set is released after all borrowed pointers are done.
        unsafe { pp_metadata_set_release(metadata) };
        assert!(error.is_null());
    }
}
