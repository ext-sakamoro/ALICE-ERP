[English](README.md) | **日本語**

# ALICE-ERP

ALICEエコシステムの統合業務 (ERP) モジュール。在庫管理、部品表 (BOM)、所要量計画 (MRP)、生産スケジューリング、原価計算、発注管理、作業指示を純Rustで実装。

## 概要

| 項目 | 値 |
|------|-----|
| **クレート名** | `alice-erp` |
| **バージョン** | 1.0.0 |
| **ライセンス** | AGPL-3.0 |
| **エディション** | 2021 |

## 機能

- **在庫管理** — 在庫数・発注点・入庫/出庫オペレーション付きSKU追跡
- **部品表 (BOM)** — 構成部品数量とコストロールアップ付き多段BOM
- **所要量計画 (MRP)** — リードタイムとロットサイズを考慮した需要駆動計画
- **生産スケジューリング** — リソース割当付き作業指示スケジューリング
- **原価計算** — SKU別の標準原価・実際原価・差異分析
- **発注管理** — 発注書作成、承認ワークフロー、入庫処理
- **作業指示** — ステータス遷移付き生産実行追跡

## アーキテクチャ

```
alice-erp (lib.rs — 単一ファイルクレート)
├── Id / Money / Timestamp       # 共通型エイリアス
├── Sku / Inventory              # 在庫管理
├── BomItem / Bom                # 部品表
├── MrpPlan / MrpEngine          # 所要量計画
├── WorkOrder / Scheduler        # 生産スケジューリング
├── PurchaseOrder                # 調達管理
└── ErpEngine                    # トップレベルオーケストレーター
```

## クイックスタート

```rust
use alice_erp::{ErpEngine, Sku};

let mut erp = ErpEngine::new();
let sku = Sku::new(1, "Widget-A", "pcs", 100, 500, 250);
erp.inventory.add_sku(sku);
erp.inventory.receive(1, 1000).unwrap();
```

## ビルド

```bash
cargo build
cargo test
cargo clippy -- -W clippy::all
```

## ライセンス

AGPL-3.0 — 詳細は [LICENSE](LICENSE) を参照。
