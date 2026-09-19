# PostProject documentation

PostProject provides neutral infrastructure for media identity,
representations, locations, metadata persistence, production provenance, and
project-local revision semantics. It is designed for applications to embed; it
is not an editor or network service.

The documentation is split by audience:

- [Application users](users/README.md) can learn why files remain linked and
  why a choice is sometimes required after media moves.
- [Integrators](integrators/README.md) can embed the native library without
  exposing Rust or Cargo to their application build.
- [Contributors](contributors/README.md) can understand the dependency
  boundaries, standards policy, and change requirements.

PostProject interoperates with established identifiers, vocabularies, and
exchange models rather than attempting to replace them. See
[standards boundaries](concepts/standards-boundaries.md).
