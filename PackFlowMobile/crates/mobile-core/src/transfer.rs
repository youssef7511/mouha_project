use crate::{
    error::{DomainError, Result},
    rows, Engine,
};
use chrono::{DateTime, FixedOffset, Utc};
use packflow_transfer::Package;
use serde_json::{json, Value};
impl Engine {
    pub fn transfer_bytes(&self, from: &str, to: &str) -> Result<Vec<u8>> {
        packflow_transfer::date(from).map_err(|e| DomainError::validation("from_date", &e))?;
        packflow_transfer::date(to).map_err(|e| DomainError::validation("to_date", &e))?;
        if from > to {
            return Err(DomainError::validation(
                "date_range",
                "Start date must precede end date.",
            ));
        }
        let tx = self.conn.unchecked_transaction()?;
        let changed = |s: &str| -> Result<bool> {
            let d = DateTime::parse_from_rfc3339(s)
                .map_err(|_| DomainError::storage("Invalid stored timestamp."))?
                .with_timezone(&FixedOffset::east_opt(10800).unwrap())
                .date_naive()
                .to_string();
            Ok(d.as_str() >= from && d.as_str() <= to)
        };
        let payments=rows(&tx,"SELECT *,CAST(amount_fils AS TEXT) AS amount_fils FROM payments ORDER BY created_at,id",[])?;
        let mut orders = vec![];
        for mut o in rows(&tx, "SELECT * FROM orders ORDER BY created_at,id", [])? {
            if changed(o["updated_at"].as_str().unwrap())?
                || payments
                    .iter()
                    .filter(|p| p["order_id"] == o["id"])
                    .any(|p| changed(p["created_at"].as_str().unwrap()).unwrap_or(false))
            {
                o["document"] = serde_json::from_str(o["document"].as_str().unwrap())?;
                orders.push(o);
            }
        }
        let clients = rows(&tx, "SELECT * FROM clients ORDER BY created_at,id", [])?
            .into_iter()
            .filter(|c| {
                changed(c["updated_at"].as_str().unwrap()).unwrap_or(false)
                    || orders.iter().any(|o| o["client_id"] == c["id"])
            })
            .collect();
        let payments = payments
            .into_iter()
            .filter(|p| orders.iter().any(|o| o["id"] == p["order_id"]))
            .collect();
        let revisions=rows(&tx,"SELECT order_id,version,document,reason,created_at,device_id FROM revisions ORDER BY order_id,version",[])?.into_iter().filter(|r|orders.iter().any(|o|o["id"]==r["order_id"])).collect();
        let meta = |key: &str| {
            tx.query_row("SELECT value FROM meta WHERE key=?1", [key], |r| {
                r.get::<_, String>(0)
            })
        };
        let package = Package {
            workspace_id: meta("workspace_id")?,
            device_id: meta("device_id")?,
            exported_at: Utc::now().to_rfc3339(),
            from_date: from.into(),
            to_date: to.into(),
            clients,
            orders,
            payments,
            revisions,
        };
        let bytes = package
            .encode()
            .map_err(|e| DomainError::validation("transfer", &e))?;
        tx.commit()?;
        Ok(bytes)
    }
    pub fn save_transfer(&self, from: &str, to: &str) -> Result<Value> {
        let bytes = self.transfer_bytes(from, to)?;
        let name = format!(
            "PackFlow-transfer-{from}-{to}-{}.xlsx",
            uuid::Uuid::new_v4()
        );
        std::fs::create_dir_all(&self.safety)?;
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new_in(&self.safety)?;
        f.write_all(&bytes)?;
        f.as_file().sync_all()?;
        f.persist_noclobber(self.safety.join(&name))
            .map_err(|_| DomainError::storage("Could not save the transfer workbook."))?;
        Ok(json!({"name":name}))
    }
}
