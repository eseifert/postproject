# Iteration 3 acceptance report

Iteration 3 turns PostProject's model into a portable, inspectable workflow and
tests it through applications outside the main repository. The delivered
release is package `0.3.0-alpha.2`, C ABI 15, and SQLite schema 6.

## Delivered scope

- Logical media-root names are production knowledge; per-call filesystem
  mappings remain machine-local. Unmapped and unavailable roots are reported
  without hiding results from usable roots.
- A disposable sidecar index accelerates bounded, read-only inventory scans.
  Inventory reports online, partial, missing, new, changed, duplicate, and
  ambiguous media without mutating the production.
- Filesystem adapters recognize numbered image sequences and recording spans,
  same-stem sidecars, and the checked-in AVCHD package fixture. Relocated
  sequences resolve as one representation.
- An optional bounded `ffprobe` subprocess records normalized technical
  metadata through the existing vocabulary model. Import remains successful
  when inspection is unavailable.
- Opt-in content verification detects replaced files and sequence damage.
  Relative paths and matching technical profiles contribute conservative
  candidate evidence without resolving ambiguity silently.
- Native release archives contain the CLI, libraries, headers, package
  metadata, licenses, and tested examples. The platform-neutral Python wheel
  consumes that same native ABI.

## Integration validation

Four validation repositories run against PostProject `main` and are also
called by PostProject CI against the revision under review:

- [OpenAssetIO Manager validation](https://github.com/eseifert/postproject-openassetio)
- [OTIO-through-OpenAssetIO validation](https://github.com/eseifert/postproject-otio-openassetio)
- [Python host validation](https://github.com/eseifert/postproject-python-host)
- [C++ NLE validation](https://github.com/eseifert/postproject-cpp-nle)

Their findings informed the maintained
[PostProject OpenAssetIO Manager](https://github.com/eseifert/postproject-openassetio-manager)
and [OTIO demonstration](https://github.com/eseifert/postproject-otio-demo).
The Manager resolves real productions and maps image-sequence structure without
splitting a sequence into per-frame entities. The demo uses the upstream OTIO
media linker and retains rational clip time.

## Verification summary

The release candidate passes:

- workspace formatting, Clippy with warnings denied, all-feature tests, and
  warning-free rustdoc on Rust 1.85-compatible code;
- both Cargo dependency policies and the separately locked fuzz package;
- generated ABI declarations and layouts, all 123 exported ABI symbols, the C
  smoke test, and installed C and C++ consumers;
- 19 Python binding integration tests against the release shared library;
- Doxygen symbol coverage, documentation helper tests, and the complete Sphinx
  build with warnings treated as errors;
- the four disposable integrations, two maintained Manager mapping tests, the
  OTIO end-to-end demo, built Python wheels, and the installed CLI.

## Published surfaces

The tag-triggered release publishes checksummed source, native Linux, macOS,
and Windows archives, plus the Python wheel. The Manager and OTIO repositories
publish their own wheels and source archives. The Pages workflows publish the
development documentation, immutable documentation built from the release tag,
and the separate landing page.

## Known constraints

- The upstream OTIO/OpenAssetIO linker handles `ExternalReference`, not
  `ImageSequenceReference`, and currently emits a deprecation warning for its
  unversioned location trait import.
- The technical-profile score uses one persisted inspection and is supporting
  evidence, not content identity. Full verification remains opt-in.
- The inventory cache is deliberately local and rebuildable; persistent shared
  indexing and background jobs remain later work.
- Releases are checksummed but not code-signed. PostProject remains pre-1.0,
  with only the subset named in ADR 0020 compatible within the 0.3 series.
