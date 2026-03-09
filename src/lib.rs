#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

//! ALICE-ERP: Enterprise Resource Planning
//!
//! Inventory management, Bill of Materials, Material Requirements Planning,
//! production scheduling, cost accounting, purchase orders, and work orders.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Common types
// ---------------------------------------------------------------------------

/// Unique identifier type alias.
pub type Id = u64;

/// Represents a monetary amount in the smallest currency unit (e.g. cents).
pub type Money = i64;

/// A point in time represented as a Unix timestamp (seconds).
pub type Timestamp = u64;

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

// ---------------------------------------------------------------------------
// Production Scheduling (simple priority-based)
// ---------------------------------------------------------------------------

/// Priority level for scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Medium = 1,
    High = 2,
    Urgent = 3,
}

/// A schedulable production job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionJob {
    pub work_order_id: Id,
    pub priority: Priority,
    pub duration_seconds: u64,
    pub earliest_start: Timestamp,
}

/// Simple forward scheduler: sorts by priority (desc) then earliest start.
#[must_use]
pub fn forward_schedule(
    jobs: &[ProductionJob],
    start_time: Timestamp,
) -> Vec<(Id, Timestamp, Timestamp)> {
    let mut sorted: Vec<_> = jobs.to_vec();
    sorted.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then(a.earliest_start.cmp(&b.earliest_start))
    });

    let mut current_time = start_time;
    sorted
        .iter()
        .map(|job| {
            let job_start = current_time.max(job.earliest_start);
            let job_end = job_start + job.duration_seconds;
            current_time = job_end;
            (job.work_order_id, job_start, job_end)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur in the ERP system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErpError {
    InvalidQuantity,
    InsufficientStock,
    InvalidStatusTransition,
    NotFound,
}

impl std::fmt::Display for ErpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidQuantity => write!(f, "invalid quantity"),
            Self::InsufficientStock => write!(f, "insufficient stock"),
            Self::InvalidStatusTransition => write!(f, "invalid status transition"),
            Self::NotFound => write!(f, "not found"),
        }
    }
}

impl std::error::Error for ErpError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // === Inventory / SKU tests ===

    #[test]
    fn sku_new_defaults() {
        let sku = Sku::new(1, "Widget", "pcs", 10, 50, 100);
        assert_eq!(sku.stock_on_hand, 0);
        assert_eq!(sku.reorder_point, 10);
        assert!(sku.needs_reorder());
    }

    #[test]
    fn sku_receive_increases_stock() {
        let mut sku = Sku::new(1, "Widget", "pcs", 10, 50, 100);
        sku.receive(25).unwrap();
        assert_eq!(sku.stock_on_hand, 25);
    }

    #[test]
    fn sku_receive_negative_error() {
        let mut sku = Sku::new(1, "Widget", "pcs", 10, 50, 100);
        assert_eq!(sku.receive(-1), Err(ErpError::InvalidQuantity));
    }

    #[test]
    fn sku_issue_decreases_stock() {
        let mut sku = Sku::new(1, "Widget", "pcs", 10, 50, 100);
        sku.receive(30).unwrap();
        sku.issue(10).unwrap();
        assert_eq!(sku.stock_on_hand, 20);
    }

    #[test]
    fn sku_issue_insufficient() {
        let mut sku = Sku::new(1, "Widget", "pcs", 10, 50, 100);
        sku.receive(5).unwrap();
        assert_eq!(sku.issue(10), Err(ErpError::InsufficientStock));
    }

    #[test]
    fn sku_issue_negative_error() {
        let mut sku = Sku::new(1, "Widget", "pcs", 10, 50, 100);
        assert_eq!(sku.issue(-1), Err(ErpError::InvalidQuantity));
    }

    #[test]
    fn sku_needs_reorder_at_point() {
        let mut sku = Sku::new(1, "Widget", "pcs", 10, 50, 100);
        sku.receive(10).unwrap();
        assert!(sku.needs_reorder());
    }

    #[test]
    fn sku_no_reorder_above_point() {
        let mut sku = Sku::new(1, "Widget", "pcs", 10, 50, 100);
        sku.receive(11).unwrap();
        assert!(!sku.needs_reorder());
    }

    #[test]
    fn inventory_add_and_get() {
        let mut inv = Inventory::new();
        let id = inv.add_sku("Bolt", "pcs", 100, 500, 5);
        assert!(inv.get_sku(id).is_some());
        assert_eq!(inv.get_sku(id).unwrap().name, "Bolt");
    }

    #[test]
    fn inventory_count() {
        let mut inv = Inventory::new();
        inv.add_sku("A", "pcs", 10, 50, 100);
        inv.add_sku("B", "kg", 5, 20, 200);
        assert_eq!(inv.sku_count(), 2);
    }

    #[test]
    fn inventory_reorder_list() {
        let mut inv = Inventory::new();
        let id1 = inv.add_sku("A", "pcs", 10, 50, 100);
        let id2 = inv.add_sku("B", "pcs", 5, 20, 200);
        inv.get_sku_mut(id1).unwrap().receive(100).unwrap();
        // id2 has 0 stock, should be on reorder list
        let reorder = inv.reorder_list();
        assert_eq!(reorder.len(), 1);
        assert_eq!(reorder[0].id, id2);
    }

    #[test]
    fn inventory_total_value() {
        let mut inv = Inventory::new();
        let id = inv.add_sku("A", "pcs", 10, 50, 100);
        inv.get_sku_mut(id).unwrap().receive(10).unwrap();
        assert_eq!(inv.total_value(), 1000);
    }

    #[test]
    fn inventory_total_value_empty() {
        let inv = Inventory::new();
        assert_eq!(inv.total_value(), 0);
    }

    #[test]
    fn inventory_get_nonexistent() {
        let inv = Inventory::new();
        assert!(inv.get_sku(999).is_none());
    }

    #[test]
    fn inventory_multiple_receive_issue() {
        let mut inv = Inventory::new();
        let id = inv.add_sku("X", "pcs", 5, 10, 50);
        inv.get_sku_mut(id).unwrap().receive(20).unwrap();
        inv.get_sku_mut(id).unwrap().issue(8).unwrap();
        inv.get_sku_mut(id).unwrap().receive(3).unwrap();
        assert_eq!(inv.get_sku(id).unwrap().stock_on_hand, 15);
    }

    // === BOM tests ===

    #[test]
    fn bom_add_components() {
        let mut bom = Bom::new(100);
        bom.add_component(1, 2);
        bom.add_component(2, 3);
        assert_eq!(bom.component_count(), 2);
    }

    #[test]
    fn bom_standard_cost() {
        let mut inv = Inventory::new();
        let c1 = inv.add_sku("CompA", "pcs", 10, 50, 100);
        let c2 = inv.add_sku("CompB", "pcs", 5, 20, 200);
        let product = inv.add_sku("Product", "pcs", 0, 10, 0);

        let mut bom = Bom::new(product);
        bom.add_component(c1, 2); // 2 * 100 = 200
        bom.add_component(c2, 3); // 3 * 200 = 600

        assert_eq!(bom.standard_cost(&inv, 1), 800);
        assert_eq!(bom.standard_cost(&inv, 5), 4000);
    }

    #[test]
    fn bom_can_produce_true() {
        let mut inv = Inventory::new();
        let c1 = inv.add_sku("A", "pcs", 10, 50, 100);
        let product = inv.add_sku("P", "pcs", 0, 10, 0);
        inv.get_sku_mut(c1).unwrap().receive(20).unwrap();

        let mut bom = Bom::new(product);
        bom.add_component(c1, 2);
        assert!(bom.can_produce(&inv, 10));
    }

    #[test]
    fn bom_can_produce_false() {
        let mut inv = Inventory::new();
        let c1 = inv.add_sku("A", "pcs", 10, 50, 100);
        let product = inv.add_sku("P", "pcs", 0, 10, 0);
        inv.get_sku_mut(c1).unwrap().receive(5).unwrap();

        let mut bom = Bom::new(product);
        bom.add_component(c1, 2);
        assert!(!bom.can_produce(&inv, 10));
    }

    #[test]
    fn bom_registry_register_and_get() {
        let mut reg = BomRegistry::new();
        let bom = Bom::new(100);
        reg.register(bom);
        assert!(reg.get(100).is_some());
        assert!(reg.get(999).is_none());
    }

    #[test]
    fn bom_registry_count() {
        let mut reg = BomRegistry::new();
        reg.register(Bom::new(1));
        reg.register(Bom::new(2));
        assert_eq!(reg.count(), 2);
    }

    #[test]
    fn bom_empty_cost() {
        let inv = Inventory::new();
        let bom = Bom::new(1);
        assert_eq!(bom.standard_cost(&inv, 10), 0);
    }

    #[test]
    fn bom_missing_component_cost() {
        let inv = Inventory::new();
        let mut bom = Bom::new(1);
        bom.add_component(999, 5);
        assert_eq!(bom.standard_cost(&inv, 1), 0);
    }

    // === MRP tests ===

    #[test]
    fn mrp_basic() {
        let mut inv = Inventory::new();
        let c1 = inv.add_sku("Raw1", "kg", 10, 100, 50);
        let c2 = inv.add_sku("Raw2", "pcs", 5, 50, 30);
        let product = inv.add_sku("Widget", "pcs", 0, 10, 0);

        inv.get_sku_mut(c1).unwrap().receive(200).unwrap();
        inv.get_sku_mut(c2).unwrap().receive(10).unwrap();

        let mut bom = Bom::new(product);
        bom.add_component(c1, 3); // need 300 for 100 units, have 200 => net 100
        bom.add_component(c2, 1); // need 100 for 100 units, have 10 => net 90

        let mut reg = BomRegistry::new();
        reg.register(bom);

        let reqs = run_mrp(product, 100, &reg, &inv);
        assert_eq!(reqs.len(), 2);

        let r1 = &reqs[0];
        assert_eq!(r1.sku_id, c1);
        assert_eq!(r1.gross_requirement, 300);
        assert_eq!(r1.on_hand, 200);
        assert_eq!(r1.net_requirement, 100);
        assert_eq!(r1.planned_order, 100);

        let r2 = &reqs[1];
        assert_eq!(r2.sku_id, c2);
        assert_eq!(r2.gross_requirement, 100);
        assert_eq!(r2.on_hand, 10);
        assert_eq!(r2.net_requirement, 90);
        assert_eq!(r2.planned_order, 100); // rounded up to reorder_quantity=50 multiples
    }

    #[test]
    fn mrp_no_bom() {
        let inv = Inventory::new();
        let reg = BomRegistry::new();
        let reqs = run_mrp(999, 10, &reg, &inv);
        assert!(reqs.is_empty());
    }

    #[test]
    fn mrp_sufficient_stock() {
        let mut inv = Inventory::new();
        let c1 = inv.add_sku("R", "pcs", 10, 50, 100);
        let product = inv.add_sku("P", "pcs", 0, 10, 0);
        inv.get_sku_mut(c1).unwrap().receive(500).unwrap();

        let mut bom = Bom::new(product);
        bom.add_component(c1, 2);

        let mut reg = BomRegistry::new();
        reg.register(bom);

        let reqs = run_mrp(product, 10, &reg, &inv);
        assert_eq!(reqs[0].net_requirement, 0);
        assert_eq!(reqs[0].planned_order, 0);
    }

    #[test]
    fn mrp_reorder_quantity_rounding() {
        let mut inv = Inventory::new();
        let c1 = inv.add_sku("R", "pcs", 10, 25, 100);
        let product = inv.add_sku("P", "pcs", 0, 10, 0);
        // stock = 0, need 10, reorder_qty = 25 => planned = 25
        let mut bom = Bom::new(product);
        bom.add_component(c1, 1);

        let mut reg = BomRegistry::new();
        reg.register(bom);

        let reqs = run_mrp(product, 10, &reg, &inv);
        assert_eq!(reqs[0].net_requirement, 10);
        assert_eq!(reqs[0].planned_order, 25);
    }

    // === Purchase Order tests ===

    #[test]
    fn po_lifecycle() {
        let mut inv = Inventory::new();
        let sku_id = inv.add_sku("Part", "pcs", 10, 50, 100);

        let mut po = PurchaseOrder::new(1, "Supplier A", 1000);
        po.add_line(sku_id, 50, 95);
        assert_eq!(po.total(), 4750);
        assert_eq!(po.status, PoStatus::Draft);

        po.submit().unwrap();
        assert_eq!(po.status, PoStatus::Submitted);

        po.approve().unwrap();
        assert_eq!(po.status, PoStatus::Approved);

        po.receive(&mut inv).unwrap();
        assert_eq!(po.status, PoStatus::Received);
        assert_eq!(inv.get_sku(sku_id).unwrap().stock_on_hand, 50);
    }

    #[test]
    fn po_cancel_from_draft() {
        let mut po = PurchaseOrder::new(1, "S", 0);
        po.cancel().unwrap();
        assert_eq!(po.status, PoStatus::Cancelled);
    }

    #[test]
    fn po_cancel_from_submitted() {
        let mut po = PurchaseOrder::new(1, "S", 0);
        po.submit().unwrap();
        po.cancel().unwrap();
        assert_eq!(po.status, PoStatus::Cancelled);
    }

    #[test]
    fn po_cancel_from_approved() {
        let mut po = PurchaseOrder::new(1, "S", 0);
        po.submit().unwrap();
        po.approve().unwrap();
        po.cancel().unwrap();
        assert_eq!(po.status, PoStatus::Cancelled);
    }

    #[test]
    fn po_cannot_cancel_received() {
        let mut inv = Inventory::new();
        let mut po = PurchaseOrder::new(1, "S", 0);
        po.submit().unwrap();
        po.approve().unwrap();
        po.receive(&mut inv).unwrap();
        assert_eq!(po.cancel(), Err(ErpError::InvalidStatusTransition));
    }

    #[test]
    fn po_cannot_double_submit() {
        let mut po = PurchaseOrder::new(1, "S", 0);
        po.submit().unwrap();
        assert_eq!(po.submit(), Err(ErpError::InvalidStatusTransition));
    }

    #[test]
    fn po_cannot_approve_draft() {
        let mut po = PurchaseOrder::new(1, "S", 0);
        assert_eq!(po.approve(), Err(ErpError::InvalidStatusTransition));
    }

    #[test]
    fn po_cannot_receive_draft() {
        let mut inv = Inventory::new();
        let mut po = PurchaseOrder::new(1, "S", 0);
        assert_eq!(po.receive(&mut inv), Err(ErpError::InvalidStatusTransition));
    }

    #[test]
    fn po_line_total() {
        let line = PoLine {
            sku_id: 1,
            quantity: 10,
            unit_cost: 250,
        };
        assert_eq!(line.line_total(), 2500);
    }

    #[test]
    fn po_line_count() {
        let mut po = PurchaseOrder::new(1, "S", 0);
        po.add_line(1, 10, 100);
        po.add_line(2, 5, 200);
        assert_eq!(po.line_count(), 2);
    }

    #[test]
    fn po_manager_create_and_get() {
        let mut mgr = PoManager::new();
        let id = mgr.create_order("Vendor", 1000);
        assert!(mgr.get(id).is_some());
        assert_eq!(mgr.count(), 1);
    }

    #[test]
    fn po_manager_list_by_status() {
        let mut mgr = PoManager::new();
        let id1 = mgr.create_order("V1", 0);
        let id2 = mgr.create_order("V2", 0);
        mgr.get_mut(id1).unwrap().submit().unwrap();
        let drafts = mgr.list_by_status(PoStatus::Draft);
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].id, id2);
    }

    // === Work Order tests ===

    #[test]
    fn wo_lifecycle() {
        let mut inv = Inventory::new();
        let c1 = inv.add_sku("Raw", "kg", 10, 50, 100);
        let product = inv.add_sku("Finished", "pcs", 0, 10, 500);
        inv.get_sku_mut(c1).unwrap().receive(100).unwrap();

        let mut bom = Bom::new(product);
        bom.add_component(c1, 5);
        let mut reg = BomRegistry::new();
        reg.register(bom);

        let mut wo = WorkOrder::new(1, product, 10, 1000, 2000);
        assert_eq!(wo.status, WoStatus::Planned);

        wo.release().unwrap();
        assert_eq!(wo.status, WoStatus::Released);

        wo.start(1100).unwrap();
        assert_eq!(wo.status, WoStatus::InProgress);

        wo.complete(1900, 45000, &reg, &mut inv).unwrap();
        assert_eq!(wo.status, WoStatus::Completed);
        assert_eq!(inv.get_sku(c1).unwrap().stock_on_hand, 50); // 100 - 5*10
        assert_eq!(inv.get_sku(product).unwrap().stock_on_hand, 10);
    }

    #[test]
    fn wo_cancel() {
        let mut wo = WorkOrder::new(1, 1, 10, 0, 100);
        wo.cancel().unwrap();
        assert_eq!(wo.status, WoStatus::Cancelled);
    }

    #[test]
    fn wo_cannot_cancel_completed() {
        let mut inv = Inventory::new();
        let reg = BomRegistry::new();
        let mut wo = WorkOrder::new(1, 999, 10, 0, 100);
        wo.release().unwrap();
        wo.start(10).unwrap();
        wo.complete(50, 1000, &reg, &mut inv).unwrap();
        assert_eq!(wo.cancel(), Err(ErpError::InvalidStatusTransition));
    }

    #[test]
    fn wo_cannot_start_planned() {
        let mut wo = WorkOrder::new(1, 1, 10, 0, 100);
        assert_eq!(wo.start(0), Err(ErpError::InvalidStatusTransition));
    }

    #[test]
    fn wo_cannot_complete_released() {
        let reg = BomRegistry::new();
        let mut inv = Inventory::new();
        let mut wo = WorkOrder::new(1, 1, 10, 0, 100);
        wo.release().unwrap();
        assert_eq!(
            wo.complete(0, 0, &reg, &mut inv),
            Err(ErpError::InvalidStatusTransition)
        );
    }

    #[test]
    fn wo_planned_duration() {
        let wo = WorkOrder::new(1, 1, 10, 1000, 3600);
        assert_eq!(wo.planned_duration(), 2600);
    }

    #[test]
    fn wo_actual_duration() {
        let reg = BomRegistry::new();
        let mut inv = Inventory::new();
        let mut wo = WorkOrder::new(1, 999, 1, 0, 100);
        wo.release().unwrap();
        wo.start(50).unwrap();
        wo.complete(150, 0, &reg, &mut inv).unwrap();
        assert_eq!(wo.actual_duration(), Some(100));
    }

    #[test]
    fn wo_actual_duration_incomplete() {
        let wo = WorkOrder::new(1, 1, 10, 0, 100);
        assert_eq!(wo.actual_duration(), None);
    }

    #[test]
    fn wo_manager_create_and_get() {
        let mut mgr = WoManager::new();
        let id = mgr.create_order(1, 10, 0, 100);
        assert!(mgr.get(id).is_some());
        assert_eq!(mgr.count(), 1);
    }

    #[test]
    fn wo_manager_schedule_sorted() {
        let mut mgr = WoManager::new();
        mgr.create_order(1, 10, 500, 600);
        mgr.create_order(2, 5, 100, 200);
        mgr.create_order(3, 8, 300, 400);
        let sched = mgr.schedule();
        assert_eq!(sched[0].scheduled_start, 100);
        assert_eq!(sched[1].scheduled_start, 300);
        assert_eq!(sched[2].scheduled_start, 500);
    }

    #[test]
    fn wo_manager_conflicts() {
        let mut mgr = WoManager::new();
        // Two overlapping WOs for same product
        mgr.create_order(1, 10, 0, 200);
        mgr.create_order(1, 5, 100, 300);
        let conflicts = mgr.conflicts();
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn wo_manager_no_conflicts_different_products() {
        let mut mgr = WoManager::new();
        mgr.create_order(1, 10, 0, 200);
        mgr.create_order(2, 5, 100, 300);
        assert!(mgr.conflicts().is_empty());
    }

    #[test]
    fn wo_manager_no_conflicts_non_overlapping() {
        let mut mgr = WoManager::new();
        mgr.create_order(1, 10, 0, 100);
        mgr.create_order(1, 5, 100, 200);
        assert!(mgr.conflicts().is_empty());
    }

    #[test]
    fn wo_manager_list_by_status() {
        let mut mgr = WoManager::new();
        let id1 = mgr.create_order(1, 10, 0, 100);
        mgr.create_order(2, 5, 0, 100);
        mgr.get_mut(id1).unwrap().release().unwrap();
        let planned = mgr.list_by_status(WoStatus::Planned);
        assert_eq!(planned.len(), 1);
    }

    #[test]
    fn wo_insufficient_material() {
        let mut inv = Inventory::new();
        let c1 = inv.add_sku("R", "pcs", 10, 50, 100);
        let product = inv.add_sku("P", "pcs", 0, 10, 0);
        inv.get_sku_mut(c1).unwrap().receive(5).unwrap();

        let mut bom = Bom::new(product);
        bom.add_component(c1, 10);
        let mut reg = BomRegistry::new();
        reg.register(bom);

        let mut wo = WorkOrder::new(1, product, 1, 0, 100);
        wo.release().unwrap();
        wo.start(10).unwrap();
        assert_eq!(
            wo.complete(50, 0, &reg, &mut inv),
            Err(ErpError::InsufficientStock)
        );
    }

    // === Cost Accounting tests ===

    #[test]
    fn cost_variance_favorable() {
        let cv = CostVariance::new(1000, 900);
        assert!(cv.is_favorable);
        assert_eq!(cv.variance, -100);
    }

    #[test]
    fn cost_variance_unfavorable() {
        let cv = CostVariance::new(1000, 1200);
        assert!(!cv.is_favorable);
        assert_eq!(cv.variance, 200);
    }

    #[test]
    fn cost_variance_zero() {
        let cv = CostVariance::new(1000, 1000);
        assert!(cv.is_favorable);
        assert_eq!(cv.variance, 0);
    }

    #[test]
    fn cost_variance_bps() {
        let cv = CostVariance::new(10000, 11000);
        assert_eq!(cv.variance_bps(), 1000); // 10%
    }

    #[test]
    fn cost_variance_bps_zero_standard() {
        let cv = CostVariance::new(0, 100);
        assert_eq!(cv.variance_bps(), 0);
    }

    #[test]
    fn material_price_variance_test() {
        // Bought 100 units at 12 vs standard 10
        let v = material_price_variance(100, 12, 10);
        assert_eq!(v, 200); // unfavorable
    }

    #[test]
    fn material_price_variance_favorable() {
        let v = material_price_variance(100, 8, 10);
        assert_eq!(v, -200); // favorable
    }

    #[test]
    fn material_usage_variance_test() {
        // Used 110 units vs standard 100, at standard price 10
        let v = material_usage_variance(110, 100, 10);
        assert_eq!(v, 100); // unfavorable
    }

    #[test]
    fn material_usage_variance_favorable() {
        let v = material_usage_variance(90, 100, 10);
        assert_eq!(v, -100);
    }

    #[test]
    fn labor_rate_variance_test() {
        let v = labor_rate_variance(40, 22, 20);
        assert_eq!(v, 80); // unfavorable
    }

    #[test]
    fn labor_rate_variance_favorable() {
        let v = labor_rate_variance(40, 18, 20);
        assert_eq!(v, -80);
    }

    #[test]
    fn labor_efficiency_variance_test() {
        let v = labor_efficiency_variance(45, 40, 20);
        assert_eq!(v, 100); // unfavorable
    }

    #[test]
    fn labor_efficiency_variance_favorable() {
        let v = labor_efficiency_variance(35, 40, 20);
        assert_eq!(v, -100);
    }

    #[test]
    fn product_standard_cost_test() {
        let mut inv = Inventory::new();
        let c1 = inv.add_sku("A", "pcs", 0, 10, 100);
        let c2 = inv.add_sku("B", "pcs", 0, 10, 50);
        let product = inv.add_sku("P", "pcs", 0, 10, 0);

        let mut bom = Bom::new(product);
        bom.add_component(c1, 2);
        bom.add_component(c2, 3);

        assert_eq!(product_standard_cost(&bom, &inv), 350);
    }

    #[test]
    fn analyze_work_order_cost_test() {
        let mut inv = Inventory::new();
        let c1 = inv.add_sku("R", "pcs", 0, 10, 100);
        let product = inv.add_sku("P", "pcs", 0, 10, 0);

        let mut bom = Bom::new(product);
        bom.add_component(c1, 5);
        let mut reg = BomRegistry::new();
        reg.register(bom);

        let wo = WorkOrder {
            id: 1,
            product_sku_id: product,
            quantity: 10,
            status: WoStatus::Completed,
            scheduled_start: 0,
            scheduled_end: 100,
            actual_start: Some(0),
            actual_end: Some(90),
            actual_cost: 5500,
        };

        let cv = analyze_work_order_cost(&wo, &reg, &inv);
        assert_eq!(cv.standard_cost, 5000);
        assert_eq!(cv.actual_cost, 5500);
        assert_eq!(cv.variance, 500);
        assert!(!cv.is_favorable);
    }

    // === Production Scheduling tests ===

    #[test]
    fn forward_schedule_basic() {
        let jobs = vec![
            ProductionJob {
                work_order_id: 1,
                priority: Priority::Medium,
                duration_seconds: 100,
                earliest_start: 0,
            },
            ProductionJob {
                work_order_id: 2,
                priority: Priority::High,
                duration_seconds: 50,
                earliest_start: 0,
            },
        ];
        let sched = forward_schedule(&jobs, 0);
        // High priority first
        assert_eq!(sched[0].0, 2);
        assert_eq!(sched[0].1, 0);
        assert_eq!(sched[0].2, 50);
        assert_eq!(sched[1].0, 1);
        assert_eq!(sched[1].1, 50);
        assert_eq!(sched[1].2, 150);
    }

    #[test]
    fn forward_schedule_earliest_start_respected() {
        let jobs = vec![ProductionJob {
            work_order_id: 1,
            priority: Priority::Low,
            duration_seconds: 60,
            earliest_start: 200,
        }];
        let sched = forward_schedule(&jobs, 0);
        assert_eq!(sched[0].1, 200);
        assert_eq!(sched[0].2, 260);
    }

    #[test]
    fn forward_schedule_urgent() {
        let jobs = vec![
            ProductionJob {
                work_order_id: 1,
                priority: Priority::Low,
                duration_seconds: 10,
                earliest_start: 0,
            },
            ProductionJob {
                work_order_id: 2,
                priority: Priority::Urgent,
                duration_seconds: 10,
                earliest_start: 0,
            },
        ];
        let sched = forward_schedule(&jobs, 0);
        assert_eq!(sched[0].0, 2);
    }

    #[test]
    fn forward_schedule_empty() {
        let sched = forward_schedule(&[], 0);
        assert!(sched.is_empty());
    }

    #[test]
    fn forward_schedule_same_priority_sorted_by_earliest() {
        let jobs = vec![
            ProductionJob {
                work_order_id: 1,
                priority: Priority::Medium,
                duration_seconds: 30,
                earliest_start: 100,
            },
            ProductionJob {
                work_order_id: 2,
                priority: Priority::Medium,
                duration_seconds: 30,
                earliest_start: 50,
            },
        ];
        let sched = forward_schedule(&jobs, 0);
        assert_eq!(sched[0].0, 2);
        assert_eq!(sched[1].0, 1);
    }

    #[test]
    fn forward_schedule_chain() {
        let jobs = vec![
            ProductionJob {
                work_order_id: 1,
                priority: Priority::High,
                duration_seconds: 100,
                earliest_start: 0,
            },
            ProductionJob {
                work_order_id: 2,
                priority: Priority::High,
                duration_seconds: 100,
                earliest_start: 0,
            },
            ProductionJob {
                work_order_id: 3,
                priority: Priority::High,
                duration_seconds: 100,
                earliest_start: 0,
            },
        ];
        let sched = forward_schedule(&jobs, 0);
        assert_eq!(sched[0].2, 100);
        assert_eq!(sched[1].1, 100);
        assert_eq!(sched[1].2, 200);
        assert_eq!(sched[2].1, 200);
        assert_eq!(sched[2].2, 300);
    }

    // === Error display tests ===

    #[test]
    fn error_display() {
        assert_eq!(ErpError::InvalidQuantity.to_string(), "invalid quantity");
        assert_eq!(
            ErpError::InsufficientStock.to_string(),
            "insufficient stock"
        );
        assert_eq!(
            ErpError::InvalidStatusTransition.to_string(),
            "invalid status transition"
        );
        assert_eq!(ErpError::NotFound.to_string(), "not found");
    }

    #[test]
    fn error_is_error_trait() {
        let e: Box<dyn std::error::Error> = Box::new(ErpError::NotFound);
        assert_eq!(e.to_string(), "not found");
    }

    // === Integration tests ===

    #[test]
    fn full_erp_cycle() {
        // Setup inventory
        let mut inv = Inventory::new();
        let steel = inv.add_sku("Steel Sheet", "pcs", 20, 100, 500);
        let bolt = inv.add_sku("M8 Bolt", "pcs", 100, 500, 10);
        let paint = inv.add_sku("Paint", "L", 5, 20, 300);
        let chassis = inv.add_sku("Chassis", "pcs", 0, 10, 0);

        // Define BOM
        let mut bom = Bom::new(chassis);
        bom.add_component(steel, 4);
        bom.add_component(bolt, 16);
        bom.add_component(paint, 2);

        let mut bom_reg = BomRegistry::new();
        bom_reg.register(bom);

        // Run MRP for 10 chassis
        let reqs = run_mrp(chassis, 10, &bom_reg, &inv);
        assert_eq!(reqs.len(), 3);

        // Create POs based on MRP
        let mut po_mgr = PoManager::new();
        for req in &reqs {
            if req.planned_order > 0 {
                let po_id = po_mgr.create_order("Materials Inc.", 1000);
                let po = po_mgr.get_mut(po_id).unwrap();
                let unit_cost = inv.get_sku(req.sku_id).unwrap().standard_cost;
                po.add_line(req.sku_id, req.planned_order, unit_cost);
                po.submit().unwrap();
                po.approve().unwrap();
                po.receive(&mut inv).unwrap();
            }
        }

        // Verify sufficient material
        assert!(bom_reg.get(chassis).unwrap().can_produce(&inv, 10));

        // Create and execute work order
        let mut wo_mgr = WoManager::new();
        let wo_id = wo_mgr.create_order(chassis, 10, 2000, 5000);
        let wo = wo_mgr.get_mut(wo_id).unwrap();
        wo.release().unwrap();
        wo.start(2100).unwrap();

        let actual_cost = 25000;
        wo.complete(4800, actual_cost, &bom_reg, &mut inv).unwrap();

        // Verify finished goods received
        assert_eq!(inv.get_sku(chassis).unwrap().stock_on_hand, 10);

        // Cost variance analysis
        let cv = analyze_work_order_cost(wo_mgr.get(wo_id).unwrap(), &bom_reg, &inv);
        let standard = 10 * (4 * 500 + 16 * 10 + 2 * 300); // 10 * (2000+160+600) = 27600
        assert_eq!(cv.standard_cost, standard);
        assert_eq!(cv.actual_cost, actual_cost);
        assert!(cv.is_favorable); // 25000 < 27600
    }

    #[test]
    fn priority_ordering() {
        assert!(Priority::Low < Priority::Medium);
        assert!(Priority::Medium < Priority::High);
        assert!(Priority::High < Priority::Urgent);
    }

    #[test]
    fn sku_description_default_empty() {
        let sku = Sku::new(1, "Test", "pcs", 0, 0, 0);
        assert!(sku.description.is_empty());
    }

    #[test]
    fn inventory_get_sku_mut_works() {
        let mut inv = Inventory::new();
        let id = inv.add_sku("Test", "pcs", 0, 0, 100);
        inv.get_sku_mut(id).unwrap().description = "Modified".to_owned();
        assert_eq!(inv.get_sku(id).unwrap().description, "Modified");
    }

    #[test]
    fn po_total_empty() {
        let po = PurchaseOrder::new(1, "S", 0);
        assert_eq!(po.total(), 0);
    }

    #[test]
    fn po_multiple_lines_total() {
        let mut po = PurchaseOrder::new(1, "S", 0);
        po.add_line(1, 10, 100);
        po.add_line(2, 5, 200);
        po.add_line(3, 20, 50);
        assert_eq!(po.total(), 10 * 100 + 5 * 200 + 20 * 50);
    }

    #[test]
    fn wo_cancel_in_progress() {
        let mut wo = WorkOrder::new(1, 1, 10, 0, 100);
        wo.release().unwrap();
        wo.start(10).unwrap();
        wo.cancel().unwrap();
        assert_eq!(wo.status, WoStatus::Cancelled);
    }

    #[test]
    fn wo_cancel_released() {
        let mut wo = WorkOrder::new(1, 1, 10, 0, 100);
        wo.release().unwrap();
        wo.cancel().unwrap();
        assert_eq!(wo.status, WoStatus::Cancelled);
    }

    #[test]
    fn wo_cannot_release_twice() {
        let mut wo = WorkOrder::new(1, 1, 10, 0, 100);
        wo.release().unwrap();
        assert_eq!(wo.release(), Err(ErpError::InvalidStatusTransition));
    }

    #[test]
    fn bom_can_produce_missing_sku() {
        let inv = Inventory::new();
        let mut bom = Bom::new(1);
        bom.add_component(999, 1);
        assert!(!bom.can_produce(&inv, 1));
    }

    #[test]
    fn po_manager_get_nonexistent() {
        let mgr = PoManager::new();
        assert!(mgr.get(999).is_none());
    }

    #[test]
    fn wo_manager_get_nonexistent() {
        let mgr = WoManager::new();
        assert!(mgr.get(999).is_none());
    }

    #[test]
    fn cost_variance_large_negative() {
        let cv = CostVariance::new(10000, 5000);
        assert!(cv.is_favorable);
        assert_eq!(cv.variance, -5000);
        assert_eq!(cv.variance_bps(), -5000); // -50%
    }

    #[test]
    fn sku_receive_zero() {
        let mut sku = Sku::new(1, "T", "pcs", 0, 0, 0);
        sku.receive(0).unwrap();
        assert_eq!(sku.stock_on_hand, 0);
    }

    #[test]
    fn sku_issue_zero() {
        let mut sku = Sku::new(1, "T", "pcs", 0, 0, 0);
        sku.issue(0).unwrap();
        assert_eq!(sku.stock_on_hand, 0);
    }

    #[test]
    fn sku_issue_exact_stock() {
        let mut sku = Sku::new(1, "T", "pcs", 0, 0, 0);
        sku.receive(10).unwrap();
        sku.issue(10).unwrap();
        assert_eq!(sku.stock_on_hand, 0);
    }

    #[test]
    fn inventory_reorder_list_empty() {
        let mut inv = Inventory::new();
        let id = inv.add_sku("X", "pcs", 5, 50, 100);
        inv.get_sku_mut(id).unwrap().receive(100).unwrap();
        assert!(inv.reorder_list().is_empty());
    }

    #[test]
    fn inventory_multiple_skus_value() {
        let mut inv = Inventory::new();
        let a = inv.add_sku("A", "pcs", 0, 10, 100);
        let b = inv.add_sku("B", "pcs", 0, 10, 200);
        inv.get_sku_mut(a).unwrap().receive(5).unwrap();
        inv.get_sku_mut(b).unwrap().receive(3).unwrap();
        assert_eq!(inv.total_value(), 5 * 100 + 3 * 200);
    }

    #[test]
    fn bom_standard_cost_zero_qty() {
        let inv = Inventory::new();
        let mut bom = Bom::new(1);
        bom.add_component(1, 5);
        assert_eq!(bom.standard_cost(&inv, 0), 0);
    }

    #[test]
    fn bom_can_produce_zero_qty() {
        let inv = Inventory::new();
        let bom = Bom::new(1);
        assert!(bom.can_produce(&inv, 0));
    }

    #[test]
    fn mrp_multiple_components() {
        let mut inv = Inventory::new();
        let c1 = inv.add_sku("C1", "pcs", 0, 10, 100);
        let c2 = inv.add_sku("C2", "pcs", 0, 20, 50);
        let c3 = inv.add_sku("C3", "kg", 0, 5, 200);
        let product = inv.add_sku("P", "pcs", 0, 10, 0);

        let mut bom = Bom::new(product);
        bom.add_component(c1, 1);
        bom.add_component(c2, 2);
        bom.add_component(c3, 3);

        let mut reg = BomRegistry::new();
        reg.register(bom);

        let reqs = run_mrp(product, 5, &reg, &inv);
        assert_eq!(reqs.len(), 3);
        assert_eq!(reqs[0].gross_requirement, 5);
        assert_eq!(reqs[1].gross_requirement, 10);
        assert_eq!(reqs[2].gross_requirement, 15);
    }

    #[test]
    fn po_receive_updates_multiple_skus() {
        let mut inv = Inventory::new();
        let a = inv.add_sku("A", "pcs", 0, 10, 100);
        let b = inv.add_sku("B", "pcs", 0, 10, 200);

        let mut po = PurchaseOrder::new(1, "Vendor", 0);
        po.add_line(a, 10, 100);
        po.add_line(b, 5, 200);
        po.submit().unwrap();
        po.approve().unwrap();
        po.receive(&mut inv).unwrap();

        assert_eq!(inv.get_sku(a).unwrap().stock_on_hand, 10);
        assert_eq!(inv.get_sku(b).unwrap().stock_on_hand, 5);
    }

    #[test]
    fn wo_planned_duration_zero() {
        let wo = WorkOrder::new(1, 1, 1, 500, 500);
        assert_eq!(wo.planned_duration(), 0);
    }

    #[test]
    fn wo_complete_no_bom_still_receives_product() {
        let mut inv = Inventory::new();
        let product = inv.add_sku("P", "pcs", 0, 10, 500);
        let reg = BomRegistry::new();

        let mut wo = WorkOrder::new(1, product, 5, 0, 100);
        wo.release().unwrap();
        wo.start(10).unwrap();
        wo.complete(50, 2000, &reg, &mut inv).unwrap();
        assert_eq!(inv.get_sku(product).unwrap().stock_on_hand, 5);
    }

    #[test]
    fn forward_schedule_with_offset_start() {
        let jobs = vec![ProductionJob {
            work_order_id: 1,
            priority: Priority::Medium,
            duration_seconds: 50,
            earliest_start: 0,
        }];
        let sched = forward_schedule(&jobs, 100);
        assert_eq!(sched[0].1, 100);
        assert_eq!(sched[0].2, 150);
    }

    #[test]
    fn cost_variance_negative_standard() {
        let cv = CostVariance::new(-100, 50);
        assert!(!cv.is_favorable);
        assert_eq!(cv.variance, 150);
    }

    #[test]
    fn material_price_variance_zero() {
        assert_eq!(material_price_variance(100, 10, 10), 0);
    }

    #[test]
    fn material_usage_variance_zero() {
        assert_eq!(material_usage_variance(100, 100, 10), 0);
    }

    #[test]
    fn labor_rate_variance_zero() {
        assert_eq!(labor_rate_variance(40, 20, 20), 0);
    }

    #[test]
    fn labor_efficiency_variance_zero() {
        assert_eq!(labor_efficiency_variance(40, 40, 20), 0);
    }
}
