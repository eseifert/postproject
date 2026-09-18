//! SQLite project-file persistence for libpostproject.
//!
//! This crate translates between domain values and a private, migrated SQLite
//! schema. SQLite types and errors are never part of the core API contract.

#![forbid(unsafe_code)]

mod migrations;

use std::{
    fs::OpenOptions,
    path::{Path, PathBuf},
    time::Duration,
};

use postproject_core::{Error, ErrorKind, Project, ProjectId, Result, Timestamp};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

pub use migrations::CURRENT_SCHEMA_VERSION;

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
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(sqlite_error("configure SQLite busy timeout"))?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON; PRAGMA trusted_schema = OFF;")
        .map_err(sqlite_error("configure SQLite connection"))?;
    Ok(connection)
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

    Ok(Project::new(
        id,
        stored.1,
        Timestamp::from_unix_micros(stored.2),
        stored.3,
    ))
}

fn id_bytes(value: Vec<u8>, label: &str) -> Result<[u8; 16]> {
    value.try_into().map_err(|value: Vec<u8>| {
        Error::new(
            ErrorKind::Storage,
            format!("stored {label} ID has {} bytes; expected 16", value.len()),
        )
    })
}

fn sqlite_error(context: &'static str) -> impl FnOnce(rusqlite::Error) -> Error {
    move |error| Error::new(ErrorKind::Storage, format!("{context}: {error}"))
}
