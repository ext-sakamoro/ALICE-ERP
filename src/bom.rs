//! bom.

use crate::common::*;
use crate::inventory::Inventory;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Bill of Materials (BOM)
// ---------------------------------------------------------------------------

/// A single component line within a BOM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BomLine {
    pub component_sku_id: Id,
    pub quantity_per: i64,
}

/// Bill of Materials for a finished product.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bom {
    pub product_sku_id: Id,
    pub lines: Vec<BomLine>,
}

impl Bom {
    #[must_use]
    pub const fn new(product_sku_id: Id) -> Self {
        Self {
            product_sku_id,
            lines: Vec::new(),
        }
    }

    /// Add a component to the BOM.
    pub fn add_component(&mut self, component_sku_id: Id, quantity_per: i64) {
        self.lines.push(BomLine {
            component_sku_id,
            quantity_per,
        });
    }

    /// Calculate the standard cost of producing `qty` units.
    #[must_use]
    pub fn standard_cost(&self, inventory: &Inventory, qty: i64) -> Money {
        self.lines
            .iter()
            .map(|line| {
                let unit_cost = inventory
                    .get_sku(line.component_sku_id)
                    .map_or(0, |s| s.standard_cost);
                unit_cost * line.quantity_per * qty
            })
            .sum()
    }

    /// Check whether inventory has enough stock to produce `qty` units.
    #[must_use]
    pub fn can_produce(&self, inventory: &Inventory, qty: i64) -> bool {
        self.lines.iter().all(|line| {
            inventory
                .get_sku(line.component_sku_id)
                .is_some_and(|s| s.stock_on_hand >= line.quantity_per * qty)
        })
    }

    /// Number of component lines.
    #[must_use]
    pub const fn component_count(&self) -> usize {
        self.lines.len()
    }
}

/// BOM registry for all products.
#[derive(Debug, Default)]
pub struct BomRegistry {
    boms: HashMap<Id, Bom>,
}

impl BomRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, bom: Bom) {
        self.boms.insert(bom.product_sku_id, bom);
    }

    #[must_use]
    pub fn get(&self, product_sku_id: Id) -> Option<&Bom> {
        self.boms.get(&product_sku_id)
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.boms.len()
    }
}
