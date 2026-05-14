//! Deposit (Flow A) — high-level helpers.
//!
//! The actual `DepositNote` construction is delegated to a future call
//! site that has `miden-client` available. This module declares the
//! types the SDK will use to construct the note.

use crate::BasketHandle;

/// A user's intent to deposit into a basket.
#[derive(Debug, Clone)]
pub struct DepositRequest {
    pub basket: BasketHandle,
    /// One entry per asset the user wants to deposit. Subset of the
    /// basket's constituents is allowed.
    pub assets: Vec<DepositAssetEntry>,
    /// Block at which the note expires if not consumed.
    pub expiry_block: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct DepositAssetEntry {
    pub faucet_id: u64,
    pub amount: u64,
}
