//! Stable C ABI for libpostproject.
//!
//! All exported calls contain Rust panics and translate domain errors into stable
//! numeric codes plus owned error objects. Native consumers should include the
//! shipped `postproject.h` rather than depending on Rust declarations.

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
    AssetId, Error, ErrorKind, MediaRoot, OriginalMediaImport, ProjectId, TransactionLifecycle,
};
use postproject_media::{prepare_media_root, prepare_original_media};
use postproject_storage_sqlite::SqliteProject;

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

/// Current iteration-1 ABI version.
pub const ABI_VERSION: u32 = 1;

/// Fixed-layout UUID-compatible public identifier.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PpUuid {
    /// UUID bytes in network order.
    pub bytes: [u8; 16],
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
}
