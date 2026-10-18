//! Versioned, text-only interchange. Reporting exports and database backups are unrelated.
use calamine::{Data, Reader, Xlsx};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    io::{Cursor, Read},
};
pub type Result<T> = std::result::Result<T, String>;
pub const MAX_FILE: usize = 8 * 1024 * 1024;
const MAX_EXPANDED: u64 = 32 * 1024 * 1024;
pub fn hash(v: &[u8]) -> String {
    format!("{:x}", Sha256::digest(v))
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub workspace_id: String,
    pub device_id: String,
    pub exported_at: String,
    pub from_date: String,
    pub to_date: String,
    pub clients: Vec<Value>,
    pub orders: Vec<Value>,
    pub payments: Vec<Value>,
    pub revisions: Vec<Value>,
}
type Row = BTreeMap<String, String>;
type Tables = BTreeMap<String, Vec<Row>>;
fn content_hash(p: &Package, tables: &Tables) -> Result<String> {
    Ok(hash(
        &serde_json::to_vec(&json!([
            p.workspace_id,
            p.device_id,
            p.exported_at,
            p.from_date,
            p.to_date,
            tables
        ]))
        .map_err(|e| e.to_string())?,
    ))
}
fn check_reference(s: &str) -> Result<()> {
    for part in s.split(':') {
        let part = part.replace('$', "");
        let digits = part
            .find(|c: char| c.is_ascii_digit())
            .ok_or("Invalid worksheet cell reference")?;
        let (letters, row) = part.split_at(digits);
        let mut col = 0usize;
        for c in letters.bytes() {
            if !c.is_ascii_uppercase() {
                return Err("Invalid worksheet column".into());
            }
            col = col
                .checked_mul(26)
                .and_then(|n| n.checked_add((c - b'A' + 1) as usize))
                .ok_or("Worksheet column overflow")?;
        }
        let row = row.parse::<usize>().map_err(|_| "Invalid worksheet row")?;
        if !(1..=32).contains(&col) || !(1..=100_001).contains(&row) {
            return Err("Workbook exceeds transfer worksheet bounds.".into());
        }
    }
    Ok(())
}
const SPECS: &[(&str, &str)] = &[
 ("Clients","id,name,contact_name,phone,email,address,notes,archived,version,created_at,updated_at"),
 ("Orders","id,number,version,archived,created_at,updated_at,client_id,name,due_date,payment_due_date,notes,status"),
 ("Box types","order_id,position,id,description,dimensions,quantity"),
 ("Stages","order_id,position,template_id,name,pricing_mode,fixed_total_fils,quantity_scaled,unit_price_fils"),
 ("Stage box prices","order_id,template_id,item_id,unit_price_fils"),
 ("Payments","id,order_id,kind,original_id,amount_fils,business_date,method,reference,notes,reason,created_at,device_id"),
 ("Quote history","order_id,version,document,reason,created_at,device_id"),
];
fn field(v: &Value, k: &str) -> String {
    match &v[k] {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
fn row(v: &Value, keys: &str) -> Row {
    keys.split(',').map(|k| (k.into(), field(v, k))).collect()
}
fn numeric(k: &str) -> bool {
    matches!(k, "version" | "quantity" | "archived")
}
fn object(r: &Row) -> Result<Value> {
    let mut m = serde_json::Map::new();
    for (k, v) in r {
        let value = if numeric(k) {
            json!(v
                .parse::<i64>()
                .map_err(|_| format!("Invalid integer in {k}"))?)
        } else if v.is_empty()
            && matches!(
                k.as_str(),
                "due_date"
                    | "payment_due_date"
                    | "original_id"
                    | "fixed_total_fils"
                    | "quantity_scaled"
                    | "unit_price_fils"
            )
        {
            Value::Null
        } else {
            json!(v)
        };
        m.insert(k.clone(), value);
    }
    Ok(Value::Object(m))
}
impl Package {
    fn tables(&self) -> Result<Tables> {
        let mut tables: Tables = SPECS.iter().map(|(n, _)| (n.to_string(), vec![])).collect();
        for v in &self.clients {
            tables.get_mut("Clients").unwrap().push(row(v, SPECS[0].1));
        }
        for o in &self.orders {
            let doc = &o["document"];
            let input = &doc["input"];
            let mut r = row(o, "id,number,version,archived,created_at,updated_at");
            r.extend(row(
                input,
                "client_id,name,due_date,payment_due_date,notes,status",
            ));
            tables.get_mut("Orders").unwrap().push(r);
            for (i, item) in input["items"]
                .as_array()
                .ok_or("Missing box types")?
                .iter()
                .enumerate()
            {
                let mut r = row(item, "id,description,dimensions,quantity");
                r.insert("order_id".into(), field(o, "id"));
                r.insert("position".into(), i.to_string());
                tables.get_mut("Box types").unwrap().push(r);
            }
            for (i, stage) in input["stages"]
                .as_array()
                .ok_or("Missing stages")?
                .iter()
                .enumerate()
            {
                let mut r = row(
                    stage,
                    "template_id,pricing_mode,fixed_total_fils,quantity_scaled,unit_price_fils",
                );
                r.insert("order_id".into(), field(o, "id"));
                r.insert("position".into(), i.to_string());
                r.insert(
                    "name".into(),
                    doc["stage_names"][i]
                        .as_str()
                        .ok_or("Missing stage name")?
                        .into(),
                );
                tables.get_mut("Stages").unwrap().push(r);
                for price in stage["item_prices"]
                    .as_array()
                    .ok_or("Missing item rates")?
                {
                    let mut r = row(price, "item_id,unit_price_fils");
                    r.insert("order_id".into(), field(o, "id"));
                    r.insert("template_id".into(), field(stage, "template_id"));
                    tables.get_mut("Stage box prices").unwrap().push(r);
                }
            }
        }
        for v in &self.payments {
            tables.get_mut("Payments").unwrap().push(row(v, SPECS[5].1));
        }
        for v in &self.revisions {
            tables
                .get_mut("Quote history")
                .unwrap()
                .push(row(v, SPECS[6].1));
        }
        if tables.values().map(Vec::len).sum::<usize>() > 100_000 {
            return Err("Too many transfer rows. Export a smaller period.".into());
        }
        Ok(tables)
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let tables = self.tables()?;
        let metadata = json!({"format":"PackFlow phone transfer","version":"1","currency":"KWD","workspace_id":self.workspace_id,"device_id":self.device_id,"exported_at":self.exported_at,"from_date":self.from_date,"to_date":self.to_date,"date_basis":"Changed on phone (Kuwait date); related records included","content_sha256":content_hash(self,&tables)?});
        let mut book = rust_xlsxwriter::Workbook::new();
        let header = rust_xlsxwriter::Format::new()
            .set_bold()
            .set_background_color("#E9EEE4");
        let s = book.add_worksheet();
        s.set_name("Transfer information")
            .map_err(|e| e.to_string())?;
        s.write_string(0, 0, "field").map_err(|e| e.to_string())?;
        s.write_string(0, 1, "value").map_err(|e| e.to_string())?;
        for (i, (k, v)) in metadata.as_object().unwrap().iter().enumerate() {
            s.write_string((i + 1) as u32, 0, k)
                .map_err(|e| e.to_string())?;
            s.write_string((i + 1) as u32, 1, v.as_str().unwrap())
                .map_err(|e| e.to_string())?;
        }
        for (name, headers) in SPECS {
            let s = book.add_worksheet();
            s.set_name(*name).map_err(|e| e.to_string())?;
            s.set_freeze_panes(1, 0).map_err(|e| e.to_string())?;
            for (col, key) in headers.split(',').enumerate() {
                s.write_string_with_format(0, col as u16, key, &header)
                    .map_err(|e| e.to_string())?;
                s.set_column_width(col as u16, 24)
                    .map_err(|e| e.to_string())?;
            }
            for (i, r) in tables.get(*name).unwrap().iter().enumerate() {
                for (j, key) in headers.split(',').enumerate() {
                    s.write_string(
                        (i + 1) as u32,
                        j as u16,
                        r.get(key).map(String::as_str).unwrap_or(""),
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
        }
        let bytes = book.save_to_buffer().map_err(|e| e.to_string())?;
        if bytes.len() > MAX_FILE {
            return Err("Transfer file exceeds 8 MB. Choose a smaller period.".into());
        }
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_FILE {
            return Err("Transfer files are limited to 8 MB.".into());
        }
        // Bound actual decompressed bytes before XML readers allocate based on workbook contents.
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes))
            .map_err(|_| "Choose a PackFlow transfer .xlsx file")?;
        if zip.len() > 100 {
            return Err("Too many workbook parts.".into());
        }
        let mut total = 0;
        for i in 0..zip.len() {
            let f = zip.by_index(i).map_err(|e| e.to_string())?;
            let name = f.name().map_err(|e| e.to_string())?;
            // Relationships can point to XML under any name or extension.
            // Check every part before Calamine follows those relationships.
            let worksheet = !name.ends_with('/');
            let mut expanded = Vec::new();
            f.take(MAX_EXPANDED + 1)
                .read_to_end(&mut expanded)
                .map_err(|e| e.to_string())?;
            total += expanded.len() as u64;
            if total > MAX_EXPANDED {
                return Err("Expanded workbook exceeds 32 MB.".into());
            }
            if worksheet {
                if std::str::from_utf8(&expanded).is_err() || expanded.contains(&0) {
                    return Err("Transfer workbooks must contain UTF-8 XML parts.".into());
                }
                let mut reader = quick_xml::Reader::from_reader(expanded.as_slice());
                loop {
                    match reader.read_event().map_err(|_| "Invalid worksheet XML")? {
                        quick_xml::events::Event::Eof => break,
                        quick_xml::events::Event::Start(e) | quick_xml::events::Event::Empty(e) => {
                            let local = e.local_name();
                            let name = local.as_ref();
                            if name == b"f" {
                                return Err(
                                    "Formulas are not allowed in transfer workbooks.".into()
                                );
                            }
                            for a in e.attributes() {
                                let a = a.map_err(|_| "Invalid worksheet attribute")?;
                                if (name == b"c" && a.key.as_ref() == b"r")
                                    || (name == b"dimension" && a.key.as_ref() == b"ref")
                                {
                                    let value = a
                                        .decoded_and_normalized_value(
                                            quick_xml::XmlVersion::Implicit1_0,
                                            reader.decoder(),
                                        )
                                        .map_err(|_| "Invalid worksheet reference")?;
                                    check_reference(&value)?;
                                }
                                if name == b"row" && a.key.as_ref() == b"r" {
                                    let n = std::str::from_utf8(a.value.as_ref())
                                        .map_err(|_| "Invalid row")?
                                        .parse::<usize>()
                                        .map_err(|_| "Invalid row")?;
                                    if !(1..=100_001).contains(&n) {
                                        return Err("Workbook exceeds row bounds.".into());
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        let mut book: Xlsx<_> =
            Xlsx::new(Cursor::new(bytes)).map_err(|e| format!("Invalid Excel file: {e}"))?;
        let expected: HashSet<_> = SPECS
            .iter()
            .map(|(n, _)| n.to_string())
            .chain(std::iter::once("Transfer information".into()))
            .collect();
        if book.sheet_names().iter().cloned().collect::<HashSet<_>>() != expected {
            return Err("Choose a version 1 phone transfer workbook; reporting exports are not import files.".into());
        }
        let mut read = |name: &str, headers: &str| -> Result<Vec<Row>> {
            if book
                .worksheet_formula(name)
                .map_err(|e| e.to_string())?
                .cells()
                .any(|(_, _, s)| !s.is_empty())
            {
                return Err(format!(
                    "Formulas are not allowed in {name}. Export again from the phone."
                ));
            }
            let range = book.worksheet_range(name).map_err(|e| e.to_string())?;
            if range.start() != Some((0, 0)) || range.height() > 100_001 || range.width() > 32 {
                return Err(format!("Invalid sheet size: {name}"));
            }
            let keys: Vec<_> = headers.split(',').collect();
            let mut rows = range.rows();
            let first = rows.next().ok_or("Missing headers")?;
            if first.len() != keys.len()
                || first
                    .iter()
                    .zip(&keys)
                    .any(|(v, k)| v != &Data::String(k.to_string()))
            {
                return Err(format!(
                    "Unexpected columns in {name}. Export an unchanged file from the phone."
                ));
            }
            rows.enumerate()
                .map(|(i, cells)| {
                    let mut r = Row::new();
                    for (j, key) in keys.iter().enumerate() {
                        let s = match cells.get(j) {
                            Some(Data::String(s)) => s.clone(),
                            Some(Data::Empty) | None => String::new(),
                            _ => {
                                return Err(format!(
                                    "{name}, row {}: transfer cells must remain text.",
                                    i + 2
                                ))
                            }
                        };
                        r.insert(key.to_string(), s);
                    }
                    Ok(r)
                })
                .collect()
        };
        let meta_rows = read("Transfer information", "field,value")?;
        let mut meta = Row::new();
        for r in meta_rows {
            if meta
                .insert(r["field"].clone(), r["value"].clone())
                .is_some()
            {
                return Err("Repeated transfer metadata.".into());
            }
        }
        if meta.get("format").map(String::as_str) != Some("PackFlow phone transfer")
            || meta.get("version").map(String::as_str) != Some("1")
            || meta.get("currency").map(String::as_str) != Some("KWD")
        {
            return Err("Unsupported transfer format/version/currency.".into());
        }
        let mut tables = Tables::new();
        for (n, h) in SPECS {
            tables.insert(n.to_string(), read(n, h)?);
        }
        if tables.values().map(Vec::len).sum::<usize>() > 100_000
            || tables["Orders"].len() > 5000
            || tables["Payments"].len() > 20000
        {
            return Err("Too many transfer rows. Export a smaller period.".into());
        }
        let mut orders = vec![];
        for r in &tables["Orders"] {
            let mut o = object(r)?;
            let oid = &r["id"];
            let mut items = vec![];
            for (i, r) in tables["Box types"]
                .iter()
                .filter(|r| &r["order_id"] == oid)
                .enumerate()
            {
                if r["position"] != i.to_string() {
                    return Err("Invalid box ordering.".into());
                }
                let mut v = object(r)?;
                v.as_object_mut().unwrap().remove("order_id");
                v.as_object_mut().unwrap().remove("position");
                items.push(v);
            }
            let mut stages = vec![];
            let mut names = vec![];
            for (i, r) in tables["Stages"]
                .iter()
                .filter(|r| &r["order_id"] == oid)
                .enumerate()
            {
                if r["position"] != i.to_string() {
                    return Err("Invalid stage ordering.".into());
                }
                let mut v = object(r)?;
                for k in ["order_id", "position", "name"] {
                    v.as_object_mut().unwrap().remove(k);
                }
                names.push(json!(r["name"]));
                let rates = tables["Stage box prices"]
                    .iter()
                    .filter(|p| &p["order_id"] == oid && p["template_id"] == r["template_id"])
                    .map(|p| json!({"item_id":p["item_id"],"unit_price_fils":p["unit_price_fils"]}))
                    .collect::<Vec<_>>();
                v["item_prices"] = json!(rates);
                stages.push(v);
            }
            let mut input = serde_json::Map::new();
            for k in [
                "client_id",
                "name",
                "due_date",
                "payment_due_date",
                "notes",
                "status",
            ] {
                input.insert(k.into(), o.as_object_mut().unwrap().remove(k).unwrap());
            }
            input.insert("items".into(), json!(items));
            input.insert("stages".into(), json!(stages));
            o["document"] = json!({"input":input,"stage_names":names});
            orders.push(o);
        }
        let get = |key: &str| {
            meta.get(key)
                .cloned()
                .ok_or_else(|| format!("Missing {key}"))
        };
        let p = Self {
            workspace_id: get("workspace_id")?,
            device_id: get("device_id")?,
            exported_at: get("exported_at")?,
            from_date: get("from_date")?,
            to_date: get("to_date")?,
            clients: tables["Clients"]
                .iter()
                .map(object)
                .collect::<Result<_>>()?,
            orders,
            payments: tables["Payments"]
                .iter()
                .map(object)
                .collect::<Result<_>>()?,
            revisions: tables["Quote history"]
                .iter()
                .map(object)
                .collect::<Result<_>>()?,
        };
        if Some(&content_hash(&p, &tables)?) != meta.get("content_sha256") {
            return Err("Transfer contents changed. Export a fresh file from the phone; do not edit its cells.".into());
        }
        p.validate()?;
        // Rebuilding tables also rejects orphan/extra detail rows that would otherwise be ignored.
        if p.tables()? != tables {
            return Err("Transfer has orphaned or inconsistent detail rows.".into());
        }
        Ok(p)
    }
    pub fn validate(&self) -> Result<()> {
        for id in [&self.workspace_id, &self.device_id] {
            uuid::Uuid::parse_str(id).map_err(|_| "Invalid workspace/device ID")?;
        }
        for d in [&self.from_date, &self.to_date] {
            date(d)?;
        }
        if self.from_date > self.to_date {
            return Err("Start date must precede end date.".into());
        }
        chrono::DateTime::parse_from_rfc3339(&self.exported_at)
            .map_err(|_| "Invalid export timestamp")?;
        let ids = |records: &[Value]| -> Result<HashSet<String>> {
            let mut out = HashSet::new();
            for r in records {
                let s = r["id"].as_str().ok_or("Missing record ID")?;
                uuid::Uuid::parse_str(s).map_err(|_| "Invalid record ID")?;
                if !out.insert(s.to_owned()) {
                    return Err("Duplicate record IDs.".into());
                }
            }
            Ok(out)
        };
        let clients = ids(&self.clients)?;
        let orders = ids(&self.orders)?;
        let payments = ids(&self.payments)?;
        if self.orders.len() > 5000
            || self.payments.len() > 20000
            || self.clients.len() > 10000
            || self.revisions.len() > 20000
        {
            return Err("Too many records. Narrow the export period.".into());
        }
        for c in &self.clients {
            valid_text(c, "name", 120, true)?;
            for (k, m) in [
                ("contact_name", 120),
                ("phone", 60),
                ("email", 254),
                ("address", 1000),
                ("notes", 4000),
            ] {
                valid_text(c, k, m, false)?;
            }
            version(c)?;
        }
        for o in &self.orders {
            version(o)?;
            let input = &o["document"]["input"];
            if !clients.contains(input["client_id"].as_str().unwrap_or("")) {
                return Err("An order is missing its client.".into());
            }
            valid_text(input, "name", 120, true)?;
            valid_text(input, "notes", 4000, false)?;
            for k in ["due_date", "payment_due_date"] {
                if !input[k].is_null() {
                    date(input[k].as_str().ok_or("Invalid date")?)?;
                }
            }
            if !matches!(input["status"].as_str(), Some("draft" | "confirmed")) {
                return Err("Invalid phone order status.".into());
            }
        }
        for p in &self.payments {
            if !orders.contains(p["order_id"].as_str().unwrap_or("")) {
                return Err("A payment is missing its order.".into());
            }
            date(p["business_date"].as_str().ok_or("Invalid payment date")?)?;
            amount(
                p["amount_fils"].as_str().ok_or("Invalid payment amount")?,
                true,
            )?;
            if !matches!(
                p["method"].as_str(),
                Some("cash" | "bank" | "card" | "other")
            ) {
                return Err("Invalid payment method.".into());
            }
            for (k, m) in [("reference", 200), ("notes", 4000), ("reason", 1000)] {
                valid_text(p, k, m, k == "reason" && p["kind"] == "correction")?;
            }
            if p["kind"] == "correction" {
                let original = self
                    .payments
                    .iter()
                    .find(|r| r["id"] == p["original_id"])
                    .ok_or("A correction is missing its original receipt")?;
                if original["kind"] != "receipt"
                    || original["order_id"] != p["order_id"]
                    || original["amount_fils"] != p["amount_fils"]
                    || original["business_date"].as_str() > p["business_date"].as_str()
                {
                    return Err("Inconsistent receipt correction.".into());
                }
            } else if p["kind"] != "receipt" || !p["original_id"].is_null() {
                return Err("Invalid receipt kind.".into());
            }
        }
        let mut reversed = HashSet::new();
        for p in &self.payments {
            if p["kind"] == "correction" && !reversed.insert(field(p, "original_id")) {
                return Err("A receipt is reversed twice.".into());
            }
        }
        let _ = payments;
        for r in &self.revisions {
            if !orders.contains(r["order_id"].as_str().unwrap_or("")) {
                return Err("Orphan quote history.".into());
            }
            let _: Value = serde_json::from_str(
                r["document"]
                    .as_str()
                    .ok_or("Missing historical document")?,
            )
            .map_err(|_| "Invalid quote history")?;
        }
        Ok(())
    }
}
pub fn date(s: &str) -> Result<()> {
    let d =
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| "Invalid YYYY-MM-DD date")?;
    if d.to_string() != s {
        return Err("Invalid YYYY-MM-DD date".into());
    }
    Ok(())
}
pub fn amount(s: &str, positive: bool) -> Result<i64> {
    if s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()) {
        return Err("Amounts must be exact integer fils text.".into());
    }
    let n = s.parse::<i64>().map_err(|_| "Amount too large")?;
    if !(if positive { 1 } else { 0 }..=9_000_000_000_000).contains(&n) {
        return Err("Amount outside allowed range.".into());
    }
    Ok(n)
}
fn version(v: &Value) -> Result<()> {
    if v["version"].as_i64().is_none_or(|n| n < 1) {
        return Err("Invalid source version.".into());
    }
    Ok(())
}
fn valid_text(v: &Value, k: &str, max: usize, required: bool) -> Result<()> {
    let s = v[k].as_str().ok_or_else(|| format!("Missing {k}"))?;
    if s.chars().count() > max || required && s.trim().is_empty() {
        return Err(format!("Invalid {k} length."));
    }
    Ok(())
}
