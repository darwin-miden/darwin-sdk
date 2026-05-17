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
//! Rebalancing engine: drift detection → trigger. Independent
//! of AggLayer.
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
    live: bool,
}

fn parse_args() -> Args {
    let mut out = Args {
        interval_s: 30,
        once: false,
        skew: 1.0,
        live: false,
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
            "--live" => out.live = true,
            "--help" | "-h" => {
                eprintln!(
                    "rebalance_bot --interval <s> --skew <f> --once [--live]\n  \
                     long-running drift-detection daemon for all M1 baskets.\n  \
                     --live: read prices from the live Pragma oracle on Miden testnet\n  \
                           (requires --features pragma-live at build time)."
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
        "rebalance_bot starting — interval={}s, skew={}, once={}, live={}",
        args.interval_s, args.skew, args.once, args.live
    );
    if args.live {
        #[cfg(not(feature = "pragma-live"))]
        {
            eprintln!(
                "ERROR: --live requires the bot to be built with --features pragma-live"
            );
            std::process::exit(2);
        }
    }
    println!();

    let baskets = all_m1();
    loop {
        let prices = if args.live {
            #[cfg(feature = "pragma-live")]
            { live_pragma_prices().await }
            #[cfg(not(feature = "pragma-live"))]
            { mock_oracle_prices() }
        } else {
            mock_oracle_prices()
        };
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

/// Mock prices used when `--live` is off (or when the binary is built
/// without the `pragma-live` feature). 8-decimal scale.
fn mock_oracle_prices() -> BTreeMap<&'static str, u64> {
    let mut m = BTreeMap::new();
    m.insert("darwin-eth", 200_000_000_000u64);    // $2000 * 1e8
    m.insert("darwin-wbtc", 6_000_000_000_000u64); // $60_000 * 1e8
    m.insert("darwin-usdt", 100_000_000u64);       // $1 * 1e8
    m.insert("darwin-dai", 100_000_000u64);        // $1 * 1e8
    m
}

#[cfg(feature = "pragma-live")]
async fn live_pragma_prices() -> BTreeMap<&'static str, u64> {
    use std::path::PathBuf;
    use std::sync::Arc;
    use darwin_oracle_adapter::pragma_live;
    use miden_client::account::AccountId;
    use miden_client::builder::ClientBuilder;
    use miden_client::keystore::FilesystemKeyStore;
    use miden_client::vm::AdviceInputs;
    use miden_client_sqlite_store::SqliteStore;

    let mut out = mock_oracle_prices(); // fallback for any pair that fails

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let store_path: PathBuf = format!("{home}/.miden/rebalance_bot_{ts}.sqlite3").into();
    let _ = std::fs::remove_file(&store_path);
    let keystore_path: PathBuf = format!("{home}/.miden/keystore").into();

    let store = match SqliteStore::new(store_path.clone()).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[live] store init failed, falling back to mock prices: {e}");
            return out;
        }
    };
    let mut client = match ClientBuilder::<FilesystemKeyStore>::new()
        .grpc_client(&miden_client::rpc::Endpoint::testnet(), None)
        .store(Arc::new(store))
        .filesystem_keystore(keystore_path)
        .and_then(|b| Ok(b))
    {
        Ok(b) => match b.build().await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[live] client build failed: {e}; falling back to mock");
                return out;
            }
        },
        Err(e) => {
            eprintln!("[live] keystore failed: {e}; falling back to mock");
            return out;
        }
    };

    let oracle_id = AccountId::from_hex(pragma_live::PRAGMA_TESTNET_ORACLE_HEX)
        .expect("oracle hex const");
    if client.sync_state().await.is_err()
        || client.import_account_by_id(oracle_id).await.is_err()
    {
        eprintln!("[live] sync/import failed; falling back to mock");
        let _ = std::fs::remove_file(&store_path);
        return out;
    }

    let publishers = match pragma_live::discover_publishers(&mut client, oracle_id).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[live] discover_publishers failed: {e}; falling back");
            let _ = std::fs::remove_file(&store_path);
            return out;
        }
    };

    let median_root = pragma_live::pragma_get_median_mast_root_hex();

    let pairs: &[(&str, &str)] = &[
        ("darwin-eth", "ETH/USD"),
        ("darwin-wbtc", "WBTC/USD"),
        ("darwin-usdt", "USDT/USD"),
        ("darwin-dai", "DAI/USD"),
    ];

    for (alias, pair) in pairs {
        let pair_word = match pragma_live::pair_word(pair) {
            Some(w) => w,
            None => continue,
        };
        let foreign = match pragma_live::build_foreign_accounts(
            &mut client,
            oracle_id,
            &publishers,
            pair_word,
        ).await {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[live] foreign accounts for {pair}: {e}; keeping mock");
                continue;
            }
        };
        let [_, _, suffix, prefix] = pair_word;
        let script_src = format!(
            "use miden::core::sys\n\nbegin\n  push.0 push.0 push.{suffix} push.{prefix}\n  call.{median_root}\n  exec.sys::truncate_stack\nend\n"
        );
        let tx_script = match client.code_builder().compile_tx_script(&script_src) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[live] compile tx script for {pair}: {e}");
                continue;
            }
        };
        let stack = match client
            .execute_program(oracle_id, tx_script, AdviceInputs::default(), foreign)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[live] execute_program for {pair}: {e}");
                continue;
            }
        };
        let found = stack[0].as_canonical_u64();
        let median = stack[1].as_canonical_u64();
        if found == 1 && median > 0 {
            out.insert(alias, median);
        }
    }

    let _ = std::fs::remove_file(&store_path);
    out
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
