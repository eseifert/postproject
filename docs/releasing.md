# Release checklist

Iteration one is pre-release software. Release candidates use the workspace
version from `Cargo.toml`, schema version from
`postproject-storage-sqlite::CURRENT_SCHEMA_VERSION`, and ABI version from
`postproject-ffi::ABI_VERSION`.

Before tagging a release:

1. Update `CHANGELOG.md` and confirm every public ABI change is recorded.
2. Verify all workspace and fuzz dependency versions are locked and both
   `cargo deny` policies pass.
3. Run formatting, Clippy, tests, rustdoc, the C/C++ installed consumers, and the
   fuzz-target compile audit exactly as CI does.
4. Run the Criterion suite and record commit, toolchain, OS, CPU, storage, and
   filesystem when publishing numbers.
5. Confirm `tests/abi/expected-symbols.txt`, ABI version, and schema version are
   intentional.
6. Configure the native package from locked release outputs, install it into an
   empty prefix, and build consumers using only that prefix.
7. Confirm the install contains shared and static libraries, C/C++ headers,
   CMake and `pkg-config` metadata, README, changelog, and both licenses.
8. Create a signed tag matching the package version and publish the CI-produced
   native archive with checksums. Never rebuild an artifact after tagging.

Linux, macOS, and Windows package artifacts are produced from
`cargo build --locked` and the same CMake install rules on every push. Windows
packaging includes the DLL and its matching import library by passing them as
`POSTPROJECT_RUNTIME_LIBRARY` and `POSTPROJECT_LIBRARY`; macOS packaging must
preserve the dylib install name expected by the CMake target.
