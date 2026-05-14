/**
 * Deposit (Flow A) — high-level helpers.
 *
 * Mirror of `darwin_sdk::deposit` (the Rust crate). Validation logic
 * runs in the browser before any Miden Web SDK call, so users get an
 * immediate, descriptive error instead of an opaque on-chain revert.
 */

import type { BasketHandle } from "./baskets";

export class SdkError extends Error {
  readonly kind: "unknown-basket" | "not-connected" | "invalid-input";

  constructor(kind: SdkError["kind"], message: string) {
    super(message);
    this.kind = kind;
    this.name = "SdkError";
  }
}

export interface DepositAssetEntry {
  /** Faucet account ID as a 0x-prefixed hex string. */
  readonly faucetId: string;
  /** Amount in faucet base units. */
  readonly amount: bigint;
}

export interface DepositRequest {
  readonly basket: BasketHandle;
  readonly assets: readonly DepositAssetEntry[];
  /** Block at which the note expires if not consumed. */
  readonly expiryBlock: bigint;
  /** Wallet that will receive the minted basket token. */
  readonly recipientAccountId: string;
}

export function depositAssetCount(req: DepositRequest): number {
  return req.assets.length;
}

export function depositTotalAmountRaw(req: DepositRequest): bigint {
  return req.assets.reduce((acc, a) => acc + a.amount, 0n);
}

/**
 * Validates a deposit request the same way the Rust SDK does:
 * - the basket handle must be resolved (both protocol account and
 *   basket-token faucet ids populated);
 * - at least one asset must be present;
 * - no duplicate faucet ids;
 * - no placeholder `"0x0"` faucet ids.
 */
export function validateDepositRequest(req: DepositRequest): void {
  if (!isResolved(req.basket)) {
    throw new SdkError(
      "not-connected",
      `basket ${req.basket.symbol} is not fully resolved on-chain yet`,
    );
  }
  if (req.assets.length === 0) {
    throw new SdkError(
      "invalid-input",
      "deposit request must contain at least one asset",
    );
  }
  const seen = new Set<string>();
  for (const asset of req.assets) {
    if (asset.faucetId === "0x0" || asset.faucetId === "0x00") {
      throw new SdkError(
        "invalid-input",
        "asset has placeholder faucet_id 0; deployment not complete",
      );
    }
    if (seen.has(asset.faucetId)) {
      throw new SdkError(
        "invalid-input",
        `duplicate asset faucet_id: ${asset.faucetId}`,
      );
    }
    if (asset.amount <= 0n) {
      throw new SdkError(
        "invalid-input",
        `asset amount must be > 0 (got ${asset.amount} on ${asset.faucetId})`,
      );
    }
    seen.add(asset.faucetId);
  }
}

function isResolved(b: BasketHandle): boolean {
  return b.protocolAccountId !== null && b.basketTokenFaucetId !== null;
}
