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

Python contributors install the pinned development tools once and run the same
checks enforced by CI:

```sh
python -m pip install -e "./python[dev]"
ruff check python/src python/tests
ruff format --check python/src python/tests
ty check --project python python/src python/tests
python -m unittest discover -s python/tests -v
```

Run `ruff check --fix python/src python/tests` followed by
`ruff format python/src python/tests` to apply safe lint and formatting fixes.
The generated `python/src/postproject/_abi.py` is checked against its generator
and is deliberately excluded from Ruff formatting.

Use conventional, imperative commit subjects. Do not commit generated build
artifacts or real production media.
