use packflow_mobile_core::{models::*, money, pricing, Engine};
use serde_json::{json, Value};
fn request(action: Value) -> Mutation {
    serde_json::from_value(json!({"request_id":uuid::Uuid::new_v4().to_string(),"action":action}))
        .unwrap()
}
fn save(e: &mut Engine, action: Value) -> Value {
    e.mutate(request(action)).unwrap()
}
fn client(e: &mut Engine) -> String {
    save(e,json!({"kind":"save_client","id":null,"expected_version":null,"name":"عميل الكويت","contact_name":"Amal","phone":"0012345","email":"","address":"Kuwait","notes":"Local only","archived":false}))["id"].as_str().unwrap().into()
}
fn input(e: &Engine, cid: &str) -> Value {
    let snapshot = e.query(Query::Snapshot).unwrap();
    let t = snapshot["templates"].as_array().unwrap();
    let a = uuid::Uuid::new_v4().to_string();
    let b = uuid::Uuid::new_v4().to_string();
    json!({"client_id":cid,"name":"Mixed gift boxes","due_date":"2026-11-10","payment_due_date":"2026-11-15","notes":"Sheets: entered as described","status":"draft","items":[{"id":a,"description":"Small","dimensions":"10 x 20","quantity":100},{"id":b,"description":"Large","dimensions":"20 x 40","quantity":50}],"stages":[
        {"template_id":t[0]["id"],"pricing_mode":"fixed","fixed_total_fils":"12500","quantity_scaled":null,"unit_price_fils":null,"item_prices":[]},
        {"template_id":t[1]["id"],"pricing_mode":"per_box","fixed_total_fils":null,"quantity_scaled":null,"unit_price_fils":null,"item_prices":[{"item_id":a,"unit_price_fils":"250"},{"item_id":b,"unit_price_fils":"400"}]},
        {"template_id":t[2]["id"],"pricing_mode":"per_unit","fixed_total_fils":null,"quantity_scaled":"3333","unit_price_fils":"125","item_prices":[]},
        {"template_id":t[3]["id"],"pricing_mode":"fixed","fixed_total_fils":"0","quantity_scaled":null,"unit_price_fils":null,"item_prices":[]}
    ]})
}
fn receipt(amount: &str) -> Value {
    json!({"amount_fils":amount,"business_date":"2026-10-18","method":"bank","reference":"Deposit","notes":""})
}
fn create(e: &mut Engine, cid: &str) -> (Mutation, String) {
    let i = input(e, cid);
    let m = request(
        json!({"kind":"save_order","id":null,"expected_version":null,"input":i,"reason":"","deposit":receipt("10000")}),
    );
    let id = e.mutate(m.clone()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    (m, id)
}
#[test]
fn fresh_phone_has_only_templates_and_uses_separate_mobile_numbering() {
    let d = tempfile::tempdir().unwrap();
    let mut e = Engine::open(&d.path().join("mobile.sqlite"), &d.path().join("backups")).unwrap();
    let b = e.query(Query::Snapshot).unwrap();
    assert_eq!(b["templates"].as_array().unwrap().len(), 4);
    assert_eq!(b["clients"], json!([]));
    assert_eq!(b["orders"], json!([]));
    assert_eq!(b["payments"], json!([]));
    let c = client(&mut e);
    let (_, id) = create(&mut e, &c);
    let o = e.query(Query::Order { id }).unwrap();
    assert!(o["number"].as_str().unwrap().starts_with("MOB-"));
    assert!(o["number"].as_str().unwrap().ends_with("000001"));
}
#[test]
fn mixed_prices_deposit_and_reply_retry_survive_restart_exactly_once() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("mobile.sqlite");
    let mut e = Engine::open(&path, &d.path().join("backups")).unwrap();
    let cid = client(&mut e);
    let (m, id) = create(&mut e, &cid);
    let o = e.query(Query::Order { id: id.clone() }).unwrap();
    assert_eq!(o["total_fils"], "57917");
    assert_eq!(o["received_fils"], "10000");
    assert_eq!(o["balance_fils"], "47917");
    assert_eq!(
        o["document"]["stage_totals"],
        json!(["12500", "45000", "417", "0"])
    );
    drop(e);
    let mut e = Engine::open(&path, &d.path().join("backups")).unwrap();
    assert_eq!(e.mutate(m.clone()).unwrap()["id"], id);
    let b = e.query(Query::Snapshot).unwrap();
    assert_eq!(b["orders"].as_array().unwrap().len(), 1);
    assert_eq!(b["payments"].as_array().unwrap().len(), 1);
    let mut collision = m;
    collision.action = Action::Profile {
        name: "Different".into(),
    };
    assert_eq!(e.mutate(collision).unwrap_err().code, "REQUEST_COLLISION");
}
#[test]
fn failed_deposit_rolls_back_order_number_quote_and_audit() {
    let d = tempfile::tempdir().unwrap();
    let mut e = Engine::open(&d.path().join("m.sqlite"), &d.path().join("backups")).unwrap();
    let c = client(&mut e);
    let i = input(&e, &c);
    assert!(e.mutate(request(json!({"kind":"save_order","id":null,"expected_version":null,"input":i,"reason":"","deposit":receipt("0")}))).is_err());
    assert!(e.query(Query::Snapshot).unwrap()["orders"]
        .as_array()
        .unwrap()
        .is_empty());
    let (_, id) = create(&mut e, &c);
    assert!(e.query(Query::Order { id }).unwrap()["number"]
        .as_str()
        .unwrap()
        .ends_with("000001"));
}
#[test]
fn unknown_prices_remain_unknown_and_per_box_prices_follow_item_identity() {
    let d = tempfile::tempdir().unwrap();
    let mut e = Engine::open(&d.path().join("m.sqlite"), &d.path().join("b")).unwrap();
    let c = client(&mut e);
    let mut i = input(&e, &c);
    i["items"].as_array_mut().unwrap().reverse();
    let parsed: OrderInput = serde_json::from_value(i.clone()).unwrap();
    assert_eq!(pricing::calculate(&parsed).unwrap().total, Some(57917));
    i["stages"][0]["pricing_mode"] = json!("unpriced");
    i["stages"][0]["fixed_total_fils"] = Value::Null;
    let q = pricing::calculate(&serde_json::from_value(i.clone()).unwrap()).unwrap();
    assert_eq!(q.total, None);
    assert_eq!(q.known, 45417);
    let r = save(
        &mut e,
        json!({"kind":"save_order","id":null,"expected_version":null,"input":i,"reason":"","deposit":receipt("10000")}),
    );
    let o = e
        .query(Query::Order {
            id: r["id"].as_str().unwrap().into(),
        })
        .unwrap();
    assert!(o["balance_fils"].is_null());
    assert_eq!(o["received_fils"], "10000");
}
#[test]
fn revisions_preserve_stage_names_and_stale_updates_fail() {
    let d = tempfile::tempdir().unwrap();
    let mut e = Engine::open(&d.path().join("m.sqlite"), &d.path().join("b")).unwrap();
    let c = client(&mut e);
    let (_, id) = create(&mut e, &c);
    let b = e.query(Query::Snapshot).unwrap();
    let t = &b["templates"][0];
    save(
        &mut e,
        json!({"kind":"save_template","id":t["id"],"expected_version":t["version"],"name":"Lamination new","notes":"","archived":true}),
    );
    let mut i = e.query(Query::Order { id: id.clone() }).unwrap()["document"]["input"].clone();
    i["name"] = json!("Revised boxes");
    let req = request(
        json!({"kind":"save_order","id":id,"expected_version":1,"input":i,"reason":"Customer changed the description","deposit":null}),
    );
    e.mutate(req.clone()).unwrap();
    assert!(e
        .mutate(request(serde_json::to_value(req.action).unwrap()))
        .is_err());
    let o = e.query(Query::Order { id: id.clone() }).unwrap();
    assert_eq!(o["version"], 2);
    assert_eq!(o["document"]["stage_names"][0], "Lamination");
    assert_eq!(
        e.query(Query::History { order_id: id }).unwrap()["revisions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
#[test]
fn receipt_correction_is_immutable_single_and_signed_and_credit_is_separate() {
    let d = tempfile::tempdir().unwrap();
    let mut e = Engine::open(&d.path().join("m.sqlite"), &d.path().join("b")).unwrap();
    let c = client(&mut e);
    let (_, id) = create(&mut e, &c);
    let payment = save(
        &mut e,
        json!({"kind":"receive","order_id":id,"receipt":receipt("100000")}),
    )["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        e.query(Query::Order { id: id.clone() }).unwrap()["balance_fils"],
        "-52083"
    );
    assert!(e.mutate(request(json!({"kind":"correct_receipt","id":payment,"reason":"wrong","business_date":"2026-10-17"}))).is_err());
    save(
        &mut e,
        json!({"kind":"correct_receipt","id":payment,"reason":"Duplicated deposit","business_date":"2026-10-18"}),
    );
    assert!(e.mutate(request(json!({"kind":"correct_receipt","id":payment,"reason":"Again","business_date":"2026-10-18"}))).is_err());
    let b = e.query(Query::Snapshot).unwrap();
    assert_eq!(b["payments"].as_array().unwrap().len(), 3);
    assert!(b["payments"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["signed_fils"] == "-100000"));
    assert_eq!(
        e.query(Query::Order { id }).unwrap()["received_fils"],
        "10000"
    );
}
#[test]
fn checked_backup_restore_has_safety_undo_and_rejects_corruption_before_writes() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("m.sqlite");
    let mut e = Engine::open(&path, &d.path().join("b")).unwrap();
    let c = client(&mut e);
    let (_, id) = create(&mut e, &c);
    let bytes = e.backup_bytes().unwrap();
    save(
        &mut e,
        json!({"kind":"receive","order_id":id,"receipt":receipt("5000")}),
    );
    let mut broken = bytes.clone();
    *broken.last_mut().unwrap() ^= 1;
    assert!(e.restore(&broken, "RESTORE").is_err());
    assert_eq!(
        e.query(Query::Order { id: id.clone() }).unwrap()["received_fils"],
        "15000"
    );
    assert!(e.restore(&bytes, "NO").is_err());
    let restored = e.restore(&bytes, "RESTORE").unwrap();
    assert_eq!(
        e.query(Query::Order { id: id.clone() }).unwrap()["received_fils"],
        "10000"
    );
    let safety = e
        .read_backup(restored["safety_backup"].as_str().unwrap())
        .unwrap();
    e.restore(&safety, "RESTORE").unwrap();
    drop(e);
    let e = Engine::open(&path, &d.path().join("b")).unwrap();
    assert_eq!(
        e.query(Query::Order { id }).unwrap()["received_fils"],
        "15000"
    );
    assert!(e.read_backup("../../desktop.sqlite").is_err());
}
#[test]
fn template_reordering_is_complete_versioned_and_snapshot_safe() {
    let d = tempfile::tempdir().unwrap();
    let mut e = Engine::open(&d.path().join("m.sqlite"), &d.path().join("b")).unwrap();
    let b = e.query(Query::Snapshot).unwrap();
    let templates = b["templates"].as_array().unwrap();
    let ids = templates
        .iter()
        .rev()
        .map(|t| t["id"].clone())
        .collect::<Vec<_>>();
    let versions = vec![1; 4];
    save(
        &mut e,
        json!({"kind":"reorder_templates","ids":ids,"versions":versions}),
    );
    assert_eq!(
        e.query(Query::Snapshot).unwrap()["templates"][0]["name"],
        "Finish"
    );
    assert!(e
        .mutate(request(
            json!({"kind":"reorder_templates","ids":ids,"versions":versions})
        ))
        .is_err());
}
#[test]
fn unsafe_prices_duplicates_and_cross_client_receipts_are_rejected() {
    let d = tempfile::tempdir().unwrap();
    let mut e = Engine::open(&d.path().join("m.sqlite"), &d.path().join("b")).unwrap();
    let c = client(&mut e);
    let mut i = input(&e, &c);
    i["stages"][0]["fixed_total_fils"] = json!("9000000000000");
    assert!(pricing::calculate(&serde_json::from_value(i).unwrap()).is_err());
    assert_eq!(money::line_total(3333, 125).unwrap(), 417);
    assert!(money::line_total(0, 1).is_err());
    let (_, id) = create(&mut e, &c);
    let other = client(&mut e);
    let mut i = e.query(Query::Order { id: id.clone() }).unwrap()["document"]["input"].clone();
    i["client_id"] = json!(other);
    assert!(e.mutate(request(json!({"kind":"save_order","id":id,"expected_version":1,"input":i,"reason":"Move customer","deposit":null}))).is_err());
}
