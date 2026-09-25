# Standards boundaries

> PostProject provides persistent production knowledge: media identity,
> compound representations and resources, locators, metadata, provenance, and
> production-local revision semantics. It maps that knowledge to established
> interoperability contracts rather than attempting to replace them.

PostProject is a carrier, query, and persistence layer. It does not claim
normative compliance with SMPTE UMID, IPTC Video Metadata Hub, EBUCore,
MovieLabs OMC, W3C PROV, FIMS, OpenTimelineIO, AAF, IMF, or C2PA merely because
its model can preserve or map concepts from them.

In particular:

- external identifiers remain separate from internal object IDs;
- vocabulary and property identities remain explicit and extensible;
- production provenance records activities with inputs and outputs;
- rational time is a reusable value, not a timeline model;
- OpenAssetIO remains an adapter boundary rather than a core dependency;
- C2PA trust and signing remain a separate cryptographic layer;
- no identifier causes an automatic network lookup.

The [mapping matrix](../reference/standards-mapping-matrix.md) distinguishes
lossless storage, conceptual mappings, and deliberately deferred adapters.

## Why there is no code variant

This page defines interpretation and non-claims rather than a callable concept.
There is therefore no operation to demonstrate across C, C++, Python, Rust, and
the CLI. Executable examples live with the modeled concepts above; the mapping
matrix records how those values may enter or leave standards-specific adapters.
