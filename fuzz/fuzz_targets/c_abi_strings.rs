#![no_main]

use std::ptr;

use libfuzzer_sys::fuzz_target;
use postproject::{pp_error_release, pp_production_open, pp_production_release};

fuzz_target!(|data: &[u8]| {
    let mut terminated = data.to_vec();
    terminated.push(0);
    let mut production = ptr::null_mut();
    let mut error = ptr::null_mut();
    // SAFETY: `terminated` is NUL-terminated and remains live for the call;
    // output pointers are writable and returned handles are released once.
    unsafe {
        pp_production_open(
            terminated.as_ptr().cast(),
            &raw mut production,
            &raw mut error,
        );
        pp_production_release(production);
        pp_error_release(error);
    }
});
