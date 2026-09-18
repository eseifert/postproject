# Testing

The required local quality gate is:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --all-features --no-deps
cargo deny check
```

Later phases add real temporary SQLite/filesystem integration tests, installed C
and C++ consumer tests, migration fixtures, fuzz targets, and relocation E2E
coverage. Tests must not require network access, user locale, or wall-clock timing.

`cargo-deny` rejects wildcard dependencies, unknown sources, known advisories, and
licenses outside the repository's explicit permissive allowlist. Duplicate crate
versions are reported for review because platform support and MSRV constraints can
make them temporarily unavoidable.

The C ABI job builds the optimized shared library, compiles a standalone C11
consumer using only the public header, runs create/open/error operations, and
compares the exported dynamic symbols against `tests/abi/expected-symbols.txt`.
