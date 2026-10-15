//! Portable checksummed snapshots. Restores use SQLite's atomic backup transaction,
//! not a filesystem swap of a live WAL database. Account credentials are separate.
use crate::{
    db,
    error::{DomainError, Result},
    service::Engine,
};
use rusqlite::{backup::Backup, Connection, OpenFlags};
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    time::Duration,
};

const MAGIC: &[u8] = b"PACKFLOW-BACKUP-1\n";
const MAX_SIZE: u64 = 512 * 1024 * 1024;

fn check(conn: &Connection) -> Result<()> {
    let valid: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if valid != "ok"
        || conn
            .prepare("PRAGMA foreign_key_check")?
            .query([])?
            .next()?
            .is_some()
    {
        return Err(DomainError::storage("Backup integrity checks failed."));
    }
    Ok(())
}

pub fn export(conn: &Connection, target: &Path) -> Result<()> {
    let dir = tempfile::tempdir()?;
    let snapshot = dir.path().join("snapshot.sqlite");
    db::snapshot(conn, &snapshot)?;
    let bytes = fs::read(snapshot)?;
    if bytes.len() as u64 + MAGIC.len() as u64 + 65 > MAX_SIZE {
        return Err(DomainError::storage(
            "Workspace exceeds the portable backup size limit. No backup file was saved.",
        ));
    }
    let parent = target
        .parent()
        .ok_or_else(|| DomainError::storage("Choose a backup folder."))?;
    let mut out = tempfile::NamedTempFile::new_in(parent)?;
    out.write_all(MAGIC)?;
    out.write_all(db::hash(&bytes).as_bytes())?;
    out.write_all(b"\n")?;
    out.write_all(&bytes)?;
    out.as_file().sync_all()?;
    out.persist_noclobber(target).map_err(|_| {
        DomainError::storage("Backup was not saved. Choose a new filename in a writable folder.")
    })?;
    Ok(())
}

impl Engine {
    pub fn backup(&self, target: &Path) -> Result<Value> {
        export(&self.conn, target)?;
        Ok(json!({"path":target,"message":"Verified backup saved."}))
    }

    pub fn restore(&mut self, source: &Path) -> Result<Value> {
        let mut bytes = Vec::new();
        fs::File::open(source)?
            .take(MAX_SIZE + 1)
            .read_to_end(&mut bytes)?;
        let offset = MAGIC.len() + 65;
        if bytes.len() as u64 > MAX_SIZE
            || bytes.len() <= offset
            || !bytes.starts_with(MAGIC)
            || bytes[MAGIC.len() + 64] != b'\n'
            || db::hash(&bytes[offset..]).as_bytes() != &bytes[MAGIC.len()..MAGIC.len() + 64]
        {
            return Err(DomainError::storage(
                "This is not a valid PackFlow backup, or its checksum has changed.",
            ));
        }
        let staging = tempfile::tempdir()?;
        let candidate = staging.path().join("candidate.sqlite");
        fs::write(&candidate, &bytes[offset..])?;
        let readonly = Connection::open_with_flags(&candidate, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        check(&readonly)?;
        let version: i64 = readonly.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if !(1..=db::SCHEMA_VERSION).contains(&version) {
            return Err(DomainError::storage(
                "Backup requires a different PackFlow version.",
            ));
        }
        drop(readonly);
        // Validate migration checksums and upgrade only the disposable copy.
        let candidate = db::open(&candidate)?;
        check(&candidate)?;
        let root = Path::new(
            self.conn
                .path()
                .ok_or_else(|| DomainError::storage("Missing workspace path."))?,
        )
        .parent()
        .ok_or_else(|| DomainError::storage("Missing workspace folder."))?
        .join("backups");
        fs::create_dir_all(&root)?;
        let safety = root.join(format!("before-restore-{}.pfbackup", uuid::Uuid::new_v4()));
        export(&self.conn, &safety)?;
        // The worker serializes all writes. SQLite commits the entire destination
        // transaction on completion; an interrupted copy rolls back during recovery.
        let backup = Backup::new(&candidate, &mut self.conn)?;
        backup.run_to_completion(128, Duration::from_millis(10), None)?;
        drop(backup);
        Ok(
            json!({"safety_backup":safety,"message":"Workspace restored. A safety backup preserves the previous workspace."}),
        )
    }
}
