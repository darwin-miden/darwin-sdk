/**
 * Off-chain rebalance / drift planner — TypeScript mirror of the Rust
 * `darwin_sdk::rebalance` module (and, transitively, the `darwin::drift`
 * MASM library in `darwin-protocol/asm/lib/drift.masm`).
 *
 * Algorithmically identical to the Rust planner:
 *   current_weight_bps = position * price * 10000 / total_pool_value
 *   drift_bps          = |current - target|
 *   trade fires when drift_bps > basket.driftThresholdBps
 *
 * The frontend (`darwin-frontend/src/lib/rebalance.ts`) and this SDK
 * module share the same algorithm; once wasm-bindgen ships, the
 * frontend will consume this SDK directly.
 */

export interface BasketConstituent {
  readonly faucetAlias: string;
  readonly targetWeightBps: number;
  readonly pragmaPair?: string;
}

export interface BasketDescriptor {
  readonly symbol: string;
  readonly constituents: readonly BasketConstituent[];
  readonly driftThresholdBps: number;
}

export interface ConstituentSnapshot {
  /** Faucet alias from the basket manifest. */
  readonly faucetAlias: string;
  /** Current position in the faucet's native base units. */
  readonly positionBaseUnits: bigint;
  /** Current price in USD with 8-decimal fixed point. */
  readonly priceX1e8: bigint;
}

export type TradeKind = "buy" | "sell";

export interface RebalanceTrade {
  readonly faucetAlias: string;
  readonly kind: TradeKind;
  readonly baseUnits: bigint;
  readonly driftBps: number;
}

export interface ConstituentDrift {
  readonly faucetAlias: string;
  readonly targetWeightBps: number;
  readonly currentWeightBps: number;
  readonly driftBps: number;
}

export interface RebalancePlan {
  readonly totalValueX1e8: bigint;
  readonly drifts: readonly ConstituentDrift[];
  readonly trades: readonly RebalanceTrade[];
}

export function totalPoolValue(snapshot: readonly ConstituentSnapshot[]): bigint {
  return snapshot.reduce(
    (acc, c) => acc + c.positionBaseUnits * c.priceX1e8,
    0n,
  );
}

export function constituentWeightBps(
  c: ConstituentSnapshot,
  total: bigint,
): number {
  if (total === 0n) return 0;
  const num = c.positionBaseUnits * c.priceX1e8 * 10_000n;
  return Number(num / total);
}

export function absDriftBps(current: number, target: number): number {
  return Math.abs(current - target);
}

export function planRebalance(
  basket: BasketDescriptor,
  snapshot: readonly ConstituentSnapshot[],
): RebalancePlan {
  if (snapshot.length !== basket.constituents.length) {
    throw new Error(
      `snapshot has ${snapshot.length} entries but basket ${basket.symbol} declares ${basket.constituents.length}`,
    );
  }
  for (const s of snapshot) {
    if (!basket.constituents.find((c) => c.faucetAlias === s.faucetAlias)) {
      throw new Error(
        `snapshot constituent '${s.faucetAlias}' is not in basket ${basket.symbol}`,
      );
    }
  }

  const total = totalPoolValue(snapshot);
  const drifts: ConstituentDrift[] = [];
  const trades: RebalanceTrade[] = [];

  for (const c of basket.constituents) {
    const snap = snapshot.find((s) => s.faucetAlias === c.faucetAlias);
    if (snap === undefined) {
      // Unreachable — verified by the loop above.
      throw new Error(`unreachable: missing snapshot for ${c.faucetAlias}`);
    }
    const current = constituentWeightBps(snap, total);
    const target = c.targetWeightBps;
    const drift = absDriftBps(current, target);
    drifts.push({
      faucetAlias: c.faucetAlias,
      targetWeightBps: target,
      currentWeightBps: current,
      driftBps: drift,
    });
    if (drift > basket.driftThresholdBps) {
      const kind: TradeKind = current > target ? "sell" : "buy";
      const driftValue = (BigInt(drift) * total) / 10_000n;
      const baseUnits =
        snap.priceX1e8 === 0n ? 0n : driftValue / snap.priceX1e8;
      trades.push({
        faucetAlias: c.faucetAlias,
        kind,
        baseUnits,
        driftBps: drift,
      });
    }
  }

  return { totalValueX1e8: total, drifts, trades };
}
