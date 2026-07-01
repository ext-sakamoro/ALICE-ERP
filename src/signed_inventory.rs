//! `signed_inventory` — tamper-evident inventory + purchase-order ledger.
//!
//! Every material movement (goods receipt, goods issue, stock adjustment,
//! purchase order, work-order consumption, cycle-count correction) is
//! captured in an `Ed25519`-signed record chained via `prev_hash → hash`.
//! Auditors can replay the log and prove no back-dated inventory
//! adjustments occurred between two dated snapshots.
//!
//! # Regulatory alignment
//!
//! - **`SOX` §404** — inventory is a material account subject to
//!   internal-control testing; ISA 315 / AICPA AU-C 315 require
//!   tamper-evident records of adjustments.
//! - **`ISO 9001` §7.1.5.2** — measurement traceability requires
//!   retained calibration and inventory records.
//! - **`PCI-DSS` v4.0 §9.4** — physical media inventory (cardholder
//!   data storage devices) requires quarterly counts with retained logs.
//! - **金融商品取引法 §193-2** — 内部統制報告書 (J-SOX); 棚卸資産の
//!   実在性は監査対象.
//!
//! Cryptographic primitives are provided by `alice-blockchain` (`Ed25519`).

#![allow(
    clippy::doc_markdown,
    clippy::missing_panics_doc,
    clippy::too_many_arguments,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation
)]

use alice_blockchain::signature::{KeyPair, PublicKey, Signature};

// ---------------------------------------------------------------------------
// InventoryEventKind
// ---------------------------------------------------------------------------

/// Inventory event captured in the trail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InventoryEventKind {
    /// Goods receipt from a purchase order or transfer.
    GoodsReceipt,
    /// Goods issue to a sales order, work order, or transfer.
    GoodsIssue,
    /// Cycle-count correction to reconcile physical vs. book stock.
    CountCorrection,
    /// Stock adjustment (scrap, damage, obsolescence write-off).
    Adjustment,
    /// A purchase order was placed with a supplier.
    PurchaseOrder,
    /// A work order consumed components against the BOM.
    WorkOrderConsumption,
}

impl InventoryEventKind {
    /// Short code used in canonical serialization.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::GoodsReceipt => "RCPT",
            Self::GoodsIssue => "ISSUE",
            Self::CountCorrection => "COUNT",
            Self::Adjustment => "ADJ",
            Self::PurchaseOrder => "PO",
            Self::WorkOrderConsumption => "WOC",
        }
    }
}

// ---------------------------------------------------------------------------
// InventoryRecord
// ---------------------------------------------------------------------------

/// One inventory movement ready to be signed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryRecord {
    /// Monotonic sequence number.
    pub seq: u64,
    /// Kind of event.
    pub kind: InventoryEventKind,
    /// Unix nanosecond timestamp.
    pub timestamp_ns: u64,
    /// SKU / material master identifier.
    pub sku: String,
    /// Warehouse / storage location code.
    pub warehouse: String,
    /// Signed quantity delta (positive for receipt, negative for issue).
    pub quantity: i64,
    /// Unit-of-measure code (`EA`, `KG`, `L`, `M`, ...).
    pub uom: String,
    /// Signed unit cost in minor currency units (0 if not applicable).
    pub unit_cost_minor: i64,
    /// Operator user id (warehouse clerk, buyer, planner).
    pub operator_id: String,
    /// Free-form reason / reference (PO number, work order id).
    pub reference: String,
    /// Hash of the previous record (0 for genesis).
    pub prev_hash: u64,
}

impl InventoryRecord {
    /// Canonical byte layout used for hashing and signing.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(200);
        buf.extend_from_slice(&self.seq.to_le_bytes());
        buf.extend_from_slice(self.kind.code().as_bytes());
        buf.push(0);
        buf.extend_from_slice(&self.timestamp_ns.to_le_bytes());
        buf.extend_from_slice(self.sku.as_bytes());
        buf.push(0);
        buf.extend_from_slice(self.warehouse.as_bytes());
        buf.push(0);
        buf.extend_from_slice(&self.quantity.to_le_bytes());
        buf.extend_from_slice(self.uom.as_bytes());
        buf.push(0);
        buf.extend_from_slice(&self.unit_cost_minor.to_le_bytes());
        buf.extend_from_slice(self.operator_id.as_bytes());
        buf.push(0);
        buf.extend_from_slice(self.reference.as_bytes());
        buf.push(0);
        buf.extend_from_slice(&self.prev_hash.to_le_bytes());
        buf
    }

    /// `FNV-1a` hash of the canonical byte layout.
    #[must_use]
    pub fn hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for &b in &self.canonical_bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        h
    }
}

// ---------------------------------------------------------------------------
// SignedInventoryRecord
// ---------------------------------------------------------------------------

/// [`InventoryRecord`] plus the operator's `Ed25519` signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedInventoryRecord {
    /// The wrapped record.
    pub record: InventoryRecord,
    /// `FNV-1a` hash of the record's canonical bytes.
    pub hash: u64,
    /// `Ed25519` signature over the canonical bytes.
    pub signature: Signature,
    /// Operator's `Ed25519` public key.
    pub operator: PublicKey,
}

impl SignedInventoryRecord {
    /// Verify signature and hash consistency.
    #[must_use]
    pub fn verify(&self) -> bool {
        if self.hash != self.record.hash() {
            return false;
        }
        self.operator
            .verify(&self.record.canonical_bytes(), &self.signature)
    }
}

// ---------------------------------------------------------------------------
// InventoryTrail
// ---------------------------------------------------------------------------

/// Append-only chain of [`SignedInventoryRecord`] records.
#[derive(Debug, Clone, Default)]
pub struct InventoryTrail {
    entries: Vec<SignedInventoryRecord>,
}

impl InventoryTrail {
    /// Construct an empty trail.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Number of entries.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the trail is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Read-only view.
    #[must_use]
    pub fn entries(&self) -> &[SignedInventoryRecord] {
        &self.entries
    }

    /// Hash of the last record (0 for empty).
    #[must_use]
    pub fn tail_hash(&self) -> u64 {
        self.entries.last().map_or(0, |e| e.hash)
    }

    /// Append a new inventory event signed with the operator's key pair.
    pub fn append(
        &mut self,
        keypair: &KeyPair,
        kind: InventoryEventKind,
        timestamp_ns: u64,
        sku: impl Into<String>,
        warehouse: impl Into<String>,
        quantity: i64,
        uom: impl Into<String>,
        unit_cost_minor: i64,
        operator_id: impl Into<String>,
        reference: impl Into<String>,
    ) -> &SignedInventoryRecord {
        let seq = self.entries.len() as u64;
        let prev_hash = self.tail_hash();
        let record = InventoryRecord {
            seq,
            kind,
            timestamp_ns,
            sku: sku.into(),
            warehouse: warehouse.into(),
            quantity,
            uom: uom.into(),
            unit_cost_minor,
            operator_id: operator_id.into(),
            reference: reference.into(),
            prev_hash,
        };
        let bytes = record.canonical_bytes();
        let hash = record.hash();
        let signature = keypair.sign(&bytes);
        let operator = keypair.public();
        self.entries.push(SignedInventoryRecord {
            record,
            hash,
            signature,
            operator,
        });
        self.entries.last().expect("entry was just pushed")
    }

    /// Verify signature and chain integrity end-to-end.
    #[must_use]
    pub fn find_first_tamper(&self) -> Option<usize> {
        let mut expected_prev: u64 = 0;
        for (i, e) in self.entries.iter().enumerate() {
            if e.record.seq as usize != i {
                return Some(i);
            }
            if e.record.prev_hash != expected_prev {
                return Some(i);
            }
            if !e.verify() {
                return Some(i);
            }
            expected_prev = e.hash;
        }
        None
    }

    /// Whether the trail is intact.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.find_first_tamper().is_none()
    }

    /// Current on-hand quantity for the given SKU and warehouse.
    #[must_use]
    pub fn on_hand(&self, sku: &str, warehouse: &str) -> i64 {
        self.entries
            .iter()
            .filter(|e| e.record.sku == sku && e.record.warehouse == warehouse)
            .map(|e| e.record.quantity)
            .sum()
    }

    /// Every distinct SKU seen in the trail.
    #[must_use]
    pub fn skus(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for e in &self.entries {
            if !out.contains(&e.record.sku) {
                out.push(e.record.sku.clone());
            }
        }
        out
    }

    /// Count of events of the given kind.
    #[must_use]
    pub fn count_kind(&self, kind: InventoryEventKind) -> usize {
        self.entries
            .iter()
            .filter(|e| e.record.kind == kind)
            .count()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn kp(seed: u8) -> KeyPair {
        KeyPair::from_seed([seed; 32])
    }

    #[test]
    fn kind_code_is_stable() {
        assert_eq!(InventoryEventKind::GoodsReceipt.code(), "RCPT");
        assert_eq!(InventoryEventKind::GoodsIssue.code(), "ISSUE");
        assert_eq!(InventoryEventKind::CountCorrection.code(), "COUNT");
        assert_eq!(InventoryEventKind::Adjustment.code(), "ADJ");
        assert_eq!(InventoryEventKind::PurchaseOrder.code(), "PO");
        assert_eq!(InventoryEventKind::WorkOrderConsumption.code(), "WOC");
    }

    #[test]
    fn canonical_bytes_are_deterministic() {
        let r = InventoryRecord {
            seq: 0,
            kind: InventoryEventKind::GoodsReceipt,
            timestamp_ns: 1,
            sku: String::from("SKU-001"),
            warehouse: String::from("WH-A"),
            quantity: 100,
            uom: String::from("EA"),
            unit_cost_minor: 1_000,
            operator_id: String::from("op-1"),
            reference: String::from("PO-100"),
            prev_hash: 0,
        };
        assert_eq!(r.canonical_bytes(), r.canonical_bytes());
    }

    #[test]
    fn hash_differs_when_quantity_changes() {
        let mut r = InventoryRecord {
            seq: 0,
            kind: InventoryEventKind::GoodsReceipt,
            timestamp_ns: 1,
            sku: String::from("SKU-001"),
            warehouse: String::from("WH-A"),
            quantity: 100,
            uom: String::from("EA"),
            unit_cost_minor: 1_000,
            operator_id: String::from("op-1"),
            reference: String::new(),
            prev_hash: 0,
        };
        let h1 = r.hash();
        r.quantity = 9_999;
        assert_ne!(h1, r.hash());
    }

    #[test]
    fn empty_trail_tail_hash_is_zero() {
        let trail = InventoryTrail::new();
        assert_eq!(trail.tail_hash(), 0);
        assert!(trail.is_empty());
    }

    #[test]
    fn signed_record_verifies_on_append() {
        let mut trail = InventoryTrail::new();
        let k = kp(1);
        trail.append(
            &k,
            InventoryEventKind::GoodsReceipt,
            1,
            "SKU-001",
            "WH-A",
            100,
            "EA",
            1_000,
            "op-1",
            "PO-100",
        );
        assert!(trail.entries()[0].verify());
    }

    #[test]
    fn chained_prev_hash_matches_predecessor() {
        let mut trail = InventoryTrail::new();
        let k = kp(1);
        trail.append(
            &k,
            InventoryEventKind::GoodsReceipt,
            1,
            "SKU-001",
            "WH-A",
            100,
            "EA",
            1_000,
            "op-1",
            "",
        );
        trail.append(
            &k,
            InventoryEventKind::GoodsIssue,
            2,
            "SKU-001",
            "WH-A",
            -30,
            "EA",
            0,
            "op-1",
            "",
        );
        let first = trail.entries()[0].hash;
        assert_eq!(trail.entries()[1].record.prev_hash, first);
    }

    #[test]
    fn intact_trail_is_valid() {
        let mut trail = InventoryTrail::new();
        let k = kp(1);
        for i in 0..5 {
            trail.append(
                &k,
                InventoryEventKind::GoodsReceipt,
                i,
                "SKU-001",
                "WH-A",
                10,
                "EA",
                100,
                "op",
                "",
            );
        }
        assert!(trail.is_valid());
    }

    #[test]
    fn tampered_quantity_is_detected() {
        let mut trail = InventoryTrail::new();
        let k = kp(1);
        trail.append(
            &k,
            InventoryEventKind::Adjustment,
            1,
            "SKU-001",
            "WH-A",
            -5,
            "EA",
            0,
            "op",
            "scrap",
        );
        // Attacker rewrites -5 → -500 to hide theft.
        trail.entries[0].record.quantity = -500;
        assert!(!trail.entries[0].verify());
        assert_eq!(trail.find_first_tamper(), Some(0));
    }

    #[test]
    fn tampered_sku_is_detected() {
        let mut trail = InventoryTrail::new();
        let k = kp(1);
        trail.append(
            &k,
            InventoryEventKind::GoodsReceipt,
            1,
            "SKU-original",
            "WH-A",
            100,
            "EA",
            1_000,
            "op",
            "",
        );
        trail.entries[0].record.sku = String::from("SKU-attacker");
        assert!(!trail.entries[0].verify());
    }

    #[test]
    fn foreign_operator_signature_is_rejected() {
        let mut trail = InventoryTrail::new();
        let genuine = kp(1);
        let attacker = kp(2);
        trail.append(
            &genuine,
            InventoryEventKind::GoodsReceipt,
            1,
            "SKU-001",
            "WH-A",
            100,
            "EA",
            1_000,
            "op",
            "",
        );
        let bytes = trail.entries[0].record.canonical_bytes();
        trail.entries[0].signature = attacker.sign(&bytes);
        assert!(!trail.entries[0].verify());
    }

    #[test]
    fn on_hand_sums_deltas_per_sku_and_warehouse() {
        let mut trail = InventoryTrail::new();
        let k = kp(1);
        trail.append(
            &k,
            InventoryEventKind::GoodsReceipt,
            1,
            "SKU-A",
            "WH-1",
            100,
            "EA",
            0,
            "op",
            "",
        );
        trail.append(
            &k,
            InventoryEventKind::GoodsIssue,
            2,
            "SKU-A",
            "WH-1",
            -30,
            "EA",
            0,
            "op",
            "",
        );
        trail.append(
            &k,
            InventoryEventKind::GoodsReceipt,
            3,
            "SKU-A",
            "WH-2",
            50,
            "EA",
            0,
            "op",
            "",
        );
        assert_eq!(trail.on_hand("SKU-A", "WH-1"), 70);
        assert_eq!(trail.on_hand("SKU-A", "WH-2"), 50);
        assert_eq!(trail.on_hand("SKU-B", "WH-1"), 0);
    }

    #[test]
    fn skus_lists_distinct() {
        let mut trail = InventoryTrail::new();
        let k = kp(1);
        trail.append(
            &k,
            InventoryEventKind::GoodsReceipt,
            1,
            "SKU-A",
            "WH-1",
            100,
            "EA",
            0,
            "op",
            "",
        );
        trail.append(
            &k,
            InventoryEventKind::GoodsReceipt,
            2,
            "SKU-B",
            "WH-1",
            50,
            "EA",
            0,
            "op",
            "",
        );
        trail.append(
            &k,
            InventoryEventKind::GoodsIssue,
            3,
            "SKU-A",
            "WH-1",
            -10,
            "EA",
            0,
            "op",
            "",
        );
        let skus = trail.skus();
        assert_eq!(skus.len(), 2);
        assert!(skus.contains(&String::from("SKU-A")));
        assert!(skus.contains(&String::from("SKU-B")));
    }

    #[test]
    fn count_kind_filters() {
        let mut trail = InventoryTrail::new();
        let k = kp(1);
        for _ in 0..3 {
            trail.append(
                &k,
                InventoryEventKind::GoodsReceipt,
                0,
                "SKU",
                "WH",
                1,
                "EA",
                0,
                "op",
                "",
            );
        }
        for _ in 0..2 {
            trail.append(
                &k,
                InventoryEventKind::PurchaseOrder,
                0,
                "SKU",
                "WH",
                0,
                "EA",
                1000,
                "op",
                "PO",
            );
        }
        assert_eq!(trail.count_kind(InventoryEventKind::GoodsReceipt), 3);
        assert_eq!(trail.count_kind(InventoryEventKind::PurchaseOrder), 2);
        assert_eq!(trail.count_kind(InventoryEventKind::Adjustment), 0);
    }

    #[test]
    fn different_kinds_produce_different_hashes() {
        let mk = |kind: InventoryEventKind| InventoryRecord {
            seq: 0,
            kind,
            timestamp_ns: 1,
            sku: String::new(),
            warehouse: String::new(),
            quantity: 0,
            uom: String::new(),
            unit_cost_minor: 0,
            operator_id: String::new(),
            reference: String::new(),
            prev_hash: 0,
        };
        assert_ne!(
            mk(InventoryEventKind::GoodsReceipt).hash(),
            mk(InventoryEventKind::GoodsIssue).hash()
        );
    }
}
