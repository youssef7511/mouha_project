use crate::{
    error::{DomainError, Result},
    models::Action,
    money::{bounded_total, integer, MAX_MONEY},
    service::{date, id, open_order, rows, text, version, Applied},
};
use chrono::NaiveDate;
use rusqlite::{params, Connection, Transaction};
use serde_json::{json, Value};

pub(crate) fn apply(tx: &Transaction, action: &Action, actor: &str, time: &str) -> Result<Applied> {
    match action {
        Action::AddSeparateCost {
            category,
            amount_fils,
            description,
            business_date,
        } => {
            let amount = integer(amount_fils, MAX_MONEY, "amount_fils")?;
            let description = text(description, 1000, "description", true)?;
            date(business_date, "business_date")?;
            let rid = uuid::Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO separate_costs VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![
                    rid,
                    category.as_str(),
                    amount,
                    description,
                    business_date,
                    time,
                    actor
                ],
            )?;
            Ok((
                json!({"id":rid}),
                vec!["separate_costs", "reports"],
                "separate_cost",
                rid,
                "separate_cost_added",
                business_date.clone(),
                None,
            ))
        }
        Action::SetPaymentDueDate {
            order_id,
            expected_version,
            payment_due_date,
        } => {
            id(order_id)?;
            version(tx, "orders", order_id, Some(*expected_version))?;
            let eligible: bool = tx.query_row(
                "SELECT lifecycle<>'cancelled' AND archived=0 FROM orders WHERE id=?1",
                [order_id],
                |r| r.get(0),
            )?;
            if !eligible {
                return Err(DomainError::validation(
                    "order_id",
                    "Choose an active, non-cancelled order.",
                ));
            }
            if let Some(due) = payment_due_date {
                date(due, "payment_due_date")?;
            }
            tx.execute(
                "UPDATE orders SET payment_due_date=?2,version=version+1,updated_at=?3 WHERE id=?1",
                params![order_id, payment_due_date, time],
            )?;
            Ok((
                json!({"order_id":order_id}),
                vec!["orders", "clients"],
                "order",
                order_id.clone(),
                "payment_due_date_updated",
                crate::service::business_today(),
                None,
            ))
        }
        Action::AddOutsideService {
            order_id,
            expected_version,
            description,
            supplier_text,
            charge_fils,
            supplier_cost_fils,
            business_date,
            reference,
        } => {
            id(order_id)?;
            open_order(tx, order_id)?;
            version(tx, "orders", order_id, Some(*expected_version))?;
            date(business_date, "business_date")?;
            let description = text(description, 1000, "description", true)?;
            let supplier = text(supplier_text, 200, "supplier_text", false)?;
            let reference = text(reference, 200, "reference", false)?;
            let charge = integer(charge_fils, MAX_MONEY, "charge_fils")?;
            let cost = integer(supplier_cost_fils, MAX_MONEY, "supplier_cost_fils")?;
            let (base, services, costs): (Option<i64>, i64, i64) = tx.query_row(
                "SELECT base_quote_fils,service_charge_fils,cost_fils FROM order_financials WHERE order_id=?1",
                [order_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
            bounded_total(i128::from(services) + i128::from(charge))?;
            if let Some(base) = base {
                bounded_total(i128::from(base) + i128::from(services) + i128::from(charge))?;
            }
            bounded_total(i128::from(costs) + i128::from(cost))?;
            let expense_id = uuid::Uuid::new_v4().to_string();
            let service_id = uuid::Uuid::new_v4().to_string();
            tx.execute("INSERT INTO expense_entries(id,order_id,kind,category,description,supplier_text,reference,pricing_basis,amount_fils,business_date,created_at,actor_id) VALUES(?1,?2,'expense','outsourced',?3,?4,?5,'total',?6,?7,?8,?9)",
                params![expense_id,order_id,description,supplier,reference,cost,business_date,time,actor])?;
            tx.execute("INSERT INTO outside_services(id,order_id,description,charge_fils,expense_id,business_date,created_at,actor_id) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                params![service_id,order_id,description,charge,expense_id,business_date,time,actor])?;
            tx.execute(
                "UPDATE orders SET version=version+1,updated_at=?2 WHERE id=?1",
                params![order_id, time],
            )?;
            Ok((
                json!({"id":service_id,"order_id":order_id}),
                vec!["orders", "costs", "reports"],
                "order",
                order_id.clone(),
                "outside_service_added",
                business_date.clone(),
                None,
            ))
        }
        Action::ReceivePayment {
            order_id,
            amount_fils,
            business_date,
            method,
            reference,
            notes,
        } => {
            id(order_id)?;
            date(business_date, "business_date")?;
            let eligible: bool = tx.query_row(
                "SELECT lifecycle<>'cancelled' AND archived=0 FROM orders WHERE id=?1",
                [order_id],
                |r| r.get(0),
            )?;
            if !eligible {
                return Err(DomainError::validation(
                    "order_id",
                    "Choose an active, non-cancelled order.",
                ));
            }
            let amount = integer(amount_fils, MAX_MONEY, "amount_fils")?;
            if amount == 0 {
                return Err(DomainError::validation(
                    "amount_fils",
                    "Enter a payment greater than zero.",
                ));
            }
            if !["cash", "bank", "card", "other"].contains(&method.as_str()) {
                return Err(DomainError::validation(
                    "method",
                    "Choose a supported payment method.",
                ));
            }
            let reference = text(reference, 200, "reference", false)?;
            let notes = text(notes, 2000, "notes", false)?;
            let paid: i64 = tx.query_row(
                "SELECT paid_fils FROM order_financials WHERE order_id=?1",
                [order_id],
                |r| r.get(0),
            )?;
            bounded_total(i128::from(paid) + i128::from(amount))?;
            let payment_id = uuid::Uuid::new_v4().to_string();
            tx.execute("INSERT INTO payment_events(id,order_id,kind,amount_fils,business_date,method,reference,notes,created_at,actor_id) VALUES(?1,?2,'receipt',?3,?4,?5,?6,?7,?8,?9)",
                params![payment_id,order_id,amount,business_date,method,reference,notes,time,actor])?;
            Ok((
                json!({"id":payment_id,"order_id":order_id}),
                vec!["orders", "payments", "reports"],
                "order",
                order_id.clone(),
                "payment_received",
                business_date.clone(),
                None,
            ))
        }
        _ => Err(DomainError::validation(
            "action",
            "Unsupported financial action.",
        )),
    }
}

pub(crate) fn client_insights(conn: &Connection, today: &str) -> Result<Value> {
    let mut clients = rows(
        conn,
        "SELECT id,name,contact_name,archived FROM clients ORDER BY archived,normalized_name",
        [],
    )?;
    let orders = rows(conn, "WITH balances AS (
        SELECT o.id,o.client_id,o.number,o.name,o.archived,o.due_date,o.payment_due_date,
          f.total_due_fils,coalesce((SELECT sum(CASE kind WHEN 'receipt' THEN amount_fils ELSE -amount_fils END)
            FROM payment_events WHERE order_id=o.id AND business_date<=?1),0) AS paid_fils,
          (SELECT count(*) FROM payment_events p WHERE order_id=o.id AND kind='receipt' AND business_date<=?1 AND p.amount_fils>coalesce((SELECT sum(a.amount_fils) FROM refund_allocations a JOIN payment_events r ON r.id=a.refund_event_id WHERE a.receipt_event_id=p.id AND r.business_date<=?1),0)) AS receipt_count,
          EXISTS(SELECT 1 FROM payment_events p WHERE order_id=o.id AND kind='receipt' AND business_date>o.payment_due_date AND business_date<=?1 AND p.amount_fils>coalesce((SELECT sum(a.amount_fils) FROM refund_allocations a JOIN payment_events r ON r.id=a.refund_event_id WHERE a.receipt_event_id=p.id AND r.business_date<=?1),0)) AS has_receipt_after_due
        FROM orders o JOIN order_financials f ON f.order_id=o.id WHERE o.lifecycle<>'cancelled'
        ) SELECT id,client_id,number,name,archived,due_date,payment_due_date,
        CAST(total_due_fils AS TEXT) AS total_due_fils,CAST(paid_fils AS TEXT) AS paid_fils,
        CAST(total_due_fils-paid_fils AS TEXT) AS balance_fils,receipt_count,has_receipt_after_due
        FROM balances ORDER BY number DESC", [today])?;
    let payments = rows(
        conn,
        "SELECT p.id,o.id AS order_id,o.client_id,o.number,o.name,p.kind,
        CAST(p.amount_fils AS TEXT) AS amount_fils,p.business_date,p.method,p.reference
        FROM payment_events p JOIN orders o ON o.id=p.order_id WHERE p.business_date<=?1
        ORDER BY p.business_date DESC,p.created_at DESC,p.rowid DESC",
        [today],
    )?;
    for client in &mut clients {
        let cid = client["id"].as_str().unwrap();
        let client_orders: Vec<_> = orders
            .iter()
            .filter(|order| order["client_id"] == cid)
            .collect();
        let mut quoted = 0_i128;
        let mut outstanding = 0_i128;
        let mut credit = 0_i128;
        let mut overdue = 0_i128;
        let mut priced = 0;
        let mut unpriced = 0;
        let mut partial = 0;
        let mut settled = 0;
        let mut overdue_orders = 0;
        let mut late_receipt_orders = 0;
        let mut instalment_orders = 0;
        let mut missing_dates = 0;
        let mut dated_orders = 0;
        let mut oldest_days = 0;
        for order in &client_orders {
            if let Some(total) = order["total_due_fils"].as_str() {
                let total: i128 = total
                    .parse()
                    .map_err(|_| DomainError::storage("Invalid stored amount."))?;
                let paid: i128 = order["paid_fils"]
                    .as_str()
                    .unwrap()
                    .parse()
                    .map_err(|_| DomainError::storage("Invalid stored amount."))?;
                let balance = total - paid;
                quoted += total;
                outstanding += balance.max(0);
                credit += (-balance).max(0);
                if total > 0 {
                    priced += 1;
                    if balance <= 0 {
                        settled += 1;
                    }
                    if paid > 0 && balance > 0 {
                        partial += 1;
                    }
                    if order["receipt_count"].as_i64().unwrap_or(0) > 1 {
                        instalment_orders += 1;
                    }
                    if let Some(due) = order["payment_due_date"].as_str() {
                        dated_orders += 1;
                        if order["has_receipt_after_due"] == 1 {
                            late_receipt_orders += 1;
                        }
                        if due < today && balance > 0 {
                            overdue += balance;
                            overdue_orders += 1;
                            let days = (NaiveDate::parse_from_str(today, "%Y-%m-%d").unwrap()
                                - NaiveDate::parse_from_str(due, "%Y-%m-%d").map_err(|_| {
                                    DomainError::storage("Invalid stored payment date.")
                                })?)
                            .num_days();
                            oldest_days = oldest_days.max(days);
                        }
                    } else {
                        missing_dates += 1;
                    }
                }
            } else {
                unpriced += 1;
            }
        }
        // Include historical/cancelled-order receipts; never offset one order's balance with another's credit.
        let mut received = 0_i128;
        for payment in payments
            .iter()
            .filter(|payment| payment["client_id"] == cid)
        {
            let amount: i128 = payment["amount_fils"]
                .as_str()
                .unwrap()
                .parse()
                .map_err(|_| DomainError::storage("Invalid stored amount."))?;
            received += if payment["kind"] == "receipt" {
                amount
            } else {
                -amount
            };
        }
        let values = json!({"order_count":client_orders.len(),"quoted_fils":quoted.to_string(),
            "net_received_fils":received.to_string(),"outstanding_fils":outstanding.to_string(),
            "credit_fils":credit.to_string(),"overdue_fils":overdue.to_string(),
            "priced_orders":priced,"unpriced_orders":unpriced,"partial_orders":partial,
            "settled_orders":settled,"overdue_orders":overdue_orders,"late_receipt_orders":late_receipt_orders,
            "instalment_orders":instalment_orders,"missing_deadline_orders":missing_dates,
            "dated_orders":dated_orders,"oldest_overdue_days":oldest_days});
        client
            .as_object_mut()
            .unwrap()
            .extend(values.as_object().unwrap().clone());
    }
    Ok(json!({"business_today":today,"clients":clients,"orders":orders,"payments":payments}))
}

pub(crate) fn separate_costs(conn: &Connection) -> Result<Value> {
    let records=rows(conn,"SELECT e.id,(SELECT reason FROM separate_cost_voids v WHERE v.original_id=e.id) AS reason,(SELECT business_date FROM separate_cost_voids v WHERE v.original_id=e.id) AS reversal_date,EXISTS(SELECT 1 FROM separate_cost_voids v WHERE v.original_id=e.id) AS reversed,e.category,CAST(e.amount_fils AS TEXT) AS amount_fils,e.description,e.business_date,e.created_at,a.display_name AS actor_name FROM separate_costs e JOIN actors a ON a.id=e.actor_id ORDER BY e.business_date DESC,e.created_at DESC,e.rowid DESC LIMIT 200",[])?;
    let categories=rows(conn,"SELECT category,CAST(sum(amount_fils) AS TEXT) AS amount_fils,count(*) AS count FROM separate_costs e WHERE NOT EXISTS(SELECT 1 FROM separate_cost_voids v WHERE v.original_id=e.id) GROUP BY category ORDER BY category",[])?;
    let mut total = 0_i128;
    let mut count = 0_i64;
    for category in &categories {
        total += category["amount_fils"]
            .as_str()
            .unwrap()
            .parse::<i128>()
            .map_err(|_| DomainError::storage("Invalid separate cost amount."))?;
        count += category["count"].as_i64().unwrap();
    }
    Ok(
        json!({"records":records,"categories":categories,"total_fils":total.to_string(),"count":count}),
    )
}
pub(crate) fn reports(
    conn: &Connection,
    from: &str,
    to: &str,
    include_portfolio: bool,
) -> Result<Value> {
    date(from, "from_date")?;
    date(to, "to_date")?;
    let days = (NaiveDate::parse_from_str(to, "%Y-%m-%d").unwrap()
        - NaiveDate::parse_from_str(from, "%Y-%m-%d").unwrap())
    .num_days();
    if !(0..=365).contains(&days) {
        return Err(DomainError::validation(
            "to_date",
            "Choose a date range of at most 366 days, with the end after the start.",
        ));
    }
    // Portfolio is current and uncapped. Event charts alone use the selected dates.
    let portfolio = if include_portfolio {
        rows(conn, "SELECT o.id,o.number,o.name,o.lifecycle,c.name AS client_name,
        CAST(f.total_due_fils AS TEXT) AS total_due_fils,CAST(f.cost_fils AS TEXT) AS cost_fils,
        CAST(f.paid_fils AS TEXT) AS paid_fils,CAST(f.balance_fils AS TEXT) AS balance_fils,
        (SELECT coalesce(sum(ordered_boxes),0) FROM order_items WHERE order_id=o.id) AS ordered_boxes
        FROM orders o JOIN clients c ON c.id=o.client_id JOIN order_financials f ON f.order_id=o.id
        WHERE o.archived=0 AND o.lifecycle<>'cancelled' ORDER BY o.number", [])?
    } else {
        Vec::new()
    };
    let portfolio_summary=rows(conn,"SELECT count(*) AS order_count,coalesce(sum((SELECT coalesce(sum(ordered_boxes),0) FROM order_items WHERE order_id=o.id)),0) AS ordered_boxes,CAST(coalesce(sum(f.total_due_fils),0) AS TEXT) AS total_due_fils,coalesce(sum(f.total_due_fils IS NULL),0) AS unpriced_orders FROM orders o JOIN order_financials f ON f.order_id=o.id WHERE o.archived=0 AND o.lifecycle<>'cancelled'",[])?.into_iter().next().unwrap();
    let activity = rows(conn, "SELECT business_date,CAST(sum(cost) AS TEXT) AS cost_fils,CAST(sum(received) AS TEXT) AS received_fils FROM (
        SELECT e.business_date,CASE e.kind WHEN 'expense' THEN e.amount_fils ELSE -e.amount_fils END AS cost,0 AS received
        FROM expense_entries e WHERE e.business_date BETWEEN ?1 AND ?2
        UNION ALL SELECT business_date,amount_fils,0 FROM separate_costs WHERE business_date BETWEEN ?1 AND ?2
        UNION ALL SELECT v.business_date,-e.amount_fils,0 FROM separate_cost_voids v JOIN separate_costs e ON e.id=v.original_id WHERE v.business_date BETWEEN ?1 AND ?2
        UNION ALL SELECT p.business_date,0,CASE p.kind WHEN 'receipt' THEN p.amount_fils ELSE -p.amount_fils END
        FROM payment_events p WHERE p.business_date BETWEEN ?1 AND ?2
        ) GROUP BY business_date ORDER BY business_date", params![from,to])?;
    let categories = rows(conn, "SELECT category,CAST(sum(amount) AS TEXT) AS amount_fils FROM (
        SELECT category,CASE kind WHEN 'expense' THEN amount_fils ELSE -amount_fils END AS amount FROM expense_entries WHERE business_date BETWEEN ?1 AND ?2
        UNION ALL SELECT 'separate_'||category,amount_fils FROM separate_costs WHERE business_date BETWEEN ?1 AND ?2
        UNION ALL SELECT 'separate_'||e.category,-e.amount_fils FROM separate_cost_voids v JOIN separate_costs e ON e.id=v.original_id WHERE v.business_date BETWEEN ?1 AND ?2
        ) GROUP BY category ORDER BY sum(amount) DESC", params![from,to])?;
    let stages = rows(conn, "SELECT s.name_snapshot AS name,sum(s.state='not_started') AS not_started,
        sum(s.state='in_progress') AS in_progress,sum(s.state='completed') AS completed
        FROM order_stages s JOIN orders o ON o.id=s.order_id WHERE s.retired=0 AND o.archived=0 AND o.lifecycle<>'cancelled'
        GROUP BY s.name_snapshot ORDER BY min(s.position),s.name_snapshot", [])?;
    Ok(
        json!({"portfolio":portfolio,"portfolio_summary":portfolio_summary,"activity":activity,"categories":categories,"stages":stages,
        "from_date":from,"to_date":to}),
    )
}
