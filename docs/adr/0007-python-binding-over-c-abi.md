# ADR 0007: Python binding over the C ABI

- Status: Proposed
- Date: 2026-09-19

## Decision

The Python package will consume only the installed public C ABI, use explicit
native-library discovery and cleanup, and duplicate no domain validation. The
specific C-ABI binding technology will be selected after the second-iteration
ABI can express identifiers, metadata, provenance, and revisions coherently.

## Consequences

Python validates the same integration boundary used by non-Rust applications.
PyO3 and direct internal Rust access are excluded from the primary binding.
