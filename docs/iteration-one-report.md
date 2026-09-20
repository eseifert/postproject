# Iteration one acceptance report

Date: 2026-09-19

## Delivered capability

The first release candidate proves durable media identity and relinking across
applications. It creates and reopens SQLite production files, imports original
media inside explicit transactions, persists stable UUID identities, file facts,
versioned BLAKE3 fingerprints, representations, locations, and ordered media
roots, then deterministically resolves moved media with inspectable evidence.
Ambiguous matches are returned without selection; explicit confirmations become
durable only through transaction commit.

The implementation is split into backend-neutral domain contracts, a SQLite
backend, filesystem fingerprint/resolution services, a CLI demonstrator, an
explicit C ABI, and a header-only C++17 wrapper. Native C and C++ consumers can
import, resolve, inspect candidate/evidence data, and confirm locations using an
installed CMake package without Cargo.

## Verification summary

- 44 Rust unit, property, integration, migration, CLI, FFI, and end-to-end tests
  pass locally with all features enabled.
- The automated three-file relocation scenario verifies unique resolution,
  deliberate ambiguity, explicit confirmation, atomic persistence, reopen, and
  known-location resolution without a root scan.
- Installed C11 and C++17 consumers build with warnings denied and exercise
  create/open, identity, import, rollback, relocation, structured evidence,
  confirmation, error propagation, and RAII ownership.
- The exported Linux symbol set is checked against an explicit allowlist.
- AddressSanitizer and UndefinedBehaviorSanitizer run the native workflows on
  Linux. Four documented fuzz targets cover production opening, fingerprint input,
  C strings/errors, and ID parsing.
- Formatting, Clippy with warnings denied, rustdoc with warnings denied, locked
  dependency builds, an explicit Rust 1.85 minimum-version compile,
  advisory/source/license policy, and shared/static package creation are CI
  gates.
- CI tests Rust on Linux, macOS, and Windows and builds conventional native
  packages and installed consumers on all three platforms. The implementation
  acceptance run is [GitHub Actions run 35428079237](https://github.com/eseifert/libpostproject/actions/runs/35428079237).

## Compatibility versions

- Package: `0.1.0-alpha.1`
- C ABI: version 1, reported by `pp_abi_version()`
- SQLite schema: version 1, with a checked-in schema-0 migration fixture
- Minimum supported Rust version: 1.85
- C standard: C11
- C++ standard: C++17
- License expression: `MIT OR Apache-2.0`

## Benchmark summary

Criterion covers all four required scale-sensitive operations. A 2026-09-19
quick run on an AMD Ryzen 7 4800H, Linux 7.2.5, Rust 1.98.1, and Btrfs measured
approximately 70.5 ms to import/fingerprint 1,000 small files, 312 µs to open a
10,000-asset production, 10.3 ms to resolve among 3,000 candidates, and 252 µs to
commit one prepared import. These quick-mode values are informational, not
performance gates; full intervals and methodology are in `docs/benchmarks.md`.

## Known limitations

- Resolution is synchronous, scans enabled local filesystem roots afresh, and is
  bounded to 64 levels and 100,000 entries by default. There is no index or cache.
- Large files use sampled fingerprints. They are strong relocation evidence but
  are not collision-proof or a substitute for an on-demand full verification.
- SQLite is the only backend. The new domain-shaped storage contracts permit
  another backend without making SQL part of the core API.
- Native handles require externally serialized access. The ABI remains a
  pre-release 0.x contract and may evolve with explicit changelog and symbol
  review.
- CI artifacts are unsigned build outputs. A tagged release still requires the
  signing, checksum, and publication steps in the release checklist.
- Timelines, collaboration, networking, server mode, decoding, proxy generation,
  metadata indexing, and editor-specific adapters remain intentionally out of
  scope.

## Recommended next iteration

Keep the current identity, transaction, and C ABI boundaries while adding typed
metadata namespaces, derived-media lineage, artifact/job records, a production
revision journal and event feed, optional filesystem indexing, and one additional
binding such as Python or GObject. PostgreSQL/server experiments should follow
the backend-neutral contracts without expanding the local SQLite schema into a
transport API.
