# Concepts

Concept pages define the **meaning of PostProject data independent of programming language**. Read them when you need to decide what should be stored or how a host should interpret a result. Use the API reference only after the semantic question is clear.

## Core media model

Start here if you are new to the domain model:

- {doc}`identity` — internal object identity versus identifiers owned by outside systems.
- {doc}`assets-representations-locators` — assets, representations, resources, content structures, locators, and availability.
- {doc}`external-identifiers` — preserving identifiers from external schemes.
- {doc}`rational-time` — exact time values and ranges without turning PostProject into a timeline model.

A useful shorthand is:

```text
Asset → Representation → Resource → Locator
```

Each arrow exists because the concepts have different lifetimes and responsibilities.

## Descriptive knowledge and production history

- {doc}`metadata` — typed metadata assertions and repeatable structure.
- {doc}`provenance` — activities, inputs, outputs, and recorded history.
- {doc}`revisions-and-events` — semantic change records consumed by other applications.

These pages distinguish “what is known” from “what happened” and from “what changed in the database.”

## Derived media and work

- {doc}`dependencies` — explicit dependency relationships and observations.
- {doc}`artifact-knowledge` — current/stale/indeterminate/diverged knowledge and reproducibility.
- {doc}`jobs` — durable knowledge about requested or ongoing work, separate from scheduler implementation.

A recurring design boundary is that PostProject can record and evaluate production knowledge without owning the actual media-processing engine.

## Standards and vocabulary boundaries

- {doc}`standards-boundaries` — what PostProject maps to, what remains outside core, and where adapters belong.
- {doc}`../reference/metadata-vocabularies` — known vocabulary registry.
- {doc}`../reference/standards-mapping-matrix` — mappings and non-claims across external standards.
- {doc}`../reference/terminology` — precise terminology used throughout the project.

PostProject is standards-aware, but it is not intended to invent a replacement ontology for the media industry.

## When a concept page feels too technical

Return to {doc}`../getting-started/core-model` for the plain-language version. The concept pages intentionally contain the edge cases and invariants an integrator needs once implementation decisions begin.
