#!/usr/bin/env bash
#
# Uniswap V3 QuoterV2 — pure read-only price discovery for the Darwin
# rebalance bot. When the Miden DEX is unavailable (M2 reality: it
# isn't live yet), the bot falls back to this script for the
# rebalance leg. The script issues a `quoteExactInputSingle` call
# against Uniswap V3 on mainnet (read-only, no tx, no signer) and
# prints the resulting amountOut + an implicit price.
#
# Uniswap V3 QuoterV2 mainnet: 0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6
# Pools used:
#   WBTC/WETH 0.3%   - the canonical BTC/ETH AMM
#   WETH/USDT 0.3%   - ETH side of stable rebalance
#   WETH/USDC 0.05%  - USDC lookup
#   WETH/DAI  0.3%   - DAI side
#
# Token addresses (mainnet):
#   WETH 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2
#   WBTC 0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599
#   USDC 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48
#   USDT 0xdAC17F958D2ee523a2206206994597C13D831ec7
#   DAI  0x6B175474E89094C44Da98b954EedeAC495271d0F
#
# Usage:
#   uniswap_quote.sh <token-in> <token-out> <amount-in> [fee-bps]
#
# Tokens may be specified by symbol (WBTC, WETH, USDC, USDT, DAI) or
# hex address. Amount is in token-in's base units (8 dp WBTC, 18 dp
# WETH/DAI, 6 dp USDC/USDT).
#
# Env:
#   RPC               Mainnet HTTP RPC (default: public).
#   QUOTER            Override the QuoterV2 address.

set -euo pipefail

RPC=${RPC:-https://ethereum-rpc.publicnode.com}
QUOTER=${QUOTER:-0x61fFE014bA17989E743c5F6cB21bF9697530B21e}

resolve() {
  local t=$1
  if [[ "$t" =~ ^0x[0-9a-fA-F]{40}$ ]]; then
    echo "$t"
    return
  fi
  local upper
  upper=$(echo "$t" | tr '[:lower:]' '[:upper:]')
  case "$upper" in
    WETH) echo "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2" ;;
    WBTC) echo "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599" ;;
    USDC) echo "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48" ;;
    USDT) echo "0xdAC17F958D2ee523a2206206994597C13D831ec7" ;;
    DAI)  echo "0x6B175474E89094C44Da98b954EedeAC495271d0F" ;;
    *)
      echo "unknown symbol: $t" >&2
      exit 1
      ;;
  esac
}

if [[ $# -lt 3 ]]; then
  cat <<USAGE
Usage: $0 <token-in> <token-out> <amount-in-base-units> [fee-bps]
       fee-bps default: 3000 (0.3%); use 500 for stable/major pairs
Examples:
  $0 WBTC WETH 100000000           # 1 WBTC -> ? WETH
  $0 WETH USDT 1000000000000000000 # 1 WETH -> ? USDT
USAGE
  exit 1
fi

TIN=$(resolve "$1")
TOUT=$(resolve "$2")
AMOUNT_IN=$3
FEE=${4:-3000}

# Uniswap V3 QuoterV2 ABI:
#   function quoteExactInputSingle(
#     (address tokenIn, address tokenOut, uint256 amountIn,
#      uint24 fee, uint160 sqrtPriceLimitX96)
#   ) external returns (
#     uint256 amountOut, uint160 sqrtPriceX96After,
#     uint32 initializedTicksCrossed, uint256 gasEstimate)
SIG="quoteExactInputSingle((address,address,uint256,uint24,uint160))(uint256,uint160,uint32,uint256)"

RESULT=$(cast call "$QUOTER" "$SIG" "($TIN, $TOUT, $AMOUNT_IN, $FEE, 0)" --rpc-url "$RPC" 2>&1)
AMOUNT_OUT=$(echo "$RESULT" | head -1 | awk '{print $1}')

echo "amountIn  $1 ($AMOUNT_IN base units, $TIN)"
echo "amountOut $2 ($AMOUNT_OUT base units, $TOUT)"
echo "fee tier  ${FEE}bps"
echo "via       Uniswap V3 QuoterV2 ($QUOTER) on $(cast chain-id --rpc-url "$RPC" 2>/dev/null || echo "?")"
