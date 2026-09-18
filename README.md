# libpostproject

`libpostproject` is application-neutral infrastructure for managing stable media
identity, physical representations, locations, and relinking in professional
post-production software.

The first iteration is deliberately narrow: create a project, import media,
persist stable IDs and fingerprints, and find that media again after files move.
It does not yet provide timelines, collaboration, networking, decoding, proxy
generation, or application-specific adapters.

> **Status:** early pre-release development. No API, ABI, or schema is stable yet.

## Architecture

The implementation is split by dependency direction:

```text
postproject-cli / postproject-ffi
             |             |
             v             v
postproject-storage-sqlite  postproject-media
             \             /
              v           v
              postproject-core
```

Rust is an implementation detail. Native consumers will use an intentionally
designed C ABI, with a thin C++17 RAII wrapper layered on top.

See [the architecture](docs/architecture.md), [domain model](docs/domain-model.md),
and [roadmap](docs/roadmap.md) for the current design.

## Build

Rust 1.85 or newer is required.

```sh
cargo build --workspace
cargo test --workspace --all-features
```

Native-library build and installation instructions will be added with the C ABI.

## License

Licensed under either the MIT License or the Apache License, Version 2.0, at your
option: `MIT OR Apache-2.0`.
