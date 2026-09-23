# Changelog

All notable changes to PostProject will be documented here. The project uses
[Semantic Versioning](https://semver.org/) once a stable API is released.

## Unreleased

### Changed

- Advanced the development version to `0.3.0-alpha.1` and introduced a named
  integration-preview subset that remains compatible within the 0.3.x series.
- Replaced absolute media-root identity with unique logical names in SQLite
  schema 6. Migrated roots retain their former URI as a lossless legacy fallback.
- Advanced the C ABI to version 15 for portable root summaries, per-call
  machine-root mappings, and unmapped/unavailable resolution evidence.
- Raised the Python binding's minimum supported version to Python 3.11.
- Renamed the durable root container from `Project` to `Production` across the
  domain, SQLite schema, CLI, C ABI version 8, C++ wrapper, and Python binding;
  `.pproj` remains the PostProject storage-format extension.
- Renamed the project and repository identity to PostProject and
  `eseifert/postproject`; native Unix library files retain the conventional
  `libpostproject` name.
- Advanced the development version to `0.2.0-alpha.1` for the standards-aware
  domain-model work. Pre-1.0 API, ABI, schema, CLI, and binding compatibility is
  not promised.
- Replaced the flat native media-resolution result with ABI version 5's nested
  representation availability, resource results, and availability issues.
- Added ABI version 6 activity creation, inspection, direct provenance queries,
  and ancestry/descendant traversal with C++17 value wrappers.
- Added ABI version 9 representation structure, ordered membership, compact
  image-sequence, concrete resource, locator, and typed fingerprint inspection.
- Added ABI version 10 host-binding formatting and parsing, with C++ and Python
  wrappers over the same public C implementation.
- Added ABI version 11 owned typed-metadata inputs, including recursive lists
  and structures, and removed the superseded text-only write function.
- Added ABI version 12 creation of single-file, image-sequence, ordered-parts,
  and package representations, with matching C++, Python, and CLI surfaces.
- Added ABI version 13 asset enumeration with immutable summaries in C, C++,
  and Python.
- Added ABI version 14 media-root enumeration, root enable/disable/removal,
  locator retirement, and their semantic revision events across C, C++, Python,
  and the CLI.
- Added a strict, versioned HTTPS host-object binding format under
  `postproject.org` for portable production-scoped references.
- Added an opt-in registry for UMID, ISAN, EIDR, and application identifier
  schemes with local syntax checks and no network behavior; PostProject-owned
  application identifiers use `https://postproject.org/id/application`.
- Added an opt-in metadata vocabulary registry with type, cardinality,
  description, validation, and cross-standard mapping hints; unknown terms
  remain independent of the registry.
- Extended external-identifier attachments to activities in SQLite schema
  version 4, including exact lookup and transactional persistence.
- Added deterministic, structure-aware representation fingerprint computation
  for single resources, image sequences, ordered parts, and packages.
- Added tagged JSON input for every recursively typed metadata value in the
  CLI, while retaining the concise text command.

### Added

- Machine-local mappings from portable root names to local directories, with
  explicit unmapped/unavailable evidence and continued scanning of usable roots.
- A tag-triggered GitHub release workflow publishing checksummed source,
  Linux, macOS, and Windows native archives plus a tested Python wheel.
- A stewardship policy covering governance, compatibility, security reports,
  releases, and the transition to broader maintainership.
- Installed, cross-platform-tested C, C++, and Python quickstarts with a small
  deterministic media fixture.
- An mdBook documentation foundation split across application-user,
  integrator, contributor, concept, and reference sections.
- Accepted architecture decisions for naming, compound media, identity,
  metadata, provenance, rational time, revision events, bindings, persistence,
  OpenAssetIO boundaries, namespaces, concurrency, and availability.
- Strong production, asset, representation, resource, locator, activity,
  revision, media-root, and transaction IDs plus typed cross-object references.
- A compound-media model for single resources, compact image sequences,
  ordered parts, and packages with required or optional resource roles.
- Canonical SQLite schema version 6 with transactional compound media,
  external identifiers, typed metadata, provenance, lifecycle operations, and
  a durable semantic revision journal.
- C ABI version 15 and C++17 wrappers for the complete public model, including
  compound-media creation and inspection, provenance, revisions, host bindings,
  asset/root enumeration, and locator/root lifecycle operations.
- A Python 3.11 binding over the public C ABI with generated signatures,
  compiler-verified layouts, structured errors, and deterministic cleanup.
- Activity-based provenance with extensible kinds and edge roles, bounded tool
  and agent identity, cycle prevention, deterministic reload, and graph queries.
- Structured, repeated, language-tagged metadata with deterministic typed
  persistence and bounded recursive values.
- Extensible external identifiers on assets, representations, resources, and
  activities, including exact scheme/value lookup and opaque round-trips.
- Exact rational-time comparison, lossless checked rescaling, half-open ranges,
  and deterministic text round-trips for common integer and fractional rates.
- A durable semantic revision feed with ordered events committed atomically
  alongside domain mutations.
- Deterministic resource and representation fingerprints, including sampled
  content evidence for compact image sequences.
- Structured five-state representation availability with resource/member
  diagnostics and explicit ambiguity.
- CLI commands for compound media, identifiers, typed metadata, provenance,
  revisions, resolution, and explicit confirmation with structured JSON output.
- Initial workspace and engineering-policy scaffolding.
- Dual MIT or Apache-2.0 licensing.
- Automated advisory, source, duplicate-dependency, and license policy checks.
- Installable CMake and `pkg-config` metadata with a standalone native consumer test.
- Criterion baselines for bulk import, large-production open, resolver scans, and commits.
- End-to-end relocation coverage for single-file, sequence, and ordered media,
  including partial availability and preserved provenance.
- `cargo-fuzz` targets for production files, fingerprints, identifiers, compound
  structures, metadata, revisions, rational time, C strings, and strong IDs.
- ASan/UBSan native-consumer CI and a standalone installed-package C example.
- Shared/static native artifacts, installed release documentation, and release checklist.
- Bounded SQLite value and row sizes when opening untrusted production files.
- Unambiguous cross-platform CMake metadata with explicit Windows DLL packaging.
- Backend-neutral, domain-shaped production read and transaction contracts.
- Iteration-one acceptance report with verification and benchmark summaries.
- Locked the compatible `yoke-derive` patch release to preserve Rust 1.85 support.
