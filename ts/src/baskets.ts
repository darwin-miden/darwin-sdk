/**
 * BasketHandle resolution and the M1 catalogue.
 *
 * Mirror of the relevant parts of `darwin_sdk::BasketHandle`. The
 * `protocolAccountId` and `basketTokenFaucetId` fields are `null`
 * until the SDK syncs with the deployed state (typically loaded from
 * `darwin-baskets/state/{network}.toml` by a thin loader the
 * frontend wires in).
 */

export type BasketSymbol = "DCC" | "DAG" | "DCO";

export interface BasketHandle {
  readonly symbol: BasketSymbol;
  readonly protocolAccountId: string | null;
  readonly basketTokenFaucetId: string | null;
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

export function withProtocolAccount(
  basket: BasketHandle,
  accountId: string,
): BasketHandle {
  return { ...basket, protocolAccountId: accountId };
}

export function withBasketTokenFaucet(
  basket: BasketHandle,
  faucetId: string,
): BasketHandle {
  return { ...basket, basketTokenFaucetId: faucetId };
}

export function isResolved(basket: BasketHandle): boolean {
  return basket.protocolAccountId !== null && basket.basketTokenFaucetId !== null;
}
