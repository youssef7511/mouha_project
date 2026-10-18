use packflow_core::{
    db,
    models::*,
    money::{line_total, MAX_MONEY},
    service::Engine,
    DbWorker, Query,
};
use serde_json::{json, Value};
use std::path::Path;

fn mutation(action: Value) -> Mutation {
    serde_json::from_value(json!({"request_id":uuid::Uuid::new_v4().to_string(),"actor_id":MANAGER_ID,"action":action})).unwrap()
}
fn client(engine: &mut Engine) -> String {
    engine.mutate(mutation(json!({"kind":"save_client","id":null,"expected_version":null,"name":"Box client","contact_name":"Amal","email":"","phone":"","address":"Kuwait","archived":false}))).unwrap()["data"]["id"].as_str().unwrap().into()
}
fn new_order(engine: &Engine, client: &str) -> Mutation {
    let bootstrap = engine.query(Query::Bootstrap).unwrap();
    mutation(
        json!({"kind":"create_order","client_id":client,"name":"Two box sizes","due_date":"2026-11-20","notes":"",
        "pricing_mode":"per_box","fixed_total_fils":null,
        "items":[{"description":"Small","dimensions_text":"10 × 20","ordered_boxes":100,"unit_price_fils":"250"},{"description":"Large","dimensions_text":"20 × 40","ordered_boxes":50,"unit_price_fils":"400"}],
        "template_ids":bootstrap["templates"].as_array().unwrap().iter().map(|v|v["id"].clone()).collect::<Vec<_>>() }),
    )
}
fn setup(path: &Path) -> (Engine, String) {
    let mut e = Engine::open(path).unwrap();
    let c = client(&mut e);
    let r = new_order(&e, &c);
    let oid = e.mutate(r).unwrap()["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    (e, oid)
}

fn outside_service(oid: &str, version: i64, charge: &str, cost: &str) -> Mutation {
    mutation(
        json!({"kind":"add_outside_service","order_id":oid,"expected_version":version,
        "description":"Custom finishing","supplier_text":"Finishing partner","charge_fils":charge,
        "supplier_cost_fils":cost,"business_date":"2026-10-12","reference":"SERVICE-1"}),
    )
}
fn receipt(oid: &str, amount: &str) -> Mutation {
    mutation(
        json!({"kind":"receive_payment","order_id":oid,"amount_fils":amount,
        "business_date":"2026-10-12","method":"bank","reference":"TRANSFER-1","notes":"Deposit"}),
    )
}

fn deadline(oid: &str, expected_version: i64, due: Option<String>) -> Mutation {
    mutation(
        json!({"kind":"set_payment_due_date","order_id":oid,"expected_version":expected_version,"payment_due_date":due}),
    )
}

#[test]
fn order_pages_search_beyond_two_hundred_and_keep_report_aggregates_complete() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.sqlite");
    let mut e = Engine::open(&path).unwrap();
    let cid = client(&mut e);
    let mut ids = Vec::new();
    for index in 0..203 {
        let mut req = new_order(&e, &cid);
        if let Action::CreateOrder { name, .. } = &mut req.action {
            *name = if index == 0 {
                "Oldest 100%_ 'quoted' العربية".into()
            } else {
                format!("Paged order {index:03}")
            };
        }
        ids.push(
            e.mutate(req).unwrap()["data"]["id"]
                .as_str()
                .unwrap()
                .to_string(),
        );
    }
    let page = |search: &str, page: i64, size: i64, scope| Query::OrderPage {
        search: search.into(),
        page,
        page_size: size,
        scope,
    };
    let first = e.query(page("", 1, 25, OrderScope::All)).unwrap();
    assert_eq!(first["total"], 203);
    assert_eq!(first["records"].as_array().unwrap().len(), 25);
    let mut seen = std::collections::HashSet::new();
    for index in 1..=9 {
        for row in e.query(page("", index, 25, OrderScope::All)).unwrap()["records"]
            .as_array()
            .unwrap()
        {
            assert!(seen.insert(row["id"].as_str().unwrap().to_string()));
        }
    }
    assert_eq!(seen.len(), 203);
    let oldest = e.query(page("OLDEST", 1, 10, OrderScope::All)).unwrap();
    assert_eq!(oldest["total"], 1);
    assert_eq!(oldest["records"][0]["id"], ids[0]);
    assert_eq!(
        e.query(page("%_", 1, 10, OrderScope::All)).unwrap()["total"],
        1
    );
    assert_eq!(
        e.query(page("العربية", 1, 10, OrderScope::All)).unwrap()["total"],
        1
    );
    assert_eq!(
        e.query(page("box CLIENT", 1, 10, OrderScope::All)).unwrap()["total"],
        203
    );
    assert_eq!(
        e.query(page("PF-000001", 1, 10, OrderScope::All)).unwrap()["records"][0]["id"],
        ids[0]
    );
    assert_eq!(
        e.query(page("", i64::MAX, 100, OrderScope::All)).unwrap()["page"],
        3
    );
    let empty = e
        .query(page("no such order", 99, 25, OrderScope::All))
        .unwrap();
    assert_eq!(empty["page"], 1);
    assert_eq!(empty["total"], 0);
    for (index, size) in [(0, 25), (1, 0), (1, 200), (1, 26)] {
        assert!(e.query(page("", index, size, OrderScope::All)).is_err());
    }
    assert!(e
        .query(page(&"a".repeat(201), 1, 25, OrderScope::All))
        .is_err());
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute("UPDATE orders SET archived=1 WHERE id=?1", [&ids[1]])
        .unwrap();
    conn.execute(
        "UPDATE orders SET lifecycle='cancelled' WHERE id=?1",
        [&ids[2]],
    )
    .unwrap();
    conn.execute(
        "UPDATE orders SET lifecycle='closed' WHERE id=?1",
        [&ids[3]],
    )
    .unwrap();
    drop(conn);
    assert_eq!(
        e.query(page("", 1, 25, OrderScope::Active)).unwrap()["total"],
        201
    );
    assert_eq!(
        e.query(page("", 1, 25, OrderScope::All)).unwrap()["total"],
        203
    );
    let report = e
        .query(Query::Reports {
            from_date: "2026-10-01".into(),
            to_date: "2026-10-31".into(),
            include_portfolio: Some(false),
        })
        .unwrap();
    assert!(report["portfolio"].as_array().unwrap().is_empty());
    assert_eq!(report["portfolio_summary"]["order_count"], 201);
    assert_eq!(report["portfolio_summary"]["ordered_boxes"], 30150);
    assert_eq!(report["portfolio_summary"]["total_due_fils"], "9045000");
}

fn separate_cost(category: &str, amount: &str, date: &str) -> Mutation {
    mutation(
        json!({"kind":"add_separate_cost","category":category,"amount_fils":amount,"description":"Standalone expense","business_date":date}),
    )
}
#[test]
fn separate_costs_are_exact_retry_safe_persisted_and_reported_without_affecting_orders() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.sqlite");
    let (mut e, oid) = setup(&path);
    let before = e.query(Query::Order { id: oid.clone() }).unwrap();
    let req = separate_cost("machine", "12345", "2026-10-14");
    let result = e.mutate(req.clone()).unwrap();
    assert_eq!(e.mutate(req).unwrap(), result);
    e.mutate(separate_cost("finish", "5432", "2026-10-14"))
        .unwrap();
    e.mutate(separate_cost("other", "1000", "2026-09-30"))
        .unwrap();
    assert_eq!(e.query(Query::Order { id: oid.clone() }).unwrap(), before);
    let d = e.query(Query::SeparateCosts).unwrap();
    assert_eq!(d["count"], 3);
    assert_eq!(d["total_fils"], "18777");
    let report = e
        .query(Query::Reports {
            include_portfolio: None,
            from_date: "2026-10-01".into(),
            to_date: "2026-10-31".into(),
        })
        .unwrap();
    assert_eq!(report["activity"][0]["cost_fils"], "17777");
    assert_eq!(report["activity"][0]["received_fils"], "0");
    assert_eq!(report["portfolio"][0]["cost_fils"], "0");
    let categories = report["categories"].as_array().unwrap();
    assert_eq!(categories.len(), 2);
    assert!(categories
        .iter()
        .any(|c| c["category"] == "separate_finish" && c["amount_fils"] == "5432"));
    drop(e);
    let e = Engine::open(&path).unwrap();
    assert_eq!(e.query(Query::SeparateCosts).unwrap(), d);
    assert_eq!(e.query(Query::Order { id: oid }).unwrap(), before);
    let conn = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM audit_events WHERE action='separate_cost_added'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        3
    );
}
#[test]
fn separate_cost_validation_preserves_data_and_totals_are_uncapped() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(&dir.path().join("app.sqlite")).unwrap();
    for req in [
        separate_cost("machine", "-1", "2026-10-14"),
        separate_cost("finish", "1.5", "2026-10-14"),
        separate_cost("other", "9000000000001", "2026-10-14"),
        separate_cost("machine", "1", "2026-02-30"),
    ] {
        assert!(e.mutate(req).is_err());
    }
    let mut req = separate_cost("other", "1", "2026-10-14");
    if let Action::AddSeparateCost { description, .. } = &mut req.action {
        *description = "  ".into();
    }
    assert!(e.mutate(req).is_err());
    assert!(serde_json::from_value::<Mutation>(json!({"request_id":uuid::Uuid::new_v4().to_string(),"actor_id":MANAGER_ID,"action":{"kind":"add_separate_cost","category":"invalid","amount_fils":"1","description":"test","business_date":"2026-10-14"}})).is_err());
    assert_eq!(e.query(Query::SeparateCosts).unwrap()["count"], 0);
    for _ in 0..201 {
        e.mutate(separate_cost("machine", "1", "2026-10-14"))
            .unwrap();
    }
    let d = e.query(Query::SeparateCosts).unwrap();
    assert_eq!(d["records"].as_array().unwrap().len(), 200);
    assert_eq!(d["count"], 201);
    assert_eq!(d["total_fils"], "201");
}
#[test]
fn separate_cost_upgrade_preserves_v5_quotes_and_monthly_salary_settings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.sqlite");
    let (mut e, oid) = setup(&path);
    e.mutate(mutation(
        json!({"kind":"set_salary_divisor","month":"2026-10","expected_version":0,"divisor":25}),
    ))
    .unwrap();
    drop(e);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("DROP TABLE mobile_links; DROP TABLE mobile_imports; DELETE FROM schema_migrations WHERE version=8; DROP TABLE service_voids; DROP TABLE separate_cost_voids; DELETE FROM schema_migrations WHERE version=7; DROP VIEW order_financials; CREATE VIEW order_financials AS  WITH amounts AS (  SELECT o.id AS order_id, q.derived_total_fils AS base_quote_fils,    coalesce((SELECT sum(charge_fils) FROM outside_services WHERE order_id=o.id),0) AS service_charge_fils,    coalesce((SELECT sum(CASE kind WHEN 'receipt' THEN amount_fils ELSE -amount_fils END) FROM payment_events WHERE order_id=o.id),0) AS paid_fils,    coalesce((SELECT sum(CASE kind WHEN 'expense' THEN amount_fils ELSE -amount_fils END) FROM expense_entries WHERE order_id=o.id),0) AS cost_fils  FROM orders o LEFT JOIN quote_revisions q ON q.id=o.current_quote_revision_id  )  SELECT *, CASE WHEN base_quote_fils IS NULL THEN NULL ELSE base_quote_fils+service_charge_fils END AS total_due_fils,  CASE WHEN base_quote_fils IS NULL THEN NULL ELSE base_quote_fils+service_charge_fils-paid_fils END AS balance_fils  FROM amounts;  DROP TABLE separate_costs; DELETE FROM schema_migrations WHERE version=6; PRAGMA user_version=5;").unwrap();
    drop(conn);
    let e = Engine::open(&path).unwrap();
    assert_eq!(
        e.query(Query::Order { id: oid }).unwrap()["order"]["quote_fils"],
        "45000"
    );
    assert_eq!(
        e.query(Query::Salaries {
            month: "2026-10".into()
        })
        .unwrap()["divisor"],
        25
    );
    assert_eq!(e.query(Query::SeparateCosts).unwrap()["count"], 0);
    assert_eq!(
        std::fs::read_dir(dir.path().join("backups"))
            .unwrap()
            .count(),
        1
    );
}

fn stage_order(engine: &Engine, cid: &str) -> Value {
    let mut action = serde_json::to_value(new_order(engine, cid)).unwrap()["action"].clone();
    action["pricing_mode"] = json!("unpriced");
    for item in action["items"].as_array_mut().unwrap() {
        item["unit_price_fils"] = Value::Null;
    }
    let ids = action["template_ids"].as_array().unwrap();
    action["stage_prices"] = json!([
        {"template_id":ids[0],"pricing_mode":"fixed","fixed_total_fils":"10000","quantity_scaled":null,"unit_price_fils":null,"item_prices_fils":[]},
        {"template_id":ids[1],"pricing_mode":"per_box","fixed_total_fils":null,"quantity_scaled":null,"unit_price_fils":null,"item_prices_fils":["250","400"]},
        {"template_id":ids[2],"pricing_mode":"per_unit","fixed_total_fils":null,"quantity_scaled":"1125","unit_price_fils":"333","item_prices_fils":[]},
        {"template_id":ids[3],"pricing_mode":"fixed","fixed_total_fils":"0","quantity_scaled":null,"unit_price_fils":null,"item_prices_fils":[]}
    ]);
    action
}
#[test]
fn mixed_stage_quotes_are_exact_persisted_retry_safe_and_feed_balances_reports() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.sqlite");
    let mut e = Engine::open(&path).unwrap();
    let cid = client(&mut e);
    let mut action = stage_order(&e, &cid);
    // Input prices may arrive in a different order; associate by template identity.
    action["stage_prices"].as_array_mut().unwrap().reverse();
    let request = mutation(action);
    let result = e.mutate(request.clone()).unwrap();
    assert_eq!(e.mutate(request).unwrap(), result);
    let oid = result["data"]["id"].as_str().unwrap().to_string();
    let d = e.query(Query::Order { id: oid.clone() }).unwrap();
    assert_eq!(d["order"]["quote_fils"], "55375");
    assert_eq!(d["stage_prices"][0]["total_fils"], "10000");
    assert_eq!(d["stage_prices"][1]["total_fils"], "45000");
    assert_eq!(d["stage_prices"][2]["total_fils"], "375");
    assert_eq!(d["stage_prices"][3]["total_fils"], "0");
    assert_eq!(d["stage_item_prices"].as_array().unwrap().len(), 2);
    assert_eq!(d["cost_fils"], "0");
    e.mutate(outside_service(&oid, 1, "18000", "12000"))
        .unwrap();
    e.mutate(receipt(&oid, "10000")).unwrap();
    drop(e);
    let e = Engine::open(&path).unwrap();
    let d = e.query(Query::Order { id: oid }).unwrap();
    assert_eq!(d["order"]["total_due_fils"], "73375");
    assert_eq!(d["order"]["balance_fils"], "63375");
    assert_eq!(d["cost_fils"], "12000");
    assert_eq!(d["stage_prices"].as_array().unwrap().len(), 4);
    assert_eq!(
        e.query(Query::Reports {
            include_portfolio: None,
            from_date: "2026-10-01".into(),
            to_date: "2026-10-31".into()
        })
        .unwrap()["portfolio"][0]["total_due_fils"],
        "73375"
    );
    assert_eq!(
        e.query(Query::ClientInsights).unwrap()["clients"][0]["quoted_fils"],
        "73375"
    );
}
#[test]
fn unpriced_stage_retains_known_components_without_creating_a_false_total() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(&dir.path().join("app.sqlite")).unwrap();
    let cid = client(&mut e);
    let mut action = stage_order(&e, &cid);
    action["stage_prices"][3]["pricing_mode"] = json!("unpriced");
    action["stage_prices"][3]["fixed_total_fils"] = Value::Null;
    let result = e.mutate(mutation(action)).unwrap();
    let d = e
        .query(Query::Order {
            id: result["data"]["id"].as_str().unwrap().into(),
        })
        .unwrap();
    assert!(d["order"]["quote_fils"].is_null());
    assert!(d["order"]["balance_fils"].is_null());
    assert_eq!(d["stage_prices"][1]["total_fils"], "45000");
    assert!(d["stage_prices"][3]["total_fils"].is_null());
}
#[test]
fn invalid_stage_pricing_rolls_back_and_does_not_consume_order_numbers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.sqlite");
    let mut e = Engine::open(&path).unwrap();
    let cid = client(&mut e);
    for case in 0..8 {
        let mut action = stage_order(&e, &cid);
        match case {
            0 => {
                action["stage_prices"].as_array_mut().unwrap().pop();
            }
            1 => {
                action["stage_prices"][1]["template_id"] =
                    action["stage_prices"][0]["template_id"].clone()
            }
            2 => action["stage_prices"][0]["template_id"] = json!(uuid::Uuid::new_v4().to_string()),
            3 => action["stage_prices"][1]["item_prices_fils"] = json!(["250"]),
            4 => action["stage_prices"][2]["quantity_scaled"] = json!("0"),
            5 => action["stage_prices"][0]["unit_price_fils"] = json!("1"),
            6 => action["stage_prices"][0]["fixed_total_fils"] = json!(MAX_MONEY.to_string()),
            _ => {
                action["stage_prices"][1]["item_prices_fils"] = json!([MAX_MONEY.to_string(), "0"])
            }
        }
        assert!(e.mutate(mutation(action)).is_err(), "case {case}");
    }
    assert!(e
        .query(Query::ListOrders)
        .unwrap()
        .as_array()
        .unwrap()
        .is_empty());
    let r = e.mutate(mutation(stage_order(&e, &cid))).unwrap();
    assert_eq!(r["data"]["number"], "PF-000001");
}
#[test]
fn v3_upgrade_preserves_legacy_quote_and_deadline_without_inventing_stage_prices() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.sqlite");
    let (mut e, oid) = setup(&path);
    e.mutate(deadline(&oid, 1, Some("2026-11-01".into())))
        .unwrap();
    drop(e);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("DROP TABLE mobile_links; DROP TABLE mobile_imports; DELETE FROM schema_migrations WHERE version=8; DROP TABLE service_voids; DROP TABLE separate_cost_voids; DELETE FROM schema_migrations WHERE version=7; DROP VIEW order_financials; CREATE VIEW order_financials AS  WITH amounts AS (  SELECT o.id AS order_id, q.derived_total_fils AS base_quote_fils,    coalesce((SELECT sum(charge_fils) FROM outside_services WHERE order_id=o.id),0) AS service_charge_fils,    coalesce((SELECT sum(CASE kind WHEN 'receipt' THEN amount_fils ELSE -amount_fils END) FROM payment_events WHERE order_id=o.id),0) AS paid_fils,    coalesce((SELECT sum(CASE kind WHEN 'expense' THEN amount_fils ELSE -amount_fils END) FROM expense_entries WHERE order_id=o.id),0) AS cost_fils  FROM orders o LEFT JOIN quote_revisions q ON q.id=o.current_quote_revision_id  )  SELECT *, CASE WHEN base_quote_fils IS NULL THEN NULL ELSE base_quote_fils+service_charge_fils END AS total_due_fils,  CASE WHEN base_quote_fils IS NULL THEN NULL ELSE base_quote_fils+service_charge_fils-paid_fils END AS balance_fils  FROM amounts;  DROP TABLE separate_costs; DELETE FROM schema_migrations WHERE version=6; DROP TABLE monthly_salaries; DROP TABLE salary_months; DELETE FROM schema_migrations WHERE version=5; DROP TABLE quote_stage_item_prices; DROP TABLE quote_stage_prices; DELETE FROM schema_migrations WHERE version=4; PRAGMA user_version=3;").unwrap();
    drop(conn);
    let e = Engine::open(&path).unwrap();
    let d = e.query(Query::Order { id: oid }).unwrap();
    assert_eq!(d["order"]["quote_fils"], "45000");
    assert_eq!(d["order"]["payment_due_date"], "2026-11-01");
    assert!(d["stage_prices"].as_array().unwrap().is_empty());
    assert_eq!(
        std::fs::read_dir(dir.path().join("backups"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn client_insights_keep_credit_separate_and_classify_only_agreed_payment_dates() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.sqlite");
    let (mut e, first) = setup(&path);
    let original = e.query(Query::Order { id: first.clone() }).unwrap();
    let cid = original["order"]["client_id"].as_str().unwrap();
    let today = e.query(Query::Bootstrap).unwrap()["business_today"]
        .as_str()
        .unwrap()
        .to_string();
    let parsed = chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap();
    let yesterday = (parsed - chrono::Duration::days(1)).to_string();
    let tomorrow = (parsed + chrono::Duration::days(1)).to_string();
    e.mutate(deadline(&first, 1, Some(yesterday))).unwrap();
    for _ in 0..2 {
        let mut payment = receipt(&first, "5000");
        if let Action::ReceivePayment { business_date, .. } = &mut payment.action {
            *business_date = today.clone();
        }
        e.mutate(payment).unwrap();
    }
    let request = new_order(&e, cid);
    let credit = e.mutate(request).unwrap()["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    e.mutate(deadline(&credit, 1, Some(tomorrow.clone())))
        .unwrap();
    let mut payment = receipt(&credit, "50000");
    if let Action::ReceivePayment { business_date, .. } = &mut payment.action {
        *business_date = today.clone();
    }
    e.mutate(payment).unwrap();
    let request = new_order(&e, cid);
    let unpaid = e.mutate(request).unwrap()["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let mut payment = receipt(&unpaid, "45000");
    if let Action::ReceivePayment { business_date, .. } = &mut payment.action {
        *business_date = tomorrow;
    }
    e.mutate(payment).unwrap(); // Future-dated receipts do not improve today's client history.
    for mode in [PricingMode::Unpriced, PricingMode::Fixed] {
        let mut request = new_order(&e, cid);
        if let Action::CreateOrder {
            pricing_mode,
            fixed_total_fils,
            items,
            ..
        } = &mut request.action
        {
            *pricing_mode = mode.clone();
            *fixed_total_fils = if mode == PricingMode::Fixed {
                Some("0".into())
            } else {
                None
            };
            for item in items {
                item.unit_price_fils = None;
            }
        }
        e.mutate(request).unwrap();
    }
    let result = e.query(Query::ClientInsights).unwrap();
    let client = &result["clients"][0];
    assert_eq!(client["order_count"], 5);
    assert_eq!(client["priced_orders"], 3);
    assert_eq!(client["partial_orders"], 1);
    assert_eq!(client["settled_orders"], 1);
    assert_eq!(client["instalment_orders"], 1);
    assert_eq!(client["overdue_orders"], 1);
    assert_eq!(client["late_receipt_orders"], 1);
    assert_eq!(client["overdue_fils"], "35000");
    assert_eq!(client["outstanding_fils"], "80000");
    assert_eq!(client["credit_fils"], "5000");
    assert_eq!(client["net_received_fils"], "60000");
    assert_eq!(client["unpriced_orders"], 1);
    assert_eq!(client["missing_deadline_orders"], 1);
    assert_eq!(client["oldest_overdue_days"], 1);
    assert_eq!(result["payments"].as_array().unwrap().len(), 3);
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute(
        "UPDATE orders SET lifecycle='cancelled',cancellation_reason='Test' WHERE id=?1",
        [&first],
    )
    .unwrap();
    let result = e.query(Query::ClientInsights).unwrap();
    assert_eq!(result["clients"][0]["net_received_fils"], "60000");
    assert_eq!(result["clients"][0]["overdue_orders"], 0);
    assert_eq!(result["clients"][0]["order_count"], 4);
}

#[test]
fn payment_deadline_is_validated_versioned_and_independent_of_production_due_date() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, oid) = setup(&dir.path().join("app.sqlite"));
    assert!(e
        .mutate(deadline(&oid, 1, Some("2026-02-30".into())))
        .is_err());
    e.mutate(deadline(&oid, 1, Some("2026-10-01".into())))
        .unwrap();
    assert_eq!(
        e.mutate(deadline(&oid, 1, None)).unwrap_err().code,
        "CONFLICT"
    );
    let d = e.query(Query::Order { id: oid.clone() }).unwrap();
    assert_eq!(d["order"]["due_date"], "2026-11-20");
    assert_eq!(d["order"]["payment_due_date"], "2026-10-01");
    assert_eq!(d["cost_fils"], "0");
    e.mutate(deadline(&oid, 2, None)).unwrap();
    assert!(e.query(Query::Order { id: oid }).unwrap()["order"]["payment_due_date"].is_null());
}

#[test]
fn migration_three_preserves_services_and_receipts_from_v2() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.sqlite");
    let (mut e, oid) = setup(&path);
    e.mutate(outside_service(&oid, 1, "18000", "12000"))
        .unwrap();
    e.mutate(receipt(&oid, "10000")).unwrap();
    drop(e);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("DROP TABLE mobile_links; DROP TABLE mobile_imports; DELETE FROM schema_migrations WHERE version=8; DROP TABLE service_voids; DROP TABLE separate_cost_voids; DELETE FROM schema_migrations WHERE version=7; DROP VIEW order_financials; CREATE VIEW order_financials AS  WITH amounts AS (  SELECT o.id AS order_id, q.derived_total_fils AS base_quote_fils,    coalesce((SELECT sum(charge_fils) FROM outside_services WHERE order_id=o.id),0) AS service_charge_fils,    coalesce((SELECT sum(CASE kind WHEN 'receipt' THEN amount_fils ELSE -amount_fils END) FROM payment_events WHERE order_id=o.id),0) AS paid_fils,    coalesce((SELECT sum(CASE kind WHEN 'expense' THEN amount_fils ELSE -amount_fils END) FROM expense_entries WHERE order_id=o.id),0) AS cost_fils  FROM orders o LEFT JOIN quote_revisions q ON q.id=o.current_quote_revision_id  )  SELECT *, CASE WHEN base_quote_fils IS NULL THEN NULL ELSE base_quote_fils+service_charge_fils END AS total_due_fils,  CASE WHEN base_quote_fils IS NULL THEN NULL ELSE base_quote_fils+service_charge_fils-paid_fils END AS balance_fils  FROM amounts;  DROP TABLE separate_costs; DELETE FROM schema_migrations WHERE version=6; DROP TABLE monthly_salaries; DROP TABLE salary_months; DELETE FROM schema_migrations WHERE version=5; DROP TABLE quote_stage_item_prices; DROP TABLE quote_stage_prices; DELETE FROM schema_migrations WHERE version=4; DROP INDEX orders_payment_due; ALTER TABLE orders DROP COLUMN payment_due_date; DELETE FROM schema_migrations WHERE version=3; PRAGMA user_version=2;").unwrap();
    drop(conn);
    let e = Engine::open(&path).unwrap();
    let d = e.query(Query::Order { id: oid }).unwrap();
    assert_eq!(d["order"]["total_due_fils"], "63000");
    assert_eq!(d["order"]["balance_fils"], "53000");
    assert_eq!(d["services"].as_array().unwrap().len(), 1);
    assert_eq!(d["payments"].as_array().unwrap().len(), 1);
    assert!(d["order"]["payment_due_date"].is_null());
    assert_eq!(
        std::fs::read_dir(dir.path().join("backups"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn outside_service_is_atomic_exact_and_retry_safe_with_canonical_cost() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, oid) = setup(&dir.path().join("app.sqlite"));
    let request = outside_service(&oid, 1, "18000", "12000");
    let result = e.mutate(request.clone()).unwrap();
    assert_eq!(result, e.mutate(request).unwrap());
    let d = e.query(Query::Order { id: oid.clone() }).unwrap();
    assert_eq!(d["order"]["quote_fils"], "45000");
    assert_eq!(d["order"]["total_due_fils"], "63000");
    assert_eq!(d["order"]["balance_fils"], "63000");
    assert_eq!(d["cost_fils"], "12000");
    assert_eq!(d["services"].as_array().unwrap().len(), 1);
    assert_eq!(d["costs"].as_array().unwrap().len(), 1);
    assert_eq!(
        e.mutate(outside_service(&oid, 1, "18000", "12000"))
            .unwrap_err()
            .code,
        "CONFLICT"
    );
    assert!(e
        .mutate(outside_service(&oid, 2, "9000000000000", "1000"))
        .is_err());
    assert!(e.mutate(outside_service(&oid, 2, "1000", "-1")).is_err());
    let d = e.query(Query::Order { id: oid }).unwrap();
    assert_eq!(d["services"].as_array().unwrap().len(), 1);
    assert_eq!(d["cost_fils"], "12000");
    assert_eq!(d["order"]["version"], 2);
}

#[test]
fn receipts_allow_partial_payment_and_credit_without_touching_stages() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, oid) = setup(&dir.path().join("app.sqlite"));
    let stages = e.query(Query::Order { id: oid.clone() }).unwrap()["stages"].clone();
    let request = receipt(&oid, "10000");
    let first = e.mutate(request.clone()).unwrap();
    assert_eq!(first, e.mutate(request).unwrap());
    let d = e.query(Query::Order { id: oid.clone() }).unwrap();
    assert_eq!(d["order"]["balance_fils"], "35000");
    assert_eq!(d["order"]["paid_fils"], "10000");
    e.mutate(receipt(&oid, "40000")).unwrap();
    assert!(e.mutate(receipt(&oid, "0")).is_err());
    let d = e.query(Query::Order { id: oid }).unwrap();
    assert_eq!(d["order"]["balance_fils"], "-5000");
    assert_eq!(d["payments"].as_array().unwrap().len(), 2);
    assert_eq!(d["stages"], stages);
    assert_eq!(d["cost_fils"], "0");
}

#[test]
fn unpriced_order_stays_unknown_with_service_and_deposit() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(&dir.path().join("app.sqlite")).unwrap();
    let c = client(&mut e);
    let mut request = new_order(&e, &c);
    if let Action::CreateOrder {
        pricing_mode,
        items,
        ..
    } = &mut request.action
    {
        *pricing_mode = PricingMode::Unpriced;
        for item in items {
            item.unit_price_fils = None;
        }
    }
    let oid = e.mutate(request).unwrap()["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    e.mutate(outside_service(&oid, 1, "18000", "12000"))
        .unwrap();
    e.mutate(receipt(&oid, "10000")).unwrap();
    let d = e.query(Query::Order { id: oid }).unwrap();
    assert!(d["order"]["total_due_fils"].is_null());
    assert!(d["order"]["balance_fils"].is_null());
    assert_eq!(d["order"]["service_charge_fils"], "18000");
    assert_eq!(d["order"]["paid_fils"], "10000");
}

#[test]
fn report_dates_filter_financial_events_but_preserve_current_portfolio() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.sqlite");
    let (mut e, oid) = setup(&path);
    e.mutate(outside_service(&oid, 1, "18000", "12000"))
        .unwrap();
    e.mutate(receipt(&oid, "10000")).unwrap();
    let q = |from: &str, to: &str| Query::Reports {
        include_portfolio: None,
        from_date: from.into(),
        to_date: to.into(),
    };
    let r = e.query(q("2026-10-12", "2026-10-12")).unwrap();
    assert_eq!(r["portfolio"][0]["total_due_fils"], "63000");
    assert_eq!(r["activity"][0]["cost_fils"], "12000");
    assert_eq!(r["activity"][0]["received_fils"], "10000");
    assert_eq!(r["categories"][0]["category"], "outsourced");
    assert_eq!(r["stages"].as_array().unwrap().len(), 4);
    let r = e.query(q("2026-10-13", "2026-10-13")).unwrap();
    assert_eq!(r["activity"], json!([]));
    assert_eq!(r["categories"], json!([]));
    assert_eq!(r["portfolio"].as_array().unwrap().len(), 1);
    assert!(e.query(q("2026-10-14", "2026-10-13")).is_err());
    assert!(e.query(q("2020-01-01", "2026-10-13")).is_err());
    assert!(e.query(q("not-a-date", "2026-10-13")).is_err());
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute(
        "UPDATE orders SET archived=1,lifecycle='cancelled',cancellation_reason='Test' WHERE id=?1",
        [&oid],
    )
    .unwrap();
    let r = e.query(q("2026-10-12", "2026-10-12")).unwrap();
    assert_eq!(r["portfolio"], json!([]));
    assert_eq!(r["stages"], json!([]));
    assert_eq!(r["activity"][0]["received_fils"], "10000");
    assert_eq!(r["categories"][0]["amount_fils"], "12000");
}

#[test]
fn migration_two_preserves_v1_orders_and_creates_verified_safety_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("app.sqlite");
    let (e, oid) = setup(&p);
    drop(e);
    let conn = rusqlite::Connection::open(&p).unwrap();
    conn.execute_batch("DROP TABLE mobile_links; DROP TABLE mobile_imports; DELETE FROM schema_migrations WHERE version=8; DROP TABLE service_voids; DROP TABLE separate_cost_voids; DELETE FROM schema_migrations WHERE version=7; DROP VIEW order_financials; CREATE VIEW order_financials AS  WITH amounts AS (  SELECT o.id AS order_id, q.derived_total_fils AS base_quote_fils,    coalesce((SELECT sum(charge_fils) FROM outside_services WHERE order_id=o.id),0) AS service_charge_fils,    coalesce((SELECT sum(CASE kind WHEN 'receipt' THEN amount_fils ELSE -amount_fils END) FROM payment_events WHERE order_id=o.id),0) AS paid_fils,    coalesce((SELECT sum(CASE kind WHEN 'expense' THEN amount_fils ELSE -amount_fils END) FROM expense_entries WHERE order_id=o.id),0) AS cost_fils  FROM orders o LEFT JOIN quote_revisions q ON q.id=o.current_quote_revision_id  )  SELECT *, CASE WHEN base_quote_fils IS NULL THEN NULL ELSE base_quote_fils+service_charge_fils END AS total_due_fils,  CASE WHEN base_quote_fils IS NULL THEN NULL ELSE base_quote_fils+service_charge_fils-paid_fils END AS balance_fils  FROM amounts;  DROP TABLE separate_costs; DELETE FROM schema_migrations WHERE version=6; DROP TABLE monthly_salaries; DROP TABLE salary_months; DELETE FROM schema_migrations WHERE version=5; DROP TABLE quote_stage_item_prices; DROP TABLE quote_stage_prices; DELETE FROM schema_migrations WHERE version=4; DROP INDEX orders_payment_due; ALTER TABLE orders DROP COLUMN payment_due_date; DROP VIEW order_financials; DROP TABLE outside_services; DELETE FROM schema_migrations WHERE version>=2; PRAGMA user_version=1;").unwrap();
    drop(conn);
    let e = Engine::open(&p).unwrap();
    assert_eq!(
        e.query(Query::Order { id: oid }).unwrap()["order"]["quote_fils"],
        "45000"
    );
    assert_eq!(
        e.query(Query::Diagnostics).unwrap()["schema_version"],
        db::SCHEMA_VERSION
    );
    let backups: Vec<_> = std::fs::read_dir(dir.path().join("backups"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(backups.len(), 1);
    let backup = rusqlite::Connection::open(&backups[0]).unwrap();
    assert_eq!(
        backup
            .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert_eq!(
        backup
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        backup
            .query_row("SELECT count(*) FROM orders", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn migration_is_durable_with_correct_pragmas_and_no_seed_business_data() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("app.sqlite");
    let e = Engine::open(&p).unwrap();
    let boot = e.query(Query::Bootstrap).unwrap();
    assert_eq!(boot["templates"].as_array().unwrap().len(), 4);
    assert_eq!(boot["clients"], json!([]));
    assert_eq!(boot["workers"], json!([]));
    assert_eq!(e.query(Query::ListOrders).unwrap(), json!([]));
    let check = rusqlite::Connection::open(&p).unwrap();
    for table in [
        "clients",
        "workers",
        "orders",
        "order_items",
        "expense_entries",
        "work_events",
        "payment_events",
        "outside_services",
        "monthly_salaries",
        "separate_costs",
    ] {
        let count: i64 = check
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "fresh install has unexpected records in {table}");
    }
    drop(check);
    let owner = packflow_core::auth::LocalAuth::open(&dir.path().join("account.sqlite")).unwrap();
    assert!(!owner.exists().unwrap());
    assert!(owner.require().is_err());
    let d = e.query(Query::Diagnostics).unwrap();
    assert_eq!(d["foreign_keys"], 1);
    assert_eq!(d["journal_mode"], "wal");
    drop(e);
    let e = Engine::open(&p).unwrap();
    assert_eq!(
        e.query(Query::Diagnostics).unwrap()["schema_version"],
        db::SCHEMA_VERSION
    );
}
#[test]
fn order_quote_snapshots_and_restart_preserve_three_decimal_money() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("app.sqlite");
    let (mut e, oid) = setup(&p);
    let detail = e.query(Query::Order { id: oid.clone() }).unwrap();
    assert_eq!(detail["order"]["quote_fils"], "45000");
    assert_eq!(detail["items"].as_array().unwrap().len(), 2);
    let t = e.query(Query::Bootstrap).unwrap()["templates"][0].clone();
    e.mutate(mutation(json!({"kind":"save_template","id":t["id"],"expected_version":t["version"],"name":"Changed lamination","default_notes":"New instructions","archived":false}))).unwrap();
    assert_eq!(
        e.query(Query::Order { id: oid.clone() }).unwrap()["stages"][0]["name_snapshot"],
        "Lamination"
    );
    drop(e);
    let e = Engine::open(&p).unwrap();
    assert_eq!(
        e.query(Query::Order { id: oid }).unwrap()["order"]["quote_fils"],
        "45000"
    );
}
#[test]
fn lost_reply_retry_is_idempotent_and_collision_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(&dir.path().join("app.sqlite")).unwrap();
    let c = client(&mut e);
    let req = new_order(&e, &c);
    let first = e.mutate(req.clone()).unwrap();
    assert_eq!(first, e.mutate(req.clone()).unwrap());
    assert_eq!(
        e.query(Query::ListOrders)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let mut bad = serde_json::to_value(req).unwrap();
    bad["action"]["name"] = json!("different");
    assert_eq!(
        e.mutate(serde_json::from_value(bad).unwrap())
            .unwrap_err()
            .code,
        "CONFLICT"
    );
}
#[test]
fn failed_order_rolls_back_sequence_items_quote_and_audit() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("app.sqlite");
    let mut e = Engine::open(&p).unwrap();
    let c = client(&mut e);
    let mut req = serde_json::to_value(new_order(&e, &c)).unwrap();
    req["action"]["template_ids"][3] = json!(uuid::Uuid::new_v4().to_string());
    assert!(e.mutate(serde_json::from_value(req).unwrap()).is_err());
    assert_eq!(e.query(Query::ListOrders).unwrap(), json!([]));
    let req = new_order(&e, &c);
    assert_eq!(e.mutate(req).unwrap()["data"]["number"], "PF-000001");
    let conn = rusqlite::Connection::open(p).unwrap();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM quote_revisions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
#[test]
fn stage_completion_never_creates_production_or_money_and_stale_form_fails() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("app.sqlite");
    let (mut e, oid) = setup(&p);
    let s = e.query(Query::Order { id: oid.clone() }).unwrap()["stages"][0].clone();
    let a = json!({"kind":"update_stage","id":s["id"],"expected_version":1,"state":"completed","notes":"Done","business_date":"2026-10-11","reason":null});
    e.mutate(mutation(a.clone())).unwrap();
    assert_eq!(e.mutate(mutation(a)).unwrap_err().code, "CONFLICT");
    let conn = rusqlite::Connection::open(p).unwrap();
    for table in [
        "production_events",
        "dispatch_events",
        "payment_events",
        "expense_entries",
    ] {
        let n: i64 = conn
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }
    assert_eq!(
        e.query(Query::Order { id: oid }).unwrap()["order"]["quote_fils"],
        "45000"
    );
}
#[test]
fn out_of_order_completion_requires_acknowledgement() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, oid) = setup(&dir.path().join("app.sqlite"));
    let s = e.query(Query::Order { id: oid }).unwrap()["stages"][2].clone();
    let mut a = json!({"kind":"update_stage","id":s["id"],"expected_version":1,"state":"completed","notes":"","business_date":"2026-10-11","reason":null});
    assert_eq!(
        e.mutate(mutation(a.clone())).unwrap_err().field.as_deref(),
        Some("reason")
    );
    a["reason"] = json!("Earlier work tracked on paper");
    assert!(e.mutate(mutation(a)).is_ok());
}
fn entry(oid: &str, stage: &Value) -> Value {
    json!({"kind":"post_stage_entry","order_id":oid,"stage_id":stage["id"],"business_date":"2026-10-11","work":{"item_id":null,"description":"Test material batch","dimensions_text":"","input_scaled":"1125","output_scaled":null,"waste_scaled":"0","worker_id":null,"worker_minutes":40,"machine_description":"Press","machine_minutes":20},"expenses":[{"item_id":null,"category":"material","description":"Direct purchase","supplier_text":"Supplier","reference":"","pricing_basis":"unit","quantity_scaled":"1125","unit_cost_fils":"333","amount_fils":null}]})
}
#[test]
fn direct_purchase_is_exact_and_work_cost_batch_rolls_back_together() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, oid) = setup(&dir.path().join("app.sqlite"));
    let s = e.query(Query::Order { id: oid.clone() }).unwrap()["stages"][0].clone();
    let a = entry(&oid, &s);
    e.mutate(mutation(a.clone())).unwrap();
    let d = e.query(Query::Order { id: oid.clone() }).unwrap();
    assert_eq!(d["cost_fils"], "375");
    assert_eq!(d["order"]["quote_fils"], "45000");
    assert_eq!(d["work"].as_array().unwrap().len(), 1);
    let mut bad = a;
    bad["expenses"][0]["item_id"] = json!(uuid::Uuid::new_v4().to_string());
    assert!(e.mutate(mutation(bad)).is_err());
    let d = e.query(Query::Order { id: oid }).unwrap();
    assert_eq!(d["work"].as_array().unwrap().len(), 1);
    assert_eq!(d["costs"].as_array().unwrap().len(), 1);
}
#[test]
fn referenced_template_cannot_be_deleted_and_reorder_is_versioned() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, _) = setup(&dir.path().join("app.sqlite"));
    let templates = e.query(Query::Bootstrap).unwrap()["templates"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(
        e.mutate(mutation(
            json!({"kind":"delete_template","id":templates[0]["id"],"expected_version":1})
        ))
        .unwrap_err()
        .code,
        "INVARIANT"
    );
    e.mutate(mutation(json!({"kind":"reorder_templates","ids":templates.iter().rev().map(|t|t["id"].clone()).collect::<Vec<_>>(),"versions":[1,1,1,1]}))).unwrap();
    assert_eq!(
        e.query(Query::Bootstrap).unwrap()["templates"][0]["name"],
        "Finish"
    );
}
#[test]
fn sqlite_online_backup_restores_real_order_records_with_wal_writes() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("app.sqlite");
    let (_e, oid) = setup(&p);
    let conn = db::open(&p).unwrap();
    let target = dir.path().join("backup.sqlite");
    db::snapshot(&conn, &target).unwrap();
    let recovered = Engine::open(&target).unwrap();
    assert_eq!(
        recovered.query(Query::Order { id: oid }).unwrap()["order"]["quote_fils"],
        "45000"
    );
    assert!(db::snapshot(&conn, &target).is_err());
}
#[test]
fn newer_schema_and_modified_migration_are_refused_without_data_loss() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("app.sqlite");
    let (_e, _) = setup(&p);
    let conn = rusqlite::Connection::open(&p).unwrap();
    conn.execute_batch(&format!("PRAGMA user_version={}", db::SCHEMA_VERSION + 1))
        .unwrap();
    assert_eq!(Engine::open(&p).err().unwrap().code, "SCHEMA_TOO_NEW");
    conn.execute_batch(&format!(
        "PRAGMA user_version={}; UPDATE schema_migrations SET checksum='wrong'",
        db::SCHEMA_VERSION
    ))
    .unwrap();
    assert!(Engine::open(&p).is_err());
    assert_eq!(
        conn.query_row("SELECT count(*) FROM orders", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}
#[test]
fn worker_serializes_concurrent_writes_without_duplicate_numbers() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("app.sqlite");
    let mut e = Engine::open(&p).unwrap();
    let c = client(&mut e);
    let req = new_order(&e, &c);
    drop(e);
    let worker = DbWorker::start(p).unwrap();
    let threads = (0..8)
        .map(|_| {
            let w = worker.clone();
            let mut r = req.clone();
            r.request_id = uuid::Uuid::new_v4().to_string();
            std::thread::spawn(move || w.mutate(r).unwrap())
        })
        .collect::<Vec<_>>();
    for t in threads {
        t.join().unwrap();
    }
    let orders = worker.query(Query::ListOrders).unwrap();
    let numbers = orders
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["number"].as_str().unwrap())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(numbers.len(), 8);
}
#[test]
fn exact_rounding_and_overflow_guard() {
    assert_eq!(line_total(1125, 333).unwrap(), 375);
    assert_eq!(line_total(500, 1).unwrap(), 1);
    assert_eq!(line_total(499, 1).unwrap(), 0);
    assert!(line_total(1_000_000_000, MAX_MONEY).is_err());
    assert!(line_total(1, MAX_MONEY + 1).is_err());
}

#[test]
fn zero_quote_unpriced_and_native_precision_validation_are_distinct() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(&dir.path().join("app.sqlite")).unwrap();
    let c = client(&mut e);
    let base = serde_json::to_value(new_order(&e, &c)).unwrap()["action"].clone();
    let mut zero = base.clone();
    zero["pricing_mode"] = json!("fixed");
    zero["fixed_total_fils"] = json!("0");
    for item in zero["items"].as_array_mut().unwrap() {
        item["unit_price_fils"] = Value::Null;
    }
    let oid = e.mutate(mutation(zero.clone())).unwrap()["data"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        e.query(Query::Order { id: oid }).unwrap()["order"]["quote_fils"],
        "0"
    );
    let mut unpriced = zero.clone();
    unpriced["pricing_mode"] = json!("unpriced");
    unpriced["fixed_total_fils"] = Value::Null;
    let oid = e.mutate(mutation(unpriced)).unwrap()["data"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(e.query(Query::Order { id: oid }).unwrap()["order"]["quote_fils"].is_null());
    zero["fixed_total_fils"] = json!("0.001");
    assert_eq!(e.mutate(mutation(zero)).unwrap_err().code, "VALIDATION");
    assert_eq!(
        e.query(Query::ListOrders)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn sql_constraints_reject_float_money_cross_order_links_and_history_deletion() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("app.sqlite");
    let (mut e, first) = setup(&p);
    let client = e.query(Query::Bootstrap).unwrap()["clients"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let first_stage = e.query(Query::Order { id: first }).unwrap()["stages"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let req = new_order(&e, &client);
    let second = e.mutate(req).unwrap()["data"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let conn = db::open(&p).unwrap();
    assert!(conn
        .execute("DELETE FROM clients WHERE id=?1", [&client])
        .is_err());
    assert!(conn
        .execute("UPDATE quote_revisions SET derived_total_fils=0.5", [])
        .is_err());
    assert!(conn.execute("INSERT INTO expense_entries(id,order_id,stage_id,kind,category,description,pricing_basis,amount_fils,business_date,created_at,actor_id) VALUES(?1,?2,?3,'expense','material','Invalid cross-order purchase','total',500,'2026-10-11',?4,?5)", rusqlite::params![uuid::Uuid::new_v4().to_string(),second,first_stage,db::now(),MANAGER_ID]).is_err());
}

#[test]
fn dashboard_counts_are_not_truncated_by_order_list_limit() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(&dir.path().join("app.sqlite")).unwrap();
    let c = client(&mut e);
    let base = new_order(&e, &c);
    for _ in 0..201 {
        let mut req = base.clone();
        req.request_id = uuid::Uuid::new_v4().to_string();
        e.mutate(req).unwrap();
    }
    assert_eq!(
        e.query(Query::ListOrders)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        200
    );
    let boot = e.query(Query::Bootstrap).unwrap();
    assert_eq!(boot["stats"]["active_orders"], 201);
    assert_eq!(boot["stats"]["ordered_boxes"], 201 * 150);
    assert_eq!(boot["clients"][0]["order_count"], 201);
    let report = e
        .query(Query::Reports {
            include_portfolio: None,
            from_date: "2026-10-01".into(),
            to_date: "2026-10-31".into(),
        })
        .unwrap();
    assert_eq!(report["portfolio"].as_array().unwrap().len(), 201);
    assert_eq!(
        e.query(Query::ClientInsights).unwrap()["clients"][0]["order_count"],
        201
    );
}

#[test]
fn portable_backup_restore_validates_before_replacing_and_preserves_safety_copy() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("business.sqlite");
    let (mut engine, oid) = setup(&path);
    engine.mutate(receipt(&oid, "12345")).unwrap();
    let before = engine.query(Query::Order { id: oid.clone() }).unwrap();
    let target = dir.path().join("portable.pfbackup");
    engine.backup(&target).unwrap();
    assert!(engine.backup(&target).is_err());
    engine.mutate(receipt(&oid, "1000")).unwrap();
    let after = engine.query(Query::Order { id: oid.clone() }).unwrap();
    let mut corrupt = std::fs::read(&target).unwrap();
    *corrupt.last_mut().unwrap() ^= 1;
    let bad = dir.path().join("bad.pfbackup");
    std::fs::write(&bad, corrupt).unwrap();
    assert!(engine.restore(&bad).is_err());
    assert_eq!(
        engine.query(Query::Order { id: oid.clone() }).unwrap(),
        after
    );
    let result = engine.restore(&target).unwrap();
    assert_eq!(
        engine.query(Query::Order { id: oid.clone() }).unwrap(),
        before
    );
    drop(engine);
    let mut reopened = Engine::open(&path).unwrap();
    assert_eq!(
        reopened.query(Query::Order { id: oid.clone() }).unwrap(),
        before
    );
    reopened
        .restore(Path::new(result["safety_backup"].as_str().unwrap()))
        .unwrap();
    assert_eq!(reopened.query(Query::Order { id: oid }).unwrap(), after);
}

#[test]
fn corrections_preserve_history_reverse_finance_once_and_survive_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("business.sqlite");
    let (mut e, oid) = setup(&path);
    let service = e.mutate(outside_service(&oid, 1, "8000", "4000")).unwrap()["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    e.mutate(receipt(&oid, "12345")).unwrap();
    let detail = e.query(Query::Order { id: oid.clone() }).unwrap();
    let payment = detail["payments"][0]["id"].as_str().unwrap();
    let separate=e.mutate(mutation(json!({"kind":"add_separate_cost","category":"machine","amount_fils":"1000","description":"Correction fixture","business_date":"2026-10-12"}))).unwrap()["data"]["id"].as_str().unwrap().to_string();
    for (kind, id) in [
        ("payment", payment),
        ("service", service.as_str()),
        ("separate_cost", separate.as_str()),
    ] {
        let request = mutation(
            json!({"kind":"reverse_entry","entry_type":kind,"id":id,"reason":"Wrong entry","business_date":"2026-10-13"}),
        );
        let first = e.mutate(request.clone()).unwrap();
        assert_eq!(e.mutate(request.clone()).unwrap(), first);
        let mut again = request.clone();
        again.request_id = uuid::Uuid::new_v4().to_string();
        assert!(e.mutate(again).is_err());
    }
    let d = e.query(Query::Order { id: oid.clone() }).unwrap();
    assert_eq!(d["order"]["total_due_fils"], "45000");
    assert_eq!(d["order"]["paid_fils"], "0");
    assert_eq!(d["cost_fils"], "0");
    assert_eq!(d["payments"].as_array().unwrap().len(), 2);
    assert_eq!(d["costs"].as_array().unwrap().len(), 2);
    assert_eq!(e.query(Query::SeparateCosts).unwrap()["total_fils"], "0");
    let report = e
        .query(Query::Reports {
            from_date: "2026-10-13".into(),
            to_date: "2026-10-13".into(),
            include_portfolio: None,
        })
        .unwrap();
    assert_eq!(report["activity"][0]["cost_fils"], "-5000");
    assert_eq!(report["activity"][0]["received_fils"], "-12345");
    drop(e);
    let e = Engine::open(&path).unwrap();
    assert_eq!(e.query(Query::Order { id: oid }).unwrap(), d);
}

#[test]
fn stage_work_and_linked_cost_corrections_are_ordered_and_immutable() {
    let dir = tempfile::tempdir().unwrap();
    let (mut e, oid) = setup(&dir.path().join("business.sqlite"));
    let stage = e.query(Query::Order { id: oid.clone() }).unwrap()["stages"][0].clone();
    e.mutate(mutation(entry(&oid, &stage))).unwrap();
    let d = e.query(Query::Order { id: oid.clone() }).unwrap();
    let work = d["work"][0]["id"].as_str().unwrap();
    let cost = d["costs"][0]["id"].as_str().unwrap();
    let reverse_work = mutation(
        json!({"kind":"reverse_entry","entry_type":"work","id":work,"reason":"Wrong work","business_date":"2026-10-12"}),
    );
    assert!(e.mutate(reverse_work.clone()).is_err());
    let early = mutation(
        json!({"kind":"reverse_entry","entry_type":"expense","id":cost,"reason":"Wrong cost","business_date":"2026-10-10"}),
    );
    assert!(e.mutate(early).is_err());
    let reversal = mutation(
        json!({"kind":"reverse_entry","entry_type":"expense","id":cost,"reason":"Wrong cost","business_date":"2026-10-12"}),
    );
    e.mutate(reversal).unwrap();
    e.mutate(reverse_work).unwrap();
    let d = e.query(Query::Order { id: oid }).unwrap();
    assert_eq!(d["cost_fils"], "0");
    assert_eq!(d["work"].as_array().unwrap().len(), 2);
    assert_eq!(d["stages"][0]["state"], stage["state"]);
}
