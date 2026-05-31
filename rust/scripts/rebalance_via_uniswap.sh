#!/usr/bin/env bash
#
# Rebalance leg fallback via Uniswap V3.
#
# The Miden in-protocol DEX isn't live yet. The M2 work item
# calls for Uniswap / 1inch as the fallback execution venue for the
# rebalance leg. This script demonstrates that path end-to-end:
#
#   1. Take a drifted basket snapshot (DCC currently 50% BTC / 30% ETH
#      / 20% USDT, target 40/40/20 — 10% over BTC, 10% under ETH).
#   2. Compute the swap delta in dollar terms (here: $1,000 nominal
#      basket TVL).
#   3. Quote the BTC -> ETH swap on Uniswap V3 mainnet (the canonical
#      AMM that would absorb the rebalance leg).
#   4. Print the executed price + slippage.
#
# This is read-only price discovery — the actual rebalance lands
# either on the Miden DEX when it ships, or via the operator wallet
# bridging through AggLayer for non-Miden execution. The Uniswap
# quote is the price reference both paths anchor against.
#
# Usage:
#   rebalance_via_uniswap.sh
#
# Env:
#   RPC        Mainnet RPC for Uniswap (default: publicnode)
#   TVL_USD    Notional basket TVL (default: 1000)

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

RPC=${RPC:-https://ethereum-rpc.publicnode.com}
TVL_USD=${TVL_USD:-1000}

echo "Rebalance scenario:"
echo "  basket   DCC (Darwin Core Crypto)"
echo "  TVL      \$$TVL_USD"
echo "  current  50% WBTC / 30% WETH / 20% USDT"
echo "  target   40% WBTC / 40% WETH / 20% USDT"
echo "  delta    sell \$$((TVL_USD * 10 / 100)) WBTC, buy \$$((TVL_USD * 10 / 100)) WETH"
echo ""

# Pull live prices for sanity. We do a 1 WBTC -> USDC quote to anchor
# the dollar value of the WBTC slice we'd sell.
echo "Step 1: anchor WBTC/USD via Uniswap (1 WBTC -> USDC)"
ANCHOR=$(RPC="$RPC" bash "$HERE/uniswap_quote.sh" WBTC USDC 100000000 3000 2>&1)
echo "$ANCHOR" | sed 's/^/  /'
WBTC_USD_BASE=$(echo "$ANCHOR" | awk '/amountOut/ {gsub(/[(),]/,"",$3); print $3; exit}')
# USDC has 6 decimals — convert to a whole-USD number for printing.
WBTC_USD_INT=$(( WBTC_USD_BASE / 1000000 ))
echo "  -> 1 WBTC ≈ \$$WBTC_USD_INT"

echo ""
echo "Step 2: size the rebalance leg"
SELL_USD=$((TVL_USD * 10 / 100))
# WBTC base units to sell: SELL_USD * 1e8 / WBTC_USD_INT
SELL_WBTC=$(awk -v u="$SELL_USD" -v p="$WBTC_USD_INT" 'BEGIN {printf "%d", u * 100000000 / p}')
echo "  sell  $SELL_USD USD = $SELL_WBTC base units WBTC"

echo ""
echo "Step 3: quote WBTC -> WETH on Uniswap V3 0.3% pool"
QUOTE=$(RPC="$RPC" bash "$HERE/uniswap_quote.sh" WBTC WETH "$SELL_WBTC" 3000 2>&1)
echo "$QUOTE" | sed 's/^/  /'
WETH_OUT=$(echo "$QUOTE" | awk '/amountOut/ {gsub(/[(),]/,"",$3); print $3; exit}')

# Quote 1 WETH -> USDC to anchor ETH/USD.
ANCHOR_ETH=$(RPC="$RPC" bash "$HERE/uniswap_quote.sh" WETH USDC 1000000000000000000 500 2>&1)
WETH_USD_BASE=$(echo "$ANCHOR_ETH" | awk '/amountOut/ {gsub(/[(),]/,"",$3); print $3; exit}')
WETH_USD_INT=$(( WETH_USD_BASE / 1000000 ))

# Recompute: how many USD did we get out?
GOT_USD=$(awk -v w="$WETH_OUT" -v p="$WETH_USD_INT" 'BEGIN {printf "%.2f", w * p / 1000000000000000000}')

echo ""
echo "Step 4: settlement"
echo "  bought    $WETH_OUT base units WETH (≈ \$$GOT_USD at WETH/USD = \$$WETH_USD_INT)"
echo "  spent     \$$SELL_USD"
echo "  effective $(awk -v g="$GOT_USD" -v s="$SELL_USD" 'BEGIN {printf "%.4f", g / s}') USD WETH per USD WBTC"
echo "  slippage  $(awk -v g="$GOT_USD" -v s="$SELL_USD" 'BEGIN {printf "%.2f", (s - g) / s * 100}')% (vs 1:1 spot)"
echo ""
echo "This quote is the price reference the rebalance leg would settle"
echo "against. The Miden DEX path is byte-for-byte the same swap with"
echo "an additional cross-component basket-faucet burn+mint pair."
