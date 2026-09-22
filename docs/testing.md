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

A separate locked all-targets compile uses Rust 1.85.0, enforcing the stated
minimum supported Rust version independently of the stable-toolchain test matrix.

Tests use real temporary SQLite databases and filesystems, a checked-in schema-0
migration fixture, and the complete multi-asset relocation scenario. Dedicated
fuzz targets cover production opening, fingerprint input, C strings/errors, and ID
parsing outside standard CI. Tests must not require network access, user locale,
or wall-clock timing.

`cargo-deny` rejects wildcard dependencies, unknown sources, known advisories, and
licenses outside the repository's explicit permissive allowlist. Duplicate crate
versions are reported for review because platform support and MSRV constraints can
make them temporarily unavoidable.

The C ABI job builds the optimized shared library, compiles a standalone C11
consumer using only the public header, runs create/open/error operations, and
compares the exported dynamic symbols against `tests/abi/expected-symbols.txt`.

The native package matrix installs the platform library, optional static archive,
C and C++ headers, CMake package files, and `pkg-config` metadata into temporary
prefixes on Linux, macOS, and Windows. It then configures a separate C11/C++17
CMake production against each prefix, builds with warnings denied, and runs
lifecycle, identity, transaction commit/rollback, relocation,
structured-evidence, confirmation, move-ownership, and error-propagation checks.
The consumer configuration and build never invokes Cargo. Each platform package
is uploaded as a CI artifact. The same job builds and runs the C and C++
quickstarts from their installed locations and runs the installed Python
quickstart against the packaged media fixture.

The native sanitizer job rebuilds both consumers with AddressSanitizer and
UndefinedBehaviorSanitizer, enables leak detection, and runs their real lifecycle
and transaction workflows against the release shared library.

The CLI integration test creates a production, imports media, removes the known
file, discovers two byte-identical candidates under a configured root, verifies
an explicit ambiguous representation and resource result, confirms one
candidate, and reopens the production to observe the persisted locator. Commands
exchange JSON in this test so the machine-readable contract is exercised
alongside the domain workflow.

The storage E2E test moves a three-file media directory, verifies every known
locator is offline, resolves two unique resources, refuses to choose between two
byte-identical candidates for the third, explicitly confirms all choices in one
transaction, reopens the production, and resolves from persisted locators with no
roots supplied.

The iteration-two E2E test creates four representations on one asset, attaches
typed metadata and an external identifier, records proxy-generation
provenance, and verifies one atomic semantic revision. It then moves the
single-file and compound media, discovers and confirms replacement locators,
retires the old locators and root, removes one sequence frame, and verifies the
resulting online/partial states and second revision after reopen.
