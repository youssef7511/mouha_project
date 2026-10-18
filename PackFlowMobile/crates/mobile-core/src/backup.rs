use crate::{
    date,
    error::{DomainError, Result},
    hash,
    models::OrderDocument,
    pricing, rows, uuid, Engine, APP_ID, SCHEMA, SQL,
};
use rusqlite::{backup::Backup, Connection, OpenFlags};
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    path::Path,
    time::Duration,
};
const MAGIC: &[u8] = b"PACKFLOW-MOBILE-1\n";
pub const MAX_BACKUP: usize = 25 * 1024 * 1024;
fn check(c: &Connection) -> Result<()> {
    let valid: String = c.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    let app: i64 = c.query_row("PRAGMA application_id", [], |r| r.get(0))?;
    let ver: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if valid != "ok"
        || app != APP_ID
        || ver != SCHEMA
        || c.prepare("PRAGMA foreign_key_check")?
            .query([])?
            .next()?
            .is_some()
    {
        return Err(DomainError::storage(
            "This is not a valid compatible mobile backup.",
        ));
    }
    let expected = Connection::open_in_memory()?;
    expected.execute_batch(SQL)?;
    let schema="SELECT type,name,tbl_name,sql FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name";
    if rows(c, schema, [])? != rows(&expected, schema, [])? {
        return Err(DomainError::storage("Backup schema has changed."));
    }
    let checksum: String =
        c.query_row("SELECT value FROM meta WHERE key='schema_hash'", [], |r| {
            r.get(0)
        })?;
    if checksum != hash(SQL.as_bytes()) {
        return Err(DomainError::storage(
            "Backup schema checksum does not match.",
        ));
    }
    Ok(())
}
fn contents(c: &Connection) -> Result<()> {
    for row in rows(c, "SELECT * FROM orders", [])? {
        let doc: OrderDocument = serde_json::from_str(row["document"].as_str().unwrap())?;
        let q = pricing::calculate(&doc.input)?;
        if doc.input.client_id != row["client_id"].as_str().unwrap()
            || doc.input.name != row["name"].as_str().unwrap()
            || doc.stage_names.len() != doc.input.stages.len()
            || doc.stage_totals != q.stages
            || row["total_fils"].as_i64() != q.total
            || row["known_total_fils"].as_i64() != Some(q.known)
        {
            return Err(DomainError::storage(
                "Backup order quotes failed validation.",
            ));
        }
        for d in [&doc.input.due_date, &doc.input.payment_due_date]
            .into_iter()
            .flatten()
        {
            date(d)?;
        }
    }
    let bad:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM payments p LEFT JOIN payments o ON o.id=p.original_id WHERE p.kind='correction' AND (o.id IS NULL OR o.kind<>'receipt' OR p.order_id<>o.order_id OR p.amount_fils<>o.amount_fils OR p.business_date<o.business_date))",[],|r|r.get(0))?;
    if bad {
        return Err(DomainError::storage(
            "Backup payment corrections are inconsistent.",
        ));
    }
    for row in rows(c, "SELECT business_date FROM payments", [])? {
        date(row["business_date"].as_str().unwrap())?;
    }
    for key in ["workspace_id", "device_id"] {
        let value: String =
            c.query_row("SELECT value FROM meta WHERE key=?1", [key], |r| r.get(0))?;
        uuid(&value)?;
    }
    let seq: String = c.query_row("SELECT value FROM meta WHERE key='next_order'", [], |r| {
        r.get(0)
    })?;
    if seq.parse::<i64>().ok().is_none_or(|n| n < 1) {
        return Err(DomainError::storage("Backup numbering is invalid."));
    }
    Ok(())
}
pub fn encode(c: &Connection) -> Result<Vec<u8>> {
    let temp = tempfile::NamedTempFile::new()?;
    let mut target = Connection::open(temp.path())?;
    Backup::new(c, &mut target)?.run_to_completion(128, Duration::from_millis(5), None)?;
    check(&target)?;
    drop(target);
    let bytes = std::fs::read(temp.path())?;
    if bytes.len() > MAX_BACKUP {
        return Err(DomainError::storage(
            "This workspace exceeds the 25 MB mobile backup limit.",
        ));
    }
    let mut out = Vec::new();
    out.extend(MAGIC);
    out.extend(hash(&bytes).as_bytes());
    out.push(b'\n');
    out.extend(bytes);
    Ok(out)
}
fn save(bytes: &[u8], target: &Path) -> Result<()> {
    let parent = target
        .parent()
        .ok_or_else(|| DomainError::storage("Missing backup folder."))?;
    std::fs::create_dir_all(parent)?;
    let mut f = tempfile::NamedTempFile::new_in(parent)?;
    f.write_all(bytes)?;
    f.as_file().sync_all()?;
    f.persist_noclobber(target)
        .map_err(|_| DomainError::storage("Could not save the backup. Choose a new filename."))?;
    Ok(())
}
impl Engine {
    pub fn restore_encoded(&mut self, data: &str, confirmation: &str) -> Result<Value> {
        use base64::Engine as _;
        if data.len() > 36 * 1024 * 1024 {
            return Err(DomainError::validation(
                "backup",
                "Mobile backups are limited to 25 MB.",
            ));
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|_| DomainError::validation("backup", "Invalid backup encoding."))?;
        self.restore(&bytes, confirmation)
    }
    pub fn backup_bytes(&self) -> Result<Vec<u8>> {
        encode(&self.conn)
    }
    pub fn save_backup(&self) -> Result<Value> {
        let path = self
            .safety
            .join(format!("mobile-{}.pfmobile", uuid::Uuid::new_v4()));
        save(&self.backup_bytes()?, &path)?;
        Ok(json!({"path":path,"name":path.file_name().unwrap().to_string_lossy()}))
    }
    pub fn read_backup(&self, name: &str) -> Result<Vec<u8>> {
        if !name.ends_with(".pfmobile")
            || name.contains("..")
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
        {
            return Err(DomainError::validation(
                "backup",
                "Choose a saved mobile backup.",
            ));
        }
        let path = self.safety.join(name);
        if !path.symlink_metadata()?.file_type().is_file() {
            return Err(DomainError::validation(
                "backup",
                "Choose a regular backup file.",
            ));
        }
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take((MAX_BACKUP + MAGIC.len() + 66) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_BACKUP + MAGIC.len() + 65 {
            return Err(DomainError::storage("Backup is too large."));
        }
        Ok(bytes)
    }
    pub fn backups(&self) -> Result<Value> {
        let mut files = Vec::new();
        if self.safety.exists() {
            for item in std::fs::read_dir(&self.safety)? {
                let item = item?;
                if item.file_type()?.is_file()
                    && item.file_name().to_string_lossy().ends_with(".pfmobile")
                {
                    files.push(item.file_name().to_string_lossy().to_string());
                }
            }
        }
        files.sort();
        Ok(json!(files))
    }
    pub fn restore(&mut self, bytes: &[u8], confirmation: &str) -> Result<Value> {
        if confirmation != "RESTORE" {
            return Err(DomainError::validation(
                "confirmation",
                "Type RESTORE to replace this phone's records.",
            ));
        }
        let offset = MAGIC.len() + 65;
        if bytes.len() <= offset
            || bytes.len() > MAX_BACKUP + offset
            || !bytes.starts_with(MAGIC)
            || bytes[MAGIC.len() + 64] != b'\n'
            || hash(&bytes[offset..]).as_bytes() != &bytes[MAGIC.len()..MAGIC.len() + 64]
        {
            return Err(DomainError::storage(
                "This mobile backup is damaged or has an invalid checksum.",
            ));
        }
        let dir = tempfile::tempdir()?;
        let candidate = dir.path().join("candidate.sqlite");
        std::fs::write(&candidate, &bytes[offset..])?;
        let c = Connection::open_with_flags(candidate, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        check(&c)?;
        contents(&c)?;
        let safety = self
            .safety
            .join(format!("before-restore-{}.pfmobile", uuid::Uuid::new_v4()));
        save(&self.backup_bytes()?, &safety)?;
        Backup::new(&c, &mut self.conn)?.run_to_completion(128, Duration::from_millis(5), None)?;
        Ok(
            json!({"safety_backup":safety.file_name().unwrap().to_string_lossy(),"message":"Restored. A safety backup preserves the previous workspace."}),
        )
    }
}
