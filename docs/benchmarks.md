# Benchmarks

Iteration-one benchmarks are informational baselines built with Criterion. They
cover the four scale-sensitive workflows named in the acceptance criteria:

- fingerprinting and importing 1,000 small files in one transaction;
- opening a SQLite production that contains 10,000 assets;
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

## Quick baseline

A smoke baseline captured on 2026-09-19 with `--quick` at commit `ed56372`
produced these Criterion intervals:

| Workload | Observed interval |
| --- | ---: |
| Import and fingerprint 1,000 small files | 70.221–70.524 ms |
| Open a production containing 10,000 assets | 304.48–313.36 µs |
| Resolve beneath a root with 3,000 candidates | 10.313–10.366 ms |
| Commit one prepared import | 247.74–252.96 µs |

The run used Rust 1.98.1 on Linux 7.2.5, an AMD Ryzen 7 4800H, and a Btrfs
filesystem. Frequency boost was enabled and no power-policy controls were
applied. Because Criterion quick mode takes few samples and the machine was not
isolated, these numbers verify benchmark operation and provide an order-of-
magnitude baseline; they are not release gates or publication-quality claims.
