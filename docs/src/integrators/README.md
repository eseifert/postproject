# Integrator guide

PostProject exposes one set of operations through several surfaces:

- the **C ABI**, with opaque handles and caller-released result sets;
- a header-only **C++17** RAII wrapper over that ABI;
- a **Python** binding over the same installed ABI;
- the **Rust** domain, storage, and media crates the ABI is built from; and
- the demonstrator **CLI**, for scripts and integration experiments.

Installed C, C++, and Python consumers do not need Rust or Cargo. Until an
explicit stability milestone, pin an exact release or commit and expect
coordinated API, ABI, schema, CLI, and binding changes.

The guides describe each operation once and show it in every surface. Pick a
language with the tabs above any example or with the **Code** selector in the
sidebar; the choice applies site-wide. Where a surface has no equivalent
operation yet, its tab says so explicitly.

Start with [installing a release](installing-a-release.md), then
[create a production and import media](first-production.md).

## Integration rules

- treat PostProject IDs as internal object identities, not industry IDs;
- preserve external scheme and value text exactly;
- never choose an ambiguous relink candidate silently;
- perform mutations through explicit transactions;
- advance revision cursors only after processing a complete revision;
- release every owned C handle with its documented release function;
- do not infer “revision”, “variant”, or “alternative” relationships from a
  processing activity.

## Guides

- [Create a production and import media](first-production.md)
- [External identifiers](external-identifiers.md)
- [Media roots and resolution](media-resolution.md)
- [Compound media](compound-media.md)
- [Metadata vocabularies](metadata-vocabularies.md)
- [Provenance](provenance.md)
- [Revision feed](revision-feed.md)
- [Host-object bindings](host-object-bindings.md)

Media roots and locators can also be listed, disabled, re-enabled, removed, and
retired. Each of these mutations is transactional and appears in the revision
feed.
