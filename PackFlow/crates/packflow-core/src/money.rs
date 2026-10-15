use crate::error::{DomainError, Result};
pub const MAX_MONEY: i64 = 9_000_000_000_000;
pub fn integer(value: &str, max: i64, field: &str) -> Result<i64> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(DomainError::validation(
            field,
            "Expected a nonnegative integer string.",
        ));
    }
    value
        .parse::<i64>()
        .ok()
        .filter(|v| *v <= max)
        .ok_or_else(|| DomainError::validation(field, "Amount is outside the supported range."))
}
pub fn line_total(quantity_scaled: i64, unit_cost_fils: i64) -> Result<i64> {
    let total = (i128::from(quantity_scaled) * i128::from(unit_cost_fils) + 500) / 1000;
    if quantity_scaled <= 0
        || quantity_scaled > 1_000_000_000
        || !(0..=MAX_MONEY).contains(&unit_cost_fils)
        || total > i128::from(MAX_MONEY)
    {
        return Err(DomainError::validation(
            "amount",
            "Line total is outside the supported range.",
        ));
    }
    Ok(total as i64)
}
pub fn bounded_total(value: i128) -> Result<i64> {
    if !(0..=i128::from(MAX_MONEY)).contains(&value) {
        return Err(DomainError::validation(
            "amount",
            "Total exceeds the supported range.",
        ));
    }
    Ok(value as i64)
}
