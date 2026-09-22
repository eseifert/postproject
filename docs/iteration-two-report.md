# Iteration two acceptance report

Date: 2026-09-22

## Delivered capability

The second iteration turns PostProject from a file-relinking foundation into a
standards-aware production knowledge library. Assets own independently identified
representations; representations describe single files, compact image sequences,
ordered parts, and packages made from typed resources and locators. Original,
proxy, optimized, and derived representations can be created through Rust, the
CLI, C, C++, and Python.

External identifiers remain distinct from internal UUIDs and support exact
lookup without closing the set of schemes. Metadata preserves repeated,
language-tagged, structured, binary, rational, temporal, URI, and reference
values. Activity graphs record fan-in and fan-out provenance with tool and agent
identity, reject generation cycles, and derive ancestry and descendant queries.

Each successful non-empty mutation transaction creates one durable, ordered
revision with semantic events and optional origin/message context. Failed,
rolled-back, and empty transactions do not advance the feed. Media roots can be
enumerated, disabled, re-enabled, and removed; superseded locators can be
retired. These lifecycle changes are transactional and journaled.

The public C ABI remains the language boundary. Header-only C++17 and Python
3.11 wrappers expose immutable typed values, deterministic ownership, compound
media, metadata, provenance, revisions, asset/root enumeration, and media
location lifecycle operations. Portable host-object references use the
versioned `https://postproject.org/ref/v1/` form.

## Standards and architecture

The core preserves concepts needed for later adapters without importing their
types. Documentation records conceptual mappings and non-claims for OpenAssetIO,
IPTC Video Metadata Hub, EBUCore/EBUCorePlus, W3C PROV, MovieLabs OMC, OTIO,
AAF, IMF, MXF, C2PA, and UMID-related identity. OpenAssetIO remains an Iteration
3 integration boundary rather than a competing core abstraction.

SQLite remains behind domain-shaped read and transaction traits. The core has
no SQL, OpenAssetIO, Python, or C-ABI dependency. Explicit SQL migrations and
typed codecs preserve domain validation at the storage boundary.

## Verification summary

- 141 Rust unit, integration, migration, CLI, FFI, and end-to-end tests pass
  locally with all features enabled.
- 19 Python integration tests run against the release shared library and cover
  ownership, typed errors, identities, all representation shapes, typed
  metadata, provenance, revision feeds, resolution, asset enumeration, and
  media-root/locator lifecycle.
- Installed C11 and C++17 consumers compile with warnings denied and exercise
  the same shared library without Cargo. The ABI declarations, exported-symbol
  allowlist, and C/Python public-struct layouts are generated or cross-checked
  from the public header.
- The installed package ships runnable C, C++, and Python quickstarts plus an
  opaque media fixture. Package CI builds and runs those installed copies on
  Linux, macOS, and Windows.
- CI runs Rust tests and native packages on Linux, macOS, and Windows. It also
  gates Rust 1.85, Python 3.11, Ruff, ty, rustfmt, Clippy, rustdoc, mdBook,
  offline link checking, spelling, dependency/license policy, sanitizers, fuzz
  target builds, migration fixtures, and installed native consumers.
- The updated acceptance workflow and installed quickstarts pass in
  [GitHub Actions run 35759535180](https://github.com/eseifert/postproject/actions/runs/35759535180).
- The end-to-end fixture persists an original, sequence, ordered spans, a
  derived proxy, structured/repeated metadata, a UMID-shaped identifier, tool
  provenance, and one semantic revision batch. It then moves the source and
  compound-media directories, relinks them, removes one sequence frame, and
  verifies online/partial availability, ordered structure, ancestry, metadata,
  and both revision batches from cursor zero.

## Compatibility versions

- Package: `0.2.0-alpha.1`
- C ABI: version 14, reported by `pp_abi_version()`
- SQLite schema: version 5
- Python: 3.11 or newer
- Minimum supported Rust version: 1.85
- C standard: C11
- C++ standard: C++17
- License expression: `MIT OR Apache-2.0`

All public contracts remain pre-1.0. Consumers should pin an exact release or
commit and expect deliberate API, ABI, schema, CLI, and binding changes until a
stability milestone is declared.

## Benchmark observations

The Criterion suite remains an informational scale baseline. Its resolver
fixture now sends 3,000 same-size decoys through fingerprint verification, and
its 10,000-asset fixture enumerates representations, resources, and non-empty
identity evidence. A quick run on 2026-09-22 at commit `1e92194`, using Rust
1.98.1 on Linux 7.2.5, an AMD Ryzen 7 4800H, and Btrfs, measured 257.42–271.36
ms to fingerprint/import 1,000 small files, 1.0835–1.0980 s to load 10,000
assets with fingerprints, 84.330–86.524 ms to resolve beneath the 3,000-decoy
root, and 742.71–756.72 µs to commit one prepared import. Quick mode and the
non-isolated machine make these regression reference points, not performance
promises or release gates.

## Known limitations

- Discovery is synchronous and scans enabled local filesystem roots without a
  persistent index. Root identities are stored as absolute URIs; per-machine
  named-root or volume mappings are not implemented yet.
- Compound structures are explicit at ingest. Automatic sequence, camera-card,
  sidecar, and vendor-span recognition is deferred.
- Large-file sampled fingerprints are relocation evidence, not collision-proof
  full-content verification.
- SQLite is the only backend. There is no daemon, remote object-store backend,
  background job system, or multi-user locking/conflict protocol.
- The revision journal is a deterministic change feed, not undo/redo, event
  sourcing, authentication, or distributed merge history.
- Standards support is conceptual preservation and mapping documentation. Full
  import/export adapters and normative conformance suites are not included.
- Release archives are not code-signed. A maintainer must create the signed
  version tag; the tag workflow then publishes checksummed source/native
  archives and the tested Python wheel to GitHub Releases.

## Deliberate deferrals

Iteration 3 owns the documentation site at `postproject.org`, automatic media
recognition and inspection, an OpenAssetIO Manager and OTIO-through-OpenAssetIO
demonstration, portable per-machine root mappings, and persistent discovery
indexes. Later work owns publishing workflows, full standards adapters,
PostgreSQL/server mode, collaboration, timeline/editorial concepts, remote
storage, technical metadata extraction, semantic search, and distributed
revision merging.
