# ADR 0003: Standards-aware metadata

- Status: Accepted
- Date: 2026-09-19

## Decision

Metadata assertions identify a vocabulary, a vocabulary-local property, and a
typed value. Values support repetition, language-tagged text, exact numeric
forms, references, ordered lists, and structured fields. Unknown vocabularies
and properties remain persistable without a schema package.

Persistence will use a bounded deterministic typed encoding with explicit size
and nesting limits, not an unconstrained JSON scalar map. Optional vocabulary
definitions may add type and cardinality validation but cannot be required to
read unknown data.

Repeated property values are separate assertions. Lists and structured fields
are ordered values in their own right, so persistence and public APIs must not
collapse either concept into the other. Core constructors enforce limits on
text, binary data, collection length, decimal scale, aggregate payload size and
recursive depth before a value reaches a backend.

## Consequences

PostProject remains a metadata carrier rather than a competing broad ontology.
Recursive C ABI access will use owned/borrowed handles and type inspection.
