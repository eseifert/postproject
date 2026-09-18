//! Stable C ABI for libpostproject.
//!
//! All exported calls contain Rust panics and translate domain errors into stable
//! numeric codes plus owned error objects. Native consumers should include the
//! shipped `postproject.h` rather than depending on Rust declarations.

use std::{
    any::Any,
    ffi::{CStr, CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    ptr,
};

use postproject_core::{Error, ErrorKind, ProjectId};
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
    inner: SqliteProject,
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
            out_project.write(Box::into_raw(Box::new(PpProject { inner: project })));
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
            out_project.write(Box::into_raw(Box::new(PpProject { inner: project })));
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
            out_id.write(uuid(project.inner.project().id()));
            Ok(())
        })
    }
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
}
