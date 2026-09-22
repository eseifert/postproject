# Release checklist

PostProject is pre-release software. Release candidates use the workspace
version from `Cargo.toml`, the corresponding PEP 440 version from
`python/pyproject.toml`, schema version from
`postproject-storage-sqlite::CURRENT_SCHEMA_VERSION`, and ABI version from
`postproject-ffi::ABI_VERSION`.

Before tagging a release:

1. Update `CHANGELOG.md` and confirm every public ABI change is recorded.
2. Run `python tools/check_versions.py`, verify all workspace and fuzz
   dependency versions are locked, and confirm both `cargo deny` policies pass.
3. Run formatting, Clippy, tests, rustdoc, the C/C++ installed consumers, and the
   fuzz-target compile audit exactly as CI does.
4. Run the Criterion suite and record commit, toolchain, OS, CPU, storage, and
   filesystem when publishing numbers.
5. Confirm `tests/abi/expected-symbols.txt`, ABI version, and schema version are
   intentional.
6. Configure the native package from locked release outputs, install it into an
   empty prefix, and run the installed C, C++, and Python quickstarts using only
   that prefix and the built wheel.
7. Confirm the install contains shared and static libraries, C/C++ headers,
   CMake and `pkg-config` metadata, the quickstarts and fixture, stewardship
   policy, README, changelog, and both licenses.
8. Create a signed `v<package-version>` tag. The release workflow verifies the
   tag against Rust, CMake, and Python package versions before publication.
9. Confirm the workflow publishes the conventional source tarball, Linux,
   macOS, and Windows native archives, and Python wheel together with a SHA-256
   checksum for each artifact. Never rebuild an artifact after tagging.

Linux, macOS, and Windows package artifacts are produced from
`cargo build --locked` and the same CMake install rules exercised on every
push. Windows packaging includes the DLL and its matching import library by
passing them as `POSTPROJECT_RUNTIME_LIBRARY` and `POSTPROJECT_LIBRARY`; macOS
packaging must preserve the dylib install name expected by the CMake target.
The wheel remains platform-neutral and requires one of those native packages;
it never performs an implicit dynamic-library search.
