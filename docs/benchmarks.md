# Benchmarks

Iteration-one benchmarks are informational baselines built with Criterion. They
cover the four scale-sensitive workflows named in the acceptance criteria:

- fingerprinting and importing 1,000 small files in one transaction;
- opening a SQLite project that contains 10,000 assets;
- resolving relocated media beneath a root containing 3,000 decoys;
- committing repeated single-import transactions.

Run them with an optimized build:

```sh
cargo bench --locked -p postproject-storage-sqlite --bench iteration_one
```

Fixture construction that is not part of the operation under measurement is
performed before timing where practical. The bulk-import benchmark deliberately
includes fingerprint preparation because fingerprinting is part of the import
contract. Resolver decoys differ in size so the benchmark measures bounded tree
discovery and cheap filtering while hashing only the credible candidate.

Criterion reports are local artifacts under `target/criterion` and are not
committed. Published numbers must record the commit, Rust version, operating
system, CPU, storage device/filesystem, power policy, and full Criterion command.
Results are not release gates yet; they exist to make regressions measurable.
