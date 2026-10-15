use crate::{
    error::{DomainError, Result},
    models::OrderScope,
    service::{rows, text},
};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
pub(crate) fn page(
    conn: &Connection,
    search: &str,
    requested: i64,
    size: i64,
    scope: &OrderScope,
) -> Result<Value> {
    let search = text(search, 200, "search", false)?;
    if requested < 1 || ![10, 25, 50, 100].contains(&size) {
        return Err(DomainError::validation(
            "page",
            "Choose a positive page and a page size of 10, 25, 50 or 100.",
        ));
    }
    let active = matches!(scope, OrderScope::Active);
    // instr treats %, _ and quotes as literal text; the search value is bound, never interpolated.
    let total:i64=conn.query_row("SELECT count(*) FROM orders o JOIN clients c ON c.id=o.client_id WHERE (?1=0 OR (o.archived=0 AND o.lifecycle<>'cancelled')) AND instr(lower(o.number||' '||o.name||' '||c.name),lower(?2))>0",params![active,search],|r|r.get(0))?;
    let last = if total == 0 {
        1
    } else {
        (total - 1) / size + 1
    };
    let page = requested.min(last);
    let offset = (page - 1) * size;
    let records=rows(conn,"SELECT o.*,c.name AS client_name,CAST(q.derived_total_fils AS TEXT) AS quote_fils,q.pricing_mode,
        CAST(f.service_charge_fils AS TEXT) AS service_charge_fils,CAST(f.total_due_fils AS TEXT) AS total_due_fils,
        CAST(f.paid_fils AS TEXT) AS paid_fils,CAST(f.balance_fils AS TEXT) AS balance_fils,CAST(f.cost_fils AS TEXT) AS cost_fils,
        (SELECT coalesce(sum(ordered_boxes),0) FROM order_items WHERE order_id=o.id) AS ordered_boxes,
        (SELECT count(*) FROM order_stages WHERE order_id=o.id AND retired=0) AS stage_count,
        (SELECT count(*) FROM order_stages WHERE order_id=o.id AND retired=0 AND state='completed') AS completed_stages
        FROM orders o JOIN clients c ON c.id=o.client_id JOIN order_financials f ON f.order_id=o.id LEFT JOIN quote_revisions q ON q.id=o.current_quote_revision_id
        WHERE (?1=0 OR (o.archived=0 AND o.lifecycle<>'cancelled')) AND instr(lower(o.number||' '||o.name||' '||c.name),lower(?2))>0
        ORDER BY o.created_at DESC,o.number DESC LIMIT ?3 OFFSET ?4",params![active,search,size,offset])?;
    Ok(json!({"records":records,"total":total,"page":page,"page_size":size}))
}
