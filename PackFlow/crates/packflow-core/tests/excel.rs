use packflow_core::{
    db,
    excel::{self, ExcelFile, ExportRequest},
    models::*,
    service::Engine,
    Query,
};
use serde_json::{json, Value};
use std::io::{Cursor, Read};
fn request(page: &str) -> ExportRequest {
    serde_json::from_value(json!({"page":page,"from_date":"2026-10-01","to_date":"2026-10-31"}))
        .unwrap()
}
fn xml(file: &ExcelFile, entry: &str) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(&file.bytes)).unwrap();
    let mut text = String::new();
    zip.by_name(entry)
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    text
}
fn worksheet(file: &ExcelFile, name: &str) -> String {
    let book = xml(file, "xl/workbook.xml");
    let suffix = book
        .split(&format!("name=\"{name}\" "))
        .nth(1)
        .unwrap_or_else(|| panic!("Missing sheet {name}: {book}"));
    let id = suffix
        .split("sheetId=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    xml(file, &format!("xl/worksheets/sheet{id}.xml"))
}
fn count(file: &ExcelFile, name: &str) -> usize {
    worksheet(file, name).matches("<row ").count() - 1
}
fn mutate(e: &mut Engine, action: Value) -> Value {
    e.mutate(serde_json::from_value(json!({"request_id":uuid::Uuid::new_v4().to_string(),"actor_id":MANAGER_ID,"action":action})).unwrap()).unwrap()["data"].clone()
}
fn client(e: &mut Engine, name: &str) -> String {
    mutate(e,json!({"kind":"save_client","id":null,"expected_version":null,"name":name,"contact_name":"Amal","email":"","phone":"0012345","address":"Kuwait","archived":false}))["id"].as_str().unwrap().into()
}
fn order(e: &mut Engine, cid: &str, name: &str) -> String {
    let templates = e.query(Query::Bootstrap).unwrap()["templates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["id"].clone())
        .collect::<Vec<_>>();
    mutate(e,json!({"kind":"create_order","client_id":cid,"name":name,"due_date":"2026-10-25","notes":"Arabic العربية","pricing_mode":"fixed","fixed_total_fils":"45678","items":[{"description":"Small","dimensions_text":"10 x 20","ordered_boxes":100,"unit_price_fils":null}],"template_ids":templates}))["id"].as_str().unwrap().into()
}
#[test]
fn all_nine_pages_export_valid_empty_workbooks_without_changing_data() {
    let dir = tempfile::tempdir().unwrap();
    let conn = db::open(&dir.path().join("app.sqlite")).unwrap();
    let before: i64 = conn
        .query_row("SELECT count(*) FROM audit_events", [], |r| r.get(0))
        .unwrap();
    for page in [
        "dashboard",
        "orders",
        "clients",
        "workers",
        "costs",
        "payments",
        "reports",
        "settings",
        "stock",
    ] {
        let file = excel::create(&conn, &request(page)).unwrap_or_else(|e| panic!("{page}: {e}"));
        assert!(file.filename.ends_with(".xlsx"));
        assert!(file.bytes.starts_with(b"PK"));
        assert!(xml(&file, "xl/workbook.xml").contains("Export information"));
        assert!(!xml(&file, "xl/sharedStrings.xml").contains("password_hash"));
    }
    assert_eq!(
        before,
        conn.query_row("SELECT count(*) FROM audit_events", [], |r| r
            .get::<_, i64>(0))
            .unwrap()
    );
}
#[test]
fn export_is_uncapped_filters_literal_search_and_writes_user_input_as_text() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.sqlite");
    let mut e = Engine::open(&path).unwrap();
    let cid = client(&mut e, "=HYPERLINK(\"https://example.com\",\"Customer\")");
    let first = order(&mut e, &cid, "Oldest 100%_ 'quote' العربية");
    for n in 1..203 {
        order(&mut e, &cid, &format!("More boxes {n}"));
    }
    let conn = db::open(&path).unwrap();
    let all = excel::create(&conn, &request("orders")).unwrap();
    assert_eq!(count(&all, "Orders"), 203);
    assert_eq!(count(&all, "Box types"), 203);
    assert!(xml(&all, "xl/sharedStrings.xml").contains("=HYPERLINK"));
    assert!(xml(&all, "xl/sharedStrings.xml").contains("العربية"));
    assert!(!worksheet(&all, "Orders").contains("<f>"));
    let mut req = request("orders");
    req.search = "100%_ 'quote'".into();
    let filtered = excel::create(&conn, &req).unwrap();
    assert_eq!(count(&filtered, "Orders"), 1);
    assert_eq!(count(&filtered, "Production stages"), 4);
    req.search.clear();
    req.order_id = Some(first);
    assert_eq!(count(&excel::create(&conn, &req).unwrap(), "Orders"), 1);
}
#[test]
fn exports_services_receipts_corrections_and_report_dates_from_canonical_ledgers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.sqlite");
    let mut e = Engine::open(&path).unwrap();
    let cid = client(&mut e, "Linked customer");
    let oid = order(&mut e, &cid, "Gift boxes");
    mutate(
        &mut e,
        json!({"kind":"add_outside_service","order_id":oid,"expected_version":1,"description":"Finishing","supplier_text":"Partner","charge_fils":"12345","supplier_cost_fils":"6789","business_date":"2026-10-12","reference":"S1"}),
    );
    mutate(
        &mut e,
        json!({"kind":"receive_payment","order_id":oid,"amount_fils":"11111","business_date":"2026-10-12","method":"bank","reference":"P1","notes":"Deposit"}),
    );
    let cost = mutate(
        &mut e,
        json!({"kind":"add_separate_cost","category":"machine","amount_fils":"2345","description":"Repair","business_date":"2026-10-12"}),
    );
    mutate(
        &mut e,
        json!({"kind":"reverse_entry","entry_type":"separate_cost","id":cost["id"],"reason":"Duplicate","business_date":"2026-10-13"}),
    );
    let conn = db::open(&path).unwrap();
    let file = excel::create(&conn, &request("payments")).unwrap();
    assert_eq!(count(&file, "Outside services"), 1);
    assert!(worksheet(&file, "Outside services").contains("<v>12.345</v>"));
    assert!(worksheet(&file, "Outside services").contains("<v>6.789</v>"));
    assert!(worksheet(&file, "Payments").contains("<v>11.111</v>"));
    assert!(worksheet(&file, "Order balances").contains("<v>46.912</v>"));
    let costs = excel::create(&conn, &request("costs")).unwrap();
    assert_eq!(count(&costs, "Separate costs"), 2);
    assert!(worksheet(&costs, "Separate costs").contains("<v>-2.345</v>"));
    let mut report = request("reports");
    report.from_date = Some("2026-10-13".into());
    report.to_date = Some("2026-10-13".into());
    report.search = "No matching orders".into();
    let file = excel::create(&conn, &report).unwrap();
    assert_eq!(count(&file, "Orders"), 0);
    assert_eq!(count(&file, "Payments"), 0);
    assert_eq!(count(&file, "Separate costs"), 1);
    assert!(worksheet(&file, "Daily activity").contains("<v>-2.345</v>"));
    // Report ledgers remain complete even when the current-performance search has no matches.
    report.from_date = Some("2026-10-12".into());
    let file = excel::create(&conn, &report).unwrap();
    assert_eq!(count(&file, "Payments"), 1);
    assert_eq!(count(&file, "Stage costs"), 1);
}
#[test]
fn exports_salaries_with_editable_divisor_and_month_filter() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.sqlite");
    let mut e = Engine::open(&path).unwrap();
    let wid=mutate(&mut e,json!({"kind":"save_worker","id":null,"expected_version":null,"name":"Worker one","role":"Printing","active":true}))["id"].as_str().unwrap().to_owned();
    for month in ["2026-09", "2026-10"] {
        mutate(
            &mut e,
            json!({"kind":"set_salary_divisor","month":month,"expected_version":0,"divisor":25}),
        );
        mutate(
            &mut e,
            json!({"kind":"save_salary","id":null,"expected_version":null,"month":month,"expected_month_version":1,"worker_id":wid,"basic_fils":"125000","normal_days_scaled":"26000","hours_per_day_scaled":"6000","fridays":2,"overtime_hours_scaled":"5000","notes":"Monthly"}),
        );
    }
    let conn = db::open(&path).unwrap();
    let all = excel::create(&conn, &request("workers")).unwrap();
    assert_eq!(count(&all, "Monthly salaries"), 2);
    assert_eq!(count(&all, "Salary totals"), 2);
    let mut req = request("workers");
    req.month = Some("2026-10".into());
    let one = excel::create(&conn, &req).unwrap();
    assert_eq!(count(&one, "Monthly salaries"), 1);
    assert!(worksheet(&one, "Monthly salaries").contains("<v>25</v>"));
    assert!(worksheet(&one, "Monthly salaries").contains("<v>125</v>"));
}
#[test]
fn export_validates_dates_and_saves_atomically_without_overwriting_existing_files() {
    let dir = tempfile::tempdir().unwrap();
    let conn = db::open(&dir.path().join("app.sqlite")).unwrap();
    let mut bad = request("reports");
    bad.from_date = Some("2026-10-31".into());
    bad.to_date = Some("2026-10-01".into());
    assert!(excel::create(&conn, &bad).is_err());
    bad = request("workers");
    bad.month = Some("2026-99".into());
    assert!(excel::create(&conn, &bad).is_err());
    let file = excel::create(&conn, &request("stock")).unwrap();
    let target = dir.path().join("export.xlsx");
    file.save(&target).unwrap();
    let before = std::fs::read(&target).unwrap();
    assert!(file.save(&target).is_err());
    assert_eq!(before, std::fs::read(&target).unwrap());
    assert!(file.save(&dir.path().join("wrong.csv")).is_err());
}
