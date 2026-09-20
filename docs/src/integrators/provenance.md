# Provenance integration

The Rust domain API represents a completed operation with `Activity`,
`ActivityInput`, and `ActivityOutput`. Each activity has a stable `ActivityId`
and open-world `ActivityKind`; edges may carry an open-world `ActivityRole`.

Construct an activity from existing representation IDs, then optionally attach:

- start and finish timestamps;
- a bounded tool name, version, and absolute URI;
- a bounded agent name and/or external identifier;
- typed metadata parameters using `ObjectRef::Activity(activity.id())`.

Stage the activity with `ProductionStoreTransaction::create_activity`. Activity,
edges, metadata, and other mutations in that production transaction commit or roll
back together. Every referenced representation must already exist in the
transaction view. A duplicate activity returns `AlreadyExists`, an absent
representation returns `NotFound`, and a generation cycle returns `Conflict`.

## Queries

`ProductionRead` exposes domain-shaped reads:

- `activities()` returns every activity in stable identity order;
- `activities_producing(representation_id)` finds producers;
- `activities_consuming(representation_id)` finds consumers;
- `ancestors(representation_id)` follows inputs transitively;
- `descendants(representation_id)` follows outputs transitively.

Edges inside an activity are canonicalized by representation ID and role. Graph
traversal returns unique representation IDs in stable order. A representation
with no provenance has empty results; an unknown representation is an error.

## Mapping guidance

An activity maps strongly at a conceptual level to a W3C PROV Activity, while
representations often map to PROV Entities. This is not a normative PROV
implementation. MovieLabs OMC task and relationship concepts may be carried by
adapters, but PostProject does not infer revision, variant, or alternative
semantics from processing lineage.

The native and Python provenance surfaces are still under development. Do not
parse the private SQLite tables or encode tool parameters as an ad hoc JSON
column; use the public domain contracts and metadata model.
