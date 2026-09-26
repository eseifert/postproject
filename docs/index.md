# PostProject documentation

PostProject gives post-production applications a **shared memory for media**. Each application keeps its own editing, compositing, timeline, and project data; PostProject stores the production knowledge that should remain meaningful between tools.

If you have just arrived from [postproject.org](https://postproject.org), this site is the next step: it explains how the model works, how to use it in a production, and how to integrate it into software.

## Choose your path

### I want to understand PostProject

Start with {doc}`src/getting-started/README`. It explains the problem PostProject solves and introduces the core model without requiring API knowledge.

Then read {doc}`src/getting-started/core-model` for the four most important objects: assets, representations, resources, and locators.

### I want to use a PostProject-enabled application or the CLI

Read {doc}`src/users/README`, then follow the {doc}`src/users/portable-production-workflow`.

The user guides focus on practical questions such as moving a production to another machine, handling image sequences, checking whether media is still available, and understanding metadata and provenance.

### I want to add PostProject to an application

Go to {doc}`src/integrators/README`. It gives a recommended integration order, explains which API surface to choose, and points to the tested C, C++, and Python examples.

### I want to understand the data model precisely

Use {doc}`src/concepts/README` as the map. Concept pages define semantics; generated API pages define exact signatures.

### I want to contribute to PostProject

Start with {doc}`src/contributors/README` and {doc}`src/project/README`. Architectural decisions, release reports, benchmarks, fuzzing notes, and persistence details are engineering reference material rather than required introductory reading.

## A useful mental model

```text
A production (.pproj)
│
├── Asset                         the logical media item
│   └── Representation            one usable form of that asset
│       └── Resource(s)           the stored pieces that make it up
│           └── Locator(s)        places those pieces can be reached
│
├── Metadata + external IDs       what is known about production objects
├── Activities + dependencies     how media and artifacts relate
└── Revisions                     what changed in the production model
```

A path is deliberately **not** the identity of an asset. Moving a file should change where it is found, not what it is.

## Documentation layers

The documentation is organized so you do not need to read implementation detail before understanding the product:

1. **Start here** — plain-language model and terminology.
2. **Using PostProject** — production workflows and operational behavior.
3. **Integrating PostProject** — application-facing rules and tested examples.
4. **Concepts** — precise semantic definitions shared across languages.
5. **API reference** — exact native and language-binding signatures.
6. **Project & engineering** — contribution rules, architecture, persistence, tests, release history, and ADRs.

```{toctree}
:maxdepth: 2
:caption: Start here

src/getting-started/README
src/getting-started/core-model
src/users/README
src/integrators/README
src/concepts/README
src/project/README
```

```{toctree}
:maxdepth: 1
:caption: API reference

reference/c-api
reference/cpp-api
reference/python-api
reference/examples
rust-api
```

```{toctree}
:hidden:

src/users/portable-production-workflow
src/users/image-sequences-and-spanned-media
src/users/metadata-and-provenance
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
