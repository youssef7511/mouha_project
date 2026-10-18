//! Read-only, consistent, complete Excel snapshots. No credentials or mutation payloads.
use crate::{
    error::Result,
    service::{business_today, date, rows, text},
    DomainError,
};
use chrono::Utc;
use rusqlite::{params, Connection};
use rust_xlsxwriter::{Format, Workbook, XlsxError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{io::Write, path::Path};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportPage {
    Dashboard,
    Orders,
    Clients,
    Workers,
    Costs,
    Payments,
    Reports,
    Settings,
    Stock,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExportRequest {
    pub page: ExportPage,
    #[serde(default)]
    pub search: String,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub month: Option<String>,
    pub order_id: Option<String>,
}
pub struct ExcelFile {
    pub filename: String,
    pub bytes: Vec<u8>,
}
impl ExcelFile {
    pub fn save(&self, path: &Path) -> Result<()> {
        if path
            .extension()
            .and_then(|s| s.to_str())
            .is_none_or(|s| !s.eq_ignore_ascii_case("xlsx"))
        {
            return Err(DomainError::validation(
                "filename",
                "Choose a filename ending in .xlsx.",
            ));
        }
        let parent = path
            .parent()
            .ok_or_else(|| DomainError::validation("filename", "Choose an export folder."))?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        file.write_all(&self.bytes)?;
        file.as_file().sync_all()?;
        file.persist_noclobber(path).map_err(|e| {
            if e.error.kind() == std::io::ErrorKind::AlreadyExists {
                DomainError::validation(
                    "filename",
                    "That file already exists. Choose a new export filename.",
                )
            } else {
                DomainError::from(e.error)
            }
        })?;
        Ok(())
    }
}
impl From<XlsxError> for DomainError {
    fn from(e: XlsxError) -> Self {
        eprintln!("Excel export failed: {e}");
        Self::new("EXPORT", "Could not create the Excel file. No records were changed. Check the selected folder and Excel's row/text limits.")
    }
}
fn heading(key: &str) -> String {
    let (base, suffix) = if let Some(base) = key.strip_suffix("_fils") {
        (base, " (KWD)")
    } else if let Some(base) = key.strip_suffix("_scaled") {
        (base, "")
    } else {
        (key, "")
    };
    let name = base
        .split('_')
        .map(|s| {
            if s == "id" {
                "ID".into()
            } else {
                let mut chars = s.chars();
                chars
                    .next()
                    .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
                    .unwrap_or_default()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    name + suffix
}
fn sheet(book: &mut Workbook, name: &str, columns: &str, records: &[Value]) -> Result<()> {
    if records.len() > 1_048_575 {
        return Err(DomainError::validation(
            "export",
            "Too many rows for one Excel sheet. Narrow the export search or date range.",
        ));
    }
    let cols: Vec<_> = columns.split(',').collect();
    let title = Format::new()
        .set_bold()
        .set_font_color("#FFFFFF")
        .set_background_color("#43594B");
    let decimal = Format::new().set_num_format("#,##0.000;[Red]-#,##0.000");
    let ws = book.add_worksheet();
    ws.set_name(name)?;
    ws.set_freeze_panes(1, 0)?;
    for (col, key) in cols.iter().enumerate() {
        ws.write_string_with_format(0, col as u16, heading(key), &title)?;
        ws.set_column_width(
            col as u16,
            if *key == "description" || *key == "notes" || *key == "value" {
                42
            } else {
                23
            },
        )?;
    }
    for (row, record) in records.iter().enumerate() {
        for (col, key) in cols.iter().enumerate() {
            let value = &record[*key];
            if value.is_null() {
                continue;
            }
            let cell_row = row as u32 + 1;
            let col = col as u16;
            if key.ends_with("_fils") || key.ends_with("_scaled") {
                let raw = value
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| value.to_string());
                let n: i128 = raw.parse().map_err(|_| {
                    DomainError::storage("Invalid stored amount; export cancelled.")
                })?;
                // Excel only preserves 15 significant decimal digits. Keep larger values as exact text.
                if n.unsigned_abs() <= 999_999_999_999_999 {
                    ws.write_number_with_format(cell_row, col, n as f64 / 1000.0, &decimal)?;
                } else {
                    let unsigned = n.unsigned_abs();
                    ws.write_string(
                        cell_row,
                        col,
                        format!(
                            "{}{}.{:03}",
                            if n < 0 { "-" } else { "" },
                            unsigned / 1000,
                            unsigned % 1000
                        ),
                    )?;
                }
            } else if let Some(n) = value.as_i64() {
                if n.unsigned_abs() <= 999_999_999_999_999 {
                    ws.write_number(cell_row, col, n as f64)?;
                } else {
                    ws.write_string(cell_row, col, n.to_string())?;
                }
            } else {
                // Explicit strings: descriptions beginning '=' / '+' are never formulas or hyperlinks.
                ws.write_string(
                    cell_row,
                    col,
                    value
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| value.to_string()),
                )?;
            }
        }
    }
    ws.autofilter(0, 0, records.len() as u32, (cols.len() - 1) as u16)?;
    Ok(())
}
const ORDER_COLUMNS: &str = "number,name,client_name,due_date,payment_due_date,lifecycle,archived,ordered_boxes,quote_fils,service_charge_fils,total_due_fils,paid_fils,balance_fils,cost_fils,completed_stages,stage_count,notes,id";
const EXPENSE_COLUMNS: &str = "order_number,order_name,client_name,stage_name,box_description,business_date,category,description,supplier_text,reference,pricing_basis,quantity_scaled,unit_cost_fils,amount_fils,kind,reversed,reason,original_id,id";
const PAYMENT_COLUMNS: &str = "order_number,order_name,client_name,business_date,kind,amount_fils,method,reference,notes,reason,id";
const SERVICE_COLUMNS: &str = "order_number,order_name,client_name,business_date,description,supplier_text,reference,charge_fils,supplier_cost_fils,voided,void_date,reason,id";
const MATCHING: &str = "WITH matching AS (SELECT o.id FROM orders o JOIN clients c ON c.id=o.client_id WHERE (?1=0 OR (o.archived=0 AND o.lifecycle<>'cancelled')) AND instr(lower(o.number||' '||o.name||' '||c.name),lower(?2))>0 AND (?3='' OR o.id=?3)) ";

pub fn create(conn: &Connection, request: &ExportRequest) -> Result<ExcelFile> {
    let search = text(&request.search, 200, "search", false)?;
    if let Some(id) = &request.order_id {
        uuid::Uuid::parse_str(id)
            .map_err(|_| DomainError::validation("order", "Choose a valid order."))?;
    }
    let today = business_today();
    let default_from = if matches!(request.page, ExportPage::Reports | ExportPage::Dashboard) {
        format!("{}01", &today[..8])
    } else {
        "0001-01-01".into()
    };
    let from = request.from_date.as_deref().unwrap_or(&default_from);
    let to = request.to_date.as_deref().unwrap_or(
        if matches!(request.page, ExportPage::Reports | ExportPage::Dashboard) {
            &today
        } else {
            "9999-12-31"
        },
    );
    date(from, "from_date")?;
    date(to, "to_date")?;
    if from > to {
        return Err(DomainError::validation(
            "date_range",
            "The start date must not be after the end date.",
        ));
    }
    let month = request.month.as_deref().unwrap_or("");
    if !month.is_empty() {
        date(&format!("{month}-01"), "month")?;
        if month.len() != 7 {
            return Err(DomainError::validation(
                "month",
                "Choose a valid salary month.",
            ));
        }
    }
    let tx = conn.unchecked_transaction()?;
    let mut book = Workbook::new();
    let page = serde_json::to_value(request.page)?
        .as_str()
        .unwrap()
        .to_owned();
    let info = vec![
        json!({"field":"Page","value":page}),
        json!({"field":"Exported at (UTC)","value":Utc::now().to_rfc3339()}),
        json!({"field":"Business date (Kuwait)","value":today}),
        json!({"field":"Currency","value":"KWD; amounts have three decimals. Blank = not priced / not applicable; zero is a recorded zero."}),
        json!({"field":"Scope","value":"All matching records, across every page. Current balances are lifetime snapshots. Report search filters only order performance; its dated ledgers, chart data and totals cover all orders, including archived/cancelled history."}),
        json!({"field":"Search (order name, number, client)","value":search}),
        json!({"field":"From date","value":from}),
        json!({"field":"To date","value":to}),
        json!({"field":"Salary month (blank = all)","value":month}),
        json!({"field":"Precision","value":"Amounts too large for Excel's 15 significant digits are exact decimal text. IDs, phones, descriptions and dates are text. Formulas are not generated from user input."}),
        json!({"field":"Corrections","value":"Original entries remain visible. Expense reversals and customer refunds are negative. Separate-cost corrections are negative on the correction date. Do not add outside-service supplier costs again to the Costs sheet."}),
        json!({"field":"Salaries","value":"Calculated payroll, separate from actual payments and order costs. Archived salary records are marked and excluded from salary totals."}),
        json!({"field":"Restore","value":"Excel is a read-only data export, not a restorable backup. Use .pfbackup for backup/restore. No account credentials are exported."}),
    ];
    sheet(&mut book, "Export information", "field,value", &info)?;
    let active = matches!(
        request.page,
        ExportPage::Payments | ExportPage::Reports | ExportPage::Dashboard
    );
    let order_id = request.order_id.as_deref().unwrap_or("");
    // Report charts cover all ledger entries, including historical archived/cancelled orders.
    // Its search box narrows only the current order-performance sheet, as in the app.
    let ledger_active = if request.page == ExportPage::Reports {
        false
    } else {
        active
    };
    let ledger_search = if request.page == ExportPage::Reports {
        ""
    } else {
        &search
    };
    let ledger_order = if request.page == ExportPage::Reports {
        ""
    } else {
        order_id
    };
    let order_rows = || {
        rows(&tx, &(MATCHING.to_owned() + "SELECT o.number,o.name,c.name AS client_name,o.due_date,o.payment_due_date,o.lifecycle,o.archived,o.notes,o.id,f.base_quote_fils AS quote_fils,f.service_charge_fils,f.total_due_fils,f.paid_fils,f.balance_fils,f.cost_fils,(SELECT coalesce(sum(ordered_boxes),0) FROM order_items WHERE order_id=o.id) AS ordered_boxes,(SELECT count(*) FROM order_stages WHERE order_id=o.id AND retired=0) AS stage_count,(SELECT count(*) FROM order_stages WHERE order_id=o.id AND retired=0 AND state='completed') AS completed_stages FROM orders o JOIN matching m ON m.id=o.id JOIN clients c ON c.id=o.client_id JOIN order_financials f ON f.order_id=o.id ORDER BY o.created_at DESC,o.number DESC"), params![active,search,order_id])
    };
    let expenses = || {
        rows(&tx, &(MATCHING.to_owned() + "SELECT o.number AS order_number,o.name AS order_name,c.name AS client_name,s.name_snapshot AS stage_name,i.description AS box_description,e.business_date,e.category,e.description,e.supplier_text,e.reference,e.pricing_basis,e.quantity_scaled,e.unit_cost_fils,CASE e.kind WHEN 'expense' THEN e.amount_fils ELSE -e.amount_fils END AS amount_fils,e.kind,EXISTS(SELECT 1 FROM expense_entries r WHERE r.original_id=e.id) AS reversed,e.reason,e.original_id,e.id FROM expense_entries e JOIN matching m ON m.id=e.order_id JOIN orders o ON o.id=e.order_id JOIN clients c ON c.id=o.client_id LEFT JOIN order_stages s ON s.id=e.stage_id LEFT JOIN order_items i ON i.id=e.item_id WHERE e.business_date BETWEEN ?4 AND ?5 ORDER BY e.business_date,e.created_at,e.id"), params![ledger_active,ledger_search,ledger_order,from,to])
    };
    let payments = || {
        rows(&tx, &(MATCHING.to_owned() + "SELECT o.number AS order_number,o.name AS order_name,c.name AS client_name,p.business_date,p.kind,CASE p.kind WHEN 'receipt' THEN p.amount_fils ELSE -p.amount_fils END AS amount_fils,p.method,p.reference,p.notes,p.reason,p.id FROM payment_events p JOIN matching m ON m.id=p.order_id JOIN orders o ON o.id=p.order_id JOIN clients c ON c.id=o.client_id WHERE p.business_date BETWEEN ?4 AND ?5 ORDER BY p.business_date,p.created_at,p.id"), params![ledger_active,ledger_search,ledger_order,from,to])
    };
    let services = || {
        rows(&tx, &(MATCHING.to_owned() + "SELECT o.number AS order_number,o.name AS order_name,c.name AS client_name,s.business_date,s.description,e.supplier_text,e.reference,s.charge_fils,e.amount_fils AS supplier_cost_fils,v.original_id IS NOT NULL AS voided,v.business_date AS void_date,v.reason,s.id FROM outside_services s JOIN matching m ON m.id=s.order_id JOIN orders o ON o.id=s.order_id JOIN clients c ON c.id=o.client_id JOIN expense_entries e ON e.id=s.expense_id LEFT JOIN service_voids v ON v.original_id=s.id WHERE s.business_date BETWEEN ?4 AND ?5 OR v.business_date BETWEEN ?4 AND ?5 ORDER BY s.business_date,s.id"), params![ledger_active,ledger_search,ledger_order,from,to])
    };
    match request.page {
        ExportPage::Orders | ExportPage::Payments | ExportPage::Dashboard | ExportPage::Reports => {
            sheet(
                &mut book,
                if request.page == ExportPage::Payments {
                    "Order balances"
                } else {
                    "Orders"
                },
                ORDER_COLUMNS,
                &order_rows()?,
            )?;
            if request.page != ExportPage::Dashboard {
                sheet(&mut book, "Payments", PAYMENT_COLUMNS, &payments()?)?;
                sheet(&mut book, "Outside services", SERVICE_COLUMNS, &services()?)?;
            }
        }
        _ => {}
    }
    if request.page == ExportPage::Orders {
        let items = rows(&tx, &(MATCHING.to_owned() + "SELECT o.number AS order_number,o.name AS order_name,i.description,i.dimensions_text,i.ordered_boxes,qi.unit_price_fils,i.id FROM order_items i JOIN matching m ON m.id=i.order_id JOIN orders o ON o.id=i.order_id LEFT JOIN quote_items qi ON qi.item_id=i.id AND qi.quote_revision_id=o.current_quote_revision_id ORDER BY o.number,i.position"),params![active,search,order_id])?;
        sheet(
            &mut book,
            "Box types",
            "order_number,order_name,description,dimensions_text,ordered_boxes,unit_price_fils,id",
            &items,
        )?;
        let stages=rows(&tx,&(MATCHING.to_owned()+"SELECT o.number AS order_number,s.position+1 AS sequence,s.name_snapshot AS stage_name,s.state,s.retired,s.notes,s.first_completed_date,s.id FROM order_stages s JOIN matching m ON m.id=s.order_id JOIN orders o ON o.id=s.order_id ORDER BY o.number,s.position"),params![active,search,order_id])?;
        sheet(
            &mut book,
            "Production stages",
            "order_number,sequence,stage_name,state,retired,notes,first_completed_date,id",
            &stages,
        )?;
        let quotes=rows(&tx,&(MATCHING.to_owned()+"SELECT o.number AS order_number,s.name_snapshot AS stage_name,q.*,q.derived_total_fils AS total_fils FROM quote_stage_prices q JOIN orders o ON o.current_quote_revision_id=q.quote_revision_id JOIN matching m ON m.id=o.id JOIN order_stages s ON s.id=q.stage_id ORDER BY o.number,s.position"),params![active,search,order_id])?;
        sheet(&mut book,"Stage customer pricing","order_number,stage_name,pricing_mode,fixed_total_fils,quantity_scaled,unit_price_fils,total_fils",&quotes)?;
        let stage_items = rows(&tx, &(MATCHING.to_owned()+"SELECT o.number AS order_number,s.name_snapshot AS stage_name,i.description AS box_description,p.unit_price_fils FROM quote_stage_item_prices p JOIN orders o ON o.current_quote_revision_id=p.quote_revision_id JOIN matching m ON m.id=o.id JOIN order_stages s ON s.id=p.stage_id JOIN order_items i ON i.id=p.item_id ORDER BY o.number,s.position,i.position"),params![active,search,order_id])?;
        sheet(
            &mut book,
            "Stage box prices",
            "order_number,stage_name,box_description,unit_price_fils",
            &stage_items,
        )?;
        let work=rows(&tx,&(MATCHING.to_owned()+"SELECT o.number AS order_number,s.name_snapshot AS stage_name,w.business_date,w.kind,w.description,w.dimensions_text,w.input_scaled,w.output_scaled,w.waste_scaled,k.name AS worker_name,w.worker_minutes,w.machine_description,w.machine_minutes,w.reason,w.original_id,w.id FROM work_events w JOIN matching m ON m.id=w.order_id JOIN orders o ON o.id=w.order_id JOIN order_stages s ON s.id=w.stage_id LEFT JOIN workers k ON k.id=w.worker_id ORDER BY w.business_date,w.created_at,w.id"),params![active,search,order_id])?;
        sheet(&mut book,"Stage work","order_number,stage_name,business_date,kind,description,dimensions_text,input_scaled,output_scaled,waste_scaled,worker_name,worker_minutes,machine_description,machine_minutes,reason,original_id,id",&work)?;
    }
    if matches!(
        request.page,
        ExportPage::Costs | ExportPage::Orders | ExportPage::Reports
    ) {
        sheet(&mut book, "Stage costs", EXPENSE_COLUMNS, &expenses()?)?;
    }
    if request.page == ExportPage::Costs {
        sheet(&mut book, "Outside services", SERVICE_COLUMNS, &services()?)?;
    }
    if matches!(request.page, ExportPage::Costs | ExportPage::Reports) {
        let separate=rows(&tx,"SELECT c.id,c.business_date,c.category,c.description,c.amount_fils,'expense' AS kind,NULL AS reason,NULL AS original_id FROM separate_costs c WHERE c.business_date BETWEEN ?1 AND ?2 UNION ALL SELECT v.original_id||'-correction',v.business_date,c.category,c.description,-c.amount_fils,'reversal',v.reason,v.original_id FROM separate_cost_voids v JOIN separate_costs c ON c.id=v.original_id WHERE v.business_date BETWEEN ?1 AND ?2 ORDER BY business_date,id",params![from,to])?;
        sheet(
            &mut book,
            "Separate costs",
            "business_date,category,description,amount_fils,kind,reason,original_id,id",
            &separate,
        )?;
    }
    if matches!(request.page, ExportPage::Reports | ExportPage::Dashboard) {
        let reports = crate::finance::reports(&tx, from, to, false)?;
        let activity = reports["activity"].as_array().unwrap();
        let total = |key: &str| -> Result<String> {
            let sum = activity.iter().try_fold(0_i128, |sum, row| {
                let amount = row[key]
                    .as_str()
                    .unwrap()
                    .parse::<i128>()
                    .map_err(|_| DomainError::storage("Invalid report amount."))?;
                sum.checked_add(amount)
                    .ok_or_else(|| DomainError::storage("Report total is too large to export."))
            })?;
            Ok(sum.to_string())
        };
        sheet(
            &mut book,
            "Period totals",
            "from_date,to_date,cost_fils,received_fils",
            &[
                json!({"from_date":from,"to_date":to,"cost_fils":total("cost_fils")?,"received_fils":total("received_fils")?}),
            ],
        )?;
        sheet(
            &mut book,
            "Portfolio summary",
            "order_count,ordered_boxes,total_due_fils,unpriced_orders",
            &[reports["portfolio_summary"].clone()],
        )?;
        for (name, key, cols) in [
            (
                "Daily activity",
                "activity",
                "business_date,cost_fils,received_fils",
            ),
            ("Cost categories", "categories", "category,amount_fils"),
            (
                "Stage progress",
                "stages",
                "name,not_started,in_progress,completed",
            ),
        ] {
            sheet(&mut book, name, cols, reports[key].as_array().unwrap())?;
        }
    }
    if request.page == ExportPage::Clients {
        let clients=rows(&tx,"SELECT c.*,(SELECT count(*) FROM orders WHERE client_id=c.id) AS order_count FROM clients c ORDER BY c.archived,c.normalized_name,c.id",[])?;
        sheet(
            &mut book,
            "Clients",
            "name,contact_name,phone,email,address,archived,order_count,id",
            &clients,
        )?;
        let insights = crate::finance::client_insights(&tx, &today)?;
        sheet(&mut book,"Client payment statistics","name,archived,order_count,quoted_fils,net_received_fils,outstanding_fils,credit_fils,overdue_fils,priced_orders,unpriced_orders,partial_orders,settled_orders,overdue_orders,late_receipt_orders,instalment_orders,missing_deadline_orders,oldest_overdue_days,id",insights["clients"].as_array().unwrap())?;
    }
    if request.page == ExportPage::Workers {
        sheet(
            &mut book,
            "Workers",
            "name,role,phone,email,active,archived,id",
            &rows(&tx, "SELECT * FROM workers ORDER BY archived,name,id", [])?,
        )?;
        let salaries=rows(&tx,"SELECT s.*,m.divisor FROM monthly_salaries s JOIN salary_months m ON m.month=s.month WHERE ?1='' OR s.month=?1 ORDER BY s.month,s.worker_name_snapshot,s.id",[month])?;
        sheet(&mut book,"Monthly salaries","month,worker_name_snapshot,basic_fils,divisor,normal_days_scaled,hours_per_day_scaled,fridays,overtime_hours_scaled,normal_fils,friday_fils,overtime_fils,total_fils,archived,notes,id",&salaries)?;
        sheet(&mut book,"Salary totals","month,workers,normal_fils,friday_fils,overtime_fils,total_fils",&rows(&tx,"SELECT month,count(*) AS workers,sum(normal_fils) AS normal_fils,sum(friday_fils) AS friday_fils,sum(overtime_fils) AS overtime_fils,sum(total_fils) AS total_fils FROM monthly_salaries WHERE archived=0 AND (?1='' OR month=?1) GROUP BY month ORDER BY month",[month])?)?;
        sheet(
            &mut book,
            "Monthly divisors",
            "month,divisor",
            &rows(
                &tx,
                "SELECT month,divisor FROM salary_months WHERE ?1='' OR month=?1 ORDER BY month",
                [month],
            )?,
        )?;
    }
    if request.page == ExportPage::Settings {
        sheet(
            &mut book,
            "Production stage templates",
            "position,name,default_notes,archived,id",
            &rows(&tx, "SELECT * FROM stage_templates ORDER BY position", [])?,
        )?;
        sheet(
            &mut book,
            "Business settings",
            "key,value_json",
            &rows(&tx, "SELECT key,value_json FROM settings ORDER BY key", [])?,
        )?;
    }
    if request.page == ExportPage::Stock {
        sheet(
            &mut book,
            "Stock (deferred)",
            "description,quantity_scaled",
            &[],
        )?;
    }
    let bytes = book.save_to_buffer()?;
    tx.commit()?;
    Ok(ExcelFile {
        filename: format!(
            "PackFlow-{page}-{}.xlsx",
            Utc::now().format("%Y%m%d-%H%M%S")
        ),
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};
    #[test]
    fn excel_distinguishes_unknown_zero_and_values_beyond_numeric_precision() {
        let mut book = Workbook::new();
        sheet(&mut book, "Precision", "unknown_fils,zero_fils,large_fils,notes", &[json!({
            "unknown_fils":null,"zero_fils":"0","large_fils":i64::MAX.to_string(),"notes":"=SUM(1,2)"
        })]).unwrap();
        let mut zip = zip::ZipArchive::new(Cursor::new(book.save_to_buffer().unwrap())).unwrap();
        let mut cells = String::new();
        zip.by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut cells)
            .unwrap();
        assert!(!cells.contains("r=\"A2\""));
        assert!(cells.contains("r=\"B2\"") && cells.contains("<v>0</v>"));
        assert!(!cells.contains("<f>"));
        let mut strings = String::new();
        zip.by_name("xl/sharedStrings.xml")
            .unwrap()
            .read_to_string(&mut strings)
            .unwrap();
        assert!(strings.contains("9223372036854775.807"));
        assert!(strings.contains("=SUM(1,2)"));
    }
}
