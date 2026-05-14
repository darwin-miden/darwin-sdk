//! Darwin Protocol client SDK — Rust core.
//!
//! Wraps `miden-client` v0.14 + `miden-agglayer` v0.14-alpha and exposes
//! a small high-level API targeted at the M3 frontend and at any
//! integration that wants to interact with Darwin baskets.
//!
//! The TypeScript bindings live alongside in `../ts/`; they are
//! generated from this crate by a future `wasm-bindgen` pipeline.

pub mod baskets {
    pub use darwin_baskets::{
        aggressive, all_m1, conservative, core_crypto, BasketFees, BasketManifest,
        BasketRebalancing, Constituent, ValidationError,
    };
}

pub mod bridge {
    pub use darwin_bridge_adapter::{
        B2AggBuildError, B2AggBuilder, ClaimRecognition, EthAddress, EthNetwork,
        IncomingBridgedAsset,
    };
}

/// Re-exports of the `miden-client` and `miden-objects` types the SDK
/// surfaces. A single migration point for downstream consumers.
pub mod miden {
    pub use miden_client::Client as MidenClient;
    pub use miden_objects::account::AccountId;
    pub use miden_objects::asset::FungibleAsset;
}

pub mod deposit;
pub mod rebalance;
pub mod redeem;

#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    #[error("basket not found: {0}")]
    UnknownBasket(String),
    #[error("wallet not connected")]
    NotConnected,
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

/// High-level handle to a Darwin basket from the SDK side.
///
/// Resolved by calling `BasketHandle::from_symbol("DCC")` etc. Holds
/// the manifest plus a placeholder for the on-chain protocol account id
/// (currently None until deployment scripts populate it).
#[derive(Debug, Clone)]
pub struct BasketHandle {
    pub manifest: darwin_baskets::BasketManifest,
    pub protocol_account_id: Option<u64>,
    pub basket_token_faucet_id: Option<u64>,
}

impl BasketHandle {
    pub fn from_symbol(symbol: &str) -> Result<Self, SdkError> {
        let manifest = darwin_baskets::by_symbol(symbol)
            .ok_or_else(|| SdkError::UnknownBasket(symbol.to_string()))?;
        Ok(Self {
            manifest,
            protocol_account_id: None,
            basket_token_faucet_id: None,
        })
    }

    /// Attaches the deployed protocol account id to this handle.
    /// Returned by reference so the caller can chain.
    pub fn with_protocol_account(mut self, account_id: u64) -> Self {
        self.protocol_account_id = Some(account_id);
        self
    }

    /// Attaches the deployed basket-token faucet id to this handle.
    pub fn with_basket_token_faucet(mut self, faucet_id: u64) -> Self {
        self.basket_token_faucet_id = Some(faucet_id);
        self
    }

    /// True once both the protocol account and the basket faucet ids
    /// have been populated from on-chain state. The deposit / redeem
    /// helpers assert this before constructing notes.
    pub fn is_resolved(&self) -> bool {
        self.protocol_account_id.is_some() && self.basket_token_faucet_id.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_baskets_resolve() {
        for symbol in ["DCC", "DAG", "DCO"] {
            let handle = BasketHandle::from_symbol(symbol).expect("known");
            assert_eq!(handle.manifest.symbol, symbol);
        }
    }

    #[test]
    fn unknown_basket_errors() {
        let err = BasketHandle::from_symbol("XYZ").unwrap_err();
        assert!(matches!(err, SdkError::UnknownBasket(_)));
    }

    #[test]
    fn handle_resolution_requires_both_ids() {
        let handle = BasketHandle::from_symbol("DCC").unwrap();
        assert!(!handle.is_resolved());

        let only_account = handle.clone().with_protocol_account(42);
        assert!(!only_account.is_resolved());

        let resolved = handle
            .with_protocol_account(42)
            .with_basket_token_faucet(99);
        assert!(resolved.is_resolved());
        assert_eq!(resolved.protocol_account_id, Some(42));
        assert_eq!(resolved.basket_token_faucet_id, Some(99));
    }
}
