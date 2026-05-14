//! Demo CLI: print the rebalance plan for one or all M1 baskets,
//! given a synthetic snapshot.
//!
//! Usage:
//!     cargo run --bin rebalance_demo                  # all baskets, at-par
//!     cargo run --bin rebalance_demo -- DCC           # one basket, at-par
//!     cargo run --bin rebalance_demo -- DCC --skew 2.0
//!                                                     # double the first
//!                                                     # constituent's position
//!
//! At-par snapshots produce zero drift (the rebalance planner returns
//! an empty trade list). The `--skew` flag scales the first
//! constituent's position so the user can see the planner trigger a
//! `Sell` trade.

use darwin_baskets::{all_m1, by_symbol, BasketManifest};
use darwin_sdk::rebalance::{plan, ConstituentSnapshot};

fn main() {
    let mut symbol: Option<String> = None;
    let mut skew: f64 = 1.0;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--skew" => {
                skew = args
                    .next()
                    .expect("--skew needs a value")
                    .parse()
                    .expect("--skew must be a float");
            }
            other if !other.starts_with("--") => symbol = Some(other.to_string()),
            other => panic!("unknown flag: {other}"),
        }
    }

    let baskets: Vec<BasketManifest> = match symbol {
        Some(s) => vec![by_symbol(&s).unwrap_or_else(|| panic!("unknown basket: {s}"))],
        None => all_m1().to_vec(),
    };

    for basket in baskets {
        print_plan_for(&basket, skew);
        println!();
    }
}

fn print_plan_for(basket: &BasketManifest, skew: f64) {
    println!("==== {} ({}) ====", basket.name, basket.symbol);
    println!(
        "  drift threshold: {} bps   ({} constituents)",
        basket.rebalancing.drift_threshold_bps,
        basket.constituents.len()
    );

    let snapshot: Vec<ConstituentSnapshot<'_>> = basket
        .constituents
        .iter()
        .enumerate()
        .map(|(idx, c)| {
            // Synthetic at-par snapshot: position scaled to target
            // weight, unit price. The first constituent gets multiplied
            // by `skew` to make the planner trigger if skew != 1.0.
            let mut position = c.target_weight_bps as u64;
            if idx == 0 {
                position = (position as f64 * skew) as u64;
            }
            ConstituentSnapshot {
                faucet_alias: &c.faucet_alias,
                position_base_units: position,
                price_x1e8: 1,
            }
        })
        .collect();

    let plan = plan(basket, &snapshot).expect("planner runs cleanly on bundled manifests");
    println!("  total pool value (x1e8): {}", plan.total_value_x1e8);
    println!("  per-constituent drift:");
    for d in &plan.drifts {
        println!(
            "    {:<14} target={:>4} bps  current={:>4} bps  drift={:>4} bps",
            d.faucet_alias, d.target_weight_bps, d.current_weight_bps, d.drift_bps
        );
    }
    if plan.trades.is_empty() {
        println!("  no rebalance trades — every constituent within threshold.");
    } else {
        println!("  rebalance trades:");
        for t in &plan.trades {
            println!(
                "    {:?} {:<14} {} base units  (drift {} bps)",
                t.kind, t.faucet_alias, t.base_units, t.drift_bps
            );
        }
    }
}
