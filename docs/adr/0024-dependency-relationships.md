# ADR 0024: Dependency relationships

- Status: Accepted
- Date: 2026-09-24

## Context

Membership says which resources constitute a representation. Provenance says
which representations a completed activity used and generated. Neither models a
live reference whose target is consulted whenever the source is used: a USD
layer referencing another layer, a Nuke script reading a plate, or an OCIO
configuration naming a LUT.

Treating such a reference as provenance would mark the source stale even though
it picks up the target's current content when opened. Treating it as membership
would erase the target's independent identity. Opaque metadata can preserve the
text but cannot support traversal or artifact evaluation.

## Decision

A dependency is a typed directed edge from one representation to either an
asset or a pinned representation. It records:

- the optional resource within the source that authored the reference;
- an open-world, namespaced kind;
- required or optional participation;
- the authored reference text exactly as supplied; and
- for an asset target, the representation to which the caller resolved it when
  known.

PostProject does not interpret file formats or resolve authored text. Hosts and
adapters provide both the edges and any resolved representation. An unresolved
asset target remains valid recorded knowledge but cannot contribute current
fingerprint evidence to an activity snapshot.

An edge has no durable ID. Its identity within one observation is its source
representation and position in the supplied complete set; its kind, target,
and authored text remain ordinary preserved values. This permits repeated
authored references without inventing identities that have no meaning to the
source format.

Dependencies are content observations, not individually editable rows. A
transaction replaces the complete ordered set for one source. An identical set
is a no-op; a changed set creates one semantic revision event. Recording a new
source-representation fingerprint marks an existing set as needing
re-extraction. Recording a replacement set, including an empty set, clears that
marker. Cycles are permitted.

Activity input capture follows required dependencies recursively. It stores the
resolved representation fingerprint evidence and the complete typed edge path
for each captured target. Optional edges are queryable but excluded. Capture is
bounded to depth 64 and 1,000 dependency representations; reaching either
bound is recorded, not silently truncated. A dirty set, unresolved asset
target, or truncated closure makes evaluation indeterminate. A changed target
fingerprint or changed required path makes the generated artifact stale and
produces a reason containing that path. The dependency source itself is never
classified as stale.

Activities created before the dependency migration retain an absent dependency
snapshot. New activity inputs record a present snapshot even when the observed
closure is empty, so migration absence is distinguishable from an empty result.

Forward and reverse reads are domain operations. Their paginated and transitive
forms adopt the common query cursor designed in the query phase; this decision
does not introduce a generic relationship query language.

## Alternatives considered

Using activity edges was rejected because a live reference is not a completed
transformation and can be withdrawn. Using compound membership was rejected
because the target remains an independently shared object. Storing only a
metadata reference was rejected because it cannot be traversed or included in
staleness evidence. Assigning every edge a UUID was rejected because complete
set replacement supplies no durable external edge identity and positions
already preserve duplicate occurrences.

## Standards impact

This decision was checked on 2026-09-24 against these authoritative sources:

- OpenUSD 26.11 documents references, payloads, and sublayers as composition
  arcs whose external changes flow into downstream composition, and its `Ar`
  API resolves authored asset paths through a caller-selected resolver context.
  Dependency edges are a conceptual record of those arcs, not a lossless USD
  composition model or asset resolver.
- OpenAssetIO's `getWithRelationship` contract describes relationships using
  trait data and returns paged related entity references. Its trait guidance
  explicitly uses dependency relationships as a runtime-dependency example.
  PostProject can translate through an adapter later but does not import
  OpenAssetIO traits into core.
- DCMI defines `dcterms:requires` for a resource needed for function, delivery,
  or coherence, and `dcterms:references` for a resource referenced or otherwise
  pointed to. Required edges map conceptually to the former; dependency kinds
  may map to either, but the broader `references` term does not imply runtime
  necessity.
- MovieLabs OMC Relationships 2.5 defines directional domain/range
  relationships with inverses, including `needs`, `uses`, and task input/output
  families. The direction and open-world kind are compatible conceptually, but
  PostProject imports neither OMC classes nor its relationship vocabulary.
- W3C PROV-DM's complete relation set covers generation, use, derivation,
  influence, alternate/specialization, and collection membership. It has no
  counterpart for a live reference whose target flows into future use.
  Dependencies therefore remain separate from PROV activity provenance and
  collection membership.

## Consequences

Hosts can preserve exact authored references while querying portable object
identity in either direction. Artifact evaluation can name a changed indirect
input without declaring the referencing source stale. Storage and public
surfaces must carry explicit observation status and path evidence, increasing
schema and result complexity. Format-specific extraction, whole-closure
availability, and OpenAssetIO relationship-query translation remain deferred.
