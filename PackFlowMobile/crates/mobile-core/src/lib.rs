mod backup;
pub mod error;
pub mod models;
pub mod money;
pub mod pricing;
use error::{DomainError, Result};
use models::*;
use rusqlite::{params, types::ValueRef, Connection, OptionalExtension, Params, Transaction};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
pub const APP_ID: i64 = 0x50464D31;
pub const SCHEMA: i64 = 1;
pub(crate) const SQL: &str = include_str!("schema.sql");
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn today() -> String {
    chrono::Utc::now()
        .with_timezone(&chrono::FixedOffset::east_opt(10800).unwrap())
        .date_naive()
        .to_string()
}
fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn text(v: &str, max: usize, required: bool) -> Result<String> {
    let s = v.trim();
    if s.chars().count() > max || (required && s.is_empty()) {
        return Err(DomainError::validation(
            "text",
            "Enter text within the allowed length.",
        ));
    }
    Ok(s.into())
}
fn date(v: &str) -> Result<()> {
    if chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d")
        .ok()
        .map(|d| d.to_string())
        .as_deref()
        != Some(v)
    {
        return Err(DomainError::validation(
            "date",
            "Choose a valid YYYY-MM-DD date.",
        ));
    }
    Ok(())
}
fn uuid(v: &str) -> Result<()> {
    uuid::Uuid::parse_str(v)
        .map_err(|_| DomainError::validation("id", "Invalid record identity."))?;
    Ok(())
}
fn version(actual: i64, expected: Option<i64>) -> Result<()> {
    if Some(actual) != expected {
        Err(DomainError::new(
            "CONFLICT",
            "This record changed. Reopen it before saving.",
        ))
    } else {
        Ok(())
    }
}
pub(crate) fn rows<P: Params>(conn: &Connection, sql: &str, p: P) -> Result<Vec<Value>> {
    let mut stmt = conn.prepare(sql)?;
    let names = stmt
        .column_names()
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let result = stmt
        .query_map(p, |r| {
            let mut obj = Map::new();
            for (i, key) in names.iter().enumerate() {
                let v = match r.get_ref(i)? {
                    ValueRef::Null => Value::Null,
                    ValueRef::Integer(n) => json!(n),
                    ValueRef::Text(s) => json!(String::from_utf8_lossy(s)),
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };
                obj.insert(key.clone(), v);
            }
            Ok(Value::Object(obj))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(result)
}
fn one<P: Params>(conn: &Connection, sql: &str, p: P) -> Result<Value> {
    rows(conn, sql, p)?
        .into_iter()
        .next()
        .ok_or_else(|| DomainError::new("NOT_FOUND", "Record no longer exists."))
}
pub struct Engine {
    pub(crate) conn: Connection,
    pub(crate) safety: PathBuf,
}
pub(crate) fn open_db(path: &Path) -> Result<Connection> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut c = Connection::open(path)?;
    c.busy_timeout(std::time::Duration::from_secs(5))?;
    c.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")?;
    let ver: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if ver == 0 {
        let tables: i64 = c.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
            [],
            |r| r.get(0),
        )?;
        if tables != 0 {
            return Err(DomainError::storage(
                "This file is not a PackFlow Mobile workspace.",
            ));
        }
        let tx = c.transaction()?;
        tx.execute_batch(SQL)?;
        let device = uid();
        for (k, v) in [
            ("schema_hash", hash(SQL.as_bytes())),
            ("workspace_id", uid()),
            ("device_id", device.clone()),
            ("name", "My workspace".into()),
            ("next_order", "1".into()),
        ] {
            tx.execute("INSERT INTO meta VALUES(?1,?2)", params![k, v])?;
        }
        for (i, name) in ["Lamination", "Impose", "Diecut", "Finish"]
            .iter()
            .enumerate()
        {
            tx.execute(
                "INSERT INTO templates VALUES(?1,?2,?3,'',?4,0,1)",
                params![uid(), name, name.to_lowercase(), i as i64],
            )?;
        }
        tx.pragma_update(None, "application_id", APP_ID)?;
        tx.pragma_update(None, "user_version", SCHEMA)?;
        tx.commit()?;
    } else if ver != SCHEMA {
        return Err(DomainError::storage(
            "This workspace needs a different PackFlow Mobile version.",
        ));
    }
    let app: i64 = c.query_row("PRAGMA application_id", [], |r| r.get(0))?;
    let checksum: String =
        c.query_row("SELECT value FROM meta WHERE key='schema_hash'", [], |r| {
            r.get(0)
        })?;
    if app != APP_ID || checksum != hash(SQL.as_bytes()) {
        return Err(DomainError::storage(
            "Workspace schema verification failed. No records were replaced.",
        ));
    }
    Ok(c)
}
impl Engine {
    pub fn open(path: &Path, safety: &Path) -> Result<Self> {
        Ok(Self {
            conn: open_db(path)?,
            safety: safety.to_owned(),
        })
    }
    pub fn query(&self, q: Query) -> Result<Value> {
        match q {
            Query::Snapshot => {
                let tx = self.conn.unchecked_transaction()?;
                let clients = rows(
                    &tx,
                    "SELECT * FROM clients ORDER BY archived,lower(name),id",
                    [],
                )?;
                let orders = orders(&tx, None)?;
                let payments=rows(&tx,"SELECT p.*,o.number,o.name AS order_name,c.name AS client_name,EXISTS(SELECT 1 FROM payments r WHERE r.original_id=p.id) AS corrected,CAST(CASE p.kind WHEN 'receipt' THEN p.amount_fils ELSE -p.amount_fils END AS TEXT) AS signed_fils,CAST(p.amount_fils AS TEXT) AS amount_fils FROM payments p JOIN orders o ON o.id=p.order_id JOIN clients c ON c.id=o.client_id ORDER BY p.business_date DESC,p.created_at DESC,p.id",[])?;
                let result = json!({"clients":clients,"orders":orders,"payments":payments,"templates":rows(&tx,"SELECT * FROM templates ORDER BY position",[])?,"profile":one(&tx,"SELECT (SELECT value FROM meta WHERE key='name') AS name,(SELECT value FROM meta WHERE key='device_id') AS device_id,(SELECT value FROM meta WHERE key='workspace_id') AS workspace_id",[])?,"today":today(),"currency":"KWD","schema":SCHEMA});
                tx.commit()?;
                Ok(result)
            }
            Query::Order { id } => {
                uuid(&id)?;
                orders(&self.conn, Some(&id))?
                    .into_iter()
                    .next()
                    .ok_or_else(|| DomainError::new("NOT_FOUND", "Order not found."))
            }
            Query::History { order_id } => {
                uuid(&order_id)?;
                Ok(
                    json!({"revisions":rows(&self.conn,"SELECT version,document,CAST(total_fils AS TEXT) AS total_fils,CAST(known_total_fils AS TEXT) AS known_total_fils,reason,created_at,device_id FROM revisions WHERE order_id=?1 ORDER BY version DESC",[&order_id])?,"audit":rows(&self.conn,"SELECT id,action,reason,device_id,created_at FROM audit WHERE entity_id=?1 ORDER BY created_at DESC",[order_id])?}),
                )
            }
            Query::Backups => self.backups(),
        }
    }
    pub fn mutate(&mut self, m: Mutation) -> Result<Value> {
        uuid(&m.request_id)?;
        let fingerprint = hash(serde_json::to_string(&m.action)?.as_bytes());
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        if let Some((old, result)) = tx
            .query_row(
                "SELECT hash,result FROM requests WHERE id=?1",
                [&m.request_id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?
        {
            if old != fingerprint {
                return Err(DomainError::new(
                    "REQUEST_COLLISION",
                    "This operation identity was already used for different data.",
                ));
            }
            return Ok(serde_json::from_str(&result)?);
        }
        let device: String =
            tx.query_row("SELECT value FROM meta WHERE key='device_id'", [], |r| {
                r.get(0)
            })?;
        let result = apply(&tx, &device, m.action)?;
        tx.execute(
            "INSERT INTO requests VALUES(?1,?2,?3)",
            params![m.request_id, fingerprint, result.to_string()],
        )?;
        tx.commit()?;
        Ok(result)
    }
}
fn orders(conn: &Connection, id: Option<&str>) -> Result<Vec<Value>> {
    let mut records=rows(conn,"SELECT o.*,c.name AS client_name,CAST(o.total_fils AS TEXT) AS total_fils,CAST(o.known_total_fils AS TEXT) AS known_total_fils,CAST(coalesce((SELECT sum(CASE p.kind WHEN 'receipt' THEN p.amount_fils ELSE -p.amount_fils END) FROM payments p WHERE p.order_id=o.id),0) AS TEXT) AS received_fils FROM orders o JOIN clients c ON c.id=o.client_id WHERE ?1 IS NULL OR o.id=?1 ORDER BY o.created_at DESC,o.number DESC",[id])?;
    for row in &mut records {
        let d: Value = serde_json::from_str(row["document"].as_str().unwrap())?;
        row["document"] = d;
        row["balance_fils"] = match row["total_fils"].as_str() {
            None => Value::Null,
            Some(t) => json!((t.parse::<i128>().unwrap()
                - row["received_fils"]
                    .as_str()
                    .unwrap()
                    .parse::<i128>()
                    .unwrap())
            .to_string()),
        };
    }
    Ok(records)
}
fn audit(
    tx: &Transaction,
    device: &str,
    id: &str,
    action: &str,
    before: Option<Value>,
    after: Option<Value>,
    reason: &str,
) -> Result<()> {
    tx.execute(
        "INSERT INTO audit VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            uid(),
            id,
            action,
            before.map(|v| v.to_string()),
            after.map(|v| v.to_string()),
            reason,
            device,
            now()
        ],
    )?;
    Ok(())
}
fn receipt(tx: &Transaction, device: &str, order: &str, r: ReceiptInput) -> Result<String> {
    uuid(order)?;
    date(&r.business_date)?;
    let amount = money::integer(&r.amount_fils, money::MAX_MONEY, "amount")?;
    if amount == 0 {
        return Err(DomainError::validation(
            "amount",
            "Enter a payment greater than zero.",
        ));
    }
    let archived: i64 = tx.query_row("SELECT archived FROM orders WHERE id=?1", [order], |r| {
        r.get(0)
    })?;
    if archived != 0 {
        return Err(DomainError::validation(
            "order",
            "Restore this archived order before recording a payment.",
        ));
    }
    let id = uid();
    let method = serde_json::to_value(r.method)?.as_str().unwrap().to_owned();
    let reference = text(&r.reference, 200, false)?;
    let notes = text(&r.notes, 1000, false)?;
    tx.execute(
        "INSERT INTO payments VALUES(?1,?2,'receipt',NULL,?3,?4,?5,?6,?7,'',?8,?9)",
        params![
            id,
            order,
            amount,
            r.business_date,
            method,
            reference,
            notes,
            now(),
            device
        ],
    )?;
    audit(
        tx,
        device,
        order,
        "receipt_recorded",
        None,
        Some(json!({"id":id,"amount_fils":amount.to_string()})),
        "",
    )?;
    Ok(id)
}
fn apply(tx: &Transaction, device: &str, a: Action) -> Result<Value> {
    match a {
        Action::SaveClient {
            id,
            expected_version,
            name,
            contact_name,
            phone,
            email,
            address,
            notes,
            archived,
        } => {
            let name = text(&name, 120, true)?;
            let contact = text(&contact_name, 120, false)?;
            let phone = text(&phone, 100, false)?;
            let email = text(&email, 200, false)?;
            let address = text(&address, 1000, false)?;
            let notes = text(&notes, 4000, false)?;
            let (id, v, before) = if let Some(id) = id {
                uuid(&id)?;
                let old = one(tx, "SELECT * FROM clients WHERE id=?1", [&id])?;
                version(old["version"].as_i64().unwrap(), expected_version)?;
                (id, old["version"].as_i64().unwrap() + 1, Some(old))
            } else {
                if expected_version.is_some() {
                    return Err(DomainError::validation(
                        "version",
                        "A new client cannot have an existing version.",
                    ));
                }
                (uid(), 1, None)
            };
            tx.execute("INSERT INTO clients VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10) ON CONFLICT(id) DO UPDATE SET name=excluded.name,contact_name=excluded.contact_name,phone=excluded.phone,email=excluded.email,address=excluded.address,notes=excluded.notes,archived=excluded.archived,version=excluded.version,updated_at=excluded.updated_at",params![id,name,contact,phone,email,address,notes,archived,v,now()])?;
            audit(
                tx,
                device,
                &id,
                "client_saved",
                before,
                Some(json!({"name":name,"version":v})),
                "",
            )?;
            Ok(json!({"id":id,"version":v}))
        }
        Action::SaveTemplate {
            id,
            expected_version,
            name,
            notes,
            archived,
        } => {
            let name = text(&name, 80, true)?;
            let notes = text(&notes, 1000, false)?;
            let (id, v, pos, before) = if let Some(id) = id {
                uuid(&id)?;
                let old = one(tx, "SELECT * FROM templates WHERE id=?1", [&id])?;
                version(old["version"].as_i64().unwrap(), expected_version)?;
                (
                    id,
                    old["version"].as_i64().unwrap() + 1,
                    old["position"].as_i64().unwrap(),
                    Some(old),
                )
            } else {
                if expected_version.is_some() {
                    return Err(DomainError::validation(
                        "version",
                        "Invalid new stage version.",
                    ));
                }
                let n: i64 = tx.query_row("SELECT count(*) FROM templates", [], |r| r.get(0))?;
                if n >= 100 {
                    return Err(DomainError::validation(
                        "stages",
                        "Keep at most 100 stage templates.",
                    ));
                }
                (
                    uid(),
                    1,
                    tx.query_row(
                        "SELECT coalesce(max(position),-1)+1 FROM templates",
                        [],
                        |r| r.get::<_, i64>(0),
                    )?,
                    None,
                )
            };
            let duplicate:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM templates WHERE id<>?1 AND normalized_name=?2 AND archived=0)",params![id,name.to_lowercase()],|r|r.get(0))?;
            if duplicate && !archived {
                return Err(DomainError::validation(
                    "name",
                    "An active stage already has this name.",
                ));
            }
            tx.execute("INSERT INTO templates VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET name=excluded.name,normalized_name=excluded.normalized_name,notes=excluded.notes,archived=excluded.archived,version=excluded.version",params![id,name,name.to_lowercase(),notes,pos,archived,v])?;
            audit(
                tx,
                device,
                &id,
                "template_saved",
                before,
                Some(json!({"name":name,"version":v})),
                "",
            )?;
            Ok(json!({"id":id,"version":v}))
        }
        Action::ReorderTemplates { ids, versions } => {
            let existing = rows(tx, "SELECT id,version FROM templates ORDER BY position", [])?;
            let unique: std::collections::HashSet<_> = ids.iter().collect();
            if ids.len() != existing.len()
                || ids.len() != versions.len()
                || unique.len() != ids.len()
            {
                return Err(DomainError::validation(
                    "stages",
                    "Reorder the complete current stage list.",
                ));
            }
            for (id, v) in ids.iter().zip(&versions) {
                let old = existing
                    .iter()
                    .find(|r| r["id"].as_str() == Some(id))
                    .ok_or_else(|| DomainError::new("CONFLICT", "Stage list changed."))?;
                version(old["version"].as_i64().unwrap(), Some(*v))?;
            }
            tx.execute("UPDATE templates SET position=-position-1", [])?;
            for (i, id) in ids.iter().enumerate() {
                tx.execute(
                    "UPDATE templates SET position=?1,version=version+1 WHERE id=?2",
                    params![i as i64, id],
                )?;
            }
            audit(
                tx,
                device,
                device,
                "templates_reordered",
                None,
                Some(json!(ids)),
                "",
            )?;
            Ok(json!({"saved":true}))
        }
        Action::SaveOrder {
            id,
            expected_version,
            mut input,
            reason,
            deposit,
        } => {
            input.name = text(&input.name, 120, true)?;
            input.notes = text(&input.notes, 4000, false)?;
            uuid(&input.client_id)?;
            for d in [&input.due_date, &input.payment_due_date]
                .into_iter()
                .flatten()
            {
                date(d)?;
            }
            let reason = text(&reason, 500, id.is_some())?;
            let (id, v, number, old) = if let Some(id) = id {
                uuid(&id)?;
                let old = orders(tx, Some(&id))?
                    .into_iter()
                    .next()
                    .ok_or_else(|| DomainError::new("NOT_FOUND", "Order not found."))?;
                version(old["version"].as_i64().unwrap(), expected_version)?;
                if old["archived"] != 0 {
                    return Err(DomainError::validation(
                        "order",
                        "Restore the archived order before editing.",
                    ));
                }
                if deposit.is_some() {
                    return Err(DomainError::validation(
                        "deposit",
                        "Record additional payments from the payments screen.",
                    ));
                }
                (
                    id,
                    old["version"].as_i64().unwrap() + 1,
                    old["number"].as_str().unwrap().to_owned(),
                    Some(old),
                )
            } else {
                if expected_version.is_some() {
                    return Err(DomainError::validation(
                        "version",
                        "Invalid new order version.",
                    ));
                }
                let seq: i64 = tx.query_row(
                    "SELECT CAST(value AS INTEGER) FROM meta WHERE key='next_order'",
                    [],
                    |r| r.get(0),
                )?;
                if seq < 1 || seq == i64::MAX {
                    return Err(DomainError::storage("Mobile numbering needs attention."));
                }
                tx.execute(
                    "UPDATE meta SET value=CAST(?1 AS TEXT) WHERE key='next_order'",
                    [seq + 1],
                )?;
                (
                    uid(),
                    1,
                    format!("MOB-{}-{seq:06}", device[..8].to_uppercase()),
                    None,
                )
            };
            let client_archived: i64 = tx.query_row(
                "SELECT archived FROM clients WHERE id=?1",
                [&input.client_id],
                |r| r.get(0),
            )?;
            if client_archived != 0
                && old
                    .as_ref()
                    .is_none_or(|r| r["client_id"].as_str() != Some(&input.client_id))
            {
                return Err(DomainError::validation(
                    "client",
                    "Choose an active client.",
                ));
            }
            if old
                .as_ref()
                .is_some_and(|r| r["client_id"].as_str() != Some(&input.client_id))
            {
                let receipts: i64 = tx.query_row(
                    "SELECT count(*) FROM payments WHERE order_id=?1",
                    [&id],
                    |r| r.get(0),
                )?;
                if receipts > 0 {
                    return Err(DomainError::validation(
                        "client",
                        "An order with payment history must keep its original client.",
                    ));
                }
            }
            let quote = pricing::calculate(&input)?;
            let mut names = Vec::new();
            for s in &input.stages {
                let previous = old.as_ref().and_then(|o| {
                    serde_json::from_value::<OrderDocument>(o["document"].clone()).ok()
                });
                if let Some(prev) = &previous {
                    if let Some(index) = prev
                        .input
                        .stages
                        .iter()
                        .position(|p| p.template_id == s.template_id)
                    {
                        names.push(prev.stage_names[index].clone());
                        continue;
                    }
                }
                let t = one(
                    tx,
                    "SELECT name,archived FROM templates WHERE id=?1",
                    [&s.template_id],
                )?;
                if t["archived"] != 0 {
                    return Err(DomainError::validation(
                        "stage",
                        "A newly selected stage must be active.",
                    ));
                }
                names.push(t["name"].as_str().unwrap().to_owned());
            }
            let status = serde_json::to_value(&input.status)?
                .as_str()
                .unwrap()
                .to_owned();
            let doc = OrderDocument {
                input,
                stage_names: names,
                stage_totals: quote.stages,
            };
            let document = serde_json::to_string(&doc)?;
            tx.execute("INSERT INTO orders VALUES(?1,?2,?3,?4,?5,?6,?7,?8,0,?9,?10,?10) ON CONFLICT(id) DO UPDATE SET client_id=excluded.client_id,name=excluded.name,status=excluded.status,document=excluded.document,total_fils=excluded.total_fils,known_total_fils=excluded.known_total_fils,version=excluded.version,updated_at=excluded.updated_at",params![id,number,doc.input.client_id,doc.input.name,status,document,quote.total,quote.known,v,now()])?;
            tx.execute(
                "INSERT INTO revisions VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    id,
                    v,
                    document,
                    quote.total,
                    quote.known,
                    reason,
                    now(),
                    device
                ],
            )?;
            if let Some(deposit) = deposit {
                receipt(tx, device, &id, deposit)?;
            }
            audit(
                tx,
                device,
                &id,
                "order_saved",
                old,
                Some(json!(doc)),
                &reason,
            )?;
            Ok(json!({"id":id,"version":v}))
        }
        Action::ArchiveOrder {
            id,
            expected_version,
            archived,
        } => {
            let old = one(tx, "SELECT version,archived FROM orders WHERE id=?1", [&id])?;
            version(old["version"].as_i64().unwrap(), Some(expected_version))?;
            tx.execute(
                "UPDATE orders SET archived=?1,version=version+1,updated_at=?2 WHERE id=?3",
                params![archived, now(), id],
            )?;
            audit(
                tx,
                device,
                &id,
                "order_archived",
                Some(old),
                Some(json!({"archived":archived})),
                "",
            )?;
            Ok(json!({"id":id}))
        }
        Action::Receive {
            order_id,
            receipt: r,
        } => Ok(json!({"id":receipt(tx,device,&order_id,r)?})),
        Action::CorrectReceipt {
            id,
            reason,
            business_date,
        } => {
            uuid(&id)?;
            let reason = text(&reason, 500, true)?;
            date(&business_date)?;
            let p=one(tx,"SELECT *,EXISTS(SELECT 1 FROM payments r WHERE r.original_id=payments.id) AS corrected FROM payments WHERE id=?1",[&id])?;
            if p["kind"] != "receipt" || p["corrected"] != 0 {
                return Err(DomainError::validation(
                    "payment",
                    "This receipt has already been corrected.",
                ));
            }
            if business_date.as_str() < p["business_date"].as_str().unwrap() {
                return Err(DomainError::validation(
                    "date",
                    "A correction cannot predate the receipt.",
                ));
            }
            let correction = uid();
            tx.execute(
                "INSERT INTO payments VALUES(?1,?2,'correction',?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    correction,
                    p["order_id"].as_str(),
                    id,
                    p["amount_fils"].as_i64(),
                    business_date,
                    p["method"].as_str(),
                    p["reference"].as_str(),
                    p["notes"].as_str(),
                    reason,
                    now(),
                    device
                ],
            )?;
            audit(
                tx,
                device,
                p["order_id"].as_str().unwrap(),
                "receipt_corrected",
                Some(p.clone()),
                Some(json!({"correction_id":correction})),
                &reason,
            )?;
            Ok(json!({"id":correction}))
        }
        Action::Profile { name } => {
            let name = text(&name, 100, true)?;
            tx.execute("UPDATE meta SET value=?1 WHERE key='name'", [&name])?;
            audit(
                tx,
                device,
                device,
                "profile_updated",
                None,
                Some(json!({"name":name})),
                "",
            )?;
            Ok(json!({"saved":true}))
        }
    }
}
mod transfer;
