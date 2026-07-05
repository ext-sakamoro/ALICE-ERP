//! cost.

use crate::bom::*;
use crate::common::*;
use crate::inventory::*;
use crate::work_order::*;

// ---------------------------------------------------------------------------
// Cost Accounting
// ---------------------------------------------------------------------------

/// Cost variance analysis result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CostVariance {
    pub standard_cost: Money,
    pub actual_cost: Money,
    pub variance: Money,
    pub is_favorable: bool,
}

impl CostVariance {
    #[must_use]
    pub const fn new(standard_cost: Money, actual_cost: Money) -> Self {
        let variance = actual_cost - standard_cost;
        Self {
            standard_cost,
            actual_cost,
            variance,
            is_favorable: variance <= 0,
        }
    }

    /// Variance percentage (basis points, i.e. 100 = 1%).
    #[must_use]
    pub const fn variance_bps(&self) -> i64 {
        if self.standard_cost == 0 {
            return 0;
        }
        (self.variance * 10_000) / self.standard_cost
    }
}

/// Material price variance for a purchase.
#[must_use]
pub const fn material_price_variance(
    actual_qty: i64,
    actual_price: Money,
    standard_price: Money,
) -> Money {
    (actual_price - standard_price) * actual_qty
}

/// Material usage variance for production.
#[must_use]
pub const fn material_usage_variance(
    actual_qty: i64,
    standard_qty: i64,
    standard_price: Money,
) -> Money {
    (actual_qty - standard_qty) * standard_price
}

/// Labor rate variance.
#[must_use]
pub const fn labor_rate_variance(
    actual_hours: i64,
    actual_rate: Money,
    standard_rate: Money,
) -> Money {
    (actual_rate - standard_rate) * actual_hours
}

/// Labor efficiency variance.
#[must_use]
pub const fn labor_efficiency_variance(
    actual_hours: i64,
    standard_hours: i64,
    standard_rate: Money,
) -> Money {
    (actual_hours - standard_hours) * standard_rate
}

/// Calculate standard cost for a BOM-based product.
#[must_use]
pub fn product_standard_cost(bom: &Bom, inventory: &Inventory) -> Money {
    bom.lines
        .iter()
        .map(|line| {
            inventory
                .get_sku(line.component_sku_id)
                .map_or(0, |s| s.standard_cost * line.quantity_per)
        })
        .sum()
}

/// Analyze cost variance for a completed work order.
#[must_use]
pub fn analyze_work_order_cost(
    wo: &WorkOrder,
    bom_registry: &BomRegistry,
    inventory: &Inventory,
) -> CostVariance {
    let standard = bom_registry
        .get(wo.product_sku_id)
        .map_or(0, |bom| bom.standard_cost(inventory, wo.quantity));
    CostVariance::new(standard, wo.actual_cost)
}
