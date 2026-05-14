//! End-to-end simulation: walk a basket through a sequence of price
//! moves and assert the planner fires rebalance trades exactly when
//! expected.
//!
//! The simulation is deterministic — no PRNG — so the assertions
//! describe exact behaviour, not statistical bounds. This is the
//! seed of the M2 rebalance bot's regression suite.

use darwin_baskets::{aggressive, core_crypto, BasketManifest};
use darwin_sdk::rebalance::{plan, ConstituentSnapshot, RebalancePlan, TradeKind};

/// Build an at-par snapshot for `basket` using `prices_x1e8` per
/// constituent (in manifest order). At par, position equals
/// target_weight_bps so the value distribution exactly matches the
/// target weights.
fn at_par_snapshot<'a>(
    basket: &'a BasketManifest,
    prices_x1e8: &[u64],
) -> Vec<ConstituentSnapshot<'a>> {
    assert_eq!(prices_x1e8.len(), basket.constituents.len());
    basket
        .constituents
        .iter()
        .zip(prices_x1e8.iter())
        .map(|(c, p)| ConstituentSnapshot {
            faucet_alias: c.faucet_alias.as_str(),
            position_base_units: c.target_weight_bps as u64,
            price_x1e8: *p,
        })
        .collect()
}

#[test]
fn flat_prices_never_trigger_a_rebalance() {
    let basket = core_crypto();
    // Three steady price ticks at the same price; no positions
    // ever change. Plan must be empty every tick.
    for _ in 0..3 {
        let snap = at_par_snapshot(&basket, &[1, 1, 1]);
        let p: RebalancePlan = plan(&basket, &snap).expect("plan ok");
        assert!(p.trades.is_empty(), "flat prices must not rebalance");
    }
}

#[test]
fn one_asset_pumping_triggers_sell_then_buy_recovery() {
    let basket = aggressive();
    let drift_threshold = basket.rebalancing.drift_threshold_bps as u32;

    // Tick 0: at par.
    let snap0 = at_par_snapshot(&basket, &[1, 1]);
    let p0 = plan(&basket, &snap0).unwrap();
    assert!(p0.trades.is_empty(), "tick 0 is at par");

    // Tick 1: WBTC price doubles. Now its value share is 2*5000 /
    // (2*5000 + 1*5000) ≈ 6666 bps, vs target 5000 → drift 1666.
    let snap1 = at_par_snapshot(&basket, &[2, 1]);
    let p1 = plan(&basket, &snap1).unwrap();
    let wbtc_trade = p1
        .trades
        .iter()
        .find(|t| t.faucet_alias == "darwin-wbtc")
        .expect("WBTC should require a sell");
    assert_eq!(wbtc_trade.kind, TradeKind::Sell);
    assert!(wbtc_trade.drift_bps > drift_threshold);

    let eth_trade = p1
        .trades
        .iter()
        .find(|t| t.faucet_alias == "darwin-eth")
        .expect("ETH should require a buy");
    assert_eq!(eth_trade.kind, TradeKind::Buy);

    // Tick 2: prices revert back to par. With unchanged positions
    // the basket returns to its target weights and the planner
    // emits no trades.
    let snap2 = at_par_snapshot(&basket, &[1, 1]);
    let p2 = plan(&basket, &snap2).unwrap();
    assert!(p2.trades.is_empty(), "post-revert returns to par");
}

#[test]
fn price_move_inside_threshold_does_not_fire() {
    let basket = aggressive();
    let threshold = basket.rebalancing.drift_threshold_bps as u32;

    // Pick a small price tilt that yields a drift below the threshold.
    // WBTC at price 11/10 vs ETH at 1 → values 5500 vs 5000, total 10500,
    // WBTC share = 5500*10000/10500 ≈ 5238 → drift ≈ 238 bps  <  500.
    let snap = at_par_snapshot(&basket, &[11, 10]);
    let p = plan(&basket, &snap).unwrap();

    let wbtc_drift = p
        .drifts
        .iter()
        .find(|d| d.faucet_alias == "darwin-wbtc")
        .unwrap()
        .drift_bps;
    assert!(
        wbtc_drift < threshold,
        "sanity: scenario must be inside threshold (got {wbtc_drift})",
    );
    assert!(p.trades.is_empty(), "no trades fire below threshold");
}

#[test]
fn rebalance_buy_value_matches_rebalance_sell_value() {
    // Across the planner's output, the total notional sold must
    // approximately equal the total notional bought — the rebalance
    // is value-conserving (in the absence of fees / slippage, which
    // M2 handles separately).
    let basket = aggressive();
    let snap = at_par_snapshot(&basket, &[3, 1]); // WBTC overweight 3x

    let p = plan(&basket, &snap).unwrap();

    let mut sell_value: u128 = 0;
    let mut buy_value: u128 = 0;
    for t in &p.trades {
        let price = snap
            .iter()
            .find(|s| s.faucet_alias == t.faucet_alias)
            .unwrap()
            .price_x1e8 as u128;
        let value = price * t.base_units as u128;
        match t.kind {
            TradeKind::Sell => sell_value += value,
            TradeKind::Buy => buy_value += value,
        }
    }

    // Allow small integer-division rounding (10 bps of the larger
    // side is plenty for the planner's accuracy target — drift is
    // expressed in bps so the rounding floor is naturally ~1 base
    // unit per trade).
    let larger = sell_value.max(buy_value);
    let smaller = sell_value.min(buy_value);
    let diff = larger - smaller;
    assert!(
        diff * 1_000 <= larger,
        "buy ({buy_value}) and sell ({sell_value}) value should match within 10 bps",
    );
}
