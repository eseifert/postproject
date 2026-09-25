# PostProject

PostProject is application-neutral infrastructure for media identity,
representations, locations, metadata, and provenance in post-production tools.
Use the guides for workflows and integration choices, then the generated API
reference for exact signatures.

```{toctree}
:maxdepth: 2
:caption: Guides

src/users/README
src/users/portable-production-workflow
src/users/image-sequences-and-spanned-media
src/users/metadata-and-provenance
src/integrators/README
src/integrators/installing-a-release
src/integrators/c-quickstart
src/integrators/cpp-quickstart
src/integrators/python
src/integrators/first-production
src/integrators/external-identifiers
src/integrators/media-resolution
src/integrators/compound-media
src/integrators/metadata-vocabularies
src/integrators/provenance
src/integrators/bounded-queries
src/integrators/jobs-and-workers
src/integrators/reference-executor
src/integrators/revision-feed
src/integrators/host-object-bindings
src/contributors/README
src/contributors/content-structure-invariants
src/contributors/metadata-model
src/contributors/provenance-model
src/contributors/standards-policy
```

```{toctree}
:maxdepth: 2
:caption: Concepts

src/concepts/identity
src/concepts/assets-representations-locators
src/concepts/external-identifiers
src/concepts/metadata
src/concepts/provenance
src/concepts/dependencies
src/concepts/artifact-knowledge
src/concepts/jobs
src/concepts/rational-time
src/concepts/revisions-and-events
src/concepts/standards-boundaries
src/reference/metadata-vocabularies
src/reference/standards-mapping-matrix
src/reference/terminology
```

```{toctree}
:maxdepth: 2
:caption: API reference

reference/c-api
reference/cpp-api
reference/python-api
reference/examples
rust-api
```

```{toctree}
:maxdepth: 1
:caption: Engineering notes

architecture
domain-model
media-resolution
persistence
testing
benchmarks
fuzzing
releasing
roadmap
release-0.1-report
release-0.2-report
release-0.3-integration-findings
release-0.3-report
standards-impact-0.3
abi-policy
adr/0001-project-naming
adr/0002-identity-model
adr/0003-compound-media-model
adr/0004-metadata-model
adr/0005-provenance-model
adr/0006-rational-time
adr/0007-revision-event-model
adr/0008-python-binding-over-c-abi
adr/0009-explicit-sql-no-orm
adr/0010-openassetio-boundary
adr/0011-host-object-binding
adr/0012-root-container-naming
adr/0013-namespace-policy
adr/0014-native-concurrency-contract
adr/0015-availability-and-collection-fingerprints
adr/0016-documentation-site-generator
adr/0017-portable-root-identity
adr/0018-discovery-index-location
adr/0019-media-inspection-boundary
adr/0020-integration-preview-compatibility
adr/0021-fingerprint-observations-and-activity-snapshots
adr/0022-managed-artifact-state-and-staleness
adr/0023-job-model-and-execution-boundary
adr/0024-dependency-relationships
adr/0025-reference-local-executor
adr/0026-domain-query-cursors
```
