# ADR 0016: Documentation site generator

- Status: Accepted
- Date: 2026-09-23

## Context

PostProject documents three audiences and four public integration surfaces. The
published site must combine narrative guides with reference material generated
from the authoritative C header, the header-only C++ wrapper, Python docstrings,
and Rust documentation. It also needs release versioning and persistent language
selection.

The existing mdBook proves and link-checks the narrative structure, but it does
not ingest Doxygen XML or Python API documentation into one reference site.
Maintaining parallel hand-written reference pages would let the public ABI and
the documentation drift.

## Decision

The published documentation site uses Sphinx. Breathe imports Doxygen XML for C
and C++, autodoc imports the Python package, and rustdoc remains a separately
built reference linked from the site.

The site includes examples from tested source files rather than copying snippets.
A generated coverage manifest compares documented C symbols with the symbol list
derived from `include/postproject/postproject.h`. CI fails if an exported symbol
is absent from the reference.

Tagged releases publish immutable versioned directories. The development branch
is visibly marked as unreleased. The version selector shows both package and C
ABI versions. A language selector stores the reader's choice in browser-local
state and presents C, C++, Python, CLI, and Rust variants consistently, including
an explicit unavailable state where no equivalent operation exists.

The initial theme may be customized locally, but the content and reference model
must not depend on a proprietary documentation service.

## Alternatives considered

- **Keep mdBook as the published site.** Its narrative output is good, but a
  unified Doxygen and Python reference would require custom generator work.
- **Doxygen alone.** It covers C and C++ well but is a weak fit for the existing
  audience-oriented guides and Python documentation.
- **Separate sites per language.** This simplifies generators but makes concepts,
  versions, and examples drift between silos.

## Consequences

Documentation builds need Python, Sphinx, Doxygen, and the Breathe extension.
Theme and language-switcher work is owned by this repository. Rust API pages do
not visually merge into Sphinx, but they remain generated and version-matched.
Every new exported C symbol must be documented before CI accepts it.
