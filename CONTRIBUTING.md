# Contributing

PostProject is in pre-1.0 development. Changes to schema, public APIs, ABI, or
domain meaning require an architecture decision record, a standards-impact
check, and coordinated updates to documentation, bindings, examples, and tests.
Keep each change focused and add tests and documentation alongside behavior.

Before submitting a change, run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --all-features --no-deps
```

Use conventional, imperative commit subjects. Do not commit generated build
artifacts or real production media.
