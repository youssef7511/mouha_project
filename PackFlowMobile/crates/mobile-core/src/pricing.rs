use crate::{
    error::{DomainError, Result},
    models::*,
    money::{bounded_total, integer, line_total, MAX_MONEY},
};
use std::collections::HashSet;
pub struct Quote {
    pub total: Option<i64>,
    pub known: i64,
    pub stages: Vec<Option<String>>,
}
pub fn calculate(input: &OrderInput) -> Result<Quote> {
    if input.items.is_empty()
        || input.items.len() > 100
        || input.stages.is_empty()
        || input.stages.len() > 50
    {
        return Err(DomainError::validation(
            "order",
            "Add 1–100 box types and 1–50 stages.",
        ));
    }
    let mut ids = HashSet::new();
    for item in &input.items {
        uuid::Uuid::parse_str(&item.id)
            .map_err(|_| DomainError::validation("items", "Invalid box identity."))?;
        if !ids.insert(item.id.clone())
            || !(1..=1_000_000).contains(&item.quantity)
            || item.description.trim().is_empty()
            || item.description.chars().count() > 160
            || item.dimensions.chars().count() > 200
        {
            return Err(DomainError::validation(
                "items",
                "Describe each box type and enter 1–1,000,000 whole boxes.",
            ));
        }
    }
    let invalid = || {
        DomainError::validation(
            "stages",
            "Price each selected stage once with only the inputs for its pricing mode.",
        )
    };
    let mut seen = HashSet::new();
    let mut known = 0_i128;
    let mut complete = true;
    let mut stages = Vec::new();
    for stage in &input.stages {
        if !seen.insert(&stage.template_id) {
            return Err(invalid());
        }
        let total = match stage.pricing_mode {
            PricingMode::Unpriced => {
                if stage.fixed_total_fils.is_some()
                    || stage.quantity_scaled.is_some()
                    || stage.unit_price_fils.is_some()
                    || !stage.item_prices.is_empty()
                {
                    return Err(invalid());
                }
                complete = false;
                None
            }
            PricingMode::Fixed => {
                if stage.quantity_scaled.is_some()
                    || stage.unit_price_fils.is_some()
                    || !stage.item_prices.is_empty()
                {
                    return Err(invalid());
                }
                Some(integer(
                    stage.fixed_total_fils.as_deref().ok_or_else(invalid)?,
                    MAX_MONEY,
                    "fixed_total",
                )?)
            }
            PricingMode::PerUnit => {
                if stage.fixed_total_fils.is_some() || !stage.item_prices.is_empty() {
                    return Err(invalid());
                }
                Some(line_total(
                    integer(
                        stage.quantity_scaled.as_deref().ok_or_else(invalid)?,
                        1_000_000_000,
                        "quantity",
                    )?,
                    integer(
                        stage.unit_price_fils.as_deref().ok_or_else(invalid)?,
                        MAX_MONEY,
                        "unit_price",
                    )?,
                )?)
            }
            PricingMode::PerBox => {
                if stage.fixed_total_fils.is_some()
                    || stage.quantity_scaled.is_some()
                    || stage.unit_price_fils.is_some()
                    || stage.item_prices.len() != input.items.len()
                {
                    return Err(invalid());
                }
                let mut seen_items = HashSet::new();
                let mut sum = 0_i128;
                for rate in &stage.item_prices {
                    if !seen_items.insert(&rate.item_id) {
                        return Err(invalid());
                    }
                    let item = input
                        .items
                        .iter()
                        .find(|i| i.id == rate.item_id)
                        .ok_or_else(invalid)?;
                    sum += i128::from(item.quantity)
                        * i128::from(integer(&rate.unit_price_fils, MAX_MONEY, "box_price")?);
                }
                Some(bounded_total(sum)?)
            }
        };
        known += i128::from(total.unwrap_or(0));
        stages.push(total.map(|v| v.to_string()));
    }
    let known = bounded_total(known)?;
    Ok(Quote {
        total: complete.then_some(known),
        known,
        stages,
    })
}
