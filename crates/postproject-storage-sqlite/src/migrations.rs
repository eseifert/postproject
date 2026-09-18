//! Numbered, transactional SQLite schema migrations.

use postproject_core::{Error, ErrorKind, Result, Timestamp};
use rusqlite::{Connection, Transaction, TransactionBehavior, params};

/// The newest schema understood by this build.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

struct Migration {
    version: u32,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: include_str!("migrations/001_initial.sql"),
}];

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
