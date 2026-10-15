use crate::{
    error::{DomainError, Result},
    models::{NewItem, StagePriceInput, StagePricingMode},
    money::{bounded_total, integer, line_total, MAX_MONEY},
};
use std::collections::HashSet;
pub(crate) struct CalculatedStagePrice {
    pub fixed: Option<i64>,
    pub quantity: Option<i64>,
    pub rate: Option<i64>,
    pub total: Option<i64>,
    pub item_rates: Vec<i64>,
}
pub(crate) fn calculate(
    inputs: &[StagePriceInput],
    templates: &[String],
    items: &[NewItem],
) -> Result<(Vec<CalculatedStagePrice>, Option<i64>)> {
    let invalid = || {
        DomainError::validation(
            "stage_prices",
            "Price each selected stage exactly once using only its selected pricing inputs.",
        )
    };
    let distinct: HashSet<_> = inputs.iter().map(|s| &s.template_id).collect();
    if inputs.len() != templates.len()
        || distinct.len() != inputs.len()
        || templates.iter().any(|t| !distinct.contains(t))
    {
        return Err(invalid());
    }
    let mut results = Vec::new();
    let mut sum = 0_i128;
    let mut complete = true;
    for input in inputs {
        let mut result = CalculatedStagePrice {
            fixed: None,
            quantity: None,
            rate: None,
            total: None,
            item_rates: vec![],
        };
        let allowed = match input.pricing_mode {
            StagePricingMode::Unpriced => {
                input.fixed_total_fils.is_none()
                    && input.quantity_scaled.is_none()
                    && input.unit_price_fils.is_none()
                    && input.item_prices_fils.is_empty()
            }
            StagePricingMode::Fixed => {
                input.fixed_total_fils.is_some()
                    && input.quantity_scaled.is_none()
                    && input.unit_price_fils.is_none()
                    && input.item_prices_fils.is_empty()
            }
            StagePricingMode::PerBox => {
                input.fixed_total_fils.is_none()
                    && input.quantity_scaled.is_none()
                    && input.unit_price_fils.is_none()
                    && input.item_prices_fils.len() == items.len()
            }
            StagePricingMode::PerUnit => {
                input.fixed_total_fils.is_none()
                    && input.quantity_scaled.is_some()
                    && input.unit_price_fils.is_some()
                    && input.item_prices_fils.is_empty()
            }
        };
        if !allowed {
            return Err(invalid());
        }
        match input.pricing_mode {
            StagePricingMode::Unpriced => complete = false,
            StagePricingMode::Fixed => {
                result.fixed = Some(integer(
                    input.fixed_total_fils.as_deref().unwrap(),
                    MAX_MONEY,
                    "stage_prices.fixed_total_fils",
                )?);
                result.total = result.fixed;
            }
            StagePricingMode::PerBox => {
                let mut stage_sum = 0_i128;
                for (item, value) in items.iter().zip(&input.item_prices_fils) {
                    let rate = integer(value, MAX_MONEY, "stage_prices.item_prices_fils")?;
                    stage_sum += i128::from(rate) * i128::from(item.ordered_boxes);
                    result.item_rates.push(rate);
                }
                result.total = Some(bounded_total(stage_sum)?);
            }
            StagePricingMode::PerUnit => {
                let quantity = integer(
                    input.quantity_scaled.as_deref().unwrap(),
                    1_000_000_000,
                    "stage_prices.quantity_scaled",
                )?;
                let rate = integer(
                    input.unit_price_fils.as_deref().unwrap(),
                    MAX_MONEY,
                    "stage_prices.unit_price_fils",
                )?;
                result.total = Some(line_total(quantity, rate)?);
                result.quantity = Some(quantity);
                result.rate = Some(rate);
            }
        }
        sum += i128::from(result.total.unwrap_or(0));
        results.push(result);
    }
    let total = bounded_total(sum)?;
    Ok((results, if complete { Some(total) } else { None }))
}
