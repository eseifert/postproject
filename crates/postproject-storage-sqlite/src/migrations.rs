//! Numbered, transactional SQLite schema migrations.

use postproject_core::{Error, ErrorKind, Result, Timestamp};
use rusqlite::{Connection, Transaction, TransactionBehavior, params};

/// The newest schema understood by this build.
pub const CURRENT_SCHEMA_VERSION: u32 = 3;

struct Migration {
    version: u32,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 2,
        sql: include_str!("migrations/002_initial.sql"),
    },
    Migration {
        version: 3,
        sql: include_str!("migrations/003_metadata.sql"),
    },
];

pub(crate) fn migrate(connection: &mut Connection) -> Result<()> {
    let current = schema_version(connection)?;
    if current > CURRENT_SCHEMA_VERSION {
        return Err(Error::new(
            ErrorKind::Unsupported,
            format!(
                "project schema version {current} is newer than supported version \
                 {CURRENT_SCHEMA_VERSION}"
            ),
        ));
    }

    if current != 0 && current < 2 {
        return Err(Error::new(
            ErrorKind::Unsupported,
            format!(
                "project schema version {current} is an unsupported pre-release format; \
                 create a new project with schema version {CURRENT_SCHEMA_VERSION}"
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
                format!("read project schema version: {error}"),
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
        assert_eq!(applied, [2, 3]);
        for table in [
            "projects",
            "assets",
            "representations",
            "fingerprints",
            "locations",
            "media_roots",
            "external_identifiers",
            "metadata_assertions",
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
    fn upgrades_supported_schema_two_projects() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        apply_migration(&mut connection, &MIGRATIONS[0]).expect("apply schema two");
        connection
            .execute(
                "INSERT INTO projects (
                    singleton, id, schema_version, created_at_micros, display_name
                 ) VALUES (1, zeroblob(16), 2, 0, NULL)",
                [],
            )
            .expect("insert schema-two project");

        migrate(&mut connection).expect("upgrade schema-two project");

        assert_eq!(schema_version(&connection).unwrap(), 3);
        assert_eq!(
            connection
                .query_row(
                    "SELECT schema_version FROM projects WHERE singleton = 1",
                    [],
                    |row| row.get::<_, u32>(0),
                )
                .unwrap(),
            3
        );
    }

    #[test]
    fn obsolete_development_schema_is_rejected_without_modification() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .pragma_update(None, "user_version", 1)
            .expect("set obsolete schema version");

        let error = migrate(&mut connection).expect_err("obsolete schema must be rejected");

        assert_eq!(error.kind(), ErrorKind::Unsupported);
        assert_eq!(schema_version(&connection).expect("read version"), 1);
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
            version: 2,
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
