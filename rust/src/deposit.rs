//! Deposit (Flow A) — high-level helpers.
//!
//! The actual `DepositNote` construction is delegated to a future call
//! site that has `miden-client` available. This module declares the
//! types the SDK uses to assemble a deposit request, and provides
//! the validation logic that gates note construction.

use crate::{BasketHandle, SdkError};

/// A user's intent to deposit into a basket.
#[derive(Debug, Clone)]
pub struct DepositRequest {
    pub basket: BasketHandle,
    /// One entry per asset the user wants to deposit. Subset of the
    /// basket's constituents is allowed.
    pub assets: Vec<DepositAssetEntry>,
    /// Block at which the note expires if not consumed.
    pub expiry_block: u64,
    /// User wallet account ID that will receive the minted DCC.
    pub recipient_account_id: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct DepositAssetEntry {
    pub faucet_id: u64,
    pub amount: u64,
}

impl DepositRequest {
    /// Returns the total number of assets in this request.
    pub fn asset_count(&self) -> usize {
        self.assets.len()
    }

    /// Total sum of the deposited amounts (in mixed native units —
    /// only meaningful when every asset has the same decimals; the
    /// USD value calculation lives in the MASM `nav::weighted_sum_N`
    /// procedures).
    pub fn total_amount_raw(&self) -> u64 {
        self.assets.iter().map(|a| a.amount).sum()
    }

    /// Validates that this request can be submitted:
    /// - the basket handle is fully resolved (protocol + faucet ids
    ///   populated by the deployment binary or testnet sync);
    /// - the request contains at least one asset;
    /// - every asset is a constituent of the basket;
    /// - no asset is duplicated.
    pub fn validate(&self) -> Result<(), SdkError> {
        if !self.basket.is_resolved() {
            return Err(SdkError::NotConnected);
        }
        if self.assets.is_empty() {
            return Err(SdkError::InvalidInput(
                "deposit request must contain at least one asset".into(),
            ));
        }

        // Every asset must correspond to a basket constituent. We
        // compare against the manifest's faucet aliases. The mapping
        // from FaucetId to faucet_alias lives in
        // `darwin-baskets/state/{network}.toml` (out of scope until
        // the deployment binary populates it) — for now we treat any
        // non-zero faucet_id as a valid placeholder.
        let mut seen = std::collections::HashSet::new();
        for asset in &self.assets {
            if asset.faucet_id == 0 {
                return Err(SdkError::InvalidInput(format!(
                    "asset has placeholder faucet_id 0; deployment not complete"
                )));
            }
            if !seen.insert(asset.faucet_id) {
                return Err(SdkError::InvalidInput(format!(
                    "duplicate asset faucet_id: 0x{:x}",
                    asset.faucet_id
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved_handle() -> BasketHandle {
        BasketHandle::from_symbol("DCC")
            .unwrap()
            .with_protocol_account(1)
            .with_basket_token_faucet(2)
    }

    #[test]
    fn validate_accepts_complete_request() {
        let req = DepositRequest {
            basket: resolved_handle(),
            assets: vec![
                DepositAssetEntry {
                    faucet_id: 0xAA,
                    amount: 100,
                },
                DepositAssetEntry {
                    faucet_id: 0xBB,
                    amount: 200,
                },
            ],
            expiry_block: 700_000,
            recipient_account_id: 0xCAFE,
        };
        req.validate().expect("validation passes");
        assert_eq!(req.asset_count(), 2);
        assert_eq!(req.total_amount_raw(), 300);
    }

    #[test]
    fn validate_rejects_unresolved_handle() {
        let req = DepositRequest {
            basket: BasketHandle::from_symbol("DCC").unwrap(),
            assets: vec![DepositAssetEntry {
                faucet_id: 0xAA,
                amount: 100,
            }],
            expiry_block: 700_000,
            recipient_account_id: 0xCAFE,
        };
        assert!(matches!(req.validate(), Err(SdkError::NotConnected)));
    }

    #[test]
    fn validate_rejects_empty_assets() {
        let req = DepositRequest {
            basket: resolved_handle(),
            assets: vec![],
            expiry_block: 700_000,
            recipient_account_id: 0xCAFE,
        };
        assert!(matches!(req.validate(), Err(SdkError::InvalidInput(_))));
    }

    #[test]
    fn validate_rejects_duplicate_assets() {
        let req = DepositRequest {
            basket: resolved_handle(),
            assets: vec![
                DepositAssetEntry {
                    faucet_id: 0xAA,
                    amount: 100,
                },
                DepositAssetEntry {
                    faucet_id: 0xAA,
                    amount: 200,
                },
            ],
            expiry_block: 700_000,
            recipient_account_id: 0xCAFE,
        };
        assert!(matches!(req.validate(), Err(SdkError::InvalidInput(_))));
    }

    #[test]
    fn validate_rejects_placeholder_faucet_id() {
        let req = DepositRequest {
            basket: resolved_handle(),
            assets: vec![DepositAssetEntry {
                faucet_id: 0,
                amount: 100,
            }],
            expiry_block: 700_000,
            recipient_account_id: 0xCAFE,
        };
        assert!(matches!(req.validate(), Err(SdkError::InvalidInput(_))));
    }
}
