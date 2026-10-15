use packflow_core::{db, models::*, service::Engine, Query};
use serde_json::{json, Value};
fn mutate(e: &mut Engine, action: Value) -> Value {
    e.mutate(request(action)).unwrap()
}
fn request(action: Value) -> Mutation {
    serde_json::from_value(json!({"request_id":uuid::Uuid::new_v4().to_string(),"actor_id":MANAGER_ID,"action":action})).unwrap()
}
fn worker(e: &mut Engine) -> String {
    mutate(e,json!({"kind":"save_worker","id":null,"expected_version":null,"name":"Salary test worker","role":"Operator","active":true}))["data"]["id"].as_str().unwrap().into()
}
fn salaries(e: &Engine, key: &str) -> Value {
    e.query(Query::Salaries { month: key.into() }).unwrap()
}
fn add(worker: &str, key: &str, mv: i64) -> Value {
    json!({"kind":"save_salary","id":null,"expected_version":null,"month":key,"expected_month_version":mv,"worker_id":worker,"basic_fils":"125000","normal_days_scaled":"26000","hours_per_day_scaled":"6000","fridays":5,"overtime_hours_scaled":"65000","notes":"Monthly review"})
}
#[test]
fn workbook_formula_saved_month_divisor_and_worker_history_are_independent_of_orders() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.sqlite");
    let mut e = Engine::open(&path).unwrap();
    let wid = worker(&mut e);
    let req = request(add(&wid, "2026-05", 0));
    let result = e.mutate(req.clone()).unwrap();
    assert_eq!(e.mutate(req).unwrap(), result);
    let data = salaries(&e, "2026-05");
    let r = &data["records"][0];
    assert_eq!(r["normal_fils"], "125000");
    assert_eq!(r["friday_fils"], "36058");
    assert_eq!(r["overtime_fils"], "65104");
    assert_eq!(r["total_fils"], "226162");
    let req = request(
        json!({"kind":"set_salary_divisor","month":"2026-05","expected_version":1,"divisor":25}),
    );
    let result = e.mutate(req.clone()).unwrap();
    assert_eq!(e.mutate(req).unwrap(), result);
    let data = salaries(&e, "2026-05");
    assert_eq!(data["divisor"], 25);
    assert_eq!(data["version"], 2);
    assert_eq!(data["records"][0]["version"], 2);
    assert_eq!(data["records"][0]["total_fils"], "235208");
    mutate(
        &mut e,
        json!({"kind":"save_worker","id":wid,"expected_version":1,"name":"Renamed worker","role":"Operator","active":false}),
    );
    assert_eq!(
        salaries(&e, "2026-05")["records"][0]["worker_name_snapshot"],
        "Salary test worker"
    );
    // An inactive historical worker's existing record remains editable.
    let mut edit = add(&wid, "2026-05", 2);
    edit["id"] = data["records"][0]["id"].clone();
    edit["expected_version"] = json!(2);
    edit["fridays"] = json!(0);
    mutate(&mut e, edit);
    assert_eq!(salaries(&e, "2026-06")["divisor"], 26);
    assert_eq!(salaries(&e, "2026-06")["version"], 0);
    assert!(salaries(&e, "2026-06")["records"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(e
        .query(Query::ListOrders)
        .unwrap()
        .as_array()
        .unwrap()
        .is_empty());
    drop(e);
    let conn = rusqlite::Connection::open(&path).unwrap();
    for table in ["expense_entries", "payment_events", "work_events"] {
        assert_eq!(
            conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    drop(conn);
    let e = Engine::open(&path).unwrap();
    assert_eq!(salaries(&e, "2026-05")["totals"]["total_fils"], "197708");
    assert_eq!(
        salaries(&e, "2026-05")["history"].as_array().unwrap().len(),
        3
    );
}
#[test]
fn salaries_validate_inputs_duplicates_and_stale_month_and_record_versions() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(&dir.path().join("app.sqlite")).unwrap();
    let wid = worker(&mut e);
    for (field, value) in [
        ("month", json!("2026-13")),
        ("basic_fils", json!("9000000000001")),
        ("hours_per_day_scaled", json!("0")),
        ("hours_per_day_scaled", json!("24001")),
        ("normal_days_scaled", json!("31001")),
        ("fridays", json!(-1)),
        ("overtime_hours_scaled", json!("744001")),
    ] {
        let mut action = add(&wid, "2026-05", 0);
        action[field] = value;
        assert!(e.mutate(request(action)).is_err());
    }
    for divisor in [0, 32] {
        assert!(e.mutate(request(json!({"kind":"set_salary_divisor","month":"2026-05","expected_version":0,"divisor":divisor}))).is_err());
    }
    assert_eq!(salaries(&e, "2026-05")["version"], 0);
    let result = mutate(&mut e, add(&wid, "2026-05", 0));
    assert!(e.mutate(request(add(&wid, "2026-05", 1))).is_err()); // same worker/month
    assert!(e
        .mutate(request(
            json!({"kind":"set_salary_divisor","month":"2026-05","expected_version":0,"divisor":25})
        ))
        .is_err());
    let mut edit = add(&wid, "2026-05", 1);
    edit["id"] = result["data"]["id"].clone();
    edit["expected_version"] = json!(99);
    assert!(e.mutate(request(edit)).is_err());
    assert_eq!(
        salaries(&e, "2026-05")["records"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        salaries(&e, "2026-05")["history"].as_array().unwrap().len(),
        1
    );
}
#[test]
fn salary_archive_restore_keep_inputs_and_month_totals_consistent() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(&dir.path().join("app.sqlite")).unwrap();
    let wid = worker(&mut e);
    let r = mutate(&mut e, add(&wid, "2026-05", 0));
    let rid = &r["data"]["id"];
    mutate(
        &mut e,
        json!({"kind":"archive_salary","id":rid,"expected_version":1,"expected_month_version":1,"archived":true}),
    );
    let d = salaries(&e, "2026-05");
    assert_eq!(d["totals"]["total_fils"], "0");
    assert_eq!(d["totals"]["workers"], 0);
    assert_eq!(d["records"][0]["total_fils"], "226162");
    assert!(e.mutate(request(add(&wid, "2026-05", 2))).is_err());
    mutate(
        &mut e,
        json!({"kind":"set_salary_divisor","month":"2026-05","expected_version":2,"divisor":25}),
    );
    mutate(
        &mut e,
        json!({"kind":"archive_salary","id":rid,"expected_version":3,"expected_month_version":3,"archived":false}),
    );
    assert_eq!(salaries(&e, "2026-05")["totals"]["total_fils"], "235208");
}
#[test]
fn divisor_recalculation_rolls_back_all_rows_on_overflow() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(&dir.path().join("app.sqlite")).unwrap();
    let a = worker(&mut e);
    let b = worker(&mut e);
    mutate(
        &mut e,
        json!({"kind":"set_salary_divisor","month":"2026-05","expected_version":0,"divisor":31}),
    );
    mutate(&mut e, add(&a, "2026-05", 1));
    let mut large = add(&b, "2026-05", 2);
    large["basic_fils"] = json!("9000000000000");
    large["normal_days_scaled"] = json!("31000");
    large["fridays"] = json!(0);
    large["overtime_hours_scaled"] = json!("0");
    mutate(&mut e, large);
    let before = salaries(&e, "2026-05");
    assert!(e
        .mutate(request(
            json!({"kind":"set_salary_divisor","month":"2026-05","expected_version":3,"divisor":1})
        ))
        .is_err());
    assert_eq!(salaries(&e, "2026-05"), before);
}
#[test]
fn salary_components_round_once_and_sum_displayed_amounts() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(&dir.path().join("app.sqlite")).unwrap();
    let wid = worker(&mut e);
    mutate(
        &mut e,
        json!({"kind":"set_salary_divisor","month":"2026-05","expected_version":0,"divisor":2}),
    );
    let mut action = add(&wid, "2026-05", 1);
    action["basic_fils"] = json!("1");
    action["normal_days_scaled"] = json!("1000");
    action["hours_per_day_scaled"] = json!("1000");
    action["fridays"] = json!(1);
    action["overtime_hours_scaled"] = json!("800");
    mutate(&mut e, action);
    let data = salaries(&e, "2026-05");
    let r = &data["records"][0];
    for field in ["normal_fils", "friday_fils", "overtime_fils"] {
        assert_eq!(r[field], "1");
    }
    assert_eq!(r["total_fils"], "3");
}
#[test]
fn salary_migration_preserves_existing_orders_and_starts_with_empty_salary_months() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.sqlite");
    let mut e = Engine::open(&path).unwrap();
    let wid = worker(&mut e);
    drop(e);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("DROP TABLE service_voids; DROP TABLE separate_cost_voids; DELETE FROM schema_migrations WHERE version=7; DROP TABLE separate_costs; DELETE FROM schema_migrations WHERE version=6; DROP TABLE monthly_salaries; DROP TABLE salary_months; DELETE FROM schema_migrations WHERE version=5; PRAGMA user_version=4;").unwrap();
    drop(conn);
    let e = Engine::open(&path).unwrap();
    let d = salaries(&e, "2026-05");
    assert_eq!(d["divisor"], 26);
    assert_eq!(d["workers"][0]["id"], wid);
    assert!(d["records"].as_array().unwrap().is_empty());
    assert!(d["months"].as_array().unwrap().is_empty());
    assert_eq!(
        e.query(Query::Diagnostics).unwrap()["schema_version"],
        db::SCHEMA_VERSION
    );
    assert_eq!(
        std::fs::read_dir(dir.path().join("backups"))
            .unwrap()
            .count(),
        1
    );
}
