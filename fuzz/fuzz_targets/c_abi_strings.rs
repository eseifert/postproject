#![no_main]

use std::ptr;

use libfuzzer_sys::fuzz_target;
use postproject::{pp_error_release, pp_project_open, pp_project_release};

fuzz_target!(|data: &[u8]| {
    let mut terminated = data.to_vec();
    terminated.push(0);
    let mut project = ptr::null_mut();
    let mut error = ptr::null_mut();
    // SAFETY: `terminated` is NUL-terminated and remains live for the call;
    // output pointers are writable and returned handles are released once.
    unsafe {
        pp_project_open(terminated.as_ptr().cast(), &raw mut project, &raw mut error);
        pp_project_release(project);
        pp_error_release(error);
    }
});
