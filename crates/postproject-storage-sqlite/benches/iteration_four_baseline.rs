//! Baseline timings for existing read paths on the iteration-four fixture.

use std::{env, hint::black_box, path::PathBuf, time::Instant};

use postproject_core::{
    AssetId, MetadataProperty, ObjectRef, PropertyId, RepresentationId, ResourceId, RevisionId,
    VocabularyId,
};
use postproject_media::InventoryScanner;
use postproject_storage_sqlite::SqliteProduction;

const ASSET_COUNT: u64 = 10_000;
const REPRESENTATION_COUNT: u64 = 100_000;
const REVISION_COUNT: u64 = 100_000;

fn main() {
    let path = env::var_os("POSTPROJECT_BENCH_FIXTURE").map_or_else(
        || PathBuf::from("target/bench-fixtures/iteration-four.pproj"),
        PathBuf::from,
    );
    let seed = env::var("POSTPROJECT_BENCH_SEED").unwrap_or_else(|_| "postproject-i4".into());
    let runs = env::var("POSTPROJECT_BENCH_RUNS").map_or(3, |value| {
        value.parse().expect("POSTPROJECT_BENCH_RUNS is an integer")
    });
    assert!(runs > 0, "POSTPROJECT_BENCH_RUNS must be positive");
    assert!(path.is_file(), "generate {} first", path.display());

    let assets = ids::<AssetId>(&seed, b'a', ASSET_COUNT, AssetId::from_bytes);
    let representations = ids::<RepresentationId>(
        &seed,
        b'p',
        REPRESENTATION_COUNT,
        RepresentationId::from_bytes,
    );
    let resources = resource_ids(&seed);
    let metadata_property = MetadataProperty::new(
        VocabularyId::new("https://postproject.org/ns/benchmark").expect("fixture vocabulary"),
        PropertyId::new("property-000").expect("fixture property"),
    );
    let first = representations[0];
    let end_of_chain = representations[50];
    let fan_out_input = representations[1_000];
    let last_revision = RevisionId::from_bytes(stable_id(&seed, b'e', REVISION_COUNT));

    println!("fixture\t{}", path.display());
    println!("seed\t{seed}");
    println!("runs\t{runs}");
    measure("open", runs, || {
        black_box(SqliteProduction::open(&path).expect("open fixture"));
    });
    measure_read("assets", runs, &path, |production| {
        black_box(production.assets().expect("load assets"));
    });
    measure_read("representations_by_asset", runs, &path, |production| {
        for id in &assets {
            black_box(
                production
                    .representations(*id)
                    .expect("load representations"),
            );
        }
    });
    measure_read("resources_by_representation", runs, &path, |production| {
        for id in &representations {
            black_box(production.resources(*id).expect("load resources"));
        }
    });
    measure_read("locators_by_resource", runs, &path, |production| {
        for id in &resources {
            black_box(production.locators(*id).expect("load locators"));
        }
    });
    measure_read("metadata_by_asset", runs, &path, |production| {
        for id in &assets {
            black_box(
                production
                    .metadata(ObjectRef::Asset(*id))
                    .expect("load metadata"),
            );
        }
    });
    measure_read("metadata_property", runs, &path, |production| {
        black_box(
            production
                .query_by_metadata_property(&metadata_property)
                .expect("query metadata property"),
        );
    });
    measure_read("activities", runs, &path, |production| {
        black_box(production.activities().expect("load activities"));
    });
    measure_read("activities_producing", runs, &path, |production| {
        black_box(
            production
                .activities_producing(end_of_chain)
                .expect("load producing activities"),
        );
    });
    measure_read("activities_consuming", runs, &path, |production| {
        black_box(
            production
                .activities_consuming(fan_out_input)
                .expect("load consuming activities"),
        );
    });
    measure_read("ancestors_depth_50", runs, &path, |production| {
        black_box(production.ancestors(end_of_chain).expect("load ancestors"));
    });
    measure_read("descendants_depth_50", runs, &path, |production| {
        black_box(production.descendants(first).expect("load descendants"));
    });
    benchmark_inventory(&path, runs);
    benchmark_revision_reads(&path, runs, last_revision);
}

fn benchmark_inventory(path: &PathBuf, runs: usize) {
    measure_read("inventory_knowledge_walk", runs, path, |production| {
        black_box(
            InventoryScanner::default()
                .scan(production, &[], None)
                .expect("scan fixture knowledge"),
        );
    });
}

fn benchmark_revision_reads(path: &PathBuf, runs: usize, last_revision: RevisionId) {
    measure_read("latest_revision", runs, path, |production| {
        black_box(production.latest_revision().expect("load latest revision"));
    });
    measure_read("changes_since_1000", runs, path, |production| {
        black_box(
            production
                .changes_since(0, 1_000)
                .expect("load revision page"),
        );
    });
    measure_read("events_for_revision", runs, path, |production| {
        black_box(
            production
                .events_for_revision(last_revision)
                .expect("load revision events"),
        );
    });
}

fn measure_read(
    name: &str,
    runs: usize,
    path: &PathBuf,
    mut operation: impl FnMut(&SqliteProduction),
) {
    measure(name, runs, || {
        let production = SqliteProduction::open(path).expect("open fixture");
        operation(&production);
    });
}

fn measure(name: &str, runs: usize, mut operation: impl FnMut()) {
    let mut samples = Vec::with_capacity(runs);
    for _ in 0..runs {
        let started = Instant::now();
        operation();
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    let median = samples[samples.len() / 2].as_secs_f64() * 1_000.0;
    let minimum = samples[0].as_secs_f64() * 1_000.0;
    let maximum = samples[samples.len() - 1].as_secs_f64() * 1_000.0;
    println!("{name}\tmedian={median:.3} ms\tmin={minimum:.3} ms\tmax={maximum:.3} ms");
}

fn ids<T>(seed: &str, domain: u8, count: u64, convert: impl Fn([u8; 16]) -> T) -> Vec<T> {
    (0..count)
        .map(|index| convert(stable_id(seed, domain, index)))
        .collect()
}

fn resource_ids(seed: &str) -> Vec<ResourceId> {
    (0..REPRESENTATION_COUNT)
        .flat_map(|index| {
            let count = if index % 4 >= 2 { 2 } else { 1 };
            (0..count).map(move |member| {
                ResourceId::from_bytes(stable_id(seed, b'r', index * 2 + member))
            })
        })
        .collect()
}

fn stable_id(seed: &str, domain: u8, index: u64) -> [u8; 16] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(seed.as_bytes());
    hasher.update(&[domain]);
    hasher.update(&index.to_be_bytes());
    let mut id: [u8; 16] = hasher.finalize().as_bytes()[..16]
        .try_into()
        .expect("BLAKE3 prefix is 16 bytes");
    id[6] = (id[6] & 0x0f) | 0x40;
    id[8] = (id[8] & 0x3f) | 0x80;
    id
}
