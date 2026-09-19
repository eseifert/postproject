# Testing

The required local quality gate is:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --all-features --no-deps
cargo deny check
```

On Windows, test `postproject-ffi` in a separate Cargo invocation from the rest
of the workspace. The public CLI executable and native library intentionally
share the installed name `postproject`, and concurrent MSVC links otherwise race
to write the same PDB file. CI runs both invocations sequentially, so no test
coverage is omitted.

Tests use real temporary SQLite databases and filesystems, a checked-in schema-0
migration fixture, and the complete multi-asset relocation scenario. Dedicated
fuzz targets cover project opening, fingerprint input, C strings/errors, and ID
parsing outside standard CI. Tests must not require network access, user locale,
or wall-clock timing.

`cargo-deny` rejects wildcard dependencies, unknown sources, known advisories, and
licenses outside the repository's explicit permissive allowlist. Duplicate crate
versions are reported for review because platform support and MSRV constraints can
make them temporarily unavoidable.

The C ABI job builds the optimized shared library, compiles a standalone C11
consumer using only the public header, runs create/open/error operations, and
compares the exported dynamic symbols against `tests/abi/expected-symbols.txt`.

The C++ package job installs the native library, C and C++ headers, CMake package
files, and `pkg-config` metadata into a temporary prefix. It then configures a
separate C++17 CMake project against that prefix, builds with warnings denied,
and runs lifecycle, identity, transaction commit/rollback, move-ownership, and
error-propagation checks. The consumer configuration and build never invokes
Cargo.

The CLI integration test creates a project, imports media, removes the known
file, discovers two byte-identical candidates under a configured root, verifies
an explicit ambiguous result, confirms one candidate, and reopens the project to
observe the persisted location. Commands exchange JSON in this test so the
machine-readable contract is exercised alongside the domain workflow.

The storage E2E test moves a three-file media directory, verifies every known
location is offline, resolves two unique files, refuses to choose between two
byte-identical candidates for the third, explicitly confirms all choices in one
transaction, reopens the project, and resolves from persisted locations with no
roots supplied.
