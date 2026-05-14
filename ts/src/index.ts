/**
 * Darwin Protocol — TypeScript SDK.
 *
 * Wraps the Miden Web SDK to expose a small, typed API for depositing
 * into a Darwin basket, reading NAV, redeeming, and (M2) bridging
 * basket tokens to Ethereum via AggLayer.
 *
 * Mirrors `darwin_sdk` (the Rust crate); the algorithms and validation
 * rules stay identical so on-chain and off-chain code never disagree.
 * Once a wasm-bindgen pipeline ships, this module re-exports the
 * Rust crate via Wasm and the local TS implementations become a
 * pure-JS fallback for environments without WebAssembly.
 */

export {
  type BasketHandle,
  type BasketSymbol,
  basketFromSymbol,
  isKnownBasket,
  isResolved,
  withBasketTokenFaucet,
  withProtocolAccount,
} from "./baskets";

export {
  type DepositAssetEntry,
  type DepositRequest,
  SdkError,
  depositAssetCount,
  depositTotalAmountRaw,
  validateDepositRequest,
} from "./deposit";

export {
  type RedeemRequest,
  validateRedeemRequest,
} from "./redeem";

export {
  type BasketConstituent,
  type BasketDescriptor,
  type ConstituentDrift,
  type ConstituentSnapshot,
  type RebalancePlan,
  type RebalanceTrade,
  type TradeKind,
  absDriftBps,
  constituentWeightBps,
  planRebalance,
  totalPoolValue,
} from "./rebalance";
