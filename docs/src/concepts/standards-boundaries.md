# Standards boundaries

> PostProject provides media identity, representations, locations, metadata
> persistence, provenance, and project-local revision semantics. It
> interoperates with industry vocabularies and exchange standards rather than
> attempting to replace them.

PostProject is a carrier, query, and persistence layer. It does not claim
normative compliance with SMPTE UMID, IPTC Video Metadata Hub, EBUCore,
MovieLabs OMC, W3C PROV, FIMS, OpenTimelineIO, AAF, IMF, or C2PA merely because
its model can preserve or map concepts from them.

In particular:

- external identifiers remain separate from internal object IDs;
- vocabulary and property identities remain explicit and extensible;
- production provenance records activities with inputs and outputs;
- rational time is a reusable value, not a timeline model;
- C2PA trust and signing remain a separate cryptographic layer;
- no identifier causes an automatic network lookup.

The [mapping matrix](../reference/standards-mapping-matrix.md) distinguishes
lossless storage, conceptual mappings, and deliberately deferred adapters.
