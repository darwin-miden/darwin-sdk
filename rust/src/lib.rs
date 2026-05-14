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
        let manifest = match symbol {
            "DCC" => darwin_baskets::core_crypto(),
            "DAG" => darwin_baskets::aggressive(),
            "DCO" => darwin_baskets::conservative(),
            _ => return Err(SdkError::UnknownBasket(symbol.to_string())),
        };
        Ok(Self {
            manifest,
            protocol_account_id: None,
            basket_token_faucet_id: None,
        })
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
}
