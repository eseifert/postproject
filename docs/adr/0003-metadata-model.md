# ADR 0003: Standards-aware metadata

- Status: Proposed
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

## Consequences

PostProject remains a metadata carrier rather than a competing broad ontology.
Recursive C ABI access will use owned/borrowed handles and type inspection.
