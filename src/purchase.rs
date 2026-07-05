//! purchase.

use crate::common::*;
use crate::errors::ErpError;
use crate::inventory::Inventory;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Purchase Orders
// ---------------------------------------------------------------------------

/// Status of a purchase order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoStatus {
    Draft,
    Submitted,
    Approved,
    Received,
    Cancelled,
}

/// A line item on a purchase order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoLine {
    pub sku_id: Id,
    pub quantity: i64,
    pub unit_cost: Money,
}

impl PoLine {
    #[must_use]
    pub const fn line_total(&self) -> Money {
        self.unit_cost * self.quantity
    }
}

/// A purchase order.
#[derive(Debug, Clone)]
pub struct PurchaseOrder {
    pub id: Id,
    pub supplier: String,
    pub status: PoStatus,
    pub lines: Vec<PoLine>,
    pub created_at: Timestamp,
}

impl PurchaseOrder {
    #[must_use]
    pub fn new(id: Id, supplier: &str, created_at: Timestamp) -> Self {
        Self {
            id,
            supplier: supplier.to_owned(),
            status: PoStatus::Draft,
            lines: Vec::new(),
            created_at,
        }
    }

    pub fn add_line(&mut self, sku_id: Id, quantity: i64, unit_cost: Money) {
        self.lines.push(PoLine {
            sku_id,
            quantity,
            unit_cost,
        });
    }

    #[must_use]
    pub fn total(&self) -> Money {
        self.lines.iter().map(PoLine::line_total).sum()
    }

    /// Submit the PO for approval.
    ///
    /// # Errors
    /// Returns an error if not in Draft status.
    pub fn submit(&mut self) -> Result<(), ErpError> {
        if self.status != PoStatus::Draft {
            return Err(ErpError::InvalidStatusTransition);
        }
        self.status = PoStatus::Submitted;
        Ok(())
    }

    /// Approve the PO.
    ///
    /// # Errors
    /// Returns an error if not in Submitted status.
    pub fn approve(&mut self) -> Result<(), ErpError> {
        if self.status != PoStatus::Submitted {
            return Err(ErpError::InvalidStatusTransition);
        }
        self.status = PoStatus::Approved;
        Ok(())
    }

    /// Mark the PO as received and update inventory.
    ///
    /// # Errors
    /// Returns an error if not in Approved status or inventory update fails.
    pub fn receive(&mut self, inventory: &mut Inventory) -> Result<(), ErpError> {
        if self.status != PoStatus::Approved {
            return Err(ErpError::InvalidStatusTransition);
        }
        for line in &self.lines {
            if let Some(sku) = inventory.get_sku_mut(line.sku_id) {
                sku.receive(line.quantity)?;
            }
        }
        self.status = PoStatus::Received;
        Ok(())
    }

    /// Cancel the PO.
    ///
    /// # Errors
    /// Returns an error if already received or cancelled.
    pub fn cancel(&mut self) -> Result<(), ErpError> {
        if self.status == PoStatus::Received || self.status == PoStatus::Cancelled {
            return Err(ErpError::InvalidStatusTransition);
        }
        self.status = PoStatus::Cancelled;
        Ok(())
    }

    #[must_use]
    pub const fn line_count(&self) -> usize {
        self.lines.len()
    }
}

/// Purchase order manager.
#[derive(Debug, Default)]
pub struct PoManager {
    orders: HashMap<Id, PurchaseOrder>,
    next_id: Id,
}

impl PoManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_order(&mut self, supplier: &str, created_at: Timestamp) -> Id {
        self.next_id += 1;
        let po = PurchaseOrder::new(self.next_id, supplier, created_at);
        self.orders.insert(self.next_id, po);
        self.next_id
    }

    #[must_use]
    pub fn get(&self, id: Id) -> Option<&PurchaseOrder> {
        self.orders.get(&id)
    }

    pub fn get_mut(&mut self, id: Id) -> Option<&mut PurchaseOrder> {
        self.orders.get_mut(&id)
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.orders.len()
    }

    /// List POs filtered by status.
    #[must_use]
    pub fn list_by_status(&self, status: PoStatus) -> Vec<&PurchaseOrder> {
        self.orders
            .values()
            .filter(|po| po.status == status)
            .collect()
    }
}
