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

## Iteration-four read-path baseline

The pre-query-API baseline was captured on 2026-09-24 at commit `7f835c0`
against the `postproject-i4` fixture: 10,000 assets, 100,000 representations,
150,000 resources and locators, 1,000,000 metadata assertions, 151 activities,
and 100,000 revisions. The 494 MiB production was generated and measured with:

```sh
cargo bench --locked -p postproject-storage-sqlite --bench large_fixture
POSTPROJECT_BENCH_RUNS=3 \
  cargo bench --locked -p postproject-storage-sqlite --bench iteration_four_baseline
```

Each row reports three optimized samples. A read sample uses a fresh open
production handle, starts timing after the open, consumes the result, and runs
with the operating-system file cache warm. `open` measures the handle open
itself. The nested rows deliberately issue the existing per-parent calls; the
inventory row exercises the complete assets → representations → resources →
locators knowledge walk that Phase 5 will change.

| Existing read path | Median | Minimum | Maximum |
| --- | ---: | ---: | ---: |
| Open production | 5.550 ms | 3.109 ms | 9.849 ms |
| Load all assets | 11.473 ms | 9.872 ms | 70.136 ms |
| Load representations for all assets | 5.820 s | 5.789 s | 8.728 s |
| Load resources for all representations | 8.275 s | 8.001 s | 10.660 s |
| Load locators for all resources | 4.429 s | 4.425 s | 5.775 s |
| Load metadata for all assets | 1.855 s | 1.645 s | 4.791 s |
| Query one metadata property | 42.369 ms | 39.812 ms | 56.494 ms |
| Load all activities | 1.439 ms | 1.264 ms | 2.373 ms |
| Activities producing one representation | 1.259 ms | 1.253 ms | 3.303 ms |
| Activities consuming one representation | 1.292 ms | 1.291 ms | 1.501 ms |
| Ancestors through a 50-deep chain | 0.369 ms | 0.302 ms | 1.033 ms |
| Descendants through a 50-deep chain | 0.305 ms | 0.301 ms | 0.326 ms |
| Complete inventory knowledge walk | 19.927 s | 19.884 s | 23.298 s |
| Latest revision | 0.042 ms | 0.037 ms | 0.828 ms |
| First 1,000 revisions | 1.184 ms | 1.184 ms | 3.470 ms |
| Events for one revision | 0.112 ms | 0.109 ms | 1.640 ms |

The run used Rust 1.98.1 on Linux 7.2.5, an AMD Ryzen 7 4800H, and a Samsung
970 EVO Plus NVMe SSD with Btrfs. The CPU governor was `ondemand`, frequency
boost was enabled, and the machine was not isolated. The spread in several
maximum samples is therefore scheduler noise; the medians are the comparison
baseline. These numbers precede Phase 5's pagination and set-oriented read-path
changes and are informational, not release budgets.

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
