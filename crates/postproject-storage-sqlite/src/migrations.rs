//! Numbered, transactional SQLite schema migrations.

use postproject_core::{Error, ErrorKind, Result, Timestamp};
use rusqlite::{Connection, Transaction, TransactionBehavior, params};

/// The newest schema understood by this build.
pub const CURRENT_SCHEMA_VERSION: u32 = 8;

struct Migration {
    version: u32,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("migrations/001_initial.sql"),
    },
    Migration {
        version: 2,
        sql: include_str!("migrations/002_activities.sql"),
    },
    Migration {
        version: 3,
        sql: include_str!("migrations/003_revisions.sql"),
    },
    Migration {
        version: 4,
        sql: include_str!("migrations/004_activity_identifiers.sql"),
    },
    Migration {
        version: 5,
        sql: include_str!("migrations/005_lifecycle_events.sql"),
    },
    Migration {
        version: 6,
        sql: include_str!("migrations/006_portable_media_roots.sql"),
    },
    Migration {
        version: 7,
        sql: include_str!("migrations/007_fingerprint_observations.sql"),
    },
    Migration {
        version: 8,
        sql: include_str!("migrations/008_dependencies.sql"),
    },
];

pub(crate) fn migrate(connection: &mut Connection) -> Result<()> {
    let current = schema_version(connection)?;
    if current > CURRENT_SCHEMA_VERSION {
        return Err(Error::new(
            ErrorKind::Unsupported,
            format!(
                "production schema version {current} is newer than supported version \
                 {CURRENT_SCHEMA_VERSION}"
            ),
        ));
    }

    for migration in MIGRATIONS {
        if migration.version > current {
            apply_migration(connection, migration)?;
        }
    }
    Ok(())
}

fn schema_version(connection: &Connection) -> Result<u32> {
    connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| {
            Error::new(
                ErrorKind::Migration,
                format!("read production schema version: {error}"),
            )
        })
}

fn apply_migration(connection: &mut Connection, migration: &Migration) -> Result<()> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(migration_error(migration.version, "begin"))?;
    apply_migration_in(&transaction, migration)?;
    transaction
        .commit()
        .map_err(migration_error(migration.version, "commit"))
}

fn apply_migration_in(transaction: &Transaction<'_>, migration: &Migration) -> Result<()> {
    transaction
        .execute_batch(migration.sql)
        .map_err(migration_error(migration.version, "apply statements"))?;
    transaction
        .execute(
            "INSERT INTO schema_migrations (version, applied_at_micros) VALUES (?1, ?2)",
            params![
                migration.version,
                Timestamp::now()
                    .map_err(|error| Error::new(ErrorKind::Migration, error.to_string()))?
                    .as_unix_micros()
            ],
        )
        .map_err(migration_error(migration.version, "record history"))?;
    transaction
        .pragma_update(None, "user_version", migration.version)
        .map_err(migration_error(migration.version, "record schema version"))?;
    Ok(())
}

fn migration_error(version: u32, action: &'static str) -> impl FnOnce(rusqlite::Error) -> Error {
    move |error| {
        Error::new(
            ErrorKind::Migration,
            format!("migration {version}: {action}: {error}"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use postproject_core::{
        ArtifactEvaluationLimits, ArtifactKnowledgeReason, ArtifactKnowledgeState, RepresentationId,
    };

    use crate::{SqliteProduction, load_production};

    #[test]
    fn migrates_schema_zero_fixture_to_current() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute_batch(include_str!("../../../tests/fixtures/schema-000.sql"))
            .expect("load schema-zero fixture");

        migrate(&mut connection).expect("migrate schema-zero fixture");

        assert_eq!(
            schema_version(&connection).expect("read migrated version"),
            CURRENT_SCHEMA_VERSION
        );
        let applied: Vec<u32> = connection
            .prepare("SELECT version FROM schema_migrations ORDER BY version")
            .expect("prepare migration-history query")
            .query_map([], |row| row.get(0))
            .expect("query migration history")
            .collect::<std::result::Result<_, _>>()
            .expect("read migration history");
        assert_eq!(applied, [1, 2, 3, 4, 5, 6, 7, 8]);
        for table in [
            "productions",
            "assets",
            "representations",
            "resources",
            "representation_resources",
            "image_sequences",
            "image_sequence_missing_frames",
            "resource_fingerprints",
            "representation_fingerprints",
            "locators",
            "media_roots",
            "external_identifiers",
            "metadata_assertions",
            "activities",
            "activity_inputs",
            "activity_outputs",
            "revisions",
            "revision_events",
            "resource_fingerprint_history",
            "representation_fingerprint_history",
            "representation_fingerprint_recomputations",
            "activity_input_fingerprint_snapshots",
            "activity_output_fingerprint_snapshots",
            "dependency_sets",
            "dependencies",
            "activity_input_dependency_snapshots",
            "activity_input_dependency_paths",
            "activity_input_dependency_path_edges",
            "activity_input_dependency_fingerprint_snapshots",
        ] {
            let count: u32 = connection
                .query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .expect("query migrated table");
            assert_eq!(count, 1, "missing migrated table {table}");
        }
    }

    #[test]
    fn later_migrations_update_existing_production_version() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        apply_migration(&mut connection, &MIGRATIONS[0]).expect("apply first migration");
        connection
            .execute(
                "INSERT INTO productions (
                    singleton, id, schema_version, created_at_micros, display_name
                 ) VALUES (1, zeroblob(16), 1, 0, NULL)",
                [],
            )
            .expect("insert version-one production");
        connection
            .execute(
                "INSERT INTO media_roots (id, uri, label, priority, enabled)
                 VALUES (?1, 'file:///mnt/media', 'Camera originals', 5, 1)",
                [vec![7_u8; 16]],
            )
            .expect("insert absolute media root");

        migrate(&mut connection).expect("migrate existing production");

        let production_version: u32 = connection
            .query_row("SELECT schema_version FROM productions", [], |row| {
                row.get(0)
            })
            .expect("read production version");
        assert_eq!(production_version, CURRENT_SCHEMA_VERSION);
        let migrated_root: (String, Option<String>, Option<String>) = connection
            .query_row(
                "SELECT name, label, legacy_uri FROM media_roots",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read migrated media root");
        assert_eq!(
            migrated_root,
            (
                "legacy-07070707070707070707070707070707".to_owned(),
                Some("Camera originals".to_owned()),
                Some("file:///mnt/media".to_owned()),
            )
        );
    }

    #[test]
    fn schema_six_activities_migrate_without_fabricated_snapshots() {
        let mut connection = Connection::open_in_memory().expect("open database");
        for migration in &MIGRATIONS[..6] {
            apply_migration(&mut connection, migration).expect("apply old migration");
        }
        connection
            .execute(
                "INSERT INTO productions (
                    singleton, id, schema_version, created_at_micros, display_name
                 ) VALUES (1, zeroblob(16), 6, 0, NULL)",
                [],
            )
            .expect("insert production");
        connection
            .execute(
                "INSERT INTO assets VALUES (?1, 0, NULL, NULL)",
                [vec![1_u8; 16]],
            )
            .expect("insert asset");
        for label in [2_u8, 3] {
            connection
                .execute(
                    "INSERT INTO representations VALUES (?1, ?2, 3, 0)",
                    params![vec![label; 16], vec![1_u8; 16]],
                )
                .expect("insert representation");
            connection
                .execute(
                    "INSERT INTO resources (id) VALUES (?1)",
                    [vec![label + 10; 16]],
                )
                .expect("insert resource");
            connection
                .execute(
                    "INSERT INTO representation_resources VALUES (?1, ?2, 0, NULL, 1)",
                    params![vec![label; 16], vec![label + 10; 16]],
                )
                .expect("insert representation resource");
        }
        connection
            .execute(
                "INSERT INTO activities (id, kind) VALUES (?1, 'example:activity')",
                [vec![4_u8; 16]],
            )
            .expect("insert activity");
        connection
            .execute(
                "INSERT INTO activity_inputs (activity_id, representation_id)
                 VALUES (?1, ?2)",
                params![vec![4_u8; 16], vec![2_u8; 16]],
            )
            .expect("insert input");
        connection
            .execute(
                "INSERT INTO activity_outputs (activity_id, representation_id)
                 VALUES (?1, ?2)",
                params![vec![4_u8; 16], vec![3_u8; 16]],
            )
            .expect("insert output");

        migrate(&mut connection).expect("migrate schema six");

        for table in ["activity_inputs", "activity_outputs"] {
            let snapshot: Option<i64> = connection
                .query_row(
                    &format!("SELECT snapshot_revision_sequence FROM {table}"),
                    [],
                    |row| row.get(0),
                )
                .expect("load migrated snapshot");
            assert_eq!(snapshot, None);
        }
        let dependency_snapshot_count: u32 = connection
            .query_row(
                "SELECT count(*) FROM activity_input_dependency_snapshots",
                [],
                |row| row.get(0),
            )
            .expect("count migrated dependency snapshots");
        assert_eq!(dependency_snapshot_count, 0);

        let production = load_production(&connection).expect("load migrated production");
        let production = SqliteProduction {
            path: std::path::PathBuf::new(),
            connection,
            production,
        };
        let evaluation = production
            .evaluate_artifact(
                RepresentationId::from_bytes([3_u8; 16]),
                ArtifactEvaluationLimits::default(),
            )
            .expect("evaluate migrated activity");
        assert_eq!(evaluation.state(), ArtifactKnowledgeState::Indeterminate);
        assert!(
            evaluation
                .reasons()
                .iter()
                .any(|reason| matches!(reason, ArtifactKnowledgeReason::SnapshotAbsent { .. }))
        );
    }

    #[test]
    fn newer_schema_is_rejected_without_modification() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION + 1)
            .expect("set future schema version");

        let error = migrate(&mut connection).expect_err("future schema must be rejected");

        assert_eq!(error.kind(), ErrorKind::Unsupported);
        assert_eq!(
            schema_version(&connection).expect("read unchanged version"),
            CURRENT_SCHEMA_VERSION + 1
        );
    }

    #[test]
    fn failed_migration_rolls_back_completely() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        let invalid = Migration {
            version: 1,
            sql: "CREATE TABLE partial (value INTEGER); INVALID SQL;",
        };

        let error = apply_migration(&mut connection, &invalid)
            .expect_err("invalid migration must be rejected");

        assert_eq!(error.kind(), ErrorKind::Migration);
        assert_eq!(schema_version(&connection).expect("read version"), 0);
        let table_count: u32 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name = 'partial'",
                [],
                |row| row.get(0),
            )
            .expect("query schema");
        assert_eq!(table_count, 0);
    }
}
