import { describe, it, expect } from "vitest";
import {
  type BasketDescriptor,
  type ConstituentSnapshot,
  absDriftBps,
  constituentWeightBps,
  planRebalance,
  totalPoolValue,
} from "../src/index.js";

const DCC: BasketDescriptor = {
  symbol: "DCC",
  driftThresholdBps: 500,
  constituents: [
    { faucetAlias: "darwin-wbtc", targetWeightBps: 4000 },
    { faucetAlias: "darwin-eth", targetWeightBps: 4000 },
    { faucetAlias: "darwin-usdt", targetWeightBps: 2000 },
  ],
};

function snap(alias: string, position: bigint, price: bigint): ConstituentSnapshot {
  return { faucetAlias: alias, positionBaseUnits: position, priceX1e8: price };
}

describe("rebalance primitives", () => {
  it("absDriftBps is symmetric", () => {
    expect(absDriftBps(100, 60)).toBe(40);
    expect(absDriftBps(60, 100)).toBe(40);
    expect(absDriftBps(100, 100)).toBe(0);
  });

  it("constituentWeightBps returns 0 on empty pool", () => {
    expect(constituentWeightBps(snap("x", 0n, 0n), 0n)).toBe(0);
  });

  it("totalPoolValue sums position * price", () => {
    const s = [
      snap("a", 100n, 2n),
      snap("b", 50n, 3n),
    ];
    expect(totalPoolValue(s)).toBe(350n);
  });
});

describe("planRebalance", () => {
  it("returns no trades when every constituent is at par", () => {
    const snapshot = DCC.constituents.map((c) =>
      snap(c.faucetAlias, BigInt(c.targetWeightBps), 1n),
    );
    const p = planRebalance(DCC, snapshot);
    expect(p.trades).toEqual([]);
    for (const d of p.drifts) {
      expect(d.currentWeightBps).toBe(d.targetWeightBps);
      expect(d.driftBps).toBe(0);
    }
  });

  it("triggers a sell on overweight + matching buys on underweight", () => {
    const snapshot = DCC.constituents.map((c) =>
      snap(c.faucetAlias, BigInt(c.targetWeightBps), 1n),
    );
    // Double the first constituent's position → overweighted.
    snapshot[0] = snap(snapshot[0]!.faucetAlias, snapshot[0]!.positionBaseUnits * 2n, 1n);

    const p = planRebalance(DCC, snapshot);
    const overweight = p.trades.find((t) => t.faucetAlias === "darwin-wbtc");
    expect(overweight).toBeDefined();
    expect(overweight!.kind).toBe("sell");
    expect(overweight!.driftBps).toBeGreaterThan(DCC.driftThresholdBps);

    const underweight_eth = p.trades.find((t) => t.faucetAlias === "darwin-eth");
    expect(underweight_eth).toBeDefined();
    expect(underweight_eth!.kind).toBe("buy");
  });

  it("rejects a snapshot with the wrong number of entries", () => {
    expect(() => planRebalance(DCC, [])).toThrow(/snapshot has 0 entries/);
  });

  it("rejects a snapshot containing an unknown alias", () => {
    const snapshot = [
      snap("not-real", 4000n, 1n),
      snap("darwin-eth", 4000n, 1n),
      snap("darwin-usdt", 2000n, 1n),
    ];
    expect(() => planRebalance(DCC, snapshot)).toThrow(/not in basket/);
  });

  it("weights sum to ~10000 bps at par", () => {
    const snapshot = DCC.constituents.map((c) =>
      snap(c.faucetAlias, BigInt(c.targetWeightBps), 1n),
    );
    const p = planRebalance(DCC, snapshot);
    const sum = p.drifts.reduce((acc, d) => acc + d.currentWeightBps, 0);
    expect(sum).toBeGreaterThanOrEqual(9_995);
    expect(sum).toBeLessThanOrEqual(10_000);
  });
});
