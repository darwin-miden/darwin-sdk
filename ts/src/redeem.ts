/**
 * Redeem (Miden-side of Flow C) — high-level helpers.
 *
 * Mirror of `darwin_sdk::redeem`. RedeemRequest carries the burn
 * amount and the recipient Miden account that will receive the
 * underlyings.
 */

import type { BasketHandle } from "./baskets";
import { SdkError } from "./deposit";

export interface RedeemRequest {
  readonly basket: BasketHandle;
  /** Basket-token base units to burn. */
  readonly burnAmount: bigint;
  /** Miden account that will receive the underlyings. */
  readonly recipientAccountId: string;
  readonly expiryBlock: bigint;
}

/**
 * Same checks as the Rust SDK: handle resolved, burn amount > 0,
 * recipient looks like a 0x-prefixed account ID.
 */
export function validateRedeemRequest(req: RedeemRequest): void {
  if (req.basket.protocolAccountId === null || req.basket.basketTokenFaucetId === null) {
    throw new SdkError(
      "not-connected",
      `basket ${req.basket.symbol} is not fully resolved on-chain yet`,
    );
  }
  if (req.burnAmount <= 0n) {
    throw new SdkError(
      "invalid-input",
      `burn amount must be > 0 (got ${req.burnAmount})`,
    );
  }
  if (!req.recipientAccountId.startsWith("0x")) {
    throw new SdkError(
      "invalid-input",
      `recipient account id must be 0x-prefixed (got ${req.recipientAccountId})`,
    );
  }
}
