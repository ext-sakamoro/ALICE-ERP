//! inventory.

use crate::common::*;
use crate::errors::ErpError;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Inventory Management
// ---------------------------------------------------------------------------

/// Stock-Keeping Unit with inventory tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sku {
    pub id: Id,
    pub name: String,
    pub description: String,
    pub unit: String,
    pub stock_on_hand: i64,
    pub reorder_point: i64,
    pub reorder_quantity: i64,
    pub standard_cost: Money,
}

impl Sku {
    #[must_use]
    pub fn new(
        id: Id,
        name: &str,
        unit: &str,
        reorder_point: i64,
        reorder_quantity: i64,
        standard_cost: Money,
    ) -> Self {
        Self {
            id,
            name: name.to_owned(),
            description: String::new(),
            unit: unit.to_owned(),
            stock_on_hand: 0,
            reorder_point,
            reorder_quantity,
            standard_cost,
        }
    }

    /// Returns `true` if stock is at or below the reorder point.
    #[must_use]
    pub const fn needs_reorder(&self) -> bool {
        self.stock_on_hand <= self.reorder_point
    }

    /// Receive stock (increase on-hand).
    ///
    /// # Errors
    /// Returns an error if `qty` is negative.
    pub const fn receive(&mut self, qty: i64) -> Result<(), ErpError> {
        if qty < 0 {
            return Err(ErpError::InvalidQuantity);
        }
        self.stock_on_hand += qty;
        Ok(())
    }

    /// Issue (consume) stock.
    ///
    /// # Errors
    /// Returns an error if `qty` is negative or exceeds stock on hand.
    pub const fn issue(&mut self, qty: i64) -> Result<(), ErpError> {
        if qty < 0 {
            return Err(ErpError::InvalidQuantity);
        }
        if qty > self.stock_on_hand {
            return Err(ErpError::InsufficientStock);
        }
        self.stock_on_hand -= qty;
        Ok(())
    }
}

/// Central inventory holding all SKUs.
#[derive(Debug, Default)]
pub struct Inventory {
    skus: HashMap<Id, Sku>,
    next_id: Id,
}

impl Inventory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new SKU and return its assigned ID.
    pub fn add_sku(
        &mut self,
        name: &str,
        unit: &str,
        reorder_point: i64,
        reorder_quantity: i64,
        standard_cost: Money,
    ) -> Id {
        self.next_id += 1;
        let id = self.next_id;
        let sku = Sku::new(
            id,
            name,
            unit,
            reorder_point,
            reorder_quantity,
            standard_cost,
        );
        self.skus.insert(id, sku);
        id
    }

    /// Get a reference to a SKU.
    #[must_use]
    pub fn get_sku(&self, id: Id) -> Option<&Sku> {
        self.skus.get(&id)
    }

    /// Get a mutable reference to a SKU.
    pub fn get_sku_mut(&mut self, id: Id) -> Option<&mut Sku> {
        self.skus.get_mut(&id)
    }

    /// List all SKUs that need reordering.
    #[must_use]
    pub fn reorder_list(&self) -> Vec<&Sku> {
        self.skus.values().filter(|s| s.needs_reorder()).collect()
    }

    /// Number of distinct SKUs.
    #[must_use]
    pub fn sku_count(&self) -> usize {
        self.skus.len()
    }

    /// Total value of inventory at standard cost.
    #[must_use]
    pub fn total_value(&self) -> Money {
        self.skus
            .values()
            .map(|s| s.standard_cost * s.stock_on_hand)
            .sum()
    }
}
