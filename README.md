**English** | [日本語](README_JP.md)

# ALICE-ERP

Enterprise Resource Planning module for the ALICE ecosystem. Pure Rust implementation covering inventory management, bill of materials, MRP, production scheduling, cost accounting, purchase orders, and work orders.

## Overview

| Item | Value |
|------|-------|
| **Crate** | `alice-erp` |
| **Version** | 1.0.0 |
| **License** | AGPL-3.0 |
| **Edition** | 2021 |

## Features

- **Inventory Management** — SKU tracking with stock-on-hand, reorder points, receive/issue operations
- **Bill of Materials (BOM)** — Multi-level BOM with component quantities and cost rollup
- **Material Requirements Planning (MRP)** — Demand-driven planning with lead time and lot sizing
- **Production Scheduling** — Work order scheduling with resource allocation
- **Cost Accounting** — Standard cost, actual cost, and variance analysis per SKU
- **Purchase Orders** — PO creation, approval workflow, and goods receipt
- **Work Orders** — Production execution tracking with status transitions

## Architecture

```
alice-erp (lib.rs — single-file crate)
├── Id / Money / Timestamp       # Common type aliases
├── Sku / Inventory              # Stock management
├── BomItem / Bom                # Bill of Materials
├── MrpPlan / MrpEngine          # Material planning
├── WorkOrder / Scheduler        # Production scheduling
├── PurchaseOrder                # Procurement
└── ErpEngine                    # Top-level orchestrator
```

## Quick Start

```rust
use alice_erp::{ErpEngine, Sku};

let mut erp = ErpEngine::new();
let sku = Sku::new(1, "Widget-A", "pcs", 100, 500, 250);
erp.inventory.add_sku(sku);
erp.inventory.receive(1, 1000).unwrap();
```

## Build

```bash
cargo build
cargo test
cargo clippy -- -W clippy::all
```

## License

`AGPL-3.0 OR LicenseRef-Commercial` — dual-licensed. Pick either.

| Option | Terms | Use it when |
|--------|-------|-------------|
| **AGPL-3.0** | [LICENSE-AGPL](LICENSE-AGPL) — free, no reporting obligation | Your project is itself AGPL-compatible open source, or you are only using it internally |
| **Commercial License** | [LICENSE-COMMERCIAL.md](LICENSE-COMMERCIAL.md) — paid, removes the copyleft | Closed-source product, proprietary SaaS, edge / firmware distribution, plugin redistribution, or a platform NDA that forbids source disclosure |

AGPL is a strong copyleft: a product, firmware image, or service that links
`alice-erp` and is distributed or served to users must be released under the AGPL
as well. That is intentional for the open ecosystem, and the Commercial
License exists for the cases where it is not something you are able to do.

Commercial licence enquiries: <contact@extoria.co.jp>
