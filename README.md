# PostProject

PostProject is application-neutral infrastructure for durable media identity,
compound representations, storage resources and locators, metadata, provenance,
and production-local change tracking in professional post-production software.

> **Status:** early `0.3.0-alpha.1` development. The named integration-preview
> subset stays compatible within the 0.3.x series; other APIs remain
> experimental. Consumers should pin a release series or exact commit.

PostProject is standards-aware infrastructure, not a new media ontology. It
preserves external identifiers and vocabulary terms and is designed to map to
industry exchange models without claiming normative compliance.

Current capabilities include stable logical asset IDs, single-resource and
compound representations, compact image-sequence descriptors, multiple
resource locators, typed fingerprints, structured metadata, activity-based
provenance, deterministic relinking with explicit ambiguity, media-root and
locator lifecycle management, and a durable semantic revision feed. SQLite
persistence, the public C ABI, the C++17 RAII wrapper, the Python 3.11 binding,
and the demonstrator CLI expose those capabilities.

PostProject deliberately does not provide a timeline editor, decoder/encoder,
job runner, collaboration server, MAM service, or automatic registry/network
lookup.

## Architecture

```text
CLI / C ABI / C++ wrapper
          |
SQLite storage + filesystem media services
          |
backend-neutral PostProject domain model
```

Rust is an implementation detail. Native applications consume the installed C
ABI, with the C++ wrapper layered only over that ABI.

## Build and test

Rust 1.85 or newer is required.

```sh
cargo build --workspace
cargo test --workspace --all-features
```

The demonstrator CLI exercises the same storage and media services as the
library:

```sh
cargo run -p postproject-cli -- init production.pproj --name "Documentary"
cargo run -p postproject-cli -- media add production.pproj examples/fixtures/sample-media.dat
cargo run -p postproject-cli -- media list production.pproj
cargo run -p postproject-cli -- root add production.pproj rushes --label "Camera originals"
cargo run -p postproject-cli -- media resolve production.pproj ASSET_ID --root-map rushes=/mnt/show/rushes
cargo run -p postproject-cli -- --json revisions since production.pproj --after 0
```

Pass `--json` before or after a subcommand for structured output. An ambiguous
resolution is never selected silently; applications must present candidates and
confirm one explicitly.

## Native integration

Build and stage the native package:

```sh
cargo build --release --locked -p postproject-ffi
cmake -S . -B target/package \
  -DPOSTPROJECT_LIBRARY="$PWD/target/release/libpostproject.so" \
  -DPOSTPROJECT_STATIC_LIBRARY="$PWD/target/release/libpostproject.a" \
  -DCMAKE_INSTALL_PREFIX="$PWD/target/install"
cmake --install target/package
```

The package exports `PostProject::postproject` for CMake and `postproject` for
`pkg-config`. Installed consumers do not invoke Cargo. Use `.dylib` on macOS;
on Windows, supply the import library and matching `postproject.dll`.

Minimal C usage:

```c
#include <postproject/postproject.h>

pp_production_t *production = NULL;
pp_error_t *error = NULL;
if (pp_production_open("production.pproj", &production, &error) != PP_OK) {
    pp_error_release(error);
    return 1;
}
pp_production_release(production);
```

Standalone installed-package examples live in [`examples/c`](examples/c) and
[`examples/cpp`](examples/cpp).

## Documentation

- [User guide](docs/src/users/README.md)
- [Integrator guide](docs/src/integrators/README.md)
- [Contributor guide](docs/src/contributors/README.md)
- [Standards boundaries](docs/src/concepts/standards-boundaries.md)
- [Roadmap](docs/roadmap.md)
- [Stewardship](STEWARDSHIP.md)
- [Iteration-one acceptance report](docs/iteration-one-report.md)
- [Iteration-two acceptance report](docs/iteration-two-report.md)

Build the documentation book with `mdbook build docs`.

## License

Licensed under either the MIT License ([`LICENSE-MIT`](LICENSE-MIT)) or the
Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE)), at your
option: `MIT OR Apache-2.0`.
