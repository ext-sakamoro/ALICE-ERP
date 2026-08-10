//! Integration tests.

#![allow(
    clippy::wildcard_imports,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::unwrap_used,
    clippy::indexing_slicing
)]

use crate::bom::*;
use crate::cost::*;
use crate::errors::*;
use crate::inventory::*;
use crate::mrp::*;
use crate::purchase::*;
use crate::scheduling::*;
use crate::work_order::*;

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
