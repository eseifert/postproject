//! Public C ABI for `PostProject`.
//!
//! All exported calls contain Rust panics and translate domain errors into stable
//! numeric codes plus owned error objects. Native consumers should include the
//! shipped `postproject.h` rather than depending on Rust declarations.

#[allow(
    dead_code,
    reason = "metadata projection is exposed by the following focused ABI changes"
)]
mod metadata;

use std::{
    any::Any,
    cell::{Cell, RefCell},
    ffi::{CStr, CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    ptr,
    rc::Rc,
};

use postproject_core::{
    AssetId, Error, ErrorKind, EvidenceKind, ExternalIdentifier, IdentifierScheme, Location,
    MediaRoot, MetadataProperty, ObjectRef, OriginalMediaImport, ProjectId,
    PropertyId, RepresentationId, Resolution, ResolutionEvidence, ResolutionState,
    TransactionLifecycle, VocabularyId,
};
use postproject_media::{
    MediaResolver, prepare_confirmed_location, prepare_media_root, prepare_original_media,
};
use postproject_storage_sqlite::SqliteProject;

use metadata::AbiMetadataValue;
pub use metadata::{PpMetadataSet, PpMetadataValue};

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

const PP_RESOLUTION_ONLINE_AT_KNOWN_LOCATION: u32 = 1;
const PP_RESOLUTION_RESOLVED_EXACT: u32 = 2;
const PP_RESOLUTION_RESOLVED_PROBABLE: u32 = 3;
const PP_RESOLUTION_MISSING: u32 = 4;
const PP_RESOLUTION_AMBIGUOUS: u32 = 5;
const PP_RESOLUTION_ERROR: u32 = 6;

const PP_EVIDENCE_KNOWN_LOCATION_EXISTS: u32 = 1;
const PP_EVIDENCE_EXACT_FINGERPRINT_MATCH: u32 = 2;
const PP_EVIDENCE_FULL_HASH_MATCH: u32 = 3;
const PP_EVIDENCE_PARTIAL_FINGERPRINT_MATCH: u32 = 4;
const PP_EVIDENCE_FILE_SIZE_MATCH: u32 = 5;
const PP_EVIDENCE_FILE_NAME_MATCH: u32 = 6;
const PP_EVIDENCE_RELATIVE_PATH_SIMILARITY: u32 = 7;
const PP_EVIDENCE_MEDIA_ROOT_RELATION: u32 = 8;
const PP_EVIDENCE_CONFLICTING_CANDIDATE: u32 = 9;
const PP_EVIDENCE_DISCOVERY_ERROR: u32 = 10;

const PP_OBJECT_PROJECT: u32 = 1;
const PP_OBJECT_ASSET: u32 = 2;
const PP_OBJECT_REPRESENTATION: u32 = 3;
const PP_OBJECT_ACTIVITY: u32 = 4;

/// Current pre-1.0 ABI version.
pub const ABI_VERSION: u32 = 3;

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

/// Opaque project handle owned by the C caller.
pub struct PpProject {
    state: Rc<ProjectState>,
}

struct ProjectState {
    inner: RefCell<SqliteProject>,
    transaction_open: Cell<bool>,
}

/// Opaque transaction handle owned by the C caller.
pub struct PpTransaction {
    state: Rc<ProjectState>,
    lifecycle: TransactionLifecycle,
    mutations: Vec<StagedMutation>,
}

enum StagedMutation {
    Import(OriginalMediaImport),
    MediaRoot(MediaRoot),
    Location(Location),
    AddExternalIdentifier(ObjectRef, ExternalIdentifier),
    RemoveExternalIdentifier(ObjectRef, ExternalIdentifier),
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
    resolutions: Vec<AbiResolution>,
}

struct AbiResolution {
    representation_id: RepresentationId,
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

/// Creates a new project file.
///
/// # Safety
///
/// `path` must point to a NUL-terminated byte string for the duration of the
/// call. `display_name` may be null or must satisfy the same rule. `out_project`
/// must be a writable pointer. `out_error` may be null or writable. Successful
/// handles must be released exactly once with [`pp_project_release`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_project_create(
    path: *const c_char,
    display_name: *const c_char,
    out_project: *mut *mut PpProject,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: The caller contract for each pointer is documented above. Helpers
    // validate nullability before dereferencing and borrow inputs only this call.
    unsafe {
        initialize_output(out_project);
        ffi_call(out_error, || {
            if out_project.is_null() {
                return Err(invalid_argument("out_project must not be null"));
            }
            let path = required_utf8(path, "path")?;
            if path.is_empty() {
                return Err(invalid_argument("path must not be empty"));
            }
            let display_name = optional_utf8(display_name, "display_name")?.map(str::to_owned);
            let project = SqliteProject::create(Path::new(path), display_name)?;
            out_project.write(Box::into_raw(Box::new(project_handle(project))));
            Ok(())
        })
    }
}

/// Opens an existing project file.
///
/// # Safety
///
/// `path` must point to a NUL-terminated byte string for the duration of the
/// call. `out_project` must be writable. `out_error` may be null or writable.
/// Successful handles must be released exactly once with [`pp_project_release`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_project_open(
    path: *const c_char,
    out_project: *mut *mut PpProject,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: The caller contract for each pointer is documented above. Helpers
    // validate nullability before dereferencing and borrow inputs only this call.
    unsafe {
        initialize_output(out_project);
        ffi_call(out_error, || {
            if out_project.is_null() {
                return Err(invalid_argument("out_project must not be null"));
            }
            let path = required_utf8(path, "path")?;
            if path.is_empty() {
                return Err(invalid_argument("path must not be empty"));
            }
            let project = SqliteProject::open(Path::new(path))?;
            out_project.write(Box::into_raw(Box::new(project_handle(project))));
            Ok(())
        })
    }
}

/// Copies the stable project identity into caller-owned storage.
///
/// # Safety
///
/// `project` must be a live handle returned by this library. `out_id` must be
/// writable. `out_error` may be null or writable. The project must not be used
/// concurrently by another thread during the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_project_id(
    project: *const PpProject,
    out_id: *mut PpUuid,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Null pointers are rejected before dereference; non-null pointer
    // validity and synchronization are guaranteed by the caller contract.
    unsafe {
        ffi_call(out_error, || {
            let project = project
                .as_ref()
                .ok_or_else(|| invalid_argument("project must not be null"))?;
            if out_id.is_null() {
                return Err(invalid_argument("out_id must not be null"));
            }
            let inner = project
                .state
                .inner
                .try_borrow()
                .map_err(|_| Error::new(ErrorKind::Conflict, "project is already in use"))?;
            out_id.write(uuid(inner.project().id()));
            Ok(())
        })
    }
}

/// Reports whether a stable asset identity exists in a project.
///
/// # Safety
///
/// `project` must be a live handle returned by this library. `asset_id` must be
/// readable and `out_exists` writable. `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_project_asset_exists(
    project: *const PpProject,
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
            let project = project
                .as_ref()
                .ok_or_else(|| invalid_argument("project must not be null"))?;
            let asset_id = asset_id
                .as_ref()
                .ok_or_else(|| invalid_argument("asset_id must not be null"))?;
            if out_exists.is_null() {
                return Err(invalid_argument("out_exists must not be null"));
            }
            let inner = project
                .state
                .inner
                .try_borrow()
                .map_err(|_| Error::new(ErrorKind::Conflict, "project is already in use"))?;
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
/// `project` and `target` must be readable live values. `out_identifiers` must
/// be writable. `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_project_external_identifiers(
    project: *const PpProject,
    target: *const PpObjectRef,
    out_identifiers: *mut *mut PpExternalIdentifierSet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Pointers are validated before use and outputs are initialized.
    unsafe {
        initialize_output(out_identifiers);
        ffi_call(out_error, || {
            let project = project
                .as_ref()
                .ok_or_else(|| invalid_argument("project must not be null"))?;
            let target = target
                .as_ref()
                .ok_or_else(|| invalid_argument("target must not be null"))?;
            if out_identifiers.is_null() {
                return Err(invalid_argument("out_identifiers must not be null"));
            }
            let target = object_ref_from_abi(*target)?;
            let inner = project
                .state
                .inner
                .try_borrow()
                .map_err(|_| Error::new(ErrorKind::Conflict, "project is already in use"))?;
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
/// `project` must be live; `scheme` and `value` must be borrowed NUL-terminated
/// UTF-8 strings; `out_objects` must be writable; and `out_error` may be null or
/// writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_project_find_by_external_identifier(
    project: *const PpProject,
    scheme: *const c_char,
    value: *const c_char,
    out_objects: *mut *mut PpObjectRefSet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Pointers are validated before use and outputs are initialized.
    unsafe {
        initialize_output(out_objects);
        ffi_call(out_error, || {
            let project = project
                .as_ref()
                .ok_or_else(|| invalid_argument("project must not be null"))?;
            if out_objects.is_null() {
                return Err(invalid_argument("out_objects must not be null"));
            }
            let scheme = IdentifierScheme::new(required_utf8(scheme, "scheme")?)?;
            let value = required_utf8(value, "value")?;
            let inner = project
                .state
                .inner
                .try_borrow()
                .map_err(|_| Error::new(ErrorKind::Conflict, "project is already in use"))?;
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
/// `project` and `target` must be readable live values. `out_metadata` must be
/// writable and `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_project_metadata(
    project: *const PpProject,
    target: *const PpObjectRef,
    out_metadata: *mut *mut PpMetadataSet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Inputs are validated before use and output ownership is explicit.
    unsafe {
        initialize_output(out_metadata);
        ffi_call(out_error, || {
            let project = project
                .as_ref()
                .ok_or_else(|| invalid_argument("project must not be null"))?;
            let target = target
                .as_ref()
                .ok_or_else(|| invalid_argument("target must not be null"))?;
            require_output(out_metadata, "out_metadata")?;
            let target = object_ref_from_abi(*target)?;
            let inner = project
                .state
                .inner
                .try_borrow()
                .map_err(|_| Error::new(ErrorKind::Conflict, "project is already in use"))?;
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
/// `project` must be live, strings must be borrowed NUL-terminated UTF-8,
/// `out_metadata` must be writable, and `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_project_find_metadata(
    project: *const PpProject,
    vocabulary: *const c_char,
    property: *const c_char,
    out_metadata: *mut *mut PpMetadataSet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Inputs are validated before use and output ownership is explicit.
    unsafe {
        initialize_output(out_metadata);
        ffi_call(out_error, || {
            let project = project
                .as_ref()
                .ok_or_else(|| invalid_argument("project must not be null"))?;
            require_output(out_metadata, "out_metadata")?;
            let property = metadata_property_from_abi(vocabulary, property)?;
            let inner = project
                .state
                .inner
                .try_borrow()
                .map_err(|_| Error::new(ErrorKind::Conflict, "project is already in use"))?;
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

/// Resolves every representation belonging to an asset without mutating the project.
///
/// The returned immutable result set owns all candidate URI and evidence-detail
/// strings exposed by its accessors.
///
/// # Safety
///
/// `project` must be a live handle, `asset_id` must be readable, and
/// `out_resolutions` must be writable. `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_project_resolve_asset(
    project: *const PpProject,
    asset_id: *const PpUuid,
    out_resolutions: *mut *mut PpResolutionSet,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Null pointers are rejected before dereference; remaining pointer
    // validity and synchronization are guaranteed by the caller contract.
    unsafe {
        initialize_output(out_resolutions);
        ffi_call(out_error, || {
            let project = project
                .as_ref()
                .ok_or_else(|| invalid_argument("project must not be null"))?;
            let asset_id = asset_id
                .as_ref()
                .ok_or_else(|| invalid_argument("asset_id must not be null"))?;
            if out_resolutions.is_null() {
                return Err(invalid_argument("out_resolutions must not be null"));
            }

            let inner = project
                .state
                .inner
                .try_borrow()
                .map_err(|_| Error::new(ErrorKind::Conflict, "project is already in use"))?;
            let asset_id = AssetId::from_bytes(asset_id.bytes);
            if !inner.assets()?.iter().any(|asset| asset.id() == asset_id) {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!("asset {asset_id} does not exist"),
                ));
            }

            let resolver = MediaResolver::default();
            let mut resolutions = Vec::new();
            for representation in inner.representations(asset_id)? {
                let locations = inner.locations(representation.id())?;
                resolutions.push(resolver.resolve(
                    &representation,
                    &locations,
                    inner.project().media_roots(),
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
pub unsafe extern "C" fn pp_resolution_set_count(resolutions: *const PpResolutionSet) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: A non-null pointer is live for the duration of this call by
        // the caller contract and is only borrowed.
        unsafe { resolutions.as_ref() }.map_or(0, |set| {
            u64::try_from(set.resolutions.len()).unwrap_or(u64::MAX)
        })
    }))
    .unwrap_or(0)
}

/// Reads one representation-level resolution result.
///
/// # Safety
///
/// `resolutions` must be live. All value outputs must be writable and
/// `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_resolution_set_get(
    resolutions: *const PpResolutionSet,
    resolution_index: u64,
    out_representation_id: *mut PpUuid,
    out_state: *mut u32,
    out_candidate_count: *mut u64,
    out_evidence_count: *mut u64,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and validated before use; the input
    // handle must remain live according to the caller contract.
    unsafe {
        initialize_uuid(out_representation_id);
        initialize_value(out_state, 0);
        initialize_value(out_candidate_count, 0);
        initialize_value(out_evidence_count, 0);
        ffi_call(out_error, || {
            require_output(out_representation_id, "out_representation_id")?;
            require_output(out_state, "out_state")?;
            require_output(out_candidate_count, "out_candidate_count")?;
            require_output(out_evidence_count, "out_evidence_count")?;
            let resolution = resolution_at(resolutions, resolution_index)?;
            out_representation_id.write(PpUuid {
                bytes: resolution.representation_id.into_bytes(),
            });
            out_state.write(resolution.state);
            out_candidate_count.write(length_as_u64(resolution.candidates.len())?);
            out_evidence_count.write(length_as_u64(resolution.evidence.len())?);
            Ok(())
        })
    }
}

/// Reads one candidate from a representation-level resolution result.
///
/// `out_uri` receives a borrowed NUL-terminated UTF-8 string that remains valid
/// until the resolution set is released.
///
/// # Safety
///
/// `resolutions` must be live. All outputs must be writable and `out_error` may
/// be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_resolution_candidate_get(
    resolutions: *const PpResolutionSet,
    resolution_index: u64,
    candidate_index: u64,
    out_uri: *mut *const c_char,
    out_confidence_basis_points: *mut u16,
    out_evidence_count: *mut u64,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Outputs are initialized and validated before use; the input
    // handle must remain live according to the caller contract.
    unsafe {
        initialize_const_output(out_uri);
        initialize_value(out_confidence_basis_points, 0);
        initialize_value(out_evidence_count, 0);
        ffi_call(out_error, || {
            require_output(out_uri, "out_uri")?;
            require_output(out_confidence_basis_points, "out_confidence_basis_points")?;
            require_output(out_evidence_count, "out_evidence_count")?;
            let resolution = resolution_at(resolutions, resolution_index)?;
            let candidate = item_at(&resolution.candidates, candidate_index, "candidate")?;
            out_uri.write(candidate.uri.as_ptr());
            out_confidence_basis_points.write(candidate.confidence);
            out_evidence_count.write(length_as_u64(candidate.evidence.len())?);
            Ok(())
        })
    }
}

/// Reads representation-level evidence from a resolution result.
///
/// `out_detail` receives null or a borrowed NUL-terminated UTF-8 string valid
/// until the resolution set is released.
///
/// # Safety
///
/// `resolutions` must be live. Outputs must be writable and `out_error` may be
/// null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_resolution_evidence_get(
    resolutions: *const PpResolutionSet,
    resolution_index: u64,
    evidence_index: u64,
    out_kind: *mut u32,
    out_detail: *mut *const c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Initializes outputs, then delegates to the shared accessor after
    // validating the live handle.
    unsafe {
        initialize_value(out_kind, 0);
        initialize_const_output(out_detail);
        ffi_call(out_error, || {
            let resolution = resolution_at(resolutions, resolution_index)?;
            write_evidence(&resolution.evidence, evidence_index, out_kind, out_detail)
        })
    }
}

/// Reads candidate-level evidence from a resolution result.
///
/// `out_detail` receives null or a borrowed NUL-terminated UTF-8 string valid
/// until the resolution set is released.
///
/// # Safety
///
/// `resolutions` must be live. Outputs must be writable and `out_error` may be
/// null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_resolution_candidate_evidence_get(
    resolutions: *const PpResolutionSet,
    resolution_index: u64,
    candidate_index: u64,
    evidence_index: u64,
    out_kind: *mut u32,
    out_detail: *mut *const c_char,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Initializes outputs, then delegates to the shared accessor after
    // validating the live handle.
    unsafe {
        initialize_value(out_kind, 0);
        initialize_const_output(out_detail);
        ffi_call(out_error, || {
            let resolution = resolution_at(resolutions, resolution_index)?;
            let candidate = item_at(&resolution.candidates, candidate_index, "candidate")?;
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
/// At most one transaction may be open for a project state. The returned handle
/// keeps that state alive even if the original project handle is released.
///
/// # Safety
///
/// `project` must be a live handle returned by this library. `out_transaction`
/// must be writable. `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_project_begin_transaction(
    project: *mut PpProject,
    out_transaction: *mut *mut PpTransaction,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: The caller contract for each pointer is documented above. Outputs
    // are initialized before validation and the project is borrowed only here.
    unsafe {
        initialize_output(out_transaction);
        ffi_call(out_error, || {
            let project = project
                .as_ref()
                .ok_or_else(|| invalid_argument("project must not be null"))?;
            if out_transaction.is_null() {
                return Err(invalid_argument("out_transaction must not be null"));
            }
            if project.state.transaction_open.replace(true) {
                return Err(Error::new(
                    ErrorKind::Conflict,
                    "project already has an open transaction",
                ));
            }
            out_transaction.write(Box::into_raw(Box::new(PpTransaction {
                state: Rc::clone(&project.state),
                lifecycle: TransactionLifecycle::new(),
                mutations: Vec::new(),
            })));
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

/// Stages an explicitly confirmed URI for a representation.
///
/// The URI is borrowed UTF-8 without embedded NUL and must be absolute.
/// Confirmation is not durable until the transaction commits.
///
/// # Safety
///
/// `transaction` must be a live transaction handle, `representation_id` must be
/// readable, `uri` must be a NUL-terminated string, and `out_error` may be null
/// or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_transaction_confirm_location(
    transaction: *mut PpTransaction,
    representation_id: *const PpUuid,
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
            let representation_id = representation_id
                .as_ref()
                .ok_or_else(|| invalid_argument("representation_id must not be null"))?;
            let uri = required_utf8(uri, "uri")?;
            if uri.is_empty() {
                return Err(invalid_argument("uri must not be empty"));
            }
            let location = prepare_confirmed_location(
                RepresentationId::from_bytes(representation_id.bytes),
                uri.to_owned(),
            )?;
            transaction
                .mutations
                .push(StagedMutation::Location(location));
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

/// Releases a project handle. Passing null is a no-op.
///
/// # Safety
///
/// A non-null pointer must have been returned by this library and not previously
/// released. No other thread may use it during or after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_project_release(project: *mut PpProject) {
    if project.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: Ownership of a live allocation is required by this function's
        // contract and is reconstructed exactly once here.
        drop(unsafe { Box::from_raw(project) });
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
        PP_OBJECT_PROJECT => Ok(ObjectRef::Project(ProjectId::from_bytes(value.id.bytes))),
        PP_OBJECT_ASSET => Ok(ObjectRef::Asset(AssetId::from_bytes(value.id.bytes))),
        PP_OBJECT_REPRESENTATION => Ok(ObjectRef::Representation(RepresentationId::from_bytes(
            value.id.bytes,
        ))),
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
        ObjectRef::Project(id) => (PP_OBJECT_PROJECT, id.into_bytes()),
        ObjectRef::Asset(id) => (PP_OBJECT_ASSET, id.into_bytes()),
        ObjectRef::Representation(id) => (PP_OBJECT_REPRESENTATION, id.into_bytes()),
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

fn uuid(id: ProjectId) -> PpUuid {
    PpUuid {
        bytes: id.into_bytes(),
    }
}

fn project_handle(project: SqliteProject) -> PpProject {
    PpProject {
        state: Rc::new(ProjectState {
            inner: RefCell::new(project),
            transaction_open: Cell::new(false),
        }),
    }
}

unsafe fn resolution_at<'a>(
    resolutions: *const PpResolutionSet,
    index: u64,
) -> Result<&'a AbiResolution, Error> {
    // SAFETY: Exported callers guarantee a non-null handle remains live for the
    // complete call. The reference never escapes an exported operation.
    let resolutions = unsafe { resolutions.as_ref() }
        .ok_or_else(|| invalid_argument("resolutions must not be null"))?;
    item_at(&resolutions.resolutions, index, "resolution")
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

impl PpResolutionSet {
    fn new(resolutions: Vec<Resolution>) -> Self {
        Self {
            resolutions: resolutions
                .into_iter()
                .map(|resolution| AbiResolution {
                    representation_id: resolution.representation_id(),
                    state: resolution_state(resolution.state()),
                    candidates: resolution
                        .candidates()
                        .iter()
                        .map(|candidate| AbiCandidate {
                            uri: sanitized_cstring(candidate.uri()),
                            confidence: candidate.confidence().basis_points(),
                            evidence: candidate.evidence().iter().map(AbiEvidence::from).collect(),
                        })
                        .collect(),
                    evidence: resolution
                        .evidence()
                        .iter()
                        .map(AbiEvidence::from)
                        .collect(),
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

const fn resolution_state(state: ResolutionState) -> u32 {
    match state {
        ResolutionState::OnlineAtKnownLocation => PP_RESOLUTION_ONLINE_AT_KNOWN_LOCATION,
        ResolutionState::ResolvedExact => PP_RESOLUTION_RESOLVED_EXACT,
        ResolutionState::ResolvedProbable => PP_RESOLUTION_RESOLVED_PROBABLE,
        ResolutionState::Missing => PP_RESOLUTION_MISSING,
        ResolutionState::Ambiguous => PP_RESOLUTION_AMBIGUOUS,
        ResolutionState::Error => PP_RESOLUTION_ERROR,
        _ => 0,
    }
}

const fn evidence_kind(kind: EvidenceKind) -> u32 {
    match kind {
        EvidenceKind::KnownLocationExists => PP_EVIDENCE_KNOWN_LOCATION_EXISTS,
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
            let mut project = self
                .state
                .inner
                .try_borrow_mut()
                .map_err(|_| Error::new(ErrorKind::Conflict, "project is already in use"))?;
            let mut transaction = project.begin_transaction()?;
            for mutation in &self.mutations {
                match mutation {
                    StagedMutation::Import(import) => transaction.import_original(import)?,
                    StagedMutation::MediaRoot(root) => transaction.add_media_root(root.clone())?,
                    StagedMutation::Location(location) => transaction.add_location(location)?,
                    StagedMutation::AddExternalIdentifier(target, identifier) => {
                        transaction.add_external_identifier(*target, identifier)?;
                    }
                    StagedMutation::RemoveExternalIdentifier(target, identifier) => {
                        transaction.remove_external_identifier(*target, identifier)?;
                    }
                }
            }
            transaction.commit()
        })();

        self.state.transaction_open.set(false);
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
        self.state.transaction_open.set(false);
        Ok(())
    }
}

impl Drop for PpTransaction {
    fn drop(&mut self) {
        self.state.transaction_open.set(false);
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
    use super::*;

    #[test]
    fn creates_reads_and_releases_project_handle() {
        let directory = tempfile::tempdir().expect("create directory");
        let path = CString::new(
            directory
                .path()
                .join("project.pproj")
                .to_string_lossy()
                .as_bytes(),
        )
        .expect("path has no NUL");
        let mut project = ptr::null_mut();
        let mut error = ptr::null_mut();

        // SAFETY: Test inputs and outputs follow the documented ABI contract.
        let status = unsafe {
            pp_project_create(path.as_ptr(), ptr::null(), &raw mut project, &raw mut error)
        };
        assert_eq!(status, PP_OK);
        assert!(!project.is_null());
        assert!(error.is_null());

        let mut id = PpUuid { bytes: [0; 16] };
        // SAFETY: `project` is live and outputs are writable.
        assert_eq!(
            unsafe { pp_project_id(project, &raw mut id, &raw mut error) },
            PP_OK
        );
        assert_ne!(id.bytes, [0; 16]);
        // SAFETY: The live handle is released exactly once.
        unsafe { pp_project_release(project) };
    }

    #[test]
    fn invalid_arguments_return_owned_error() {
        let mut project = ptr::null_mut();
        let mut error = ptr::null_mut();

        // SAFETY: Null is intentionally supplied where the API validates it.
        let status = unsafe { pp_project_open(ptr::null(), &raw mut project, &raw mut error) };
        assert_eq!(status, PP_ERROR_INVALID_ARGUMENT);
        assert!(project.is_null());
        assert!(!error.is_null());
        // SAFETY: `error` is a live library-owned error handle.
        assert_eq!(unsafe { pp_error_code(error) }, status);
        // SAFETY: The message is borrowed while `error` remains live.
        assert!(!unsafe { pp_error_message(error) }.is_null());
        // SAFETY: The live error is released exactly once.
        unsafe { pp_error_release(error) };
    }

    #[test]
    fn transaction_retains_project_state_and_commits_import() {
        let directory = tempfile::tempdir().expect("create directory");
        let project_path = directory.path().join("project.pproj");
        let media_path = directory.path().join("clip.mov");
        std::fs::write(&media_path, b"FFI transaction media").expect("write media");
        let project_path = CString::new(project_path.to_string_lossy().as_bytes())
            .expect("project path has no NUL");
        let media_path =
            CString::new(media_path.to_string_lossy().as_bytes()).expect("media path has no NUL");
        let mut project = ptr::null_mut();
        let mut transaction = ptr::null_mut();
        let mut error = ptr::null_mut();

        // SAFETY: Test inputs and outputs follow the documented ABI contract.
        assert_eq!(
            unsafe {
                pp_project_create(
                    project_path.as_ptr(),
                    ptr::null(),
                    &raw mut project,
                    &raw mut error,
                )
            },
            PP_OK
        );
        // SAFETY: `project` is live and the transaction output is writable.
        assert_eq!(
            unsafe { pp_project_begin_transaction(project, &raw mut transaction, &raw mut error,) },
            PP_OK
        );
        // SAFETY: The transaction retains shared ownership of the state.
        unsafe { pp_project_release(project) };

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

        let reopened = SqliteProject::open(Path::new(
            project_path.to_str().expect("project path is UTF-8"),
        ))
        .expect("reopen project");
        let assets = reopened.assets().expect("load committed assets");
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].id().into_bytes(), asset_id.bytes);
        assert!(error.is_null());
    }

    #[test]
    fn resolution_accessors_reject_out_of_range_indices() {
        let resolutions = Box::into_raw(Box::new(PpResolutionSet::new(Vec::new())));
        let mut representation_id = PpUuid { bytes: [9; 16] };
        let mut state = 99;
        let mut candidate_count = 99;
        let mut evidence_count = 99;
        let mut error = ptr::null_mut();

        // SAFETY: The handle is live and every output is writable.
        let status = unsafe {
            pp_resolution_set_get(
                resolutions,
                0,
                &raw mut representation_id,
                &raw mut state,
                &raw mut candidate_count,
                &raw mut evidence_count,
                &raw mut error,
            )
        };
        assert_eq!(status, PP_ERROR_INVALID_ARGUMENT);
        assert_eq!(representation_id.bytes, [0; 16]);
        assert_eq!(state, 0);
        assert_eq!(candidate_count, 0);
        assert_eq!(evidence_count, 0);
        assert!(!error.is_null());

        // SAFETY: Both handles are live and released exactly once.
        unsafe {
            pp_error_release(error);
            pp_resolution_set_release(resolutions);
        }
        // SAFETY: Null is explicitly accepted by the count accessor.
        assert_eq!(unsafe { pp_resolution_set_count(ptr::null()) }, 0);
    }
}
