//! Deterministic, cacheable large-production fixture generator.

use std::{env, fs, path::PathBuf};

use postproject_storage_sqlite::SqliteProduction;
use rusqlite::{Connection, params};

const ASSETS: u64 = 10_000;
const REPRESENTATIONS_PER_ASSET: u64 = 10;
const METADATA_PER_ASSET: u64 = 100;
const REVISIONS: u64 = 100_000;

fn main() {
    let path = env::var_os("POSTPROJECT_BENCH_FIXTURE").map_or_else(
        || PathBuf::from("target/bench-fixtures/release-0.4.pproj"),
        PathBuf::from,
    );
    let seed = env::var("POSTPROJECT_BENCH_SEED").unwrap_or_else(|_| "postproject-0.4".into());
    if path.is_file() {
        validate(&path, &seed);
        println!("reusing {}", path.display());
        return;
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture cache directory");
    }
    SqliteProduction::create(&path, Some(format!("Release 0.4 fixture ({seed})")))
        .expect("create fixture production");
    populate(&path, &seed);
    validate(&path, &seed);
    println!("created {}", path.display());
}

fn populate(path: &PathBuf, seed: &str) {
    let mut connection = Connection::open(path).expect("open fixture database");
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )
        .expect("configure fixture connection");
    let transaction = connection.transaction().expect("begin fixture population");
    transaction
        .execute(
            "CREATE TABLE benchmark_fixture (
                singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                seed TEXT NOT NULL,
                generator_version INTEGER NOT NULL
             )",
            [],
        )
        .expect("create fixture marker");
    transaction
        .execute("INSERT INTO benchmark_fixture VALUES (1, ?1, 1)", [seed])
        .expect("record fixture seed");

    for asset_index in 0..ASSETS {
        let asset_id = stable_id(seed, b'a', asset_index);
        transaction
            .execute(
                "INSERT INTO assets (id, created_at_micros, display_name, import_source)
                 VALUES (?1, ?2, ?3, 'benchmark')",
                params![
                    asset_id,
                    i64::try_from(asset_index).unwrap(),
                    format!("Asset {asset_index:05}")
                ],
            )
            .expect("insert fixture asset");
        for local_index in 0..REPRESENTATIONS_PER_ASSET {
            let representation_index = asset_index * REPRESENTATIONS_PER_ASSET + local_index;
            insert_representation(
                &transaction,
                seed,
                asset_id.as_slice(),
                representation_index,
            );
        }
        for property_index in 0..METADATA_PER_ASSET {
            let value = asset_index * METADATA_PER_ASSET + property_index;
            transaction
                .execute(
                    "INSERT INTO metadata_assertions (
                        target_kind, target_id, vocabulary, property, position, encoded_value
                     ) VALUES (1, ?1, 'https://postproject.org/ns/benchmark', ?2, 0, ?3)",
                    params![
                        asset_id,
                        format!("property-{property_index:03}"),
                        encode_u64(value),
                    ],
                )
                .expect("insert fixture metadata");
        }
    }
    insert_provenance(&transaction, seed);
    insert_revisions(&transaction, seed);
    transaction.commit().expect("commit fixture population");
    connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); ANALYZE;")
        .expect("finalize fixture database");
}

fn insert_representation(
    transaction: &rusqlite::Transaction<'_>,
    seed: &str,
    asset_id: &[u8],
    index: u64,
) {
    let representation_id = stable_id(seed, b'p', index);
    let structure_kind = i64::try_from(index % 4).unwrap();
    let representation_kind = if index % REPRESENTATIONS_PER_ASSET == 0 {
        0
    } else {
        3
    };
    transaction
        .execute(
            "INSERT INTO representations (id, asset_id, kind, structure_kind)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                representation_id,
                asset_id,
                representation_kind,
                structure_kind
            ],
        )
        .expect("insert fixture representation");
    let member_count = if structure_kind >= 2 { 2 } else { 1 };
    let mut representation_hasher = blake3::Hasher::new();
    for member_index in 0..member_count {
        let resource_index = index * 2 + u64::try_from(member_index).unwrap();
        let resource_id = stable_id(seed, b'r', resource_index);
        let locator_id = stable_id(seed, b'l', resource_index);
        let resource_digest = blake3::hash(format!("{seed}:resource:{resource_index}").as_bytes());
        representation_hasher.update(resource_digest.as_bytes());
        transaction
            .execute(
                "INSERT INTO resources (id, file_size_bytes, modified_at_micros)
                 VALUES (?1, ?2, ?3)",
                params![
                    resource_id,
                    1_000_000_i64 + i64::try_from(resource_index).unwrap(),
                    i64::try_from(index).unwrap()
                ],
            )
            .expect("insert fixture resource");
        transaction
            .execute(
                "INSERT INTO resource_fingerprints
                 (resource_id, algorithm, algorithm_version, value)
                 VALUES (?1, 'pp-blake3-full-file', 1, ?2)",
                params![resource_id, resource_digest.as_bytes().as_slice()],
            )
            .expect("insert fixture resource fingerprint");
        transaction
            .execute(
                "INSERT INTO representation_resources
                 (representation_id, resource_id, position, role, required)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    representation_id,
                    resource_id,
                    member_index,
                    if structure_kind >= 2 {
                        Some(if member_index == 0 {
                            "org.postproject:essence"
                        } else {
                            "org.postproject:sidecar"
                        })
                    } else {
                        None
                    },
                    i64::from(structure_kind != 3 || member_index == 0),
                ],
            )
            .expect("insert fixture membership");
        transaction
            .execute(
                "INSERT INTO locators (id, resource_id, uri, last_seen_micros, availability)
                 VALUES (?1, ?2, ?3, 0, 1)",
                params![
                    locator_id,
                    resource_id,
                    format!("root://media/{index:06}/{member_index}.mov")
                ],
            )
            .expect("insert fixture locator");
        if structure_kind == 1 {
            transaction
                .execute(
                    "INSERT INTO image_sequences (
                        representation_id, resource_id, prefix, suffix, padding,
                        start_frame, end_frame, frame_step, rate_numerator, rate_denominator
                     ) VALUES (?1, ?2, 'shot.', '.exr', 6, 1001, 1100, 1, 24, 1)",
                    params![representation_id, resource_id],
                )
                .expect("insert fixture sequence");
        }
    }
    let digest = representation_hasher.finalize();
    transaction
        .execute(
            "INSERT INTO representation_fingerprints
             (representation_id, algorithm, algorithm_version, value)
             VALUES (?1, 'pp-blake3-representation', 1, ?2)",
            params![representation_id, digest.as_bytes().as_slice()],
        )
        .expect("insert fixture representation fingerprint");
}

fn insert_provenance(transaction: &rusqlite::Transaction<'_>, seed: &str) {
    for index in 0..50_u64 {
        insert_activity(transaction, seed, index, &[index], index + 1);
    }
    for index in 0..100_u64 {
        insert_activity(transaction, seed, 100 + index, &[1_000], 1_001 + index);
    }
    let fan_in: Vec<_> = (2_000..2_100).collect();
    insert_activity(transaction, seed, 1_000, &fan_in, 2_100);
}

fn insert_activity(
    transaction: &rusqlite::Transaction<'_>,
    seed: &str,
    activity_index: u64,
    inputs: &[u64],
    output: u64,
) {
    let activity_id = stable_id(seed, b'v', activity_index);
    transaction
        .execute(
            "INSERT INTO activities (id, kind, tool_name, tool_version)
             VALUES (?1, 'org.postproject:benchmark', 'fixture-generator', '1')",
            [activity_id.as_slice()],
        )
        .expect("insert fixture activity");
    for input in inputs {
        transaction
            .execute(
                "INSERT INTO activity_inputs (activity_id, representation_id, role)
                 VALUES (?1, ?2, 'org.postproject:input')",
                params![activity_id, stable_id(seed, b'p', *input)],
            )
            .expect("insert fixture input");
    }
    transaction
        .execute(
            "INSERT INTO activity_outputs (activity_id, representation_id, role)
             VALUES (?1, ?2, 'org.postproject:output')",
            params![activity_id, stable_id(seed, b'p', output)],
        )
        .expect("insert fixture output");
}

fn insert_revisions(transaction: &rusqlite::Transaction<'_>, seed: &str) {
    let target = stable_id(seed, b'a', 0);
    for sequence in 1..=REVISIONS {
        let stored_sequence = i64::try_from(sequence).expect("revision sequence fits SQLite");
        let revision_id = stable_id(seed, b'e', sequence);
        transaction
            .execute(
                "INSERT INTO revisions (
                    id, sequence, transaction_id, committed_at_micros, origin_name, message
                 ) VALUES (?1, ?2, ?3, ?4, 'fixture-generator', 'scale history')",
                params![
                    revision_id,
                    stored_sequence,
                    stable_id(seed, b't', sequence),
                    stored_sequence
                ],
            )
            .expect("insert fixture revision");
        transaction
            .execute(
                "INSERT INTO revision_events (revision_id, position, kind, primary_id)
                 VALUES (?1, 0, 1, ?2)",
                params![revision_id, target],
            )
            .expect("insert fixture revision event");
    }
}

fn stable_id(seed: &str, domain: u8, index: u64) -> Vec<u8> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(seed.as_bytes());
    hasher.update(&[domain]);
    hasher.update(&index.to_be_bytes());
    let mut id = hasher.finalize().as_bytes()[..16].to_vec();
    id[6] = (id[6] & 0x0f) | 0x40;
    id[8] = (id[8] & 0x3f) | 0x80;
    id
}

fn encode_u64(value: u64) -> Vec<u8> {
    let mut encoded = b"PPMV\x01\x03".to_vec();
    encoded.extend_from_slice(&value.to_be_bytes());
    encoded
}

fn validate(path: &PathBuf, seed: &str) {
    let connection = Connection::open(path).expect("open cached fixture");
    let stored_seed: String = connection
        .query_row(
            "SELECT seed FROM benchmark_fixture WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .expect("read fixture marker");
    assert_eq!(stored_seed, seed, "cached fixture uses another seed");
    for (table, minimum) in [
        ("assets", ASSETS),
        ("representations", ASSETS * REPRESENTATIONS_PER_ASSET),
        ("resources", ASSETS * REPRESENTATIONS_PER_ASSET),
        ("locators", ASSETS * REPRESENTATIONS_PER_ASSET),
        ("metadata_assertions", ASSETS * METADATA_PER_ASSET),
        ("revisions", REVISIONS),
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("count fixture rows");
        assert!(
            count >= i64::try_from(minimum).expect("fixture minimum fits SQLite"),
            "fixture has only {count} {table}"
        );
    }
}
