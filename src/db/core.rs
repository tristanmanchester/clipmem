use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use rusqlite::{Connection, OpenFlags};

use super::schema::{legacy_prerelease_schema_detected, prepare_schema};
use super::sqlite_helpers::row_usize;
use super::store::revision::bump_revision;
use super::types::{
    ArchiveChangeKind, Database, StorageCheckpointReport, StorageCompactReport, StorageFileSizes,
};

impl Database {
    /// Open the archive database at `path`, creating parent directories and schema state as needed.
    ///
    /// This method also hardens parent-directory, database-file, and SQLite sidecar permissions
    /// after opening.
    ///
    /// # Errors
    ///
    /// Returns an error if the parent directory cannot be created, the database cannot be opened,
    /// or the connection cannot be configured and bootstrapped.
    pub fn open_or_init(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            Context::with_context(std::fs::create_dir_all(parent), || {
                format!("failed to create {}", parent.display())
            })?;
            harden_path_permissions(parent, 0o700)?;
        }

        let mut conn = open_connection(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        prepare_connection(&mut conn)?;
        harden_path_permissions(path, 0o600)?;
        harden_existing_sqlite_sidecar_permissions(path)?;

        Ok(Self {
            conn,
            path: path.to_path_buf(),
        })
    }

    /// Open an existing archive database without creating parent directories or schema state.
    ///
    /// This method also hardens parent-directory, database-file, and SQLite sidecar permissions
    /// after opening.
    ///
    /// # Errors
    ///
    /// Returns an error if the database file does not already exist, cannot be opened, or the
    /// connection cannot be configured.
    pub fn open_existing(path: &Path) -> Result<Self> {
        let mut conn = open_connection(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        prepare_connection(&mut conn)?;
        if let Some(parent) = path.parent() {
            harden_path_permissions(parent, 0o700)?;
        }
        harden_path_permissions(path, 0o600)?;
        harden_existing_sqlite_sidecar_permissions(path)?;

        Ok(Self {
            conn,
            path: path.to_path_buf(),
        })
    }

    pub(crate) fn ensure_supported_schema_shape(&self) -> Result<()> {
        if legacy_prerelease_schema_detected(&self.conn)?
            || self
                .conn
                .prepare("SELECT kind FROM item_representations LIMIT 0")
                .is_err_and(|error| error.to_string().contains("no such column: kind"))
        {
            bail!(
                "database operation failed; this may be an incompatible prerelease schema. Move the database aside and run `clipmem setup`."
            );
        }
        Ok(())
    }

    #[must_use]
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn compact_storage(&mut self, dry_run: bool) -> Result<StorageCompactReport> {
        let before = storage_file_sizes(&self.path)?;
        let total_before_bytes = before.total_bytes();

        let checkpoint = if dry_run {
            run_wal_checkpoint(&self.conn, "PASSIVE")?
        } else {
            let _ = run_wal_checkpoint(&self.conn, "TRUNCATE")?;
            self.conn
                .execute_batch("VACUUM")
                .context("vacuum database")?;
            self.conn
                .execute_batch("PRAGMA optimize")
                .context("optimize database")?;
            run_wal_checkpoint(&self.conn, "TRUNCATE")?
        };

        if !dry_run {
            bump_revision(&self.conn, &[ArchiveChangeKind::Storage])?;
        }

        let page_count = pragma_usize(&self.conn, "page_count")?;
        let freelist_count = pragma_usize(&self.conn, "freelist_count")?;
        let page_size = pragma_usize(&self.conn, "page_size")?;
        let after = storage_file_sizes(&self.path)?;
        let total_after_bytes = after.total_bytes();

        Ok(StorageCompactReport {
            db_path: self.path.display().to_string(),
            before,
            after,
            total_before_bytes,
            total_after_bytes,
            reclaimed_bytes: total_before_bytes.saturating_sub(total_after_bytes),
            estimated_reclaimable_bytes: (page_size as u64).saturating_mul(freelist_count as u64),
            page_count,
            freelist_count,
            checkpoint,
            dry_run,
            completed: !dry_run,
        })
    }

    #[cfg(test)]
    pub(crate) fn open_in_memory() -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        prepare_connection(&mut conn)?;
        Ok(Self {
            conn,
            path: PathBuf::from(":memory:"),
        })
    }
}

impl StorageFileSizes {
    #[must_use]
    pub(crate) const fn total_bytes(&self) -> u64 {
        self.db + self.wal + self.shm
    }
}

pub(in crate::db) fn storage_file_sizes(path: &Path) -> Result<StorageFileSizes> {
    Ok(StorageFileSizes {
        db: metadata_len(path)?,
        wal: metadata_len(&sidecar_path(path, "-wal"))?,
        shm: metadata_len(&sidecar_path(path, "-shm"))?,
    })
}

pub(in crate::db) fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}{suffix}", path.display()))
}

pub(in crate::db) fn harden_existing_sqlite_sidecar_permissions(path: &Path) -> Result<()> {
    for suffix in ["-wal", "-shm"] {
        let sidecar = sidecar_path(path, suffix);
        match std::fs::metadata(&sidecar) {
            Ok(_) => harden_path_permissions(&sidecar, 0o600)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("read metadata for {}", sidecar.display()));
            }
        }
    }
    Ok(())
}

pub(in crate::db) fn metadata_len(path: &Path) -> Result<u64> {
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error).with_context(|| format!("read size of {}", path.display())),
    }
}

pub(in crate::db) fn pragma_usize(conn: &Connection, pragma: &str) -> Result<usize> {
    let sql = format!("PRAGMA {pragma}");
    conn.query_row(&sql, [], |row| row_usize(row, 0))
        .with_context(|| format!("read PRAGMA {pragma}"))
}

pub(in crate::db) fn run_wal_checkpoint(
    conn: &Connection,
    mode: &str,
) -> Result<StorageCheckpointReport> {
    let sql = format!("PRAGMA wal_checkpoint({mode})");
    conn.query_row(&sql, [], |row| {
        Ok(StorageCheckpointReport {
            busy: row.get(0)?,
            log: row.get(1)?,
            checkpointed: row.get(2)?,
        })
    })
    .with_context(|| format!("run WAL checkpoint {mode}"))
}

pub(in crate::db) fn clamp_result_limit(limit: usize) -> usize {
    limit.clamp(1, 250)
}

pub(in crate::db) fn open_connection(path: &Path, flags: OpenFlags) -> Result<Connection> {
    anyhow::Context::with_context(Connection::open_with_flags(path, flags), || {
        format!("failed to open {}", path.display())
    })
}

pub(in crate::db) fn prepare_connection(conn: &mut Connection) -> Result<()> {
    Context::context(configure_connection(conn), "configure database connection")?;
    Context::context(prepare_schema(conn), "prepare database schema")?;
    Ok(())
}

#[cfg(unix)]
pub(in crate::db) fn harden_path_permissions(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    anyhow::Context::with_context(
        std::fs::set_permissions(path, PermissionsExt::from_mode(mode)),
        || format!("failed to set permissions on {}", path.display()),
    )
}

#[cfg(not(unix))]
pub(in crate::db) fn harden_path_permissions(_path: &Path, _mode: u32) -> Result<()> {
    Ok(())
}

pub(in crate::db) fn configure_connection(conn: &Connection) -> Result<()> {
    configure_pragma(conn, "journal_mode", "WAL")?;
    configure_pragma(conn, "synchronous", "NORMAL")?;
    configure_pragma(conn, "foreign_keys", "ON")?;
    configure_pragma(conn, "temp_store", "MEMORY")?;
    conn.busy_timeout(Duration::from_millis(1_500))
        .context("configure SQLite busy timeout")?;
    Ok(())
}

pub(in crate::db) fn configure_pragma(conn: &Connection, pragma: &str, value: &str) -> Result<()> {
    Context::with_context(conn.pragma_update(None, pragma, value), || {
        format!("configure {pragma} pragma")
    })?;
    Ok(())
}
