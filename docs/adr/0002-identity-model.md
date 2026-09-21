# ADR 0002: Internal and external identity

- Status: Accepted
- Date: 2026-09-19

## Context

A PostProject UUID identifies a domain object. It cannot also stand for the
UMIDs, registry identifiers, content identifiers, camera IDs, and application
IDs that media may carry.

## Decision

Each object kind has a distinct strong ID. External identifiers are first-class,
multi-valued values containing an extensible scheme string, an opaque exact
value, and an optional qualifier. Schemes are not an ABI enum.

Core applies bounded generic validation. Scheme-specific validation is opt-in
and is added only after checking the authoritative specification. Unknown
schemes round-trip unchanged, and identifier storage never initiates a network
lookup. Persistence must not impose global uniqueness unless a scheme's
semantics and the product use case justify it.

The built-in registry is informational and deliberately small. It defines exact
scheme strings, labels, specification references, and optional lexical checks.
Validation checks syntax only: it does not normalize values, verify assignment,
resolve an identifier, or prove registry membership. A known invalid value may
be rejected by an application that opts into validation; persistence itself
continues to preserve identifiers under unknown schemes.

## Consequences

Applications can find objects by external identifiers without conflating those
identifiers with storage identity. Adding a scheme does not require changing the
domain enum or C ABI.
