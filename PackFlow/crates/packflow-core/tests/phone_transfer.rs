use base64::Engine as _;
use packflow_core::{models::*, service::Engine, transfer::ImportRequest};
use packflow_mobile_core::Engine as Phone;
use serde_json::{json, Value};
use std::collections::BTreeMap;
fn save(e: &mut Phone, a: Value) -> Value {
    e.mutate(
        serde_json::from_value(json!({"request_id":uuid::Uuid::new_v4().to_string(),"action":a}))
            .unwrap(),
    )
    .unwrap()
}
fn fixture() -> (tempfile::TempDir, Phone, Engine, String, String) {
    let dir = tempfile::tempdir().unwrap();
    let mut p = Phone::open(
        &dir.path().join("phone.sqlite"),
        &dir.path().join("phone-backups"),
    )
    .unwrap();
    let m = Engine::open(&dir.path().join("mac.sqlite")).unwrap();
    let cid=save(&mut p,json!({"kind":"save_client","id":null,"expected_version":null,"name":"عميل الكويت","contact_name":"Amal","phone":"0012345","email":"","address":"Kuwait","notes":"Phone contact","archived":false}))["id"].as_str().unwrap().to_owned();
    let t = p
        .query(packflow_mobile_core::models::Query::Snapshot)
        .unwrap()["templates"]
        .clone();
    let a = uuid::Uuid::new_v4().to_string();
    let b = uuid::Uuid::new_v4().to_string();
    let input = json!({"client_id":cid,"name":"Phone two box types","due_date":null,"payment_due_date":"2026-11-15","notes":"Sheets: 100 as described","status":"draft","items":[{"id":a,"description":"Small","dimensions":"10 × 20","quantity":100},{"id":b,"description":"Large","dimensions":"20 × 40","quantity":50}],"stages":[{"template_id":t[0]["id"],"pricing_mode":"fixed","fixed_total_fils":"12500","quantity_scaled":null,"unit_price_fils":null,"item_prices":[]},{"template_id":t[1]["id"],"pricing_mode":"per_box","fixed_total_fils":null,"quantity_scaled":null,"unit_price_fils":null,"item_prices":[{"item_id":a,"unit_price_fils":"250"},{"item_id":b,"unit_price_fils":"400"}]},{"template_id":t[2]["id"],"pricing_mode":"per_unit","fixed_total_fils":null,"quantity_scaled":"3333","unit_price_fils":"125","item_prices":[]},{"template_id":t[3]["id"],"pricing_mode":"fixed","fixed_total_fils":"0","quantity_scaled":null,"unit_price_fils":null,"item_prices":[]}]});
    let oid=save(&mut p,json!({"kind":"save_order","id":null,"expected_version":null,"input":input,"reason":"","deposit":{"amount_fils":"10000","business_date":"2026-10-18","method":"cash","reference":"Deposit","notes":""}}))["id"].as_str().unwrap().to_owned();
    (dir, p, m, cid, oid)
}
fn request(p: &Phone, c: &str) -> ImportRequest {
    ImportRequest {
        data: base64::engine::general_purpose::STANDARD
            .encode(p.transfer_bytes("2026-01-01", "2027-01-01").unwrap()),
        client_links: BTreeMap::from([(c.into(), "new".into())]),
        preview_token: None,
    }
}
fn apply(m: &mut Engine, r: &ImportRequest) -> Value {
    let preview = m.import_phone(r.clone()).unwrap();
    assert_eq!(preview["can_apply"], true, "{preview}");
    let mut a = r.clone();
    a.preview_token = Some(preview["preview_token"].as_str().unwrap().into());
    m.import_phone(a).unwrap()
}
fn oid(m: &Engine) -> String {
    m.query(Query::ListOrders).unwrap()[0]["id"]
        .as_str()
        .unwrap()
        .into()
}
fn revise(p: &mut Phone, id: &str, change: impl FnOnce(&mut Value)) {
    let o = p
        .query(packflow_mobile_core::models::Query::Order { id: id.into() })
        .unwrap();
    let mut input = o["document"]["input"].clone();
    change(&mut input);
    save(
        p,
        json!({"kind":"save_order","id":id,"expected_version":o["version"],"input":input,"reason":"Customer changed quote","deposit":null}),
    );
}
#[test]
fn real_phone_excel_import_preserves_prices_relationships_and_deduplicates_after_restart() {
    let (dir, p, mut m, c, _) = fixture();
    let r = request(&p, &c);
    let no_mapping = m
        .import_phone(ImportRequest {
            client_links: BTreeMap::new(),
            ..r.clone()
        })
        .unwrap();
    assert_eq!(no_mapping["can_apply"], false);
    assert_eq!(m.query(Query::ListOrders).unwrap(), json!([]));
    let result = apply(&mut m, &r);
    assert!(std::path::Path::new(result["safety_backup"].as_str().unwrap()).exists());
    let id = oid(&m);
    let o = m.query(Query::Order { id: id.clone() }).unwrap();
    assert_eq!(o["order"]["number"], "PF-000001");
    assert_eq!(o["order"]["due_date"], "");
    assert_eq!(o["order"]["balance_fils"], "47917");
    assert_eq!(o["stage_prices"][2]["total_fils"], "417");
    assert_eq!(o["items"][0]["dimensions_text"], "10 × 20");
    drop(m);
    let mut m = Engine::open(&dir.path().join("mac.sqlite")).unwrap();
    let p = m.import_phone(r).unwrap();
    assert_eq!(p["ready"], 0);
    assert_eq!(
        m.query(Query::ListOrders)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    // Useful standalone fixture for browser tests; only opt-in writes fake data outside source.
    if let Ok(out) = std::env::var("PACKFLOW_TRANSFER_TEST_OUTPUT") {
        std::fs::write(
            out,
            base64::engine::general_purpose::STANDARD
                .decode(
                    request(
                        &Phone::open(&dir.path().join("phone.sqlite"), &dir.path().join("b"))
                            .unwrap(),
                        &c,
                    )
                    .data,
                )
                .unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn unchanged_mac_accepts_phone_revision_but_mac_stage_work_prevents_overwrite() {
    let (_dir, mut p, mut m, c, source) = fixture();
    apply(&mut m, &request(&p, &c));
    let id = oid(&m);
    revise(&mut p, &source, |i| i["items"][0]["quantity"] = json!(200));
    apply(&mut m, &request(&p, &c));
    let o = m.query(Query::Order { id: id.clone() }).unwrap();
    assert_eq!(o["order"]["balance_fils"], "72917");
    m.mutate(Mutation {
        request_id: uuid::Uuid::new_v4().to_string(),
        actor_id: MANAGER_ID.into(),
        action: Action::UpdateStage {
            id: o["stages"][0]["id"].as_str().unwrap().into(),
            expected_version: 1,
            state: StageState::InProgress,
            notes: "Work already started on Mac".into(),
            business_date: "2026-10-18".into(),
            reason: None,
        },
    })
    .unwrap();
    revise(&mut p, &source, |i| i["items"][0]["quantity"] = json!(300));
    let preview = m.import_phone(request(&p, &c)).unwrap();
    assert!(preview["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["entity"] == "order" && r["status"] == "conflict"));
    let after = m.query(Query::Order { id }).unwrap();
    assert_eq!(after["items"][0]["ordered_boxes"], 200);
    assert_eq!(after["stages"][0]["state"], "in_progress");
}
#[test]
fn corrections_import_once_and_stale_preview_or_modified_file_cannot_commit() {
    let (_dir, mut p, mut m, c, source) = fixture();
    apply(&mut m, &request(&p, &c));
    let snap = p
        .query(packflow_mobile_core::models::Query::Snapshot)
        .unwrap();
    let rid = snap["payments"][0]["id"].as_str().unwrap();
    save(
        &mut p,
        json!({"kind":"correct_receipt","id":rid,"reason":"Duplicate deposit","business_date":"2026-10-18"}),
    );
    let r = request(&p, &c);
    let preview = m.import_phone(r.clone()).unwrap();
    m.mutate(Mutation {
        request_id: uuid::Uuid::new_v4().to_string(),
        actor_id: MANAGER_ID.into(),
        action: Action::SaveClient {
            id: None,
            expected_version: None,
            name: "Mac addition".into(),
            contact_name: "".into(),
            email: "".into(),
            phone: "".into(),
            address: "".into(),
            archived: false,
        },
    })
    .unwrap();
    let mut stale = r.clone();
    stale.preview_token = Some(preview["preview_token"].as_str().unwrap().into());
    assert_eq!(m.import_phone(stale).unwrap_err().code, "CONFLICT");
    apply(&mut m, &r);
    let id = oid(&m);
    assert_eq!(
        m.query(Query::Order { id: id.clone() }).unwrap()["order"]["paid_fils"],
        "0"
    );
    assert_eq!(m.import_phone(r).unwrap()["ready"], 0);
    let mut package =
        packflow_transfer::Package::decode(&p.transfer_bytes("2026-01-01", "2027-01-01").unwrap())
            .unwrap();
    package.orders[0]["document"]["input"]["stages"][0]["fixed_total_fils"] = json!("oops");
    let mut bad = request(&p, &c);
    bad.data = base64::engine::general_purpose::STANDARD.encode(package.encode().unwrap());
    assert!(m.import_phone(bad).is_err());
    assert_eq!(
        m.query(Query::Order { id }).unwrap()["order"]["paid_fils"],
        "0"
    );
    let _ = source;
}
#[test]
fn date_range_includes_old_dependencies_and_explicit_existing_client_mapping() {
    let (_dir, p, mut m, c, _) = fixture();
    let empty = p.transfer_bytes("2020-01-01", "2020-01-02").unwrap();
    let pack = packflow_transfer::Package::decode(&empty).unwrap();
    assert!(pack.orders.is_empty());
    let existing = m
        .mutate(Mutation {
            request_id: uuid::Uuid::new_v4().to_string(),
            actor_id: MANAGER_ID.into(),
            action: Action::SaveClient {
                id: None,
                expected_version: None,
                name: "Existing Mac contact".into(),
                contact_name: "".into(),
                email: "".into(),
                phone: "555".into(),
                address: "".into(),
                archived: false,
            },
        })
        .unwrap()["data"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut r = request(&p, &c);
    r.client_links.insert(c, existing.clone());
    apply(&mut m, &r);
    assert_eq!(
        m.query(Query::Order { id: oid(&m) }).unwrap()["order"]["client_id"],
        existing
    );
    assert_eq!(
        m.query(Query::Bootstrap).unwrap()["clients"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

fn rewrite_xlsx(bytes: &[u8], part: &str, change: impl FnOnce(String) -> String) -> Vec<u8> {
    use std::io::{Cursor, Read, Write};
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let mut edit = Some(change);
    for i in 0..archive.len() {
        let mut f = archive.by_index(i).unwrap();
        let name = f.name().unwrap().into_owned();
        let mut bytes = vec![];
        f.read_to_end(&mut bytes).unwrap();
        if name == part {
            bytes = edit.take().unwrap()(String::from_utf8(bytes).unwrap()).into_bytes();
        }
        out.start_file(
            name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
        out.write_all(&bytes).unwrap();
    }
    out.finish().unwrap().into_inner()
}
#[test]
fn edited_cells_formulas_and_huge_sparse_coordinates_are_rejected_before_import() {
    let (_dir, p, mut m, c, _) = fixture();
    let request = request(&p, &c);
    let original = base64::engine::general_purpose::STANDARD
        .decode(&request.data)
        .unwrap();
    for (part, edit) in [
        ("xl/sharedStrings.xml", 0),
        ("xl/worksheets/sheet1.xml", 1),
        ("xl/worksheets/sheet1.xml", 2),
    ] {
        let bad = rewrite_xlsx(&original, part, move |s| match edit {
            0 => s.replace("Phone two box types", "Edited order"),
            1 => s.replacen("<v>", "<f>1+1</f><v>", 1),
            _ => s.replacen("r=\"A1\"", "r=\"XFD1048576\"", 1),
        });
        assert_ne!(bad, original);
        let r = ImportRequest {
            data: base64::engine::general_purpose::STANDARD.encode(bad),
            ..request.clone()
        };
        assert!(m.import_phone(r).is_err());
        assert_eq!(m.query(Query::ListOrders).unwrap(), json!([]));
    }
}

#[test]
fn transfer_distinguishes_unpriced_quote_and_explicit_zero_with_customer_credit() {
    let (_dir, mut p, mut m, c, source) = fixture();
    revise(&mut p, &source, |i| {
        for s in i["stages"].as_array_mut().unwrap() {
            s["pricing_mode"] = json!("unpriced");
            s["fixed_total_fils"] = Value::Null;
            s["quantity_scaled"] = Value::Null;
            s["unit_price_fils"] = Value::Null;
            s["item_prices"] = json!([]);
        }
    });
    apply(&mut m, &request(&p, &c));
    let id = oid(&m);
    let o = m.query(Query::Order { id: id.clone() }).unwrap();
    assert!(o["order"]["total_due_fils"].is_null());
    assert!(o["order"]["balance_fils"].is_null());
    assert_eq!(o["order"]["paid_fils"], "10000");
    revise(&mut p, &source, |i| {
        for s in i["stages"].as_array_mut().unwrap() {
            s["pricing_mode"] = json!("fixed");
            s["fixed_total_fils"] = json!("0");
        }
    });
    apply(&mut m, &request(&p, &c));
    let o = m.query(Query::Order { id }).unwrap();
    assert_eq!(o["order"]["total_due_fils"], "0");
    assert_eq!(o["order"]["balance_fils"], "-10000");
}

#[test]
fn export_period_selects_creation_and_historical_edits_even_after_later_updates() {
    let (dir, p, _m, _c, _id) = fixture();
    let sql = rusqlite::Connection::open(dir.path().join("phone.sqlite")).unwrap();
    sql.execute_batch("UPDATE clients SET created_at='2026-01-02T10:00:00Z',updated_at='2026-11-02T10:00:00Z';UPDATE orders SET created_at='2026-01-02T10:00:00Z',updated_at='2026-11-02T10:00:00Z';UPDATE payments SET created_at='2026-11-02T10:00:00Z';UPDATE audit SET created_at='2026-01-02T22:30:00Z'").unwrap();
    drop(sql);
    for day in ["2026-01-02", "2026-01-03"] {
        let package =
            packflow_transfer::Package::decode(&p.transfer_bytes(day, day).unwrap()).unwrap();
        assert_eq!(package.orders.len(), 1);
        assert_eq!(package.clients.len(), 1);
        assert_eq!(package.payments.len(), 1);
    }
    let empty =
        packflow_transfer::Package::decode(&p.transfer_bytes("2026-01-04", "2026-01-04").unwrap())
            .unwrap();
    assert!(empty.orders.is_empty());
    assert!(empty.clients.is_empty());
}

#[test]
fn workbook_reference_bounds_are_checked_for_renamed_parts() {
    use std::io::{Cursor, Read, Write};
    let (_dir, p, _m, c, _) = fixture();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(request(&p, &c).data)
        .unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..archive.len() {
        let mut f = archive.by_index(i).unwrap();
        let mut name = f.name().unwrap().into_owned();
        let mut bytes = vec![];
        f.read_to_end(&mut bytes).unwrap();
        if name == "xl/worksheets/sheet1.xml" {
            name = "xl/worksheets/sheet1.XML".into();
            bytes = String::from_utf8(bytes)
                .unwrap()
                .replacen("r=\"A1\"", "r=\"AG1\"", 1)
                .into_bytes();
        } else if name == "xl/_rels/workbook.xml.rels" {
            bytes = String::from_utf8(bytes)
                .unwrap()
                .replace("worksheets/sheet1.xml", "worksheets/sheet1.XML")
                .into_bytes();
        }
        out.start_file(
            name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
        out.write_all(&bytes).unwrap();
    }
    let bad = out.finish().unwrap().into_inner();
    assert!(packflow_transfer::Package::decode(&bad)
        .unwrap_err()
        .contains("worksheet bounds"));
}
#[test]
fn dated_export_carries_older_client_order_and_original_receipt_and_restore_undoes_import() {
    let (dir, mut p, mut m, c, source) = fixture();
    let sql = rusqlite::Connection::open(dir.path().join("phone.sqlite")).unwrap();
    sql.execute_batch("UPDATE clients SET created_at='2020-01-01T00:00:00Z',updated_at='2020-01-01T00:00:00Z';UPDATE orders SET created_at='2020-01-01T00:00:00Z',updated_at='2020-01-01T00:00:00Z';UPDATE payments SET created_at='2020-01-01T00:00:00Z'").unwrap();
    drop(sql);
    let receipt = save(
        &mut p,
        json!({"kind":"receive","order_id":source,"receipt":{"amount_fils":"5000","business_date":"2026-10-18","method":"cash","reference":"Later receipt","notes":""}}),
    );
    let _ = receipt;
    let bytes = p.transfer_bytes("2026-10-01", "2027-01-01").unwrap();
    let package = packflow_transfer::Package::decode(&bytes).unwrap();
    assert_eq!(package.clients.len(), 1);
    assert_eq!(package.orders.len(), 1);
    assert_eq!(package.payments.len(), 2);
    let r = ImportRequest {
        data: base64::engine::general_purpose::STANDARD.encode(bytes),
        ..request(&p, &c)
    };
    let imported = apply(&mut m, &r);
    assert_eq!(m.query(Query::ListOrders).unwrap()[0]["paid_fils"], "15000");
    m.restore(std::path::Path::new(
        imported["safety_backup"].as_str().unwrap(),
    ))
    .unwrap();
    assert_eq!(m.query(Query::ListOrders).unwrap(), json!([]));
    assert_eq!(m.import_phone(r).unwrap()["ready"], 4);
}
