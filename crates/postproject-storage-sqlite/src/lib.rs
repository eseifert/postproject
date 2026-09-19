//! SQLite project-file persistence for libpostproject.
//!
//! This crate translates between domain values and a private, migrated SQLite
//! schema. SQLite types and errors are never part of the core API contract.

#![forbid(unsafe_code)]

mod migrations;
mod transaction;

use std::{
    fs::OpenOptions,
    path::{Path, PathBuf},
    time::Duration,
};

use postproject_core::{
    Asset, AssetId, Error, ErrorKind, FileFacts, Fingerprint, Location, LocationAvailability,
    MediaRoot, MediaRootId, Project, ProjectId, Representation, RepresentationId,
    RepresentationKind, Result, Timestamp,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension, limits::Limit, params};

pub use migrations::CURRENT_SCHEMA_VERSION;
pub use transaction::SqliteTransaction;

const MAX_SQLITE_VALUE_BYTES: i32 = 16 * 1024 * 1024;

/// A project backed by one SQLite project file.
#[derive(Debug)]
pub struct SqliteProject {
    path: PathBuf,
    connection: Connection,
    project: Project,
}

impl SqliteProject {
    /// Creates a new project file and persists its identity atomically.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::AlreadyExists`] if `path` already exists. I/O,
    /// migration, and storage failures are mapped to their domain categories.
    pub fn create(path: impl AsRef<Path>, display_name: Option<String>) -> Result<Self> {
        let path = path.as_ref();
        reserve_new_file(path)?;

        let mut connection = open_connection(path)?;
        migrations::migrate(&mut connection)?;

        let project = Project::new(
            ProjectId::new(),
            CURRENT_SCHEMA_VERSION,
            Timestamp::now()?,
            display_name,
        );
        persist_new_project(&mut connection, &project)?;

        Ok(Self {
            path: path.to_path_buf(),
            connection,
            project,
        })
    }

    /// Opens an existing project file, applying supported migrations first.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::NotFound`] if `path` does not exist. Invalid,
    /// unsupported, or inaccessible databases return a migration or storage error.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.is_file() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("project file does not exist: {}", path.display()),
            ));
        }

        let mut connection = open_connection(path)?;
        migrations::migrate(&mut connection)?;
        let project = load_project(&connection)?;

        Ok(Self {
            path: path.to_path_buf(),
            connection,
            project,
        })
    }

    /// Returns the project-file path used by this backend.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the loaded project value.
    #[must_use]
    pub const fn project(&self) -> &Project {
        &self.project
    }

    /// Begins an explicit domain transaction for project mutations.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Storage`] if SQLite cannot start the transaction.
    pub fn begin_transaction(&mut self) -> Result<SqliteTransaction<'_>> {
        let (connection, project) = (&mut self.connection, &mut self.project);
        SqliteTransaction::begin(connection, project)
    }

    /// Loads all assets in deterministic creation/identity order.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Storage`] for query failures or invalid stored data.
    pub fn assets(&self) -> Result<Vec<Asset>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, created_at_micros, display_name, import_source
                 FROM assets ORDER BY created_at_micros, id",
            )
            .map_err(sqlite_error("prepare asset query"))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(sqlite_error("query assets"))?;

        rows.map(|row| {
            let (id, created_at, display_name, import_source) =
                row.map_err(sqlite_error("read asset row"))?;
            Ok(Asset::new(
                AssetId::from_bytes(id_bytes(id, "asset")?),
                Timestamp::from_unix_micros(created_at),
                display_name,
                import_source,
            ))
        })
        .collect()
    }

    /// Loads representations belonging to `asset_id` in stable identity order.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Storage`] for query failures or invalid stored data.
    pub fn representations(&self, asset_id: AssetId) -> Result<Vec<Representation>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT r.id, r.kind, r.file_size_bytes, r.modified_at_micros,
                        f.algorithm, f.algorithm_version, f.value
                 FROM representations r
                 LEFT JOIN fingerprints f ON f.representation_id = r.id
                 WHERE r.asset_id = ?1 ORDER BY r.id",
            )
            .map_err(sqlite_error("prepare representation query"))?;
        let rows = statement
            .query_map(params![asset_id.as_bytes().as_slice()], |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<u16>>(5)?,
                    row.get::<_, Option<Vec<u8>>>(6)?,
                ))
            })
            .map_err(sqlite_error("query representations"))?;

        rows.map(|row| {
            let (id, kind, size, modified_at, algorithm, version, value) =
                row.map_err(sqlite_error("read representation row"))?;
            let kind = decode_representation_kind(kind)?;
            let file_facts = decode_file_facts(size, modified_at)?;
            let fingerprint = decode_fingerprint(algorithm, version, value)?;
            Ok(Representation::new(
                RepresentationId::from_bytes(id_bytes(id, "representation")?),
                asset_id,
                kind,
                fingerprint,
                file_facts,
            ))
        })
        .collect()
    }

    /// Loads known locations for `representation_id` in stable identity order.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Storage`] for query failures or invalid stored data.
    pub fn locations(&self, representation_id: RepresentationId) -> Result<Vec<Location>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, uri, last_seen_micros, availability FROM locations
                 WHERE representation_id = ?1 ORDER BY id",
            )
            .map_err(sqlite_error("prepare location query"))?;
        let rows = statement
            .query_map(params![representation_id.as_bytes().as_slice()], |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(sqlite_error("query locations"))?;

        rows.map(|row| {
            let (id, uri, last_seen, availability) =
                row.map_err(sqlite_error("read location row"))?;
            Location::new(
                postproject_core::LocationId::from_bytes(id_bytes(id, "location")?),
                representation_id,
                uri,
                last_seen.map(Timestamp::from_unix_micros),
                decode_availability(availability)?,
            )
            .map_err(stored_domain_error("location"))
        })
        .collect()
    }

    /// Reports whether SQLite foreign-key enforcement is active on this connection.
    ///
    /// This is primarily useful for diagnostics and integration tests.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Storage`] if SQLite cannot query the setting.
    pub fn foreign_keys_enabled(&self) -> Result<bool> {
        self.connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, bool>(0))
            .map_err(sqlite_error("query foreign-key enforcement"))
    }
}

fn reserve_new_file(path: &Path) -> Result<()> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map(|_| ())
        .map_err(|error| {
            let kind = if error.kind() == std::io::ErrorKind::AlreadyExists {
                ErrorKind::AlreadyExists
            } else {
                ErrorKind::Io
            };
            Error::new(
                kind,
                format!("cannot create project file {}: {error}", path.display()),
            )
        })
}

fn open_connection(path: &Path) -> Result<Connection> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(sqlite_error("open project database"))?;
    configure_length_limit(&connection, MAX_SQLITE_VALUE_BYTES)?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(sqlite_error("configure SQLite busy timeout"))?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON; PRAGMA trusted_schema = OFF;")
        .map_err(sqlite_error("configure SQLite connection"))?;
    Ok(connection)
}

fn configure_length_limit(connection: &Connection, maximum: i32) -> Result<()> {
    connection
        .set_limit(Limit::SQLITE_LIMIT_LENGTH, maximum)
        .map(|_| ())
        .map_err(sqlite_error("configure SQLite value length limit"))
}

fn persist_new_project(connection: &mut Connection, project: &Project) -> Result<()> {
    let transaction = connection
        .transaction()
        .map_err(sqlite_error("begin project creation"))?;
    transaction
        .execute(
            "INSERT INTO projects (
                singleton, id, schema_version, created_at_micros, display_name
             ) VALUES (1, ?1, ?2, ?3, ?4)",
            params![
                project.id().as_bytes().as_slice(),
                project.schema_version(),
                project.created_at().as_unix_micros(),
                project.display_name(),
            ],
        )
        .map_err(sqlite_error("persist new project"))?;
    transaction
        .commit()
        .map_err(sqlite_error("commit project creation"))
}

fn load_project(connection: &Connection) -> Result<Project> {
    let stored = connection
        .query_row(
            "SELECT id, schema_version, created_at_micros, display_name
             FROM projects WHERE singleton = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(sqlite_error("load project record"))?
        .ok_or_else(|| Error::new(ErrorKind::Storage, "database has no project record"))?;

    let id = ProjectId::from_bytes(id_bytes(stored.0, "project")?);
    if stored.1 != CURRENT_SCHEMA_VERSION {
        return Err(Error::new(
            ErrorKind::Migration,
            format!(
                "project record schema version {} does not match database version {}",
                stored.1, CURRENT_SCHEMA_VERSION
            ),
        ));
    }

    let mut project = Project::new(
        id,
        stored.1,
        Timestamp::from_unix_micros(stored.2),
        stored.3,
    );
    project.set_media_roots(load_media_roots(connection)?);
    Ok(project)
}

fn load_media_roots(connection: &Connection) -> Result<Vec<MediaRoot>> {
    let mut statement = connection
        .prepare("SELECT id, uri, label, priority, enabled FROM media_roots ORDER BY priority, id")
        .map_err(sqlite_error("prepare media-root query"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, bool>(4)?,
            ))
        })
        .map_err(sqlite_error("query media roots"))?;
    rows.map(|row| {
        let (id, uri, label, priority, enabled) =
            row.map_err(sqlite_error("read media-root row"))?;
        MediaRoot::new(
            MediaRootId::from_bytes(id_bytes(id, "media root")?),
            uri,
            label,
            priority,
            enabled,
        )
        .map_err(stored_domain_error("media root"))
    })
    .collect()
}

fn decode_representation_kind(value: i64) -> Result<RepresentationKind> {
    match value {
        0 => Ok(RepresentationKind::Original),
        1 => Ok(RepresentationKind::Proxy),
        2 => Ok(RepresentationKind::Optimized),
        3 => Ok(RepresentationKind::Derived),
        _ => Err(Error::new(
            ErrorKind::Storage,
            format!("stored representation kind {value} is invalid"),
        )),
    }
}

fn decode_availability(value: i64) -> Result<LocationAvailability> {
    match value {
        0 => Ok(LocationAvailability::Unknown),
        1 => Ok(LocationAvailability::Online),
        2 => Ok(LocationAvailability::Offline),
        _ => Err(Error::new(
            ErrorKind::Storage,
            format!("stored location availability {value} is invalid"),
        )),
    }
}

fn decode_file_facts(size: Option<i64>, modified_at: Option<i64>) -> Result<Option<FileFacts>> {
    match (size, modified_at) {
        (None, None) => Ok(None),
        (Some(size), modified_at) => {
            let size = u64::try_from(size).map_err(|error| {
                Error::new(
                    ErrorKind::Storage,
                    format!("stored file size is invalid: {error}"),
                )
            })?;
            Ok(Some(FileFacts::new(
                size,
                modified_at.map(Timestamp::from_unix_micros),
            )))
        }
        (None, Some(_)) => Err(Error::new(
            ErrorKind::Storage,
            "stored modification time has no corresponding file size",
        )),
    }
}

fn decode_fingerprint(
    algorithm: Option<String>,
    version: Option<u16>,
    value: Option<Vec<u8>>,
) -> Result<Option<Fingerprint>> {
    match (algorithm, version, value) {
        (None, None, None) => Ok(None),
        (Some(algorithm), Some(version), Some(value)) => {
            Fingerprint::new(algorithm, version, value)
                .map(Some)
                .map_err(stored_domain_error("fingerprint"))
        }
        _ => Err(Error::new(
            ErrorKind::Storage,
            "stored fingerprint fields are incomplete",
        )),
    }
}

pub(crate) fn id_bytes(value: Vec<u8>, label: &str) -> Result<[u8; 16]> {
    value.try_into().map_err(|value: Vec<u8>| {
        Error::new(
            ErrorKind::Storage,
            format!("stored {label} ID has {} bytes; expected 16", value.len()),
        )
    })
}

pub(crate) fn sqlite_error(context: &'static str) -> impl FnOnce(rusqlite::Error) -> Error {
    move |error| Error::new(ErrorKind::Storage, format!("{context}: {error}"))
}

fn stored_domain_error(label: &'static str) -> impl FnOnce(Error) -> Error {
    move |error| {
        Error::new(
            ErrorKind::Storage,
            format!("stored {label} is invalid: {error}"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_value_limit_rejects_oversized_results() {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        configure_length_limit(&connection, 1_024).expect("configure test limit");

        let result =
            connection.query_row("SELECT zeroblob(1025)", [], |row| row.get::<_, Vec<u8>>(0));

        assert!(result.is_err(), "oversized value unexpectedly loaded");
    }
}
