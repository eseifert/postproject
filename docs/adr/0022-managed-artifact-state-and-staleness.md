# ADR 0022: Managed-artifact state and staleness

- Status: Accepted
- Date: 2026-09-24

## Context

An activity records that representations were consumed and produced, and ADR
0021 records the fingerprint evidence observed on those edges. Applications
also need to know whether a generated proxy, thumbnail, or render still agrees
with the production knowledge from which it was made.

That question is distinct from whether the artifact's files are reachable and
from whether regeneration work is pending or failed. A stored `stale` flag
would duplicate facts from the provenance graph and would become incorrect
when an upstream observation or activity changed.

W3C PROV describes entities, activities, use, generation, and specialization.
It does not define a normative artifact-staleness state or a job state. Those
states are application conclusions over PostProject's recorded knowledge.

## Decision

A managed artifact is a representation produced by a recorded activity. Its
knowledge state is computed on demand and is never stored on the representation:

- `current` means every comparable input and output fingerprint still matches
  its activity-edge snapshot and every managed upstream input is current;
- `stale` means an input changed or a managed upstream input is not current;
- `indeterminate` means the production lacks enough evidence to decide; and
- `diverged` means the artifact's own current fingerprint differs from its
  output snapshot.

Every non-current result carries structured reasons naming the activity edge,
representation, fingerprint domain, captured and current values, upstream
state, or traversal limit as applicable. Evaluation reads production knowledge
only and never resolves paths, reads media, records fingerprints, enqueues work,
or mutates the production.

Transitive evaluation uses caller-visible positive depth and representation
bounds. Reaching either bound produces an explicit truncated, indeterminate
result. Implementations cache results during one traversal so shared upstream
graphs are not repeatedly evaluated.

Availability, knowledge, and work remain independent public dimensions:

```text
knowledge      current · stale · indeterminate · diverged
availability   online · partial · offline · ambiguous · error
work           pending · failed · none
```

No combined state is introduced in the initial `0.4` contract. The Kdenlive
pilot and OpenAssetIO Manager must first show whether a combined presentation
state helps consumers or hides distinctions they need. This ADR will record
that evidence before a combined state is accepted or permanently rejected.

Reproducibility is a separate structured report. It names every missing
condition: an unambiguous producing activity, tool identity, activity parameter
metadata, and input representations. It does not claim that the external tool
or media is available and does not execute anything.

## Alternatives considered

A stored representation status was rejected because it is derived, can become
stale independently of the row, and would require mutation during read-only
observation. Treating an unavailable file as a stale artifact was rejected
because relocation and content validity are separate questions. Returning only
a boolean was rejected because callers could not explain or remediate the
result without reimplementing evaluation.

## Standards impact

The snapshot-to-specialized-entity mapping and its deliberate limits are
recorded in ADR 0021 and the standards mapping matrix. Artifact state and
reproducibility are PostProject-derived assessments, not W3C PROV terms, and no
new canonical scheme, vocabulary mapping, cardinality, or normalization rule is
introduced. This was checked against the same W3C PROV-DM and PROV-O provisions
recorded in ADR 0021 on 2026-09-24.

## Consequences

Callers can explain currentness without filesystem access and can present
availability beside it without losing information. Legacy activities whose
snapshots are absent remain indeterminate. Long graphs are predictably bounded.
The query and job phases can reuse the computed state without adding a mutable
status column, but a caller that needs all three dimensions must currently read
them separately.
