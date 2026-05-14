//! Redeem (Miden-side of Flow C) — high-level helpers.

use crate::BasketHandle;

/// A user's intent to redeem from a basket.
#[derive(Debug, Clone)]
pub struct RedeemRequest {
    pub basket: BasketHandle,
    /// Amount of basket token to burn, in basket-token base units.
    pub burn_amount: u64,
    /// Miden account id that receives the underlyings. For ETH-user
    /// flows (M2), this is the Miden Guardian relay wallet that then
    /// chains B2AGG notes to bridge each underlying to L1.
    pub recipient_account_id: u64,
    pub expiry_block: u64,
}
