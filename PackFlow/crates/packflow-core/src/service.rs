use crate::{
    db,
    error::{DomainError, Result},
    models::*,
    money::{bounded_total, integer, line_total, MAX_MONEY},
};
use chrono::{FixedOffset, NaiveDate, Utc};
use rusqlite::{
    params, types::ValueRef, Connection, OptionalExtension, Params, Transaction,
    TransactionBehavior,
};
use serde_json::{json, Map, Value};
use std::{collections::HashSet, path::Path};

pub struct Engine {
    pub(crate) conn: Connection,
}
pub fn business_today() -> String {
    Utc::now()
        .with_timezone(&FixedOffset::east_opt(3 * 3600).unwrap())
        .date_naive()
        .to_string()
}
fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}
pub(crate) fn date(value: &str, field: &str) -> Result<()> {
    let parsed = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| DomainError::validation(field, "Enter a valid YYYY-MM-DD date."))?;
    if parsed.to_string() != value {
        return Err(DomainError::validation(
            field,
            "Enter a valid YYYY-MM-DD date.",
        ));
    }
    Ok(())
}
pub(crate) fn text(value: &str, max: usize, field: &str, required: bool) -> Result<String> {
    let value = value.trim();
    if value.chars().count() > max || (required && value.is_empty()) {
        return Err(DomainError::validation(
            field,
            "Text is missing or exceeds the allowed length.",
        ));
    }
    Ok(value.to_string())
}
pub(crate) fn id(value: &str) -> Result<()> {
    uuid::Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| DomainError::validation("id", "Invalid record identifier."))
}

pub(crate) fn rows<P: Params>(conn: &Connection, sql: &str, parameters: P) -> Result<Vec<Value>> {
    let mut statement = conn.prepare(sql)?;
    let names: Vec<String> = statement
        .column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let output = statement
        .query_map(parameters, |row| {
            let mut object = Map::new();
            for (i, name) in names.iter().enumerate() {
                let value = match row.get_ref(i)? {
                    ValueRef::Null => Value::Null,
                    ValueRef::Integer(n) => json!(n),
                    ValueRef::Real(_) => return Err(rusqlite::Error::InvalidQuery),
                    ValueRef::Text(s) => json!(String::from_utf8_lossy(s)),
                    ValueRef::Blob(_) => return Err(rusqlite::Error::InvalidQuery),
                };
                object.insert(name.clone(), value);
            }
            Ok(Value::Object(object))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(output)
}
fn one<P: Params>(conn: &Connection, sql: &str, params: P) -> Result<Value> {
    rows(conn, sql, params)?
        .into_iter()
        .next()
        .ok_or_else(|| DomainError::new("NOT_FOUND", "Record not found."))
}

impl Engine {
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            conn: db::open(path)?,
        })
    }
    pub fn query(&self, query: Query) -> Result<Value> {
        match query {
            Query::Bootstrap=>Ok(json!({
                "actors":rows(&self.conn,"SELECT * FROM actors WHERE active=1",[])?,
                "clients":rows(&self.conn,"SELECT c.*,(SELECT count(*) FROM orders WHERE client_id=c.id) AS order_count FROM clients c ORDER BY archived,normalized_name LIMIT 200",[])?,
                "workers":rows(&self.conn,"SELECT * FROM workers ORDER BY archived,name LIMIT 200",[])?,
                "templates":rows(&self.conn,"SELECT * FROM stage_templates ORDER BY position",[])?,
                "settings":rows(&self.conn,"SELECT * FROM settings",[])?,
                "business_today":business_today(), "schema_version":db::SCHEMA_VERSION,
                "stats":one(&self.conn,"SELECT (SELECT count(*) FROM orders WHERE lifecycle='open' AND archived=0) AS active_orders,
                    (SELECT coalesce(sum(i.ordered_boxes),0) FROM order_items i JOIN orders o ON o.id=i.order_id WHERE o.lifecycle='open' AND o.archived=0) AS ordered_boxes,
                    (SELECT count(*) FROM order_stages s JOIN orders o ON o.id=s.order_id WHERE s.retired=0 AND s.state='completed' AND o.archived=0 AND o.lifecycle<>'cancelled') AS completed_stages",[])?,
            })),
            Query::Diagnostics=>Ok(json!({"schema_version":db::SCHEMA_VERSION,"sqlite_version":self.conn.query_row("SELECT sqlite_version()",[],|r|r.get::<_,String>(0))?,"foreign_keys":self.conn.query_row("PRAGMA foreign_keys",[],|r|r.get::<_,i64>(0))?,"journal_mode":self.conn.query_row("PRAGMA journal_mode",[],|r|r.get::<_,String>(0))?})),
            Query::Reports{from_date,to_date,include_portfolio}=>crate::finance::reports(&self.conn,&from_date,&to_date,include_portfolio.unwrap_or(true)),
            Query::OrderPage{search,page,page_size,scope}=>crate::order_lists::page(&self.conn,&search,page,page_size,&scope),
            Query::ClientInsights=>crate::finance::client_insights(&self.conn, &business_today()),
            Query::Salaries{month}=>crate::salaries::query(&self.conn,&month),
            Query::SeparateCosts=>crate::finance::separate_costs(&self.conn),
            Query::ListOrders=>Ok(json!(rows(&self.conn,"SELECT o.*,c.name AS client_name,CAST(q.derived_total_fils AS TEXT) AS quote_fils,q.pricing_mode,
                CAST(f.service_charge_fils AS TEXT) AS service_charge_fils,CAST(f.total_due_fils AS TEXT) AS total_due_fils,
                CAST(f.paid_fils AS TEXT) AS paid_fils,CAST(f.balance_fils AS TEXT) AS balance_fils,
                (SELECT coalesce(sum(ordered_boxes),0) FROM order_items WHERE order_id=o.id) AS ordered_boxes,
                (SELECT count(*) FROM order_stages WHERE order_id=o.id AND retired=0) AS stage_count,
                (SELECT count(*) FROM order_stages WHERE order_id=o.id AND retired=0 AND state='completed') AS completed_stages
                FROM orders o JOIN clients c ON c.id=o.client_id JOIN order_financials f ON f.order_id=o.id LEFT JOIN quote_revisions q ON q.id=o.current_quote_revision_id
                ORDER BY o.created_at DESC,o.number DESC LIMIT 200",[]) ?)),
            Query::Order{id:order_id}=>self.order(&order_id),
        }
    }
    fn order(&self, order_id: &str) -> Result<Value> {
        id(order_id)?;
        let order=one(&self.conn,"SELECT o.*,c.name AS client_name,q.pricing_mode,CAST(q.derived_total_fils AS TEXT) AS quote_fils,
            CAST(f.service_charge_fils AS TEXT) AS service_charge_fils,CAST(f.total_due_fils AS TEXT) AS total_due_fils,
            CAST(f.paid_fils AS TEXT) AS paid_fils,CAST(f.balance_fils AS TEXT) AS balance_fils
            FROM orders o JOIN clients c ON c.id=o.client_id JOIN order_financials f ON f.order_id=o.id LEFT JOIN quote_revisions q ON q.id=o.current_quote_revision_id WHERE o.id=?1",[order_id])?;
        let items=rows(&self.conn,"SELECT i.*,CAST(qi.unit_price_fils AS TEXT) AS unit_price_fils FROM order_items i LEFT JOIN quote_items qi ON qi.item_id=i.id AND qi.quote_revision_id=?2 WHERE i.order_id=?1 ORDER BY i.position",params![order_id,order["current_quote_revision_id"].as_str()])?;
        let stages=rows(&self.conn,"SELECT s.*,CAST((SELECT coalesce(sum(CASE e.kind WHEN 'expense' THEN e.amount_fils ELSE -e.amount_fils END),0) FROM expense_entries e WHERE e.stage_id=s.id) AS TEXT) AS cost_fils FROM order_stages s WHERE s.order_id=?1 ORDER BY s.position",[order_id])?;
        let costs=rows(&self.conn,"SELECT id,stage_id,item_id,kind,original_id,reason,EXISTS(SELECT 1 FROM expense_entries r WHERE r.original_id=expense_entries.id) AS reversed,EXISTS(SELECT 1 FROM outside_services s WHERE s.expense_id=expense_entries.id) AS service_linked,category,description,supplier_text,reference,pricing_basis,CAST(quantity_scaled AS TEXT) AS quantity_scaled,CAST(unit_cost_fils AS TEXT) AS unit_cost_fils,CAST(amount_fils AS TEXT) AS amount_fils,business_date FROM expense_entries WHERE order_id=?1 ORDER BY created_at DESC",[order_id])?;
        let work=rows(&self.conn,"SELECT id,stage_id,item_id,kind,original_id,reason,EXISTS(SELECT 1 FROM work_events r WHERE r.original_id=work_events.id) AS reversed,description,dimensions_text,CAST(input_scaled AS TEXT) AS input_scaled,CAST(output_scaled AS TEXT) AS output_scaled,CAST(waste_scaled AS TEXT) AS waste_scaled,worker_id,worker_minutes,machine_description,machine_minutes,business_date FROM work_events WHERE order_id=?1 ORDER BY created_at DESC",[order_id])?;
        let cost:i64=self.conn.query_row("SELECT coalesce(sum(CASE kind WHEN 'expense' THEN amount_fils ELSE -amount_fils END),0) FROM expense_entries WHERE order_id=?1",[order_id],|r|r.get(0))?;
        let history=rows(&self.conn,"SELECT a.id,a.action,a.entity_type,a.entity_id,a.business_date,a.created_at,a.reason,actors.display_name AS actor_name FROM audit_events a JOIN actors ON actors.id=a.actor_id WHERE a.entity_id=?1 OR a.entity_id IN (SELECT id FROM order_stages WHERE order_id=?1) ORDER BY a.created_at DESC,a.rowid DESC LIMIT 100",[order_id])?;
        let services = rows(&self.conn,"SELECT s.id,(SELECT reason FROM service_voids v WHERE v.original_id=s.id) AS reason,EXISTS(SELECT 1 FROM service_voids v WHERE v.original_id=s.id) AS reversed,s.description,CAST(s.charge_fils AS TEXT) AS charge_fils,CAST(e.amount_fils AS TEXT) AS supplier_cost_fils,e.supplier_text,e.reference,s.business_date FROM outside_services s JOIN expense_entries e ON e.id=s.expense_id WHERE s.order_id=?1 ORDER BY s.created_at DESC",[order_id])?;
        let payments = rows(&self.conn,"SELECT id,kind,reason,EXISTS(SELECT 1 FROM refund_allocations a WHERE a.receipt_event_id=payment_events.id) AS reversed,CAST(amount_fils AS TEXT) AS amount_fils,business_date,method,reference,notes FROM payment_events WHERE order_id=?1 ORDER BY business_date DESC,created_at DESC",[order_id])?;
        let stage_prices = rows(&self.conn,"SELECT p.stage_id,s.template_id,s.name_snapshot,p.pricing_mode,CAST(p.fixed_total_fils AS TEXT) AS fixed_total_fils,CAST(p.quantity_scaled AS TEXT) AS quantity_scaled,CAST(p.unit_price_fils AS TEXT) AS unit_price_fils,CAST(p.derived_total_fils AS TEXT) AS total_fils FROM quote_stage_prices p JOIN order_stages s ON s.id=p.stage_id WHERE p.order_id=?1 AND p.quote_revision_id=?2 ORDER BY s.position",params![order_id,order["current_quote_revision_id"].as_str()])?;
        let stage_item_prices = rows(&self.conn,"SELECT p.stage_id,p.item_id,CAST(p.unit_price_fils AS TEXT) AS unit_price_fils FROM quote_stage_item_prices p JOIN order_items i ON i.id=p.item_id WHERE p.order_id=?1 AND p.quote_revision_id=?2 ORDER BY i.position",params![order_id,order["current_quote_revision_id"].as_str()])?;
        let mobile_source = rows(&self.conn,"SELECT workspace_id,source_id,source_version,json_extract(source_json,'$.number') AS number FROM mobile_links WHERE entity='order' AND target_id=?1",[order_id])?.into_iter().next();
        Ok(
            json!({"order":order,"items":items,"stages":stages,"costs":costs,"work":work,"cost_fils":cost.to_string(),"history":history,"services":services,"payments":payments,"stage_prices":stage_prices,"stage_item_prices":stage_item_prices,"mobile_source":mobile_source}),
        )
    }
    pub fn mutate(&mut self, request: Mutation) -> Result<Value> {
        id(&request.request_id)?;
        id(&request.actor_id)?;
        let serialized = serde_json::to_vec(&request)?;
        let payload_hash = db::hash(&serialized);
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<(String, String)> = tx
            .query_row(
                "SELECT payload_hash,result_json FROM mutation_requests WHERE request_id=?1",
                [&request.request_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((hash, result)) = existing {
            if hash != payload_hash {
                return Err(DomainError::new(
                    "CONFLICT",
                    "Request ID was already used for different data.",
                ));
            }
            return Ok(serde_json::from_str(&result)?);
        }
        let active: bool = tx.query_row(
            "SELECT active FROM actors WHERE id=?1",
            [&request.actor_id],
            |r| r.get(0),
        )?;
        if !active {
            return Err(DomainError::validation(
                "actor_id",
                "Choose an active actor.",
            ));
        }
        let time = db::now();
        let (data, changed, entity_type, entity_id, action_name, business_date, reason) =
            apply(&tx, &request.action, &request.actor_id, &time)?;
        tx.execute("INSERT INTO audit_events(id,actor_id,request_id,entity_type,entity_id,action,business_date,created_at,reason,after_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![uid(),request.actor_id,request.request_id,entity_type,entity_id,action_name,business_date,time,reason,serde_json::to_string(&request.action)?])?;
        let result = json!({"data":data,"changed_entities":changed,"committed_at":time});
        tx.execute(
            "INSERT INTO mutation_requests VALUES(?1,?2,?3,?4,?5)",
            params![
                request.request_id,
                action_name,
                payload_hash,
                result.to_string(),
                time
            ],
        )?;
        tx.commit()?;
        Ok(result)
    }
}

pub(crate) type Applied = (
    Value,
    Vec<&'static str>,
    &'static str,
    String,
    &'static str,
    String,
    Option<String>,
);
pub(crate) fn version(
    tx: &Transaction,
    table: &str,
    record_id: &str,
    expected: Option<i64>,
) -> Result<()> {
    id(record_id)?;
    // table is exclusively a hardcoded domain call-site constant, never IPC input.
    let current: i64 = tx.query_row(
        &format!("SELECT version FROM {table} WHERE id=?1"),
        [record_id],
        |r| r.get(0),
    )?;
    if expected != Some(current) {
        return Err(DomainError::new(
            "CONFLICT",
            "This record changed. Reload before saving.",
        ));
    }
    Ok(())
}
pub(crate) fn open_order(tx: &Transaction, order_id: &str) -> Result<()> {
    let lifecycle: String = tx.query_row(
        "SELECT lifecycle FROM orders WHERE id=?1 AND archived=0",
        [order_id],
        |r| r.get(0),
    )?;
    if lifecycle != "open" {
        return Err(DomainError::new(
            "INVARIANT",
            "Reopen this order before adding work.",
        ));
    }
    Ok(())
}
fn item_belongs(tx: &Transaction, order_id: &str, item_id: Option<&str>) -> Result<()> {
    if let Some(item) = item_id {
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM order_items WHERE order_id=?1 AND id=?2)",
            params![order_id, item],
            |r| r.get(0),
        )?;
        if !exists {
            return Err(DomainError::validation(
                "item_id",
                "Box item does not belong to this order.",
            ));
        }
    }
    Ok(())
}

fn apply(tx: &Transaction, action: &Action, actor: &str, time: &str) -> Result<Applied> {
    let today = business_today();
    match action {
        Action::ReverseEntry { .. } => crate::corrections::apply(tx, action, actor, time),
        Action::SetSalaryDivisor { .. }
        | Action::SaveSalary { .. }
        | Action::ArchiveSalary { .. } => crate::salaries::apply(tx, action, time),
        Action::AddOutsideService { .. }
        | Action::AddSeparateCost { .. }
        | Action::ReceivePayment { .. }
        | Action::SetPaymentDueDate { .. } => crate::finance::apply(tx, action, actor, time),
        Action::SaveClient {
            id: record_id,
            expected_version,
            name,
            contact_name,
            email,
            phone,
            address,
            archived,
        } => {
            let name = text(name, 120, "name", true)?;
            let contact = text(contact_name, 120, "contact_name", false)?;
            let email = text(email, 254, "email", false)?;
            let phone = text(phone, 60, "phone", false)?;
            let address = text(address, 1000, "address", false)?;
            let new_id = record_id.clone().unwrap_or_else(uid);
            if let Some(existing) = record_id {
                version(tx, "clients", existing, *expected_version)?;
                tx.execute("UPDATE clients SET name=?2,normalized_name=?3,contact_name=?4,email=?5,phone=?6,address=?7,archived=?8,version=version+1,updated_at=?9 WHERE id=?1",params![existing,name,name.to_lowercase(),contact,email,phone,address,archived,time])?;
            } else {
                tx.execute("INSERT INTO clients(id,name,normalized_name,contact_name,email,phone,address,archived,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?9)",params![new_id,name,name.to_lowercase(),contact,email,phone,address,archived,time])?;
            }
            Ok((
                json!({"id":new_id}),
                vec!["clients"],
                "client",
                new_id,
                "client_saved",
                today,
                None,
            ))
        }
        Action::SaveWorker {
            id: record_id,
            expected_version,
            name,
            role,
            active,
        } => {
            let name = text(name, 120, "name", true)?;
            let role = text(role, 80, "role", false)?;
            let new_id = record_id.clone().unwrap_or_else(uid);
            if let Some(existing) = record_id {
                version(tx, "workers", existing, *expected_version)?;
                tx.execute("UPDATE workers SET name=?2,role=?3,active=?4,version=version+1,updated_at=?5 WHERE id=?1",params![existing,name,role,active,time])?;
            } else {
                tx.execute("INSERT INTO workers(id,name,role,active,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?5)",params![new_id,name,role,active,time])?;
            }
            Ok((
                json!({"id":new_id}),
                vec!["workers"],
                "worker",
                new_id,
                "worker_saved",
                today,
                None,
            ))
        }
        Action::SaveTemplate {
            id: record_id,
            expected_version,
            name,
            default_notes,
            archived,
        } => {
            let name = text(name, 80, "name", true)?;
            let notes = text(default_notes, 2000, "default_notes", false)?;
            let new_id = record_id.clone().unwrap_or_else(uid);
            if let Some(existing) = record_id {
                version(tx, "stage_templates", existing, *expected_version)?;
                tx.execute("UPDATE stage_templates SET name=?2,normalized_name=?3,default_notes=?4,archived=?5,version=version+1,updated_at=?6 WHERE id=?1",params![existing,name,name.to_lowercase(),notes,archived,time])?;
            } else {
                let position: i64 = tx.query_row(
                    "SELECT coalesce(max(position),-1)+1 FROM stage_templates",
                    [],
                    |r| r.get(0),
                )?;
                tx.execute("INSERT INTO stage_templates(id,name,normalized_name,default_notes,position,archived,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?7)",params![new_id,name,name.to_lowercase(),notes,position,archived,time])?;
            }
            Ok((
                json!({"id":new_id}),
                vec!["templates"],
                "stage_template",
                new_id,
                "template_saved",
                today,
                None,
            ))
        }
        Action::ReorderTemplates { ids, versions } => {
            let count: i64 =
                tx.query_row("SELECT count(*) FROM stage_templates", [], |r| r.get(0))?;
            if ids.len() != count as usize
                || versions.len() != ids.len()
                || ids.iter().collect::<HashSet<_>>().len() != ids.len()
            {
                return Err(DomainError::validation(
                    "ids",
                    "Reordering requires every stage template exactly once.",
                ));
            }
            for (template, expected) in ids.iter().zip(versions) {
                version(tx, "stage_templates", template, Some(*expected))?;
            }
            tx.execute("UPDATE stage_templates SET position=-position-1", [])?;
            for (position, template) in ids.iter().enumerate() {
                tx.execute("UPDATE stage_templates SET position=?2,version=version+1,updated_at=?3 WHERE id=?1",params![template,position as i64,time])?;
            }
            Ok((
                json!({}),
                vec!["templates"],
                "stage_template",
                "all".into(),
                "templates_reordered",
                today,
                None,
            ))
        }
        Action::DeleteTemplate {
            id: template,
            expected_version,
        } => {
            version(tx, "stage_templates", template, Some(*expected_version))?;
            let referenced: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM order_stages WHERE template_id=?1)",
                [template],
                |r| r.get(0),
            )?;
            if referenced {
                return Err(DomainError::new(
                    "INVARIANT",
                    "This stage is used by an order. Archive it to preserve history.",
                ));
            }
            tx.execute("DELETE FROM stage_templates WHERE id=?1", [template])?;
            Ok((
                json!({}),
                vec!["templates"],
                "stage_template",
                template.clone(),
                "template_deleted",
                today,
                None,
            ))
        }
        Action::CreateOrder {
            client_id,
            name,
            due_date,
            notes,
            pricing_mode,
            fixed_total_fils,
            items,
            template_ids,
            stage_prices,
        } => {
            let name = text(name, 120, "name", true)?;
            let notes = text(notes, 4000, "notes", false)?;
            date(due_date, "due_date")?;
            let active: bool = tx.query_row(
                "SELECT archived=0 FROM clients WHERE id=?1",
                [client_id],
                |r| r.get(0),
            )?;
            if !active {
                return Err(DomainError::validation(
                    "client_id",
                    "Choose an active client.",
                ));
            }
            if items.is_empty() || items.len() > 100 {
                return Err(DomainError::validation(
                    "items",
                    "Add between 1 and 100 box types.",
                ));
            }
            if template_ids.is_empty()
                || template_ids.len() > 100
                || template_ids.iter().collect::<HashSet<_>>().len() != template_ids.len()
            {
                return Err(DomainError::validation(
                    "template_ids",
                    "Choose distinct production stages.",
                ));
            }
            let fixed = match pricing_mode {
                PricingMode::Fixed => Some(integer(
                    fixed_total_fils.as_deref().ok_or_else(|| {
                        DomainError::validation("fixed_total_fils", "Enter the customer quote.")
                    })?,
                    MAX_MONEY,
                    "fixed_total_fils",
                )?),
                _ => {
                    if fixed_total_fils.is_some() {
                        return Err(DomainError::validation(
                            "fixed_total_fils",
                            "Only fixed quotes accept a total.",
                        ));
                    }
                    None
                }
            };
            let mut prices = Vec::new();
            let mut total = 0_i128;
            for item in items {
                text(&item.description, 160, "description", true)?;
                text(&item.dimensions_text, 200, "dimensions_text", false)?;
                if !(1..=1_000_000).contains(&item.ordered_boxes) {
                    return Err(DomainError::validation(
                        "ordered_boxes",
                        "Boxes must be a whole number from 1 to 1,000,000.",
                    ));
                }
                let price = if *pricing_mode == PricingMode::PerBox {
                    Some(integer(
                        item.unit_price_fils.as_deref().ok_or_else(|| {
                            DomainError::validation(
                                "unit_price_fils",
                                "Every box type needs a unit selling price.",
                            )
                        })?,
                        MAX_MONEY,
                        "unit_price_fils",
                    )?)
                } else {
                    if item.unit_price_fils.is_some() {
                        return Err(DomainError::validation(
                            "unit_price_fils",
                            "Unit prices belong to per-box quotes.",
                        ));
                    }
                    None
                };
                total += i128::from(price.unwrap_or(0)) * i128::from(item.ordered_boxes);
                prices.push(price);
            }
            let mut quoted = match pricing_mode {
                PricingMode::Unpriced => None,
                PricingMode::Fixed => fixed,
                PricingMode::PerBox => Some(bounded_total(total)?),
            };
            let calculated = if let Some(inputs) = stage_prices {
                if *pricing_mode != PricingMode::Unpriced || fixed_total_fils.is_some() {
                    return Err(DomainError::validation(
                        "stage_prices",
                        "Use stage pricing without an additional order-level quote.",
                    ));
                }
                let (calculated, total) =
                    crate::stage_quotes::calculate(inputs, template_ids, items)?;
                quoted = total;
                Some(calculated)
            } else {
                None
            };
            let effective_mode = if calculated.is_some() {
                if quoted.is_some() {
                    "fixed"
                } else {
                    "unpriced"
                }
            } else {
                pricing_mode.as_str()
            };
            let effective_fixed = if calculated.is_some() { quoted } else { fixed };
            let order_id = uid();
            let quote_id = uid();
            let next: i64 = tx.query_row(
                "SELECT next_value FROM sequences WHERE name='order'",
                [],
                |r| r.get(0),
            )?;
            let number = format!("PF-{next:06}");
            tx.execute(
                "UPDATE sequences SET next_value=next_value+1 WHERE name='order'",
                [],
            )?;
            tx.execute("INSERT INTO orders(id,number,client_id,name,due_date,notes,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?7)",params![order_id,number,client_id,name,due_date,notes,time])?;
            tx.execute("INSERT INTO quote_revisions(id,order_id,revision_no,pricing_mode,fixed_total_fils,derived_total_fils,business_date,created_at,actor_id) VALUES(?1,?2,1,?3,?4,?5,?6,?7,?8)",params![quote_id,order_id,effective_mode,effective_fixed,quoted,today,time,actor])?;
            let mut item_ids = Vec::new();
            for (position, (item, price)) in items.iter().zip(prices).enumerate() {
                let item_id = uid();
                tx.execute(
                    "INSERT INTO order_items VALUES(?1,?2,?3,?4,?5,?6)",
                    params![
                        item_id,
                        order_id,
                        position as i64,
                        item.description.trim(),
                        item.dimensions_text.trim(),
                        item.ordered_boxes
                    ],
                )?;
                tx.execute(
                    "INSERT INTO quote_items VALUES(?1,?2,?3,?4,?5,?6,?7)",
                    params![
                        quote_id,
                        order_id,
                        item_id,
                        item.description.trim(),
                        item.dimensions_text.trim(),
                        item.ordered_boxes,
                        price
                    ],
                )?;
                item_ids.push(item_id);
            }
            for (position, template) in template_ids.iter().enumerate() {
                let (label, default_notes): (String, String) = tx.query_row(
                    "SELECT name,default_notes FROM stage_templates WHERE id=?1 AND archived=0",
                    [template],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?;
                let stage_id = uid();
                tx.execute("INSERT INTO order_stages(id,order_id,template_id,name_snapshot,position,notes) VALUES(?1,?2,?3,?4,?5,?6)",params![stage_id,order_id,template,label,position as i64,default_notes])?;
                if let (Some(inputs), Some(calculated)) = (stage_prices, &calculated) {
                    let index = inputs
                        .iter()
                        .position(|p| p.template_id == *template)
                        .unwrap();
                    let input = &inputs[index];
                    let result = &calculated[index];
                    tx.execute(
                        "INSERT INTO quote_stage_prices VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                        params![
                            quote_id,
                            order_id,
                            stage_id,
                            input.pricing_mode.as_str(),
                            result.fixed,
                            result.quantity,
                            result.rate,
                            result.total
                        ],
                    )?;
                    for (item_id, rate) in item_ids.iter().zip(&result.item_rates) {
                        tx.execute(
                            "INSERT INTO quote_stage_item_prices VALUES(?1,?2,?3,?4,?5)",
                            params![quote_id, order_id, stage_id, item_id, rate],
                        )?;
                    }
                }
            }
            tx.execute(
                "UPDATE orders SET current_quote_revision_id=?2 WHERE id=?1",
                params![order_id, quote_id],
            )?;
            Ok((
                json!({"id":order_id,"number":number}),
                vec!["orders"],
                "order",
                order_id,
                "order_created",
                today,
                None,
            ))
        }
        Action::UpdateStage {
            id: stage_id,
            expected_version,
            state,
            notes,
            business_date,
            reason,
        } => {
            date(business_date, "business_date")?;
            let notes = text(notes, 4000, "notes", false)?;
            version(tx, "order_stages", stage_id, Some(*expected_version))?;
            let (order_id, position): (String, i64) = tx.query_row(
                "SELECT order_id,position FROM order_stages WHERE id=?1 AND retired=0",
                [stage_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            open_order(tx, &order_id)?;
            let incomplete:i64=tx.query_row("SELECT count(*) FROM order_stages WHERE order_id=?1 AND position<?2 AND retired=0 AND state<>'completed'",params![order_id,position],|r|r.get(0))?;
            let reason = reason
                .as_deref()
                .map(|r| text(r, 1000, "reason", true))
                .transpose()?;
            if *state != StageState::NotStarted && incomplete > 0 && reason.is_none() {
                return Err(DomainError::validation(
                    "reason",
                    "Earlier stages are incomplete. Enter a reason to continue out of order.",
                ));
            }
            tx.execute("UPDATE order_stages SET state=?2,notes=?3,version=version+1,first_completed_date=CASE WHEN ?2='completed' THEN coalesce(first_completed_date,?4) ELSE first_completed_date END WHERE id=?1",params![stage_id,state.as_str(),notes,business_date])?;
            Ok((
                json!({"id":stage_id,"order_id":order_id}),
                vec!["orders"],
                "stage",
                stage_id.clone(),
                "stage_updated",
                business_date.clone(),
                reason,
            ))
        }
        Action::PostStageEntry {
            order_id,
            stage_id,
            business_date,
            work,
            expenses,
        } => {
            date(business_date, "business_date")?;
            open_order(tx, order_id)?;
            let valid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM order_stages WHERE id=?1 AND order_id=?2 AND retired=0)",params![stage_id,order_id],|r|r.get(0))?;
            if !valid {
                return Err(DomainError::validation(
                    "stage_id",
                    "Stage does not belong to this order.",
                ));
            }
            if work.is_none() && expenses.is_empty() || expenses.len() > 100 {
                return Err(DomainError::validation(
                    "expenses",
                    "Add work or between 1 and 100 cost lines.",
                ));
            }
            let work_id = if let Some(w) = work {
                let description = text(&w.description, 1000, "description", true)?;
                let dimensions = text(&w.dimensions_text, 200, "dimensions_text", false)?;
                item_belongs(tx, order_id, w.item_id.as_deref())?;
                if let Some(worker) = &w.worker_id {
                    let active: bool = tx.query_row(
                        "SELECT active=1 AND archived=0 FROM workers WHERE id=?1",
                        [worker],
                        |r| r.get(0),
                    )?;
                    if !active {
                        return Err(DomainError::validation(
                            "worker_id",
                            "Choose an active worker.",
                        ));
                    }
                }
                for value in [w.worker_minutes, w.machine_minutes].into_iter().flatten() {
                    if !(0..=1_000_000).contains(&value) {
                        return Err(DomainError::validation(
                            "minutes",
                            "Time is outside supported range.",
                        ));
                    }
                }
                let input = w
                    .input_scaled
                    .as_deref()
                    .map(|s| integer(s, 1_000_000_000, "input_scaled"))
                    .transpose()?;
                let output = w
                    .output_scaled
                    .as_deref()
                    .map(|s| integer(s, 1_000_000_000, "output_scaled"))
                    .transpose()?;
                let waste = w
                    .waste_scaled
                    .as_deref()
                    .map(|s| integer(s, 1_000_000_000, "waste_scaled"))
                    .transpose()?;
                let machine = text(&w.machine_description, 200, "machine_description", false)?;
                let record = uid();
                tx.execute("INSERT INTO work_events(id,order_id,stage_id,item_id,kind,business_date,description,dimensions_text,input_scaled,output_scaled,waste_scaled,worker_id,worker_minutes,machine_description,machine_minutes,created_at,actor_id) VALUES(?1,?2,?3,?4,'work',?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",params![record,order_id,stage_id,w.item_id,business_date,description,dimensions,input,output,waste,w.worker_id,w.worker_minutes,machine,w.machine_minutes,time,actor])?;
                Some(record)
            } else {
                None
            };
            let mut new_cost = 0_i128;
            for e in expenses {
                item_belongs(tx, order_id, e.item_id.as_deref())?;
                let description = text(&e.description, 1000, "description", true)?;
                let supplier = text(&e.supplier_text, 200, "supplier_text", false)?;
                let reference = text(&e.reference, 200, "reference", false)?;
                let qty = e
                    .quantity_scaled
                    .as_deref()
                    .map(|s| integer(s, 1_000_000_000, "quantity_scaled"))
                    .transpose()?;
                if qty == Some(0) {
                    return Err(DomainError::validation(
                        "quantity_scaled",
                        "Expense quantity must be positive.",
                    ));
                }
                let (basis, rate, amount) = match e.pricing_basis {
                    ExpenseBasis::Unit => {
                        if e.amount_fils.is_some() {
                            return Err(DomainError::validation(
                                "amount_fils",
                                "Unit-cost rows cannot have a separate editable total.",
                            ));
                        }
                        let qty = qty.ok_or_else(|| {
                            DomainError::validation("quantity_scaled", "Enter quantity in Unit.")
                        })?;
                        let rate = integer(
                            e.unit_cost_fils.as_deref().ok_or_else(|| {
                                DomainError::validation(
                                    "unit_cost_fils",
                                    "Enter unit purchase cost.",
                                )
                            })?,
                            MAX_MONEY,
                            "unit_cost_fils",
                        )?;
                        ("unit", Some(rate), line_total(qty, rate)?)
                    }
                    ExpenseBasis::Total => {
                        if e.unit_cost_fils.is_some() {
                            return Err(DomainError::validation(
                                "unit_cost_fils",
                                "Total-amount rows cannot also have a unit cost.",
                            ));
                        }
                        let amount = integer(
                            e.amount_fils.as_deref().ok_or_else(|| {
                                DomainError::validation("amount_fils", "Enter the cost amount.")
                            })?,
                            MAX_MONEY,
                            "amount_fils",
                        )?;
                        ("total", None, amount)
                    }
                };
                new_cost += i128::from(amount);
                tx.execute("INSERT INTO expense_entries(id,order_id,stage_id,item_id,work_event_id,kind,category,description,supplier_text,reference,pricing_basis,quantity_scaled,unit_cost_fils,amount_fils,business_date,created_at,actor_id) VALUES(?1,?2,?3,?4,?5,'expense',?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",params![uid(),order_id,stage_id,e.item_id,work_id,e.category.as_str(),description,supplier,reference,basis,qty,rate,amount,business_date,time,actor])?;
            }
            let existing:i64=tx.query_row("SELECT coalesce(sum(CASE kind WHEN 'expense' THEN amount_fils ELSE -amount_fils END),0) FROM expense_entries WHERE order_id=?1",[order_id],|r|r.get(0))?;
            bounded_total(i128::from(existing))?;
            Ok((
                json!({"order_id":order_id,"work_id":work_id,"posted_cost_fils":new_cost.to_string()}),
                vec!["orders", "costs"],
                "stage",
                stage_id.clone(),
                "stage_entry_posted",
                business_date.clone(),
                None,
            ))
        }
    }
}
