//! mrp.

use crate::bom::*;
use crate::common::*;
use crate::inventory::*;

// ---------------------------------------------------------------------------
// MRP (Material Requirements Planning)
// ---------------------------------------------------------------------------

/// A planned material requirement produced by MRP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialRequirement {
    pub sku_id: Id,
    pub gross_requirement: i64,
    pub on_hand: i64,
    pub net_requirement: i64,
    pub planned_order: i64,
}

/// Run MRP for a single product.
///
/// Given a demand quantity, explode the BOM and compute net requirements.
#[must_use]
pub fn run_mrp(
    product_sku_id: Id,
    demand_qty: i64,
    bom_registry: &BomRegistry,
    inventory: &Inventory,
) -> Vec<MaterialRequirement> {
    let Some(bom) = bom_registry.get(product_sku_id) else {
        return Vec::new();
    };

    bom.lines
        .iter()
        .map(|line| {
            let gross = line.quantity_per * demand_qty;
            let on_hand = inventory
                .get_sku(line.component_sku_id)
                .map_or(0, |s| s.stock_on_hand);
            let net = (gross - on_hand).max(0);
            let planned = if net > 0 {
                let reorder_qty = inventory
                    .get_sku(line.component_sku_id)
                    .map_or(net, |s| s.reorder_quantity);
                // Round up to multiples of reorder quantity.
                ((net + reorder_qty - 1) / reorder_qty) * reorder_qty
            } else {
                0
            };
            MaterialRequirement {
                sku_id: line.component_sku_id,
                gross_requirement: gross,
                on_hand,
                net_requirement: net,
                planned_order: planned,
            }
        })
        .collect()
}
