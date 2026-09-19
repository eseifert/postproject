#![no_main]

use libfuzzer_sys::fuzz_target;
use postproject_core::{RepresentationFingerprint, ResourceFingerprint};

fuzz_target!(|data: &[u8]| {
    if data.len() < 3 {
        return;
    }
    let algorithm_length = usize::from(data[0]).min(data.len() - 3);
    let algorithm_end = 1 + algorithm_length;
    let algorithm = String::from_utf8_lossy(&data[1..algorithm_end]);
    let version = u16::from_le_bytes([data[algorithm_end], data[algorithm_end + 1]]);
    let value = data[algorithm_end + 2..].to_vec();
    let _ = ResourceFingerprint::new(algorithm.as_ref(), version, value.clone());
    let _ = RepresentationFingerprint::new(algorithm.as_ref(), version, value);
});
