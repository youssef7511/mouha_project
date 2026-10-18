//! Reviewed one-way phone imports, staged against a full disposable database.
use crate::{
    db,
    error::{DomainError, Result},
    models::*,
    service::{rows, text, Engine},
    stage_quotes,
};
use base64::Engine as _;
use packflow_transfer::Package;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
    time::Duration,
};
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImportRequest {
    pub data: String,
    #[serde(default)]
    pub client_links: BTreeMap<String, String>,
    pub preview_token: Option<String>,
}
fn err(s: impl AsRef<str>) -> DomainError {
    DomainError::validation("transfer", s.as_ref())
}
fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or("")
}
fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn hash(v: &Value) -> String {
    db::hash(v.to_string().as_bytes())
}
fn first(conn: &Connection, sql: &str, id: &str) -> Result<Value> {
    rows(conn, sql, [id])?
        .into_iter()
        .next()
        .ok_or_else(|| err("Linked Mac record no longer exists."))
}
fn local_client(conn: &Connection, id: &str) -> Result<String> {
    Ok(hash(&first(conn, "SELECT * FROM clients WHERE id=?1", id)?))
}
fn local_order(conn: &Connection, id: &str) -> Result<String> {
    let mut state = vec![];
    for (table, key) in [
        ("orders", "id"),
        ("order_items", "order_id"),
        ("order_stages", "order_id"),
        ("quote_revisions", "order_id"),
        ("quote_items", "order_id"),
        ("quote_stage_prices", "order_id"),
        ("quote_stage_item_prices", "order_id"),
        ("payment_events", "order_id"),
        ("refund_allocations", "order_id"),
        ("work_events", "order_id"),
        ("expense_entries", "order_id"),
        ("production_events", "order_id"),
        ("production_lines", "order_id"),
        ("dispatch_events", "order_id"),
        ("dispatch_lines", "order_id"),
        ("outside_services", "order_id"),
        ("fulfillment_milestones", "order_id"),
    ] {
        state.push(json!(rows(
            conn,
            &format!("SELECT * FROM {table} WHERE {key}=?1 ORDER BY rowid"),
            [id]
        )?));
    }
    Ok(hash(&json!(state)))
}
fn fingerprint(conn: &Connection) -> Result<String> {
    let mut all = vec![];
    for t in rows(conn,"SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",[])? {let name=s(&t,"name");let quoted=name.replace('"',"\"\"");all.push(json!([name,rows(conn,&format!("SELECT * FROM \"{quoted}\" ORDER BY rowid"),[])?]));}
    Ok(hash(&json!(all)))
}
fn link(c: &Connection, w: &str, kind: &str, id: &str) -> Result<Option<Value>> {
    Ok(rows(
        c,
        "SELECT * FROM mobile_links WHERE workspace_id=?1 AND entity=?2 AND source_id=?3",
        params![w, kind, id],
    )?
    .into_iter()
    .next())
}
fn remember(
    c: &Connection,
    w: &str,
    kind: &str,
    source: &Value,
    target: &str,
    local: &str,
) -> Result<()> {
    c.execute("INSERT INTO mobile_links VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(workspace_id,entity,source_id) DO UPDATE SET source_version=excluded.source_version,source_hash=excluded.source_hash,local_hash=excluded.local_hash,source_json=excluded.source_json",params![w,kind,s(source,"id"),target,source["version"].as_i64().unwrap_or(1),hash(source),local,source.to_string()])?;
    Ok(())
}
fn write(e: &mut Engine, action: Action) -> Result<Value> {
    Ok(e.mutate(Mutation {
        request_id: uid(),
        actor_id: MANAGER_ID.into(),
        action,
    })?["data"]
        .clone())
}
fn client_action(v: &Value, id: Option<String>, version: Option<i64>, archived: bool) -> Action {
    Action::SaveClient {
        id,
        expected_version: version,
        name: s(v, "name").into(),
        contact_name: s(v, "contact_name").into(),
        email: s(v, "email").into(),
        phone: s(v, "phone").into(),
        address: s(v, "address").into(),
        archived,
    }
}
#[derive(Clone)]
struct OrderData {
    items: Vec<NewItem>,
    prices: Vec<StagePriceInput>,
    names: Vec<String>,
    source_items: Vec<String>,
    source_stages: Vec<String>,
}
fn order_data(o: &Value) -> Result<OrderData> {
    let d = &o["document"];
    let input = &d["input"];
    let array = |key: &str| {
        input[key]
            .as_array()
            .ok_or_else(|| err(format!("Missing {key}")))
    };
    let mut items = vec![];
    let mut source_items = vec![];
    for item in array("items")? {
        let id = s(item, "id");
        crate::service::id(id)?;
        if source_items.contains(&id.to_string()) {
            return Err(err("Duplicate box identity."));
        }
        source_items.push(id.into());
        let quantity = item["quantity"]
            .as_i64()
            .ok_or_else(|| err("Invalid box quantity"))?;
        if !(1..=1_000_000).contains(&quantity) {
            return Err(err("Box quantity must be 1–1,000,000."));
        }
        items.push(NewItem {
            description: text(s(item, "description"), 160, "description", true)?,
            dimensions_text: text(s(item, "dimensions"), 200, "dimensions", false)?,
            ordered_boxes: quantity,
            unit_price_fils: None,
        });
    }
    if items.is_empty() || items.len() > 100 {
        return Err(err("Add 1–100 box types."));
    }
    let mut prices = vec![];
    let mut source_stages = vec![];
    let mut names = vec![];
    for (i, stage) in array("stages")?.iter().enumerate() {
        let id = s(stage, "template_id");
        crate::service::id(id)?;
        if source_stages.contains(&id.to_string()) {
            return Err(err("Duplicate stage identity."));
        }
        source_stages.push(id.into());
        let name = text(
            d["stage_names"][i].as_str().unwrap_or(""),
            80,
            "stage name",
            true,
        )?;
        if names
            .iter()
            .any(|n: &String| n.to_lowercase() == name.to_lowercase())
        {
            return Err(err("Stage snapshot names must be distinct for Mac import."));
        }
        names.push(name);
        let rates = stage["item_prices"]
            .as_array()
            .ok_or_else(|| err("Missing box rates"))?;
        let mode: StagePricingMode = serde_json::from_value(stage["pricing_mode"].clone())?;
        let mut item_rates = vec![];
        if mode == StagePricingMode::PerBox {
            if rates.len() != items.len() {
                return Err(err("Each box needs its own stage rate."));
            }
            for id in &source_items {
                let matching: Vec<_> = rates.iter().filter(|r| s(r, "item_id") == id).collect();
                if matching.len() != 1 {
                    return Err(err("Invalid box rate identity."));
                }
                item_rates.push(s(matching[0], "unit_price_fils").into());
            }
        } else if !rates.is_empty() {
            return Err(err("Only per-box stages accept box rates."));
        }
        prices.push(StagePriceInput {
            template_id: id.into(),
            pricing_mode: mode,
            fixed_total_fils: stage["fixed_total_fils"].as_str().map(str::to_owned),
            quantity_scaled: stage["quantity_scaled"].as_str().map(str::to_owned),
            unit_price_fils: stage["unit_price_fils"].as_str().map(str::to_owned),
            item_prices_fils: item_rates,
        });
    }
    if prices.is_empty()
        || prices.len() > 50
        || d["stage_names"].as_array().map(Vec::len) != Some(prices.len())
    {
        return Err(err("Add 1–50 stages with matching names."));
    }
    stage_quotes::calculate(&prices, &source_stages, &items)?;
    Ok(OrderData {
        items,
        prices,
        names,
        source_items,
        source_stages,
    })
}
fn templates(e: &mut Engine, data: &OrderData) -> Result<Vec<String>> {
    let mut out = vec![];
    for name in &data.names {
        let existing = e
            .conn
            .query_row(
                "SELECT id FROM stage_templates WHERE normalized_name=?1 AND archived=0",
                [name.to_lowercase()],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        let id = if let Some(id) = existing {
            id
        } else {
            s(
                &write(
                    e,
                    Action::SaveTemplate {
                        id: None,
                        expected_version: None,
                        name: name.clone(),
                        default_notes: String::new(),
                        archived: false,
                    },
                )?,
                "id",
            )
            .to_owned()
        };
        out.push(id);
    }
    Ok(out)
}
fn revise(e: &mut Engine, id: &str, source: &Value, data: &OrderData, client: &str) -> Result<()> {
    let old = first(&e.conn, "SELECT * FROM orders WHERE id=?1", id)?;
    if old["lifecycle"] != "open" || old["archived"] != 0 {
        return Err(err("Reopen the Mac order before importing changes."));
    }
    let has_payments: bool = e.conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM payment_events WHERE order_id=?1)",
        [id],
        |r| r.get(0),
    )?;
    if s(&old, "client_id") != client && has_payments {
        return Err(err("Cannot replace the client after payment history."));
    }
    let items = rows(
        &e.conn,
        "SELECT * FROM order_items WHERE order_id=?1 ORDER BY position",
        [id],
    )?;
    let stages = rows(
        &e.conn,
        "SELECT * FROM order_stages WHERE order_id=?1 AND retired=0 ORDER BY position",
        [id],
    )?;
    if items.len() != data.items.len() || stages.len() != data.prices.len() {
        return Err(err("Mac order structure changed."));
    }
    let (calc, total) = stage_quotes::calculate(&data.prices, &data.source_stages, &data.items)?;
    let input = &source["document"]["input"];
    let qid = uid();
    let time = db::now();
    let today = crate::service::business_today();
    let tx = e.conn.transaction()?;
    tx.execute("UPDATE orders SET client_id=?2,name=?3,due_date=?4,payment_due_date=?5,notes=?6,version=version+1,updated_at=?7 WHERE id=?1",params![id,client,s(input,"name"),s(input,"due_date"),input["payment_due_date"].as_str(),s(input,"notes"),time])?;
    let no: i64 = tx.query_row(
        "SELECT coalesce(max(revision_no),0)+1 FROM quote_revisions WHERE order_id=?1",
        [id],
        |r| r.get(0),
    )?;
    tx.execute("INSERT INTO quote_revisions(id,order_id,revision_no,pricing_mode,fixed_total_fils,derived_total_fils,reason,business_date,created_at,actor_id) VALUES(?1,?2,?3,?4,?5,?5,'Phone Excel revision',?6,?7,?8)",params![qid,id,no,if total.is_some(){"fixed"}else{"unpriced"},total,today,time,MANAGER_ID])?;
    for (local, input) in items.iter().zip(&data.items) {
        let iid = s(local, "id");
        tx.execute(
            "UPDATE order_items SET description=?2,dimensions_text=?3,ordered_boxes=?4 WHERE id=?1",
            params![
                iid,
                input.description,
                input.dimensions_text,
                input.ordered_boxes
            ],
        )?;
        tx.execute(
            "INSERT INTO quote_items VALUES(?1,?2,?3,?4,?5,?6,NULL)",
            params![
                qid,
                id,
                iid,
                input.description,
                input.dimensions_text,
                input.ordered_boxes
            ],
        )?;
    }
    for ((local, input), calc) in stages.iter().zip(&data.prices).zip(calc) {
        let sid = s(local, "id");
        tx.execute(
            "INSERT INTO quote_stage_prices VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                qid,
                id,
                sid,
                input.pricing_mode.as_str(),
                calc.fixed,
                calc.quantity,
                calc.rate,
                calc.total
            ],
        )?;
        for (item, rate) in items.iter().zip(calc.item_rates) {
            tx.execute(
                "INSERT INTO quote_stage_item_prices VALUES(?1,?2,?3,?4,?5)",
                params![qid, id, sid, s(item, "id"), rate],
            )?;
        }
    }
    tx.execute(
        "UPDATE orders SET current_quote_revision_id=?2 WHERE id=?1",
        params![id, qid],
    )?;
    tx.execute("INSERT INTO audit_events(id,actor_id,request_id,entity_type,entity_id,action,business_date,created_at,reason,after_json) VALUES(?1,?2,?3,'order',?4,'phone_order_revised',?5,?6,'Reviewed Excel transfer',?7)",params![uid(),MANAGER_ID,uid(),id,today,time,source.to_string()])?;
    tx.commit()?;
    Ok(())
}
fn run(e: &mut Engine, p: &Package, choices: &BTreeMap<String, String>) -> Result<Value> {
    let order_numbers: BTreeMap<_, _> = p
        .orders
        .iter()
        .map(|o| (s(o, "id"), s(o, "number")))
        .collect();
    let w = &p.workspace_id;
    let mut result = vec![];
    let mut blocked = HashSet::new();
    let mut baseline = HashSet::new();
    let row = |kind: &str, v: &Value, status: &str, message: &str, target: &str| json!({"entity":kind,"source_id":v["id"],"label":v["name"].as_str().or(v["document"]["input"]["name"].as_str()).or(v["number"].as_str()).unwrap_or(s(v,"business_date")),"status":status,"message":message,"target_id":target,"amount_fils":v["amount_fils"],"payment_kind":v["kind"],"method":v["method"],"reference":v["reference"],"source_order":order_numbers.get(s(v,"order_id")),"source_number":v["number"]});
    for c in &p.clients {
        let previous = link(&e.conn, w, "client", s(c, "id"))?;
        if let Some(old) = previous {
            let target = s(&old, "target_id");
            if s(&old, "source_hash") == hash(c) {
                result.push(row("client", c, "duplicate", "Already transferred", target));
                continue;
            }
            if c["version"].as_i64() <= old["source_version"].as_i64() {
                result.push(row(
                    "client",
                    c,
                    "conflict",
                    "Stale or changed source version",
                    target,
                ));
                continue;
            }
            if s(&old, "local_hash") == "linked" {
                remember(&e.conn, w, "client", c, target, "linked")?;
                result.push(row(
                    "client",
                    c,
                    "linked",
                    "Existing Mac contact retained",
                    target,
                ));
                continue;
            }
            if local_client(&e.conn, target)? != s(&old, "local_hash") {
                result.push(row(
                    "client",
                    c,
                    "conflict",
                    "Contact changed on Mac; retained",
                    target,
                ));
                continue;
            }
            let local = first(&e.conn, "SELECT * FROM clients WHERE id=?1", target)?;
            write(
                e,
                client_action(
                    c,
                    Some(target.into()),
                    local["version"].as_i64(),
                    local["archived"] == 1,
                ),
            )?;
            remember(
                &e.conn,
                w,
                "client",
                c,
                target,
                &local_client(&e.conn, target)?,
            )?;
            result.push(row(
                "client",
                c,
                "update",
                "Update imported contact",
                target,
            ));
        } else {
            match choices.get(s(c, "id")).map(String::as_str) {
                Some("new") => {
                    let saved = write(e, client_action(c, None, None, false))?;
                    let target = s(&saved, "id");
                    remember(
                        &e.conn,
                        w,
                        "client",
                        c,
                        target,
                        &local_client(&e.conn, target)?,
                    )?;
                    result.push(row("client", c, "new", "Create client", target));
                }
                Some(target) if !target.is_empty() => {
                    crate::service::id(target)?;
                    let local = first(&e.conn, "SELECT * FROM clients WHERE id=?1", target)?;
                    if local["archived"] == 1 {
                        return Err(err("Choose an active Mac client."));
                    }
                    remember(&e.conn, w, "client", c, target, "linked")?;
                    result.push(row(
                        "client",
                        c,
                        "linked",
                        "Link to existing client; keep Mac contact details",
                        target,
                    ));
                }
                _ => {
                    result.push(row(
                        "client",
                        c,
                        "needs_mapping",
                        "Choose new client or an existing Mac client",
                        "",
                    ));
                }
            }
        }
    }
    for o in &p.orders {
        let data = order_data(o)?;
        let input = &o["document"]["input"];
        let Some(client) = link(&e.conn, w, "client", s(input, "client_id"))? else {
            blocked.insert(s(o, "id").to_owned());
            result.push(row("order", o, "blocked", "Client mapping required", ""));
            continue;
        };
        let previous = link(&e.conn, w, "order", s(o, "id"))?;
        if let Some(old) = previous {
            let target = s(&old, "target_id");
            let current = local_order(&e.conn, target)?;
            if s(&old, "source_hash") == hash(o) {
                if current == s(&old, "local_hash") {
                    baseline.insert(target.to_owned());
                }
                result.push(row("order", o, "duplicate", "Already transferred", target));
                continue;
            }
            let old_source: Value = serde_json::from_str(s(&old, "source_json"))?;
            let old_data = order_data(&old_source)?;
            let problem = if o["version"].as_i64() <= old["source_version"].as_i64() {
                Some("Older or altered phone revision")
            } else if current != s(&old, "local_hash") {
                Some("Order changed on Mac; no phone overwrite")
            } else if data.source_items != old_data.source_items
                || data.source_stages != old_data.source_stages
                || data.names != old_data.names
            {
                Some("Box/stage structure changed; resolve manually on Mac")
            } else {
                None
            };
            if let Some(message) = problem {
                blocked.insert(s(o, "id").into());
                result.push(row("order", o, "conflict", message, target));
                continue;
            }
            if let Err(error) = revise(e, target, o, &data, s(&client, "target_id")) {
                blocked.insert(s(o, "id").into());
                result.push(row("order", o, "conflict", &error.message, target));
                continue;
            }
            remember(&e.conn, w, "order", o, target, "")?;
            baseline.insert(target.to_owned());
            result.push(row(
                "order",
                o,
                "update",
                "Append quote revision; keep production state",
                target,
            ));
        } else {
            let ts = templates(e, &data)?;
            let mut prices = data.prices.clone();
            for (p, t) in prices.iter_mut().zip(&ts) {
                p.template_id = t.clone();
            }
            let created = write(
                e,
                Action::CreateOrder {
                    client_id: s(&client, "target_id").into(),
                    name: s(input, "name").into(),
                    due_date: input["due_date"]
                        .as_str()
                        .unwrap_or(&crate::service::business_today())
                        .into(),
                    notes: s(input, "notes").into(),
                    pricing_mode: PricingMode::Unpriced,
                    fixed_total_fils: None,
                    items: data.items.clone(),
                    template_ids: ts,
                    stage_prices: Some(prices),
                },
            )?;
            let target = s(&created, "id");
            e.conn.execute(
                "UPDATE orders SET due_date=?2,payment_due_date=?3 WHERE id=?1",
                params![
                    target,
                    s(input, "due_date"),
                    input["payment_due_date"].as_str()
                ],
            )?;
            remember(&e.conn, w, "order", o, target, "")?;
            baseline.insert(target.to_owned());
            let message = format!(
                "Create {} (phone {})",
                s(&created, "number"),
                s(o, "number")
            );
            e.conn.execute("INSERT INTO audit_events(id,actor_id,request_id,entity_type,entity_id,action,business_date,created_at,reason,after_json) VALUES(?1,?2,?3,'order',?4,'phone_order_imported',?5,?6,'Reviewed phone Excel transfer',?7)",params![uid(),MANAGER_ID,uid(),target,crate::service::business_today(),db::now(),o.to_string()])?;
            result.push(row("order", o, "new", &message, target));
        }
    }
    for kind in ["receipt", "correction"] {
        for pmt in p.payments.iter().filter(|r| r["kind"] == kind) {
            if let Some(old) = link(&e.conn, w, "payment", s(pmt, "id"))? {
                result.push(row(
                    "payment",
                    pmt,
                    if s(&old, "source_hash") == hash(pmt) {
                        "duplicate"
                    } else {
                        "conflict"
                    },
                    if s(&old, "source_hash") == hash(pmt) {
                        "Already transferred"
                    } else {
                        "An imported receipt cannot be altered"
                    },
                    s(&old, "target_id"),
                ));
                continue;
            }
            if blocked.contains(s(pmt, "order_id")) {
                result.push(row(
                    "payment",
                    pmt,
                    "blocked",
                    "Resolve its order conflict first",
                    "",
                ));
                continue;
            }
            let Some(order) = link(&e.conn, w, "order", s(pmt, "order_id"))? else {
                result.push(row("payment", pmt, "blocked", "Missing order mapping", ""));
                continue;
            };
            let oid = s(&order, "target_id");
            let action = if kind == "receipt" {
                Action::ReceivePayment {
                    order_id: oid.into(),
                    amount_fils: s(pmt, "amount_fils").into(),
                    business_date: s(pmt, "business_date").into(),
                    method: s(pmt, "method").into(),
                    reference: s(pmt, "reference").into(),
                    notes: s(pmt, "notes").into(),
                }
            } else {
                let Some(original) = link(&e.conn, w, "payment", s(pmt, "original_id"))? else {
                    result.push(row(
                        "payment",
                        pmt,
                        "blocked",
                        "Original receipt was not imported",
                        "",
                    ));
                    continue;
                };
                let remaining:i64=e.conn.query_row("SELECT amount_fils-coalesce((SELECT sum(amount_fils) FROM refund_allocations WHERE receipt_event_id=p.id),0) FROM payment_events p WHERE id=?1 AND kind='receipt'",[s(&original,"target_id")],|r|r.get(0))?;
                if Some(remaining) != s(pmt, "amount_fils").parse::<i64>().ok() {
                    result.push(row(
                        "payment",
                        pmt,
                        "conflict",
                        "Receipt was already corrected/refunded on Mac",
                        s(&original, "target_id"),
                    ));
                    continue;
                }
                Action::ReverseEntry {
                    entry_type: CorrectionKind::Payment,
                    id: s(&original, "target_id").into(),
                    reason: s(pmt, "reason").into(),
                    business_date: s(pmt, "business_date").into(),
                }
            };
            match write(e, action) {
                Ok(saved) => {
                    let target = s(&saved, "id");
                    remember(&e.conn, w, "payment", pmt, target, "")?;
                    result.push(row(
                        "payment",
                        pmt,
                        "new",
                        if kind == "receipt" {
                            "Receive payment"
                        } else {
                            "Reverse original receipt"
                        },
                        target,
                    ));
                }
                Err(error) => result.push(row("payment", pmt, "conflict", &error.message, "")),
            }
        }
    }
    for target in baseline {
        e.conn.execute("UPDATE mobile_links SET local_hash=?3 WHERE workspace_id=?1 AND entity='order' AND target_id=?2",params![w,target,local_order(&e.conn,&target)?])?;
    }
    let ready = result
        .iter()
        .filter(|r| matches!(s(r, "status"), "new" | "update" | "linked"))
        .count();
    let unmapped = result.iter().any(|r| r["status"] == "needs_mapping");
    Ok(
        json!({"rows":result,"ready":ready,"can_apply":ready>0&&!unmapped,"workspace_id":p.workspace_id,"device_id":p.device_id,"from_date":p.from_date,"to_date":p.to_date,"clients":p.clients,"mac_clients":rows(&e.conn,"SELECT id,name,phone,email FROM clients WHERE archived=0 ORDER BY normalized_name",[])?,"source_history_count":p.revisions.len()}),
    )
}
impl Engine {
    pub fn import_phone(&mut self, request: ImportRequest) -> Result<Value> {
        if request.data.len() > 12 * 1024 * 1024 {
            return Err(err("Transfer files are limited to 8 MB."));
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&request.data)
            .map_err(|_| err("Invalid file encoding"))?;
        let p = Package::decode(&bytes).map_err(err)?;
        // Validate every quote even if its parent is waiting for client mapping.
        for o in &p.orders {
            order_data(o)?;
        }
        for key in request.client_links.keys() {
            if !p.clients.iter().any(|c| s(c, "id") == key) {
                return Err(err("Unknown client mapping."));
            }
        }
        let state = fingerprint(&self.conn)?;
        let token = hash(&json!([state, db::hash(&bytes), request.client_links]));
        if request.preview_token.as_ref().is_some_and(|t| t != &token) {
            return Err(DomainError::new(
                "CONFLICT",
                "Workspace or import choices changed. Preview this file again.",
            ));
        }
        let folder = tempfile::tempdir()?;
        let path = folder.path().join("import.sqlite");
        db::snapshot(&self.conn, &path)?;
        let mut candidate = Engine::open(&path)?;
        let mut plan = run(&mut candidate, &p, &request.client_links)?;
        plan["preview_token"] = json!(token);
        plan["mac_clients"] = json!(rows(
            &self.conn,
            "SELECT id,name,phone,email FROM clients WHERE archived=0 ORDER BY normalized_name",
            []
        )?);
        if request.preview_token.is_none() {
            return Ok(plan);
        }
        if plan["can_apply"] != true {
            return Err(err(
                "No ready records, or client mapping is incomplete. Preview again.",
            ));
        }
        let parent = Path::new(
            self.conn
                .path()
                .ok_or_else(|| err("Missing database path"))?,
        )
        .parent()
        .ok_or_else(|| err("Missing database folder"))?
        .join("backups");
        std::fs::create_dir_all(&parent)?;
        let backup = parent.join(format!("before-phone-import-{}.pfbackup", uid()));
        crate::recovery::export(&self.conn, &backup)?;
        let batch = uid();
        candidate.conn.execute(
            "INSERT INTO mobile_imports VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                batch,
                p.workspace_id,
                p.device_id,
                db::hash(&bytes),
                db::now(),
                MANAGER_ID,
                backup.to_string_lossy(),
                plan.to_string(),
                serde_json::to_string(&p.revisions)?
            ],
        )?;
        let fk = candidate
            .conn
            .prepare("PRAGMA foreign_key_check")?
            .query([])?
            .next()?
            .is_some();
        if fk {
            return Err(err("Candidate import failed its relationship checks."));
        }
        rusqlite::backup::Backup::new(&candidate.conn, &mut self.conn)?.run_to_completion(
            128,
            Duration::from_millis(5),
            None,
        )?;
        plan["safety_backup"] = json!(backup);
        plan["import_id"] = json!(batch);
        plan["message"] = json!("Ready records imported. Conflicts were left unchanged.");
        Ok(plan)
    }
}
