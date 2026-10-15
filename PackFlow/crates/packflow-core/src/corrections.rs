use crate::{
    error::{DomainError, Result},
    models::{Action, CorrectionKind},
    service::{date, id, text, Applied},
};
use rusqlite::{params, Transaction};
use serde_json::json;

pub(crate) fn apply(tx: &Transaction, action: &Action, actor: &str, time: &str) -> Result<Applied> {
    let Action::ReverseEntry {
        entry_type,
        id: original,
        reason,
        business_date,
    } = action
    else {
        unreachable!()
    };
    id(original)?;
    let reason = text(reason, 1000, "reason", true)?;
    date(business_date, "business_date")?;
    let table = match entry_type {
        CorrectionKind::Expense => "expense_entries",
        CorrectionKind::Work => "work_events",
        CorrectionKind::Payment => "payment_events",
        CorrectionKind::Service => "outside_services",
        CorrectionKind::SeparateCost => "separate_costs",
    };
    let original_date: String = tx.query_row(
        &format!("SELECT business_date FROM {table} WHERE id=?1"),
        [original],
        |r| r.get(0),
    )?;
    if business_date < &original_date {
        return Err(DomainError::validation(
            "business_date",
            "A correction cannot precede its original entry.",
        ));
    }
    let rid = uuid::Uuid::new_v4().to_string();
    let order: Option<String> = if matches!(entry_type, CorrectionKind::SeparateCost) {
        None
    } else {
        Some(tx.query_row(
            &format!("SELECT order_id FROM {table} WHERE id=?1"),
            [original],
            |r| r.get(0),
        )?)
    };
    match entry_type {
        CorrectionKind::Expense | CorrectionKind::Service => {
            let expense = if matches!(entry_type, CorrectionKind::Service) {
                tx.execute(
                    "INSERT INTO service_voids VALUES(?1,?2,?3,?4,?5)",
                    params![original, business_date, reason, time, actor],
                )?;
                tx.query_row(
                    "SELECT expense_id FROM outside_services WHERE id=?1",
                    [original],
                    |r| r.get::<_, String>(0),
                )?
            } else {
                let linked: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM outside_services WHERE expense_id=?1)",
                    [original],
                    |r| r.get(0),
                )?;
                if linked {
                    return Err(DomainError::validation("id","Reverse the outside service to correct its charge and supplier cost together."));
                }
                original.clone()
            };
            let inserted = tx.execute("INSERT INTO expense_entries(id,order_id,stage_id,item_id,work_event_id,worker_id,kind,original_id,category,description,supplier_text,reference,pricing_basis,quantity_scaled,unit_cost_fils,amount_fils,business_date,reason,created_at,actor_id) SELECT ?2,order_id,stage_id,item_id,work_event_id,worker_id,'reversal',id,category,description,supplier_text,reference,pricing_basis,quantity_scaled,unit_cost_fils,amount_fils,?3,?4,?5,?6 FROM expense_entries WHERE id=?1 AND kind='expense'",params![expense,rid,business_date,reason,time,actor])?;
            if inserted != 1 {
                return Err(DomainError::validation(
                    "id",
                    "Only an original cost can be reversed.",
                ));
            }
        }
        CorrectionKind::Work => {
            let linked: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM expense_entries e WHERE work_event_id=?1 AND kind='expense' AND NOT EXISTS(SELECT 1 FROM expense_entries r WHERE r.original_id=e.id))",[original],|r|r.get(0))?;
            if linked {
                return Err(DomainError::validation(
                    "id",
                    "Reverse the costs linked to this work entry first.",
                ));
            }
            let inserted = tx.execute("INSERT INTO work_events(id,order_id,stage_id,item_id,kind,original_id,business_date,description,dimensions_text,input_scaled,output_scaled,waste_scaled,worker_id,worker_minutes,machine_description,machine_minutes,reason,created_at,actor_id) SELECT ?2,order_id,stage_id,item_id,'reversal',id,?3,description,dimensions_text,input_scaled,output_scaled,waste_scaled,worker_id,worker_minutes,machine_description,machine_minutes,?4,?5,?6 FROM work_events WHERE id=?1 AND kind='work'", params![original,rid,business_date,reason,time,actor])?;
            if inserted != 1 {
                return Err(DomainError::validation(
                    "id",
                    "Only an original work entry can be reversed.",
                ));
            }
        }
        CorrectionKind::Payment => {
            let remaining: i64 = tx.query_row("SELECT amount_fils-coalesce((SELECT sum(amount_fils) FROM refund_allocations WHERE receipt_event_id=p.id),0) FROM payment_events p WHERE id=?1 AND kind='receipt'",[original],|r|r.get(0))?;
            if remaining <= 0 {
                return Err(DomainError::validation(
                    "id",
                    "This receipt has already been fully reversed or refunded.",
                ));
            }
            tx.execute("INSERT INTO payment_events(id,order_id,kind,amount_fils,business_date,method,reference,notes,reason,created_at,actor_id) SELECT ?2,order_id,'refund',?3,?4,method,reference,'Receipt correction',?5,?6,?7 FROM payment_events WHERE id=?1",params![original,rid,remaining,business_date,reason,time,actor])?;
            tx.execute(
                "INSERT INTO refund_allocations VALUES(?1,?2,?3,?4)",
                params![rid, original, order, remaining],
            )?;
        }
        CorrectionKind::SeparateCost => {
            tx.execute(
                "INSERT INTO separate_cost_voids VALUES(?1,?2,?3,?4,?5)",
                params![original, business_date, reason, time, actor],
            )?;
        }
    }
    if let Some(ref oid) = order {
        tx.execute(
            "UPDATE orders SET version=version+1,updated_at=?2 WHERE id=?1",
            params![oid, time],
        )?;
    }
    Ok((
        json!({"id":rid,"original_id":original}),
        vec!["orders", "reports", "costs"],
        if order.is_some() {
            "order"
        } else {
            "separate_cost"
        },
        order.unwrap_or(original.clone()),
        "entry_reversed",
        business_date.clone(),
        Some(reason),
    ))
}
