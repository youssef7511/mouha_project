use crate::{
    error::{DomainError, Result},
    models::MANAGER_ID,
};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{path::Path, time::Duration};

pub const SCHEMA_VERSION: i64 = 8;
const SCHEMA: &str = include_str!("../migrations/001_foundation.sql");
const FINANCE_SCHEMA: &str = include_str!("../migrations/002_order_finance.sql");
const DEADLINE_SCHEMA: &str = include_str!("../migrations/003_payment_deadlines.sql");
const STAGE_QUOTE_SCHEMA: &str = include_str!("../migrations/004_stage_quotes.sql");
const SALARY_SCHEMA: &str = include_str!("../migrations/005_monthly_salaries.sql");
const SEPARATE_COST_SCHEMA: &str = include_str!("../migrations/006_separate_costs.sql");
const CORRECTION_SCHEMA: &str = include_str!("../migrations/007_corrections.sql");
const TRANSFER_SCHEMA: &str = include_str!("../migrations/008_phone_transfer.sql");
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn now() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.execute_batch(
        "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
    )?;
    let check: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    if check != "ok" {
        return Err(DomainError::storage(
            "Database integrity check failed. Use a verified backup; data was not replaced.",
        ));
    }
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(DomainError::new(
            "SCHEMA_TOO_NEW",
            "This database needs a newer PackFlow version.",
        ));
    }
    if version == 0 {
        let tables: i64 = conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
            [],
            |r| r.get(0),
        )?;
        if tables > 0 {
            return Err(DomainError::storage(
                "Unrecognized database schema. Existing data was preserved.",
            ));
        }
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, checksum TEXT NOT NULL, applied_at TEXT NOT NULL, app_version TEXT NOT NULL) STRICT;")?;
        tx.execute_batch(SCHEMA)?;
        let time = now();
        tx.execute(
            "INSERT INTO actors(id,display_name) VALUES(?1,'Manager')",
            [MANAGER_ID],
        )?;
        for (position, name) in ["Lamination", "Impose", "Diecut", "Finish"]
            .iter()
            .enumerate()
        {
            tx.execute("INSERT INTO stage_templates(id,name,normalized_name,position,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?5)", params![uuid::Uuid::new_v4().to_string(),name,name.to_lowercase(),position as i64,time])?;
        }
        tx.execute("INSERT INTO sequences VALUES('order',1)", [])?;
        for (key, value) in [
            ("currency", json!({"code":"KWD","exponent":3})),
            ("business_timezone", json!("Asia/Kuwait")),
            ("locale", json!("en")),
        ] {
            tx.execute(
                "INSERT INTO settings VALUES(?1,?2,?3)",
                params![key, value.to_string(), time],
            )?;
        }
        tx.execute(
            "INSERT INTO schema_migrations VALUES(1,?1,?2,'0.3.0')",
            params![hash(SCHEMA.as_bytes()), time],
        )?;
        tx.execute_batch("PRAGMA user_version=1;")?;
        tx.commit()?;
    } else {
        for (number, sql) in [
            (1, SCHEMA),
            (2, FINANCE_SCHEMA),
            (3, DEADLINE_SCHEMA),
            (4, STAGE_QUOTE_SCHEMA),
            (5, SALARY_SCHEMA),
            (6, SEPARATE_COST_SCHEMA),
            (7, CORRECTION_SCHEMA),
            (8, TRANSFER_SCHEMA),
        ] {
            if number > version {
                break;
            }
            let checksum: Option<String> = conn
                .query_row(
                    "SELECT checksum FROM schema_migrations WHERE version=?1",
                    [number],
                    |r| r.get(0),
                )
                .optional()?;
            if checksum.as_deref() != Some(hash(sql.as_bytes()).as_str()) {
                return Err(DomainError::storage(
                    "Migration checksum mismatch. Existing data was preserved.",
                ));
            }
        }
    }
    if version > 0 && version < SCHEMA_VERSION {
        let target = path
            .parent()
            .unwrap_or(Path::new("."))
            .join("backups")
            .join(format!(
                "pre-upgrade-{SCHEMA_VERSION}-{}.sqlite",
                uuid::Uuid::new_v4()
            ));
        snapshot(&conn, &target)?;
    }
    if version < 2 {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(FINANCE_SCHEMA)?;
        tx.execute(
            "INSERT INTO schema_migrations VALUES(2,?1,?2,'0.3.0')",
            params![hash(FINANCE_SCHEMA.as_bytes()), now()],
        )?;
        tx.execute_batch("PRAGMA user_version=2;")?;
        tx.commit()?;
    }
    if version < 3 {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(DEADLINE_SCHEMA)?;
        tx.execute(
            "INSERT INTO schema_migrations VALUES(3,?1,?2,'0.3.0')",
            params![hash(DEADLINE_SCHEMA.as_bytes()), now()],
        )?;
        tx.execute_batch("PRAGMA user_version=3;")?;
        tx.commit()?;
    }
    if version < 4 {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(STAGE_QUOTE_SCHEMA)?;
        tx.execute(
            "INSERT INTO schema_migrations VALUES(4,?1,?2,'0.3.0')",
            params![hash(STAGE_QUOTE_SCHEMA.as_bytes()), now()],
        )?;
        tx.execute_batch("PRAGMA user_version=4;")?;
        tx.commit()?;
    }
    if version < 5 {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(SALARY_SCHEMA)?;
        tx.execute(
            "INSERT INTO schema_migrations VALUES(5,?1,?2,'0.3.0')",
            params![hash(SALARY_SCHEMA.as_bytes()), now()],
        )?;
        tx.execute_batch("PRAGMA user_version=5;")?;
        tx.commit()?;
    }
    if version < 6 {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(SEPARATE_COST_SCHEMA)?;
        tx.execute(
            "INSERT INTO schema_migrations VALUES(6,?1,?2,'0.3.0')",
            params![hash(SEPARATE_COST_SCHEMA.as_bytes()), now()],
        )?;
        tx.execute_batch("PRAGMA user_version=6;")?;
        tx.commit()?;
    }
    if version < 7 {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(CORRECTION_SCHEMA)?;
        tx.execute(
            "INSERT INTO schema_migrations VALUES(7,?1,?2,'0.3.0')",
            params![hash(CORRECTION_SCHEMA.as_bytes()), now()],
        )?;
        tx.execute_batch("PRAGMA user_version=7;")?;
        tx.commit()?;
    }
    if version < 8 {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(TRANSFER_SCHEMA)?;
        tx.execute(
            "INSERT INTO schema_migrations VALUES(8,?1,?2,'0.4.0')",
            params![hash(TRANSFER_SCHEMA.as_bytes()), now()],
        )?;
        tx.execute_batch("PRAGMA user_version=8;")?;
        tx.commit()?;
    }
    let fk_error = conn
        .prepare("PRAGMA foreign_key_check")?
        .query([])?
        .next()?
        .is_some();
    if fk_error {
        return Err(DomainError::storage(
            "Database references failed integrity checks.",
        ));
    }
    Ok(conn)
}

pub fn snapshot(conn: &Connection, target: &Path) -> Result<()> {
    if target.exists() {
        return Err(DomainError::new(
            "FILE_ACCESS",
            "Backup destination already exists.",
        ));
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut backup_conn = Connection::open(target)?;
    let backup = rusqlite::backup::Backup::new(conn, &mut backup_conn)?;
    backup.run_to_completion(32, Duration::from_millis(10), None)?;
    drop(backup);
    let integrity: String = backup_conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    let invalid_fk = backup_conn
        .prepare("PRAGMA foreign_key_check")?
        .query([])?
        .next()?
        .is_some();
    if integrity != "ok" || invalid_fk {
        return Err(DomainError::storage("Backup validation failed."));
    }
    Ok(())
}
