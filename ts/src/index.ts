/**
 * Darwin Protocol — TypeScript SDK.
 *
 * Wraps the Miden Web SDK to expose a small, typed API for depositing
 * into a Darwin basket, reading NAV, redeeming, and (M2) bridging
 * basket tokens to Ethereum via AggLayer.
 *
 * This file is a skeleton. The actual integration with
 * `@miden-sdk/miden-sdk` is wired in step with `darwin-protocol`'s
 * deployment to testnet.
 */

export type BasketSymbol = "DCC" | "DAG" | "DCO";

export interface BasketHandle {
  readonly symbol: BasketSymbol;
  readonly protocolAccountId: string | null;
  readonly basketTokenFaucetId: string | null;
}

export interface DepositAsset {
  readonly faucetId: string;
  readonly amount: bigint;
}

export interface DepositRequest {
  readonly basket: BasketHandle;
  readonly assets: readonly DepositAsset[];
  readonly expiryBlock: bigint;
}

export interface RedeemRequest {
  readonly basket: BasketHandle;
  readonly burnAmount: bigint;
  readonly recipientAccountId: string;
  readonly expiryBlock: bigint;
}

const KNOWN_BASKETS: readonly BasketSymbol[] = ["DCC", "DAG", "DCO"] as const;

export function isKnownBasket(symbol: string): symbol is BasketSymbol {
  return (KNOWN_BASKETS as readonly string[]).includes(symbol);
}

export function basketFromSymbol(symbol: BasketSymbol): BasketHandle {
  return {
    symbol,
    protocolAccountId: null,
    basketTokenFaucetId: null,
  };
}
