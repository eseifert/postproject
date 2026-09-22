//! Owned typed metadata values supplied by C callers.

use std::ffi::c_char;

use postproject_core::{DecimalValue, Error, MetadataValue, Timestamp};

use crate::{
    PpError, ffi_call, initialize_output, invalid_argument, optional_utf8, require_output,
    required_utf8,
};

/// Opaque owned metadata input handle.
pub struct PpMetadataInput {
    pub(crate) value: MetadataValue,
}

unsafe fn create_input(
    out_input: *mut *mut PpMetadataInput,
    out_error: *mut *mut PpError,
    operation: impl FnOnce() -> Result<MetadataValue, Error>,
) -> u32 {
    // SAFETY: The exported caller guarantees writable output pointers.
    unsafe {
        initialize_output(out_input);
        ffi_call(out_error, || {
            require_output(out_input, "out_input")?;
            let value = operation()?;
            out_input.write(Box::into_raw(Box::new(PpMetadataInput { value })));
            Ok(())
        })
    }
}

/// Creates an owned plain or language-tagged string input.
///
/// # Safety
///
/// `text` must be readable NUL-terminated UTF-8, `language` may be null or
/// readable NUL-terminated UTF-8, `out_input` must be writable, and `out_error`
/// may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_input_create_string(
    text: *const c_char,
    language: *const c_char,
    out_input: *mut *mut PpMetadataInput,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Pointer contracts are forwarded to the checked conversion helpers.
    unsafe {
        create_input(out_input, out_error, || {
            let text = required_utf8(text, "text")?;
            optional_utf8(language, "language")?.map_or_else(
                || MetadataValue::string(text),
                |language| MetadataValue::language_string(text, language),
            )
        })
    }
}

/// Creates an owned signed-integer input.
///
/// # Safety
///
/// `out_input` must be writable and `out_error` may be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_input_create_i64(
    value: i64,
    out_input: *mut *mut PpMetadataInput,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: The caller upholds the output-pointer contract.
    unsafe { create_input(out_input, out_error, || Ok(MetadataValue::i64(value))) }
}

/// Creates an owned unsigned-integer input.
///
/// # Safety
///
/// Pointer rules match [`pp_metadata_input_create_i64`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_input_create_u64(
    value: u64,
    out_input: *mut *mut PpMetadataInput,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: The caller upholds the output-pointer contract.
    unsafe { create_input(out_input, out_error, || Ok(MetadataValue::u64(value))) }
}

/// Creates an owned exact decimal input.
///
/// # Safety
///
/// `coefficient` must be readable NUL-terminated UTF-8. Output pointers follow
/// [`pp_metadata_input_create_i64`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_input_create_decimal(
    coefficient: *const c_char,
    scale: u32,
    out_input: *mut *mut PpMetadataInput,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: Pointer contracts are forwarded to the checked conversion helpers.
    unsafe {
        create_input(out_input, out_error, || {
            let coefficient = required_utf8(coefficient, "coefficient")?
                .parse::<i128>()
                .map_err(|_| invalid_argument("coefficient must be a signed base-10 integer"))?;
            Ok(MetadataValue::decimal(DecimalValue::new(
                coefficient,
                scale,
            )?))
        })
    }
}

/// Creates an owned Boolean input from exactly zero or one.
///
/// # Safety
///
/// Pointer rules match [`pp_metadata_input_create_i64`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_input_create_bool(
    value: u8,
    out_input: *mut *mut PpMetadataInput,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: The caller upholds the output-pointer contract.
    unsafe {
        create_input(out_input, out_error, || match value {
            0 => Ok(MetadataValue::boolean(false)),
            1 => Ok(MetadataValue::boolean(true)),
            _ => Err(invalid_argument("Boolean metadata must be zero or one")),
        })
    }
}

/// Creates an owned UTC timestamp input from Unix microseconds.
///
/// # Safety
///
/// Pointer rules match [`pp_metadata_input_create_i64`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_input_create_timestamp(
    unix_micros: i64,
    out_input: *mut *mut PpMetadataInput,
    out_error: *mut *mut PpError,
) -> u32 {
    // SAFETY: The caller upholds the output-pointer contract.
    unsafe {
        create_input(out_input, out_error, || {
            Ok(MetadataValue::timestamp(Timestamp::from_unix_micros(
                unix_micros,
            )))
        })
    }
}

/// Releases an owned metadata input. Null is a no-op.
///
/// # Safety
///
/// A non-null pointer must have been returned by a metadata-input constructor
/// and not previously released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pp_metadata_input_release(input: *mut PpMetadataInput) {
    if input.is_null() {
        return;
    }
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: Ownership of this allocation is transferred exactly once.
        drop(unsafe { Box::from_raw(input) });
    }));
}

#[cfg(test)]
mod tests {
    use std::{ffi::CString, ptr};

    use super::*;
    use crate::{PP_ERROR_INVALID_ARGUMENT, PP_OK, pp_error_release};

    #[test]
    fn decimal_constructor_owns_an_exact_domain_value() {
        let coefficient = CString::new("-12345").expect("valid C string");
        let mut input = ptr::null_mut();
        let mut error = ptr::null_mut();

        // SAFETY: Inputs remain live and outputs are writable for the call.
        let status = unsafe {
            pp_metadata_input_create_decimal(
                coefficient.as_ptr(),
                3,
                &raw mut input,
                &raw mut error,
            )
        };

        assert_eq!(status, PP_OK);
        assert!(error.is_null());
        // SAFETY: The successful call returned a live input handle.
        let decimal = unsafe { input.as_ref() }
            .expect("input")
            .value
            .as_decimal()
            .expect("decimal value");
        assert_eq!(decimal.coefficient(), -12_345);
        assert_eq!(decimal.scale(), 3);
        // SAFETY: The input is released exactly once.
        unsafe { pp_metadata_input_release(input) };
    }

    #[test]
    fn bool_constructor_rejects_non_boolean_bytes() {
        let mut input = ptr::null_mut();
        let mut error = ptr::null_mut();

        // SAFETY: Outputs are writable for the call.
        let status = unsafe { pp_metadata_input_create_bool(2, &raw mut input, &raw mut error) };

        assert_eq!(status, PP_ERROR_INVALID_ARGUMENT);
        assert!(input.is_null());
        assert!(!error.is_null());
        // SAFETY: The failed call returned one owned error.
        unsafe { pp_error_release(error) };
    }
}
