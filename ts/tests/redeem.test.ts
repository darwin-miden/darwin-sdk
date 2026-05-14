import { describe, it, expect } from "vitest";
import {
  basketFromSymbol,
  type BasketHandle,
  type RedeemRequest,
  SdkError,
  validateRedeemRequest,
  withBasketTokenFaucet,
  withProtocolAccount,
} from "../src/index.js";

function resolvedHandle(): BasketHandle {
  return withBasketTokenFaucet(
    withProtocolAccount(basketFromSymbol("DCC"), "0xaa"),
    "0xbb",
  );
}

describe("validateRedeemRequest", () => {
  it("accepts a complete request", () => {
    const req: RedeemRequest = {
      basket: resolvedHandle(),
      burnAmount: 1000n,
      recipientAccountId: "0xed3cd5befa3207805f8529207cfc0d",
      expiryBlock: 700_000n,
    };
    expect(() => validateRedeemRequest(req)).not.toThrow();
  });

  it("rejects unresolved handles", () => {
    const req: RedeemRequest = {
      basket: basketFromSymbol("DAG"),
      burnAmount: 1000n,
      recipientAccountId: "0xcafe",
      expiryBlock: 700_000n,
    };
    expect(() => validateRedeemRequest(req)).toThrowError(SdkError);
  });

  it("rejects burn amount of zero", () => {
    const req: RedeemRequest = {
      basket: resolvedHandle(),
      burnAmount: 0n,
      recipientAccountId: "0xcafe",
      expiryBlock: 700_000n,
    };
    expect(() => validateRedeemRequest(req)).toThrowError(/burn amount/);
  });

  it("rejects unprefixed recipient ids", () => {
    const req: RedeemRequest = {
      basket: resolvedHandle(),
      burnAmount: 100n,
      recipientAccountId: "ed3cd5be",
      expiryBlock: 700_000n,
    };
    expect(() => validateRedeemRequest(req)).toThrowError(/0x-prefixed/);
  });
});
