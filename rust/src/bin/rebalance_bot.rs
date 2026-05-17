//! Darwin rebalance bot (M2 prototype).
//!
//! A long-running tokio loop that:
//!
//!   1. Polls the on-chain Pragma-style oracle for the latest prices
//!      of every constituent across all M1 baskets.
//!   2. Reads each basket's current pool positions (in this skeleton:
//!      from a manifest-derived synthetic snapshot — production
//!      reads them from the controller's StorageMap slot 2 once
//!      Track C ships).
//!   3. Runs `darwin_sdk::rebalance::plan` to compute drift +
//!      per-constituent trades for each basket.
//!   4. When any basket's drift exceeds its manifest threshold, emits
//!      a "rebalance trigger" decision (printed in this skeleton;
//!      production wires this to a Flow B note submission via
//!      miden-client + the in-protocol Miden DEX).
//!
//! Headline M2 deliverable per the grant proposal §2 (rebalancing
//! engine, drift detection → trigger). Independent of AggLayer.
//!
//! Usage:
//!     cargo run -p darwin-sdk --bin rebalance_bot                # default 30s tick
//!     cargo run -p darwin-sdk --bin rebalance_bot -- --interval 10  # custom tick
//!     cargo run -p darwin-sdk --bin rebalance_bot -- --once       # run a single pass and exit
//!     cargo run -p darwin-sdk --bin rebalance_bot -- --skew 1.5   # perturb prices for demo

use std::collections::BTreeMap;
use std::time::Duration;

use darwin_baskets::{all_m1, BasketManifest};
use darwin_sdk::rebalance::{plan, ConstituentSnapshot, RebalancePlan};

#[derive(Debug)]
struct Args {
    interval_s: u64,
    once: bool,
    skew: f64,
}

fn parse_args() -> Args {
    let mut out = Args {
        interval_s: 30,
        once: false,
        skew: 1.0,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--interval" => {
                out.interval_s = it
                    .next()
                    .expect("--interval needs a value")
                    .parse()
                    .expect("--interval must be u64 seconds");
            }
            "--once" => out.once = true,
            "--skew" => {
                out.skew = it
                    .next()
                    .expect("--skew needs a float value")
                    .parse()
                    .expect("--skew must be a float");
            }
            "--help" | "-h" => {
                eprintln!(
                    "rebalance_bot --interval <s> --skew <f> --once\n  \
                     long-running drift-detection daemon for all M1 baskets."
                );
                std::process::exit(0);
            }
            other => panic!("unknown flag: {other}"),
        }
    }
    out
}

#[tokio::main]
async fn main() {
    let args = parse_args();
    println!(
        "rebalance_bot starting — interval={}s, skew={}, once={}",
        args.interval_s, args.skew, args.once
    );
    println!();

    let baskets = all_m1();
    loop {
        let prices = mock_oracle_prices();
        let now = chrono::Utc::now().format("%H:%M:%S").to_string();
        println!("[{now}] tick — oracle prices: {prices:?}");

        for basket in &baskets {
            let snapshot = synthesize_snapshot(basket, &prices, args.skew);
            let plan = plan(basket, &snapshot).expect("plan ok");
            print_tick(basket, &plan);
        }

        println!();

        if args.once {
            return;
        }
        tokio::time::sleep(Duration::from_secs(args.interval_s)).await;
    }
}

/// Mock on-chain oracle read. Production replaces this with a real
/// `miden client exec` call against the deployed Pragma-style mock
/// oracle (`0x085ba19a…6fd`) or, once Pragma's mainnet build pipeline
/// stabilises, the canonical Pragma testnet oracle.
fn mock_oracle_prices() -> BTreeMap<&'static str, u64> {
    // 8-decimal scale, plausible ETH/WBTC/USDT/DAI testnet anchor.
    let mut m = BTreeMap::new();
    m.insert("darwin-eth", 200_000_000_000u64);    // $2000 * 1e8
    m.insert("darwin-wbtc", 6_000_000_000_000u64); // $60_000 * 1e8
    m.insert("darwin-usdt", 100_000_000u64);       // $1 * 1e8
    m.insert("darwin-dai", 100_000_000u64);        // $1 * 1e8
    m
}

/// Synthetic snapshot — manifests' target weights at unit positions
/// then `skew` applied to the first constituent. Production reads
/// pool positions from controller storage.
fn synthesize_snapshot<'a>(
    basket: &'a BasketManifest,
    prices: &BTreeMap<&'static str, u64>,
    skew: f64,
) -> Vec<ConstituentSnapshot<'a>> {
    basket
        .constituents
        .iter()
        .enumerate()
        .map(|(idx, c)| {
            let mut position = c.target_weight_bps as u64;
            if idx == 0 {
                position = (position as f64 * skew) as u64;
            }
            let price = *prices.get(c.faucet_alias.as_str()).unwrap_or(&1);
            ConstituentSnapshot {
                faucet_alias: c.faucet_alias.as_str(),
                position_base_units: position,
                price_x1e8: price,
            }
        })
        .collect()
}

fn print_tick(basket: &BasketManifest, plan: &RebalancePlan) {
    let threshold = basket.rebalancing.drift_threshold_bps;
    let max_drift = plan.drifts.iter().map(|d| d.drift_bps).max().unwrap_or(0);
    let status = if max_drift > threshold {
        format!("🔴 REBALANCE (max_drift={max_drift} bps > threshold={threshold} bps)")
    } else if max_drift > threshold / 2 {
        format!("🟡 watch  (max_drift={max_drift} bps, threshold={threshold} bps)")
    } else {
        format!("🟢 within (max_drift={max_drift} bps, threshold={threshold} bps)")
    };
    println!("  {:<5} {}", basket.symbol, status);

    if !plan.trades.is_empty() {
        for trade in &plan.trades {
            println!(
                "    → {:?} {:<14} {} base units  (drift {} bps)",
                trade.kind, trade.faucet_alias, trade.base_units, trade.drift_bps
            );
        }
        println!(
            "    [{}] would submit a Flow B trigger note to the controller",
            basket.symbol
        );
    }
}
