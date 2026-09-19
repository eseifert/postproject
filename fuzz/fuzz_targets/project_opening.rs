#![no_main]

use std::fs;

use libfuzzer_sys::fuzz_target;
use postproject_storage_sqlite::SqliteProject;

fuzz_target!(|data: &[u8]| {
    let Ok(directory) = tempfile::tempdir() else {
        return;
    };
    let path = directory.path().join("input.pproj");
    if fs::write(&path, data).is_err() {
        return;
    }
    let _ = SqliteProject::open(path);
});
