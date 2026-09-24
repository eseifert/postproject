# Benchmarks

Iteration-one benchmarks are informational baselines built with Criterion. They
cover the four scale-sensitive workflows named in the acceptance criteria:

- fingerprinting and importing 1,000 small files in one transaction;
- opening and enumerating a SQLite production containing 10,000 assets with
  resource and representation fingerprints;
- resolving relocated media beneath a root containing 3,000 decoys;
- committing repeated single-import transactions.

Run them with an optimized build:

```sh
cargo bench --locked -p postproject-storage-sqlite --bench iteration_one
```

Fixture construction that is not part of the operation under measurement is
performed before timing where practical. The bulk-import benchmark deliberately
includes fingerprint preparation because fingerprinting is part of the import
contract. Resolver decoys have the same size as the target, so all 3,000
candidates reach fingerprint verification instead of being discarded by the
cheap size filter. The large-production workload traverses representations,
resources, and their non-empty fingerprint evidence rather than timing only the
root production row.

Criterion reports are local artifacts under `target/criterion` and are not
committed. Published numbers must record the commit, Rust version, operating
system, CPU, storage device/filesystem, power policy, and full Criterion command.
Results are not release gates yet; they exist to make regressions measurable.

## Iteration-four scale fixture

The `large_fixture` benchmark target is a deterministic generator rather than
a timed benchmark. It creates the representative production used by the
iteration-four query, staleness, job, and revision-feed benchmarks:

- 10,000 assets and 100,000 representations;
- more than 100,000 resources and locators across single-file, sequence,
  ordered-part, and package structures;
- 1,000,000 typed metadata assertions;
- fan-in, fan-out, and a provenance chain 50 activities deep;
- 100,000 revisions with events;
- real BLAKE3-shaped resource and representation fingerprint evidence.

The seed and cache path are explicit, and fixture construction is never part of
the measured operation:

```sh
POSTPROJECT_BENCH_SEED=postproject-i4 \
POSTPROJECT_BENCH_FIXTURE=target/bench-fixtures/iteration-four.pproj \
  cargo bench --locked -p postproject-storage-sqlite --bench large_fixture
```

An existing cache is validated and reused. Delete that one explicit file to
regenerate it after the generator version or seed changes. Generation uses no
network, locale, or wall-clock input.

## Quick baseline

A smoke baseline captured on 2026-09-22 with `--quick` at commit `1e92194`
produced these Criterion intervals:

| Workload | Observed interval |
| --- | ---: |
| Import and fingerprint 1,000 small files | 257.42–271.36 ms |
| Load 10,000 assets with identity evidence | 1.0835–1.0980 s |
| Resolve and fingerprint 3,000 same-size candidates | 84.330–86.524 ms |
| Commit one prepared import | 742.71–756.72 µs |

The run used Rust 1.98.1 on Linux 7.2.5, an AMD Ryzen 7 4800H, and a Btrfs
filesystem. Frequency boost was enabled and no power-policy controls were
applied. Because Criterion quick mode takes few samples and the machine was not
isolated, these numbers verify benchmark operation and provide an order-of-
magnitude baseline; they are not release gates or publication-quality claims.
