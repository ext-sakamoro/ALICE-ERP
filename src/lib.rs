//! ALICE-ERP: Enterprise Resource Planning.

#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::module_name_repetitions,
    clippy::doc_markdown,
    clippy::wildcard_imports,
    clippy::too_many_lines,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::return_self_not_must_use
)]

pub mod bom;
pub mod common;
pub mod cost;
pub mod errors;
pub mod inventory;
pub mod mrp;
pub mod prelude;
pub mod purchase;
pub mod scheduling;
pub mod signed_inventory;
pub mod work_order;

#[cfg(test)]
mod integration_tests;

pub use crate::bom::*;
pub use crate::common::*;
pub use crate::cost::*;
pub use crate::errors::*;
pub use crate::inventory::*;
pub use crate::mrp::*;
pub use crate::purchase::*;
pub use crate::scheduling::*;
pub use crate::work_order::*;
