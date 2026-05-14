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
│       └── redeem.rs     # Flow C (Miden side) helpers
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

## TypeScript

```bash
cd ts
npm install
npm run lint    # type-check only
```

The TS API is intentionally a thin skeleton. Wire-up with `@miden-sdk/miden-sdk` is part of `darwin-frontend` integration work.

## License

MIT.
