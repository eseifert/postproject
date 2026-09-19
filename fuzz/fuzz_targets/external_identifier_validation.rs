#![no_main]

use libfuzzer_sys::fuzz_target;
use postproject_core::{ExternalIdentifier, IdentifierScheme};

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }

    let payload = &data[2..];
    let scheme_end = usize::from(data[0]) % (payload.len() + 1);
    let remaining = payload.len() - scheme_end;
    let value_length = usize::from(data[1]) % (remaining + 1);
    let value_end = scheme_end + value_length;

    let Ok(scheme) = std::str::from_utf8(&payload[..scheme_end]) else {
        return;
    };
    let Ok(value) = std::str::from_utf8(&payload[scheme_end..value_end]) else {
        return;
    };
    let Ok(qualifier) = std::str::from_utf8(&payload[value_end..]) else {
        return;
    };

    if let Ok(scheme) = IdentifierScheme::new(scheme) {
        let qualifier = (!qualifier.is_empty()).then(|| qualifier.to_owned());
        let _ = ExternalIdentifier::new(scheme, value, qualifier);
    }
});
