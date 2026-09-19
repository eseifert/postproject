#![no_main]

use std::str::FromStr;

use libfuzzer_sys::fuzz_target;
use postproject_core::{
    AssetId, LocatorId, MediaRootId, ProjectId, RepresentationId, ResourceId, TransactionId,
};

fuzz_target!(|data: &[u8]| {
    let input = String::from_utf8_lossy(data);
    let _ = ProjectId::from_str(&input);
    let _ = AssetId::from_str(&input);
    let _ = RepresentationId::from_str(&input);
    let _ = ResourceId::from_str(&input);
    let _ = LocatorId::from_str(&input);
    let _ = MediaRootId::from_str(&input);
    let _ = TransactionId::from_str(&input);
});
