# Provenance integration

Record a completed operation, such as a transcode or render, as an *activity*
that consumed input representations and produced output representations. Each
activity has a stable ID and an open-world kind; each input and output edge may
carry an open-world role.

An activity can optionally carry:

- start and finish timestamps;
- a bounded tool name, version, and absolute URI;
- a bounded agent name and/or external identifier;
- external identifiers for the activity itself, such as render-job IDs; and
- typed [metadata](metadata-vocabularies.md) parameters that target the
  activity.

## Record an activity

The example records a render that consumed the original camera file and
produced an image sequence, then queries the provenance graph from both ends:

```{code-variants} provenance
```

The activity, its edges, and other mutations in the same transaction commit or
roll back together. Every referenced representation must already exist in the
transaction's view. A duplicate activity is rejected as already existing, an
absent representation as not found, and an edge that would create a generation
cycle as a conflict.

## Queries

The graph can be read in four directions:

- activities *producing* a representation;
- activities *consuming* a representation;
- transitive *ancestors*, following inputs; and
- transitive *descendants*, following outputs.

All activities can also be listed in stable identity order. Edges inside an
activity are canonicalized by representation ID and role, and traversal returns
unique representation IDs in stable order. A representation without provenance
has empty results; an unknown representation is an error.

## Mapping guidance

An activity maps strongly at a conceptual level to a W3C PROV Activity, while
representations often map to PROV Entities. This is not a normative PROV
implementation. MovieLabs OMC task and relationship concepts may be carried by
adapters, but PostProject does not infer revision, variant, or alternative
semantics from processing lineage.

Do not parse the private SQLite tables or encode tool parameters as an ad hoc
JSON column; use the public domain contracts and metadata model.
