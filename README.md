# darwin-sdk

Client SDK for Darwin Protocol — Rust core + TypeScript bindings.

The Rust crate wraps `miden-client` v0.14 and (once enabled) `miden-agglayer` v0.14-alpha. The TypeScript package wraps the Miden Web SDK so the M3 frontend at `darwin.xyz` can mint, redeem, and bridge basket tokens directly from the browser with client-side STARK proving.

## Layout

```
darwin-sdk/
├── rust/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs        # public Rust API, BasketHandle resolution
│       ├── deposit.rs    # Flow A helpers
│       ├── redeem.rs     # Flow C (Miden side) helpers
│       ├── rebalance.rs  # off-chain drift / trade planner (M2 prep)
│       └── bin/
│           └── rebalance_demo.rs   # CLI: print a plan for any basket
└── ts/
    ├── package.json
    ├── tsconfig.json
    └── src/
        └── index.ts      # public TS API skeleton
```

## Rust

```bash
cd rust
cargo build
cargo test
```

`BasketHandle::from_symbol("DCC")` resolves the manifest for the Core Crypto basket; `"DAG"` and `"DCO"` resolve the Aggressive and Conservative baskets.

### Rebalance demo

The `rebalance_demo` binary prints the off-chain drift planner's output for a synthetic snapshot.

```bash
cargo run --bin rebalance_demo                   # all 3 baskets, at-par
cargo run --bin rebalance_demo -- DCC --skew 2.0 # DCC, first constituent doubled
```

Output for the perturbed case:

```
==== Core Crypto (DCC) ====
  drift threshold: 500 bps   (3 constituents)
  total pool value (x1e8): 14000
  per-constituent drift:
    darwin-wbtc    target=4000 bps  current=5714 bps  drift=1714 bps
    darwin-eth     target=4000 bps  current=2857 bps  drift=1143 bps
    darwin-usdt    target=2000 bps  current=1428 bps  drift= 572 bps
  rebalance trades:
    Sell darwin-wbtc    2399 base units  (drift 1714 bps)
    Buy darwin-eth     1600 base units  (drift 1143 bps)
    Buy darwin-usdt    800 base units  (drift 572 bps)
```

The same formula runs in MASM (`darwin-protocol/asm/lib/drift.masm`) and TypeScript (`darwin-frontend/src/lib/rebalance.ts`); the three implementations stay algorithmically identical so the on-chain trigger and the off-chain dashboards never disagree.

## TypeScript

```bash
cd ts
npm install
npm test           # vitest: 18 tests across deposit/redeem/rebalance
npm run build      # tsc → dist/
```

The TS SDK now mirrors the Rust crate one-for-one: `BasketHandle`, `DepositRequest` + `validateDepositRequest`, `RedeemRequest` + `validateRedeemRequest`, and the off-chain `planRebalance` planner. Same validation rules, same algorithms, same error shapes. Wire-up with `@miden-sdk/miden-sdk` (the actual Web SDK calls) is part of `darwin-frontend` integration work; once wasm-bindgen ships, the TS layer can re-export the Rust crate directly.

## License

MIT.
