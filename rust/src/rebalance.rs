//! Off-chain rebalance planner.
//!
//! Mirror in Rust of the `darwin::drift` MASM library
//! (`darwin-protocol/asm/lib/drift.masm`). The SDK uses this to surface
//! the per-constituent drift on the frontend and to plan the swap legs
//! a Flow B rebalance transaction will execute on-chain in M2.
//!
//! On-chain math is the source of truth — this module is intentionally
//! kept thin and re-uses the same `current_weight = position*price *
//! 10000 / total_pool_value` formula. Frontend code and the future
//! rebalance bot consume `PortfolioDrift::plan_trades` to get the list
//! of swaps that bring every constituent back inside the manifest's
//! `drift_threshold_bps`.

use crate::SdkError;
use darwin_baskets::{BasketManifest, Constituent};

/// One basket constituent's current state as observed off-chain.
#[derive(Debug, Clone, Copy)]
pub struct ConstituentSnapshot<'a> {
    /// Faucet alias as it appears in the basket manifest
    /// (`darwin-eth`, `darwin-wbtc`, ...). Used to join against
    /// `BasketManifest::constituents`.
    pub faucet_alias: &'a str,

    /// Current position held by the protocol account for this
    /// constituent, in the faucet's native base units.
    pub position_base_units: u64,

    /// Current price in USD, expressed in the same fixed-point scale
    /// the oracle adapter uses (8 decimals — i.e. `2_000_000_000` is
    /// `$20.0`).
    pub price_x1e8: u64,
}

/// Sum of `position * price` across all constituents — the
/// total pool value in the oracle's price units.
fn total_pool_value(snapshot: &[ConstituentSnapshot<'_>]) -> u64 {
    snapshot
        .iter()
        .map(|c| c.position_base_units.saturating_mul(c.price_x1e8))
        .sum()
}

/// Computes the current weight of `c` inside the snapshot, in basis
/// points. Matches the MASM body of `drift::constituent_weight_bps`:
/// `position * price * 10000 / total`.
pub fn constituent_weight_bps(c: &ConstituentSnapshot<'_>, total: u64) -> u32 {
    if total == 0 {
        return 0;
    }
    let numerator = (c.position_base_units as u128)
        .saturating_mul(c.price_x1e8 as u128)
        .saturating_mul(10_000);
    let bps = numerator / (total as u128);
    bps.min(u32::MAX as u128) as u32
}

/// Absolute deviation between an observed weight and its target.
/// Matches `drift::abs_drift_bps`.
pub fn abs_drift_bps(current: u32, target: u32) -> u32 {
    current.abs_diff(target)
}

/// One element of a rebalance plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebalanceTrade {
    pub faucet_alias: String,
    pub kind: TradeKind,
    /// Magnitude of the trade in the same units the SDK uses on
    /// deposit / redeem — faucet base units.
    pub base_units: u64,
    pub drift_bps: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeKind {
    /// Constituent is below its target weight; the rebalance bot
    /// should buy more of it.
    Buy,
    /// Constituent is above its target weight; the bot should sell
    /// some.
    Sell,
}

/// Top-level rebalance plan: per-constituent drift + the list of
/// trades needed to bring every constituent back to its target.
#[derive(Debug, Clone)]
pub struct RebalancePlan {
    pub total_value_x1e8: u64,
    pub drifts: Vec<ConstituentDrift>,
    pub trades: Vec<RebalanceTrade>,
}

#[derive(Debug, Clone)]
pub struct ConstituentDrift {
    pub faucet_alias: String,
    pub target_weight_bps: u32,
    pub current_weight_bps: u32,
    pub drift_bps: u32,
}

/// Builds the rebalance plan for `manifest` given a current
/// `snapshot`. Returns an error if the snapshot misses a constituent
/// the manifest declares, or carries an alias the manifest doesn't.
pub fn plan(
    manifest: &BasketManifest,
    snapshot: &[ConstituentSnapshot<'_>],
) -> Result<RebalancePlan, SdkError> {
    if snapshot.len() != manifest.constituents.len() {
        return Err(SdkError::InvalidInput(format!(
            "snapshot has {} entries but manifest declares {}",
            snapshot.len(),
            manifest.constituents.len()
        )));
    }

    for s in snapshot {
        if !manifest
            .constituents
            .iter()
            .any(|c| c.faucet_alias == s.faucet_alias)
        {
            return Err(SdkError::InvalidInput(format!(
                "snapshot constituent '{}' is not in the manifest",
                s.faucet_alias
            )));
        }
    }

    let total = total_pool_value(snapshot);
    let threshold = manifest.rebalancing.drift_threshold_bps;

    let mut drifts = Vec::with_capacity(manifest.constituents.len());
    let mut trades = Vec::new();

    for c in &manifest.constituents {
        let snap = snapshot
            .iter()
            .find(|s| s.faucet_alias == c.faucet_alias)
            .expect("alias presence verified above");
        let current = constituent_weight_bps(snap, total);
        let target = c.target_weight_bps;
        let drift = abs_drift_bps(current, target);
        drifts.push(ConstituentDrift {
            faucet_alias: c.faucet_alias.clone(),
            target_weight_bps: target,
            current_weight_bps: current,
            drift_bps: drift,
        });
        if drift > threshold {
            let trade = build_trade(c, snap, total, current, target, drift);
            trades.push(trade);
        }
    }

    Ok(RebalancePlan {
        total_value_x1e8: total,
        drifts,
        trades,
    })
}

fn build_trade(
    c: &Constituent,
    snap: &ConstituentSnapshot<'_>,
    total: u64,
    current_bps: u32,
    target_bps: u32,
    drift_bps: u32,
) -> RebalanceTrade {
    let kind = if current_bps > target_bps {
        TradeKind::Sell
    } else {
        TradeKind::Buy
    };

    // Notional drift in price-units. `drift_value = drift_bps * total / 10000`.
    let drift_value = (drift_bps as u128)
        .saturating_mul(total as u128)
        .saturating_div(10_000) as u64;

    // Translate value → base units of this constituent at its
    // current price. Saturate to keep the planner allocation-free.
    let base_units = if snap.price_x1e8 == 0 {
        0
    } else {
        (drift_value as u128)
            .saturating_div(snap.price_x1e8 as u128)
            .min(u64::MAX as u128) as u64
    };

    RebalanceTrade {
        faucet_alias: c.faucet_alias.clone(),
        kind,
        base_units,
        drift_bps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darwin_baskets::core_crypto;

    fn snap<'a>(alias: &'a str, position: u64, price: u64) -> ConstituentSnapshot<'a> {
        ConstituentSnapshot {
            faucet_alias: alias,
            position_base_units: position,
            price_x1e8: price,
        }
    }

    #[test]
    fn drift_at_par_is_zero_and_yields_no_trades() {
        let manifest = core_crypto();
        // Build a snapshot whose value distribution exactly matches
        // the target weights. Use unit prices so weights map 1:1 to
        // positions; pick positions in the manifest's weight ratio.
        let snap_vec: Vec<_> = manifest
            .constituents
            .iter()
            .map(|c| snap(&c.faucet_alias, c.target_weight_bps as u64, 1))
            .collect();
        let plan = plan(&manifest, &snap_vec).expect("plan ok");
        assert!(plan.trades.is_empty(), "no drift => no trades");
        for d in plan.drifts {
            assert_eq!(d.current_weight_bps, d.target_weight_bps);
            assert_eq!(d.drift_bps, 0);
        }
    }

    #[test]
    fn over_weighted_constituent_triggers_a_sell() {
        let manifest = core_crypto();
        let drift_threshold = manifest.rebalancing.drift_threshold_bps as u32;
        // Inflate the first constituent's position to push it well
        // above its target weight.
        let mut snap_vec: Vec<_> = manifest
            .constituents
            .iter()
            .map(|c| snap(&c.faucet_alias, c.target_weight_bps as u64, 1))
            .collect();
        snap_vec[0].position_base_units = snap_vec[0].position_base_units.saturating_mul(3);

        let plan = plan(&manifest, &snap_vec).unwrap();
        let first_alias = &manifest.constituents[0].faucet_alias;
        let trade = plan
            .trades
            .iter()
            .find(|t| t.faucet_alias == *first_alias)
            .expect("first constituent should rebalance");
        assert_eq!(trade.kind, TradeKind::Sell);
        assert!(trade.drift_bps > drift_threshold);
    }

    #[test]
    fn under_weighted_constituent_triggers_a_buy() {
        let manifest = core_crypto();
        let drift_threshold = manifest.rebalancing.drift_threshold_bps as u32;
        let mut snap_vec: Vec<_> = manifest
            .constituents
            .iter()
            .map(|c| snap(&c.faucet_alias, c.target_weight_bps as u64, 1))
            .collect();
        // Halve the first constituent so it drifts well below target.
        snap_vec[0].position_base_units /= 10;

        let plan = plan(&manifest, &snap_vec).unwrap();
        let first_alias = &manifest.constituents[0].faucet_alias;
        let trade = plan
            .trades
            .iter()
            .find(|t| t.faucet_alias == *first_alias)
            .expect("under-weighted constituent should buy");
        assert_eq!(trade.kind, TradeKind::Buy);
        assert!(trade.drift_bps > drift_threshold);
    }

    #[test]
    fn unknown_alias_in_snapshot_errors() {
        let manifest = core_crypto();
        let snap_vec = vec![snap("not-a-real-alias", 100, 1); manifest.constituents.len()];
        assert!(matches!(
            plan(&manifest, &snap_vec),
            Err(SdkError::InvalidInput(_))
        ));
    }

    #[test]
    fn snapshot_size_mismatch_errors() {
        let manifest = core_crypto();
        // Empty snapshot — won't match the manifest's constituents count.
        let err = plan(&manifest, &[]).unwrap_err();
        assert!(matches!(err, SdkError::InvalidInput(_)));
    }

    #[test]
    fn weights_sum_close_to_ten_thousand_at_par() {
        let manifest = core_crypto();
        let snap_vec: Vec<_> = manifest
            .constituents
            .iter()
            .map(|c| snap(&c.faucet_alias, c.target_weight_bps as u64, 1))
            .collect();
        let plan = plan(&manifest, &snap_vec).unwrap();
        let sum: u32 = plan.drifts.iter().map(|d| d.current_weight_bps).sum();
        // Integer-division round-off allowed.
        assert!((9_995..=10_000).contains(&sum), "got {sum}");
    }
}
