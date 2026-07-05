//! errors.

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
