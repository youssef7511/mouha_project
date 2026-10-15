use crate::{
    error::{DomainError, Result},
    models::Action,
    money::{bounded_total, integer, MAX_MONEY},
    service::{business_today, date, id, rows, text, version, Applied},
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};

fn month(value: &str) -> Result<()> {
    date(&format!("{value}-01"), "month")
}
fn month_state(conn: &Connection, key: &str) -> Result<(i64, i64)> {
    Ok(conn
        .query_row(
            "SELECT divisor,version FROM salary_months WHERE month=?1",
            [key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .unwrap_or((26, 0)))
}
fn check_month(conn: &Connection, key: &str, expected: i64) -> Result<i64> {
    month(key)?;
    let (divisor, current) = month_state(conn, key)?;
    if expected != current {
        return Err(DomainError::new(
            "CONFLICT",
            "This salary month changed. Reload before saving.",
        ));
    }
    Ok(divisor)
}
fn touch_month(tx: &Transaction, key: &str, time: &str) -> Result<()> {
    tx.execute("INSERT INTO salary_months(month,divisor,created_at,updated_at) VALUES(?1,26,?2,?2) ON CONFLICT(month) DO UPDATE SET version=version+1,updated_at=excluded.updated_at",params![key,time])?;
    Ok(())
}
fn rounded(numerator: i128, denominator: i128) -> Result<i64> {
    bounded_total((numerator + denominator / 2) / denominator)
}
pub(crate) fn calculate(
    basic: i64,
    days: i64,
    hours: i64,
    fridays: i64,
    overtime: i64,
    divisor: i64,
) -> Result<[i64; 4]> {
    if !(0..=MAX_MONEY).contains(&basic)
        || !(0..=31000).contains(&days)
        || !(1..=24000).contains(&hours)
        || !(0..=31).contains(&fridays)
        || !(0..=744000).contains(&overtime)
        || !(1..=31).contains(&divisor)
    {
        return Err(DomainError::validation(
            "salary",
            "Enter valid salary, days and hours within the supported ranges.",
        ));
    }
    let b = i128::from(basic);
    let d = i128::from(divisor);
    let normal = rounded(b * i128::from(days), d * 1000)?;
    let friday = rounded(b * i128::from(fridays) * 3, d * 2)?;
    let ot = rounded(b * i128::from(overtime) * 5, d * i128::from(hours) * 4)?;
    let total = bounded_total(i128::from(normal) + i128::from(friday) + i128::from(ot))?;
    Ok([normal, friday, ot, total])
}
const RECORD_SQL:&str="SELECT id,month,worker_id,worker_name_snapshot,CAST(basic_fils AS TEXT) AS basic_fils,CAST(normal_days_scaled AS TEXT) AS normal_days_scaled,CAST(hours_per_day_scaled AS TEXT) AS hours_per_day_scaled,fridays,CAST(overtime_hours_scaled AS TEXT) AS overtime_hours_scaled,CAST(normal_fils AS TEXT) AS normal_fils,CAST(friday_fils AS TEXT) AS friday_fils,CAST(overtime_fils AS TEXT) AS overtime_fils,CAST(total_fils AS TEXT) AS total_fils,notes,archived,version FROM monthly_salaries WHERE month=?1 ORDER BY archived,worker_name_snapshot,id";
pub(crate) fn query(conn: &Connection, key: &str) -> Result<Value> {
    month(key)?;
    let (divisor, version) = month_state(conn, key)?;
    let records = rows(conn, RECORD_SQL, [key])?;
    let mut totals = [0_i128; 4];
    let mut count = 0;
    for record in records.iter().filter(|r| r["archived"] == 0) {
        count += 1;
        for (index, field) in ["normal_fils", "friday_fils", "overtime_fils", "total_fils"]
            .iter()
            .enumerate()
        {
            totals[index] += record[field]
                .as_str()
                .unwrap()
                .parse::<i128>()
                .map_err(|_| DomainError::storage("Invalid salary amount."))?;
        }
    }
    let workers = rows(
        conn,
        "SELECT id,name,active,archived FROM workers ORDER BY archived,name",
        [],
    )?;
    let months = rows(
        conn,
        "SELECT month,divisor,version FROM salary_months ORDER BY month DESC",
        [],
    )?;
    let history=rows(conn,"SELECT a.id,a.action,a.created_at,a.after_json,actors.display_name AS actor_name FROM audit_events a JOIN actors ON actors.id=a.actor_id WHERE (a.entity_type='salary_month' AND a.entity_id=?1) OR (a.entity_type='salary' AND a.entity_id IN (SELECT id FROM monthly_salaries WHERE month=?1)) ORDER BY a.created_at DESC,a.rowid DESC LIMIT 50",[key])?;
    Ok(
        json!({"month":key,"divisor":divisor,"version":version,"records":records,"workers":workers,"months":months,"history":history,"totals":{"workers":count,"normal_fils":totals[0].to_string(),"friday_fils":totals[1].to_string(),"overtime_fils":totals[2].to_string(),"total_fils":totals[3].to_string()}}),
    )
}
pub(crate) fn apply(tx: &Transaction, action: &Action, time: &str) -> Result<Applied> {
    match action {
        Action::SetSalaryDivisor {
            month: key,
            expected_version,
            divisor,
        } => {
            check_month(tx, key, *expected_version)?;
            if !(1..=31).contains(divisor) {
                return Err(DomainError::validation(
                    "divisor",
                    "Choose a divisor from 1 to 31 days.",
                ));
            }
            let records=rows(tx,"SELECT id,basic_fils,normal_days_scaled,hours_per_day_scaled,fridays,overtime_hours_scaled FROM monthly_salaries WHERE month=?1",[key])?;
            for r in records {
                let amounts = calculate(
                    r["basic_fils"].as_i64().unwrap(),
                    r["normal_days_scaled"].as_i64().unwrap(),
                    r["hours_per_day_scaled"].as_i64().unwrap(),
                    r["fridays"].as_i64().unwrap(),
                    r["overtime_hours_scaled"].as_i64().unwrap(),
                    *divisor,
                )?;
                tx.execute("UPDATE monthly_salaries SET normal_fils=?2,friday_fils=?3,overtime_fils=?4,total_fils=?5,version=version+1,updated_at=?6 WHERE id=?1",params![r["id"].as_str().unwrap(),amounts[0],amounts[1],amounts[2],amounts[3],time])?;
            }
            tx.execute("INSERT INTO salary_months(month,divisor,created_at,updated_at) VALUES(?1,?2,?3,?3) ON CONFLICT(month) DO UPDATE SET divisor=excluded.divisor,version=version+1,updated_at=excluded.updated_at",params![key,divisor,time])?;
            Ok((
                json!({"month":key}),
                vec!["salaries"],
                "salary_month",
                key.clone(),
                "salary_divisor_updated",
                business_today(),
                None,
            ))
        }
        Action::SaveSalary {
            id: record_id,
            expected_version,
            month: key,
            expected_month_version,
            worker_id,
            basic_fils,
            normal_days_scaled,
            hours_per_day_scaled,
            fridays,
            overtime_hours_scaled,
            notes,
        } => {
            let divisor = check_month(tx, key, *expected_month_version)?;
            id(worker_id)?;
            let basic = integer(basic_fils, MAX_MONEY, "basic_fils")?;
            let days = integer(normal_days_scaled, 31000, "normal_days_scaled")?;
            let hours = integer(hours_per_day_scaled, 24000, "hours_per_day_scaled")?;
            let overtime = integer(overtime_hours_scaled, 744000, "overtime_hours_scaled")?;
            let amounts = calculate(basic, days, hours, *fridays, overtime, divisor)?;
            let notes = text(notes, 2000, "notes", false)?;
            let rid = if let Some(rid) = record_id {
                version(tx, "monthly_salaries", rid, *expected_version)?;
                let matches:bool=tx.query_row("SELECT month=?2 AND worker_id=?3 AND archived=0 FROM monthly_salaries WHERE id=?1",params![rid,key,worker_id],|r|r.get(0))?;
                if !matches {
                    return Err(DomainError::validation(
                        "salary",
                        "Edit the same worker and month on an active salary record.",
                    ));
                }
                tx.execute("UPDATE monthly_salaries SET basic_fils=?2,normal_days_scaled=?3,hours_per_day_scaled=?4,fridays=?5,overtime_hours_scaled=?6,normal_fils=?7,friday_fils=?8,overtime_fils=?9,total_fils=?10,notes=?11,version=version+1,updated_at=?12 WHERE id=?1",params![rid,basic,days,hours,fridays,overtime,amounts[0],amounts[1],amounts[2],amounts[3],notes,time])?;
                rid.clone()
            } else {
                if expected_version.is_some() {
                    return Err(DomainError::validation(
                        "expected_version",
                        "New salaries have no existing record version.",
                    ));
                }
                let existing: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM monthly_salaries WHERE month=?1 AND worker_id=?2)",
                    params![key, worker_id],
                    |r| r.get(0),
                )?;
                if existing {
                    return Err(DomainError::validation("worker_id","This worker already has a salary for this month. Edit or restore that record."));
                }
                let name: String = tx.query_row(
                    "SELECT name FROM workers WHERE id=?1 AND archived=0 AND active=1",
                    [worker_id],
                    |r| r.get(0),
                )?;
                touch_month(tx, key, time)?;
                let rid = uuid::Uuid::new_v4().to_string();
                tx.execute("INSERT INTO monthly_salaries(id,month,worker_id,worker_name_snapshot,basic_fils,normal_days_scaled,hours_per_day_scaled,fridays,overtime_hours_scaled,normal_fils,friday_fils,overtime_fils,total_fils,notes,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?15)",params![rid,key,worker_id,name,basic,days,hours,fridays,overtime,amounts[0],amounts[1],amounts[2],amounts[3],notes,time])?;
                rid
            };
            if record_id.is_some() {
                touch_month(tx, key, time)?;
            }
            Ok((
                json!({"id":rid}),
                vec!["salaries"],
                "salary",
                rid,
                "salary_saved",
                business_today(),
                None,
            ))
        }
        Action::ArchiveSalary {
            id: rid,
            expected_version,
            expected_month_version,
            archived,
        } => {
            version(tx, "monthly_salaries", rid, Some(*expected_version))?;
            let key: String = tx.query_row(
                "SELECT month FROM monthly_salaries WHERE id=?1",
                [rid],
                |r| r.get(0),
            )?;
            check_month(tx, &key, *expected_month_version)?;
            tx.execute("UPDATE monthly_salaries SET archived=?2,version=version+1,updated_at=?3 WHERE id=?1",params![rid,archived,time])?;
            touch_month(tx, &key, time)?;
            Ok((
                json!({"id":rid}),
                vec!["salaries"],
                "salary",
                rid.clone(),
                if *archived {
                    "salary_archived"
                } else {
                    "salary_restored"
                },
                business_today(),
                None,
            ))
        }
        _ => Err(DomainError::validation(
            "action",
            "Unsupported salary action.",
        )),
    }
}
