//! work order.

use crate::bom::*;
use crate::common::*;
use crate::errors::ErpError;
use crate::inventory::*;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Work Orders & Production Scheduling
// ---------------------------------------------------------------------------

/// Status of a work order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WoStatus {
    Planned,
    Released,
    InProgress,
    Completed,
    Cancelled,
}

/// A work order for production.
#[derive(Debug, Clone)]
pub struct WorkOrder {
    pub id: Id,
    pub product_sku_id: Id,
    pub quantity: i64,
    pub status: WoStatus,
    pub scheduled_start: Timestamp,
    pub scheduled_end: Timestamp,
    pub actual_start: Option<Timestamp>,
    pub actual_end: Option<Timestamp>,
    pub actual_cost: Money,
}

impl WorkOrder {
    #[must_use]
    pub const fn new(
        id: Id,
        product_sku_id: Id,
        quantity: i64,
        scheduled_start: Timestamp,
        scheduled_end: Timestamp,
    ) -> Self {
        Self {
            id,
            product_sku_id,
            quantity,
            status: WoStatus::Planned,
            scheduled_start,
            scheduled_end,
            actual_start: None,
            actual_end: None,
            actual_cost: 0,
        }
    }

    /// Release the work order for production.
    ///
    /// # Errors
    /// Returns an error if not in Planned status.
    pub fn release(&mut self) -> Result<(), ErpError> {
        if self.status != WoStatus::Planned {
            return Err(ErpError::InvalidStatusTransition);
        }
        self.status = WoStatus::Released;
        Ok(())
    }

    /// Start production.
    ///
    /// # Errors
    /// Returns an error if not in Released status.
    pub fn start(&mut self, actual_start: Timestamp) -> Result<(), ErpError> {
        if self.status != WoStatus::Released {
            return Err(ErpError::InvalidStatusTransition);
        }
        self.status = WoStatus::InProgress;
        self.actual_start = Some(actual_start);
        Ok(())
    }

    /// Complete production and consume materials.
    ///
    /// # Errors
    /// Returns an error if not in `InProgress` status or material consumption fails.
    pub fn complete(
        &mut self,
        actual_end: Timestamp,
        actual_cost: Money,
        bom_registry: &BomRegistry,
        inventory: &mut Inventory,
    ) -> Result<(), ErpError> {
        if self.status != WoStatus::InProgress {
            return Err(ErpError::InvalidStatusTransition);
        }

        // Consume materials per BOM
        if let Some(bom) = bom_registry.get(self.product_sku_id) {
            for line in &bom.lines {
                if let Some(sku) = inventory.get_sku_mut(line.component_sku_id) {
                    sku.issue(line.quantity_per * self.quantity)?;
                }
            }
        }

        // Receive finished goods
        if let Some(product) = inventory.get_sku_mut(self.product_sku_id) {
            product.receive(self.quantity)?;
        }

        self.status = WoStatus::Completed;
        self.actual_end = Some(actual_end);
        self.actual_cost = actual_cost;
        Ok(())
    }

    /// Cancel the work order.
    ///
    /// # Errors
    /// Returns an error if already completed or cancelled.
    pub fn cancel(&mut self) -> Result<(), ErpError> {
        if self.status == WoStatus::Completed || self.status == WoStatus::Cancelled {
            return Err(ErpError::InvalidStatusTransition);
        }
        self.status = WoStatus::Cancelled;
        Ok(())
    }

    /// Planned duration in seconds.
    #[must_use]
    pub const fn planned_duration(&self) -> u64 {
        self.scheduled_end.saturating_sub(self.scheduled_start)
    }

    /// Actual duration in seconds, if completed.
    #[must_use]
    pub const fn actual_duration(&self) -> Option<u64> {
        match (self.actual_start, self.actual_end) {
            (Some(s), Some(e)) => Some(e.saturating_sub(s)),
            _ => None,
        }
    }
}

/// Work order manager with simple scheduling.
#[derive(Debug, Default)]
pub struct WoManager {
    orders: HashMap<Id, WorkOrder>,
    next_id: Id,
}

impl WoManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_order(
        &mut self,
        product_sku_id: Id,
        quantity: i64,
        scheduled_start: Timestamp,
        scheduled_end: Timestamp,
    ) -> Id {
        self.next_id += 1;
        let wo = WorkOrder::new(
            self.next_id,
            product_sku_id,
            quantity,
            scheduled_start,
            scheduled_end,
        );
        self.orders.insert(self.next_id, wo);
        self.next_id
    }

    #[must_use]
    pub fn get(&self, id: Id) -> Option<&WorkOrder> {
        self.orders.get(&id)
    }

    pub fn get_mut(&mut self, id: Id) -> Option<&mut WorkOrder> {
        self.orders.get_mut(&id)
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.orders.len()
    }

    /// List work orders filtered by status.
    #[must_use]
    pub fn list_by_status(&self, status: WoStatus) -> Vec<&WorkOrder> {
        self.orders
            .values()
            .filter(|wo| wo.status == status)
            .collect()
    }

    /// Get all work orders sorted by scheduled start time.
    #[must_use]
    pub fn schedule(&self) -> Vec<&WorkOrder> {
        let mut orders: Vec<_> = self.orders.values().collect();
        orders.sort_by_key(|wo| wo.scheduled_start);
        orders
    }

    /// Check for scheduling conflicts (overlapping work orders for same product).
    #[must_use]
    pub fn conflicts(&self) -> Vec<(Id, Id)> {
        let active: Vec<_> = self
            .orders
            .values()
            .filter(|wo| wo.status != WoStatus::Cancelled && wo.status != WoStatus::Completed)
            .collect();
        let mut conflicts = Vec::new();
        for i in 0..active.len() {
            for j in (i + 1)..active.len() {
                let a = active[i];
                let b = active[j];
                let same_product = a.product_sku_id == b.product_sku_id;
                let a_starts_before_b_ends = a.scheduled_start < b.scheduled_end;
                let b_starts_before_a_ends = b.scheduled_start < a.scheduled_end;
                if same_product && a_starts_before_b_ends && b_starts_before_a_ends {
                    conflicts.push((a.id, b.id));
                }
            }
        }
        conflicts
    }
}
