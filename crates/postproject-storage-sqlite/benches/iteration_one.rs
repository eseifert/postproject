//! Informational performance baselines for iteration-one workloads.

#![allow(
    missing_docs,
    reason = "Criterion generates public harness entry points with no public API"
)]

use std::{fs, hint::black_box, path::PathBuf};

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use postproject_core::{
    Asset, AssetId, Location, LocationAvailability, LocationId, OriginalMediaImport,
    Representation, RepresentationId, RepresentationKind, Timestamp,
};
use postproject_media::{MediaResolver, prepare_media_root, prepare_original_media};
use postproject_storage_sqlite::SqliteProject;
use tempfile::TempDir;

const BULK_IMPORT_COUNT: usize = 1_000;
const LARGE_PROJECT_ASSET_COUNT: usize = 10_000;
const RESOLVER_ENTRY_COUNT: usize = 3_000;

fn benchmarks(criterion: &mut Criterion) {
    benchmark_bulk_import(criterion);
    benchmark_large_project_open(criterion);
    benchmark_resolver_scan(criterion);
    benchmark_transaction_commit(criterion);
}

fn benchmark_bulk_import(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("media_import");
    group.sample_size(10);
    group.bench_function("1000_small_files", |bencher| {
        bencher.iter_batched(
            || media_fixture(BULK_IMPORT_COUNT, "bulk"),
            |(directory, media)| {
                let mut project = SqliteProject::create(directory.path().join("bulk.pproj"), None)
                    .expect("create benchmark project");
                let prepared: Vec<_> = media
                    .iter()
                    .map(|path| {
                        prepare_original_media(path, None, Some("benchmark".to_owned()))
                            .expect("prepare benchmark import")
                    })
                    .collect();
                let mut transaction = project
                    .begin_transaction()
                    .expect("begin bulk-import transaction");
                for import in &prepared {
                    transaction
                        .import_original(import)
                        .expect("stage benchmark import");
                }
                transaction.commit().expect("commit benchmark imports");
                drop(transaction);
                black_box(project);
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

fn benchmark_large_project_open(criterion: &mut Criterion) {
    let directory = tempfile::tempdir().expect("create large-project fixture");
    let path = directory.path().join("large.pproj");
    let mut project = SqliteProject::create(&path, None).expect("create large project");
    let mut transaction = project
        .begin_transaction()
        .expect("begin large-project transaction");
    for index in 0..LARGE_PROJECT_ASSET_COUNT {
        transaction
            .import_original(&synthetic_import(index))
            .expect("stage synthetic import");
    }
    transaction.commit().expect("commit large-project fixture");
    drop(transaction);
    drop(project);

    criterion.bench_function("project_open/10000_assets", |bencher| {
        bencher.iter(|| black_box(SqliteProject::open(&path).expect("open benchmark project")));
    });
}

fn benchmark_resolver_scan(criterion: &mut Criterion) {
    let directory = tempfile::tempdir().expect("create resolver fixture");
    let original = directory.path().join("original.mov");
    let matching_bytes = vec![42_u8; 4_096];
    fs::write(&original, &matching_bytes).expect("write original media");
    let import = prepare_original_media(&original, None, None).expect("prepare original media");
    fs::remove_file(&original).expect("remove known location");

    let root_path = directory.path().join("search-root");
    fs::create_dir(&root_path).expect("create resolver root");
    for index in 0..RESOLVER_ENTRY_COUNT {
        fs::write(root_path.join(format!("decoy-{index:04}.mov")), [0_u8])
            .expect("write resolver decoy");
    }
    fs::write(root_path.join("relocated.mov"), matching_bytes).expect("write relocated media");
    let root = prepare_media_root(&root_path, None, 0).expect("prepare resolver root");
    let resolver = MediaResolver::default();

    criterion.bench_function("media_resolve/3000_candidates", |bencher| {
        bencher.iter(|| {
            black_box(
                resolver
                    .resolve(
                        import.representation(),
                        std::slice::from_ref(import.location()),
                        std::slice::from_ref(&root),
                    )
                    .expect("resolve benchmark media"),
            )
        });
    });
}

fn benchmark_transaction_commit(criterion: &mut Criterion) {
    let directory = tempfile::tempdir().expect("create transaction fixture");
    let mut project = SqliteProject::create(directory.path().join("transactions.pproj"), None)
        .expect("create transaction project");
    let mut index = 0_usize;

    criterion.bench_function("transaction_commit/single_import", |bencher| {
        bencher.iter(|| {
            let import = synthetic_import(index);
            index = index.wrapping_add(1);
            let mut transaction = project
                .begin_transaction()
                .expect("begin benchmark transaction");
            transaction
                .import_original(&import)
                .expect("stage benchmark mutation");
            transaction.commit().expect("commit benchmark transaction");
        });
    });
}

fn media_fixture(count: usize, prefix: &str) -> (TempDir, Vec<PathBuf>) {
    let directory = tempfile::tempdir().expect("create media fixture");
    let mut paths = Vec::with_capacity(count);
    for index in 0..count {
        let path = directory.path().join(format!("{prefix}-{index:04}.mov"));
        fs::write(&path, format!("fixture media {index}")).expect("write media fixture");
        paths.push(path);
    }
    (directory, paths)
}

fn synthetic_import(index: usize) -> OriginalMediaImport {
    let asset = Asset::new(
        AssetId::new(),
        Timestamp::from_unix_micros(i64::try_from(index).expect("benchmark index fits i64")),
        None,
        Some("benchmark".to_owned()),
    );
    let representation = Representation::new(
        RepresentationId::new(),
        asset.id(),
        RepresentationKind::Original,
        None,
        None,
    );
    let location = Location::new(
        LocationId::new(),
        representation.id(),
        format!("file:///benchmark/{index}.mov"),
        None,
        LocationAvailability::Unknown,
    )
    .expect("construct benchmark location");
    OriginalMediaImport::new(asset, representation, location).expect("construct benchmark import")
}

criterion_group!(iteration_one, benchmarks);
criterion_main!(iteration_one);
