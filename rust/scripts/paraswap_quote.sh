#!/usr/bin/env bash
#
# Paraswap V5 quote -- cross-chain trading SDK fallback for the
# Darwin rebalance leg. Where `uniswap_quote.sh` reads a single AMM
# (Uniswap V3 QuoterV2), Paraswap aggregates across ~30 venues
# (Uniswap V3, Curve, Balancer, BancorV3, KyberSwap, etc.) and
# returns the best executable route.
#
# Read-only via apiv5.paraswap.io -- no API key required for quotes,
# no signer needed; the result is a price reference the rebalance
# bot anchors against.
#
# Usage:
#   paraswap_quote.sh <token-in> <token-out> <amount-in-base-units>
#
# Tokens by symbol (WETH/WBTC/USDC/USDT/DAI) or hex.
#
# Env:
#   PARASWAP_URL    Base URL (default: https://apiv5.paraswap.io)
#   NETWORK         Chain id (default: 1 = mainnet)

set -euo pipefail

PARASWAP_URL=${PARASWAP_URL:-https://apiv5.paraswap.io}
NETWORK=${NETWORK:-1}

decimals_for() {
  case "$(echo "$1" | tr '[:lower:]' '[:upper:]')" in
    WETH) echo 18 ;;
    WBTC) echo 8 ;;
    USDC) echo 6 ;;
    USDT) echo 6 ;;
    DAI)  echo 18 ;;
    *)    echo 18 ;;
  esac
}

resolve() {
  local t=$1
  if [[ "$t" =~ ^0x[0-9a-fA-F]{40}$ ]]; then
    echo "$t"; return
  fi
  case "$(echo "$t" | tr '[:lower:]' '[:upper:]')" in
    WETH) echo "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2" ;;
    WBTC) echo "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599" ;;
    USDC) echo "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48" ;;
    USDT) echo "0xdAC17F958D2ee523a2206206994597C13D831ec7" ;;
    DAI)  echo "0x6B175474E89094C44Da98b954EedeAC495271d0F" ;;
    *) echo "unknown symbol: $t" >&2; exit 1 ;;
  esac
}

if [[ $# -lt 3 ]]; then
  cat <<USAGE
Usage: $0 <token-in> <token-out> <amount-in-base-units>

Examples:
  $0 WBTC WETH 100000000           # 1 WBTC -> ? WETH
  $0 WETH USDT 1000000000000000000 # 1 WETH -> ? USDT
USAGE
  exit 1
fi

TIN=$(resolve "$1")
TOUT=$(resolve "$2")
AMOUNT=$3
SRC_DEC=$(decimals_for "$1")
DST_DEC=$(decimals_for "$2")

URL="$PARASWAP_URL/prices?srcToken=$TIN&destToken=$TOUT&amount=$AMOUNT&srcDecimals=$SRC_DEC&destDecimals=$DST_DEC&side=SELL&network=$NETWORK"

RESP=$(curl -sS "$URL")
if echo "$RESP" | grep -q '"error"'; then
  echo "paraswap error: $RESP" >&2
  exit 2
fi

AMOUNT_OUT=$(echo "$RESP" | python3 -c 'import sys, json; r = json.load(sys.stdin); print(r["priceRoute"]["destAmount"])')
SRC_USD=$(echo "$RESP" | python3 -c 'import sys, json; r = json.load(sys.stdin); print(r["priceRoute"].get("srcUSD","-"))')
DST_USD=$(echo "$RESP" | python3 -c 'import sys, json; r = json.load(sys.stdin); print(r["priceRoute"].get("destUSD","-"))')
GAS_USD=$(echo "$RESP" | python3 -c 'import sys, json; r = json.load(sys.stdin); print(r["priceRoute"].get("gasCostUSD","-"))')
VENUE=$(echo "$RESP" | python3 -c 'import sys, json; r = json.load(sys.stdin); ex = r["priceRoute"]["bestRoute"][0]["swaps"][0]["swapExchanges"][0]["exchange"]; print(ex)')

echo "amountIn  $1 ($AMOUNT base units, $TIN)"
echo "amountOut $2 ($AMOUNT_OUT base units, $TOUT)"
echo "srcUSD    \$$SRC_USD"
echo "dstUSD    \$$DST_USD"
echo "gasUSD    \$$GAS_USD"
echo "venue     $VENUE (best of ~30 via Paraswap V5 aggregator)"
echo "via       $PARASWAP_URL"
