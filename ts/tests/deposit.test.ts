import { describe, it, expect } from "vitest";
import {
  basketFromSymbol,
  type BasketHandle,
  type DepositRequest,
  depositAssetCount,
  depositTotalAmountRaw,
  SdkError,
  validateDepositRequest,
  withBasketTokenFaucet,
  withProtocolAccount,
} from "../src/index.js";

function resolvedHandle(): BasketHandle {
  return withBasketTokenFaucet(
    withProtocolAccount(basketFromSymbol("DCC"), "0xaa20da7d98c2e29022510aa786948f"),
    "0x2066f2da1f91ba202af5251d39101c",
  );
}

describe("validateDepositRequest", () => {
  it("accepts a complete request", () => {
    const req: DepositRequest = {
      basket: resolvedHandle(),
      assets: [
        { faucetId: "0xa095d9b3831e96206ff70c2218a6a9", amount: 100n },
        { faucetId: "0x7a45cb24ada22120246bcf54196e12", amount: 200n },
      ],
      expiryBlock: 700_000n,
      recipientAccountId: "0xed3cd5befa3207805f8529207cfc0d",
    };
    expect(() => validateDepositRequest(req)).not.toThrow();
    expect(depositAssetCount(req)).toBe(2);
    expect(depositTotalAmountRaw(req)).toBe(300n);
  });

  it("rejects an unresolved handle", () => {
    const req: DepositRequest = {
      basket: basketFromSymbol("DCC"),
      assets: [{ faucetId: "0xaa", amount: 100n }],
      expiryBlock: 700_000n,
      recipientAccountId: "0xcafe",
    };
    expect(() => validateDepositRequest(req)).toThrowError(SdkError);
    try {
      validateDepositRequest(req);
    } catch (e) {
      expect((e as SdkError).kind).toBe("not-connected");
    }
  });

  it("rejects an empty asset list", () => {
    const req: DepositRequest = {
      basket: resolvedHandle(),
      assets: [],
      expiryBlock: 700_000n,
      recipientAccountId: "0xcafe",
    };
    expect(() => validateDepositRequest(req)).toThrowError(/at least one asset/);
  });

  it("rejects duplicate faucet ids", () => {
    const req: DepositRequest = {
      basket: resolvedHandle(),
      assets: [
        { faucetId: "0xaa", amount: 100n },
        { faucetId: "0xaa", amount: 200n },
      ],
      expiryBlock: 700_000n,
      recipientAccountId: "0xcafe",
    };
    expect(() => validateDepositRequest(req)).toThrowError(/duplicate/);
  });

  it("rejects placeholder faucet id 0x0", () => {
    const req: DepositRequest = {
      basket: resolvedHandle(),
      assets: [{ faucetId: "0x0", amount: 100n }],
      expiryBlock: 700_000n,
      recipientAccountId: "0xcafe",
    };
    expect(() => validateDepositRequest(req)).toThrowError(/placeholder/);
  });

  it("rejects non-positive amounts", () => {
    const req: DepositRequest = {
      basket: resolvedHandle(),
      assets: [{ faucetId: "0xaa", amount: 0n }],
      expiryBlock: 700_000n,
      recipientAccountId: "0xcafe",
    };
    expect(() => validateDepositRequest(req)).toThrowError(/amount must be/);
  });
});
