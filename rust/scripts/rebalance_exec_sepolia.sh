#!/usr/bin/env bash
#
# Real swap execution for Flow B (the rebalance leg), against the
# Uniswap V3 SwapRouter02 deployment on Sepolia.
#
# An earlier audit flagged Flow B's swap exec as 🟡
# ("read-only quotes via Uniswap + Paraswap, execution swap réelle
# pas tentée"). This script closes the gap: it actually signs and
# submits the WETH→USDC leg, captures the tx hash, and prints
# before/after balances so the result is verifiable on
# sepolia.etherscan.io.
#
# The amounts are intentionally small — this is a proof-of-execution,
# not a real rebalance. The same calldata shape is what the
# rebalance bot would emit; the only difference is the trigger (a
# DriftDetected event on the controller vs. a manual run here).
#
# Pre-reqs:
#   - cast (foundry)
#   - the dev key has Sepolia ETH (we use ~0.005 ETH worth of
#     wrap + swap + gas)
#
# Env:
#   USER_PK    dev key (default: project DEV-ONLY key per memory)
#   RPC        Sepolia RPC (default: publicnode)
#   AMOUNT_ETH amount of ETH to swap, as a decimal (default: 0.001)

set -euo pipefail

RPC=${RPC:-https://ethereum-sepolia-rpc.publicnode.com}

# Operator key sourced from $HOME/.darwin-env (gitignored) rather than
# defaulted inline — see darwin-infra/.env.example for the template.
# Testnet burner only.
if [[ -f "$HOME/.darwin-env" ]]; then
    set -a; source "$HOME/.darwin-env"; set +a
fi
: "${USER_PK:?USER_PK must be set (export it or put it in \$HOME/.darwin-env)}"
AMOUNT_ETH=${AMOUNT_ETH:-0.001}

# Sepolia Uniswap V3 canonical deployment, verified live 2026-05-27.
SWAP_ROUTER02="0x3bFA4769FB09eefC5a80d6E87c3B9C650f7Ae48E"
WETH9="0x7b79995e5f793A07Bc00c21412e50Ecae098E7f9"
USDC="0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238"
FEE_TIER=500  # WETH/USDC 0.05% pool — best liquidity on Sepolia
USER=$(cast wallet address --private-key "$USER_PK")

echo "============================================================"
echo "  Flow B rebalance leg — live exec on Sepolia"
echo "  user:    $USER"
echo "  amount:  $AMOUNT_ETH ETH → WETH → USDC (fee=$FEE_TIER)"
echo "============================================================"

AMOUNT_WEI=$(cast --to-wei "$AMOUNT_ETH" ether)
echo

echo "--- before ---"
ETH_BEFORE=$(cast balance "$USER" --rpc-url "$RPC")
WETH_BEFORE=$(cast call "$WETH9" "balanceOf(address)(uint256)" "$USER" --rpc-url "$RPC" | awk '{print $1}')
USDC_BEFORE=$(cast call "$USDC" "balanceOf(address)(uint256)" "$USER" --rpc-url "$RPC" | awk '{print $1}')
printf "  ETH   %s wei\n" "$ETH_BEFORE"
printf "  WETH  %s wei\n" "$WETH_BEFORE"
printf "  USDC  %s base-units (6 dec)\n" "$USDC_BEFORE"
echo

echo "--- step 1: wrap ETH → WETH via WETH9.deposit() ---"
WRAP_OUT=$(cast send "$WETH9" "deposit()" --value "$AMOUNT_WEI" \
  --rpc-url "$RPC" --private-key "$USER_PK" --json)
WRAP_TX=$(echo "$WRAP_OUT" | jq -r .transactionHash)
echo "  tx: $WRAP_TX"
echo

echo "--- step 2: approve SwapRouter02 to spend WETH ---"
APPROVE_OUT=$(cast send "$WETH9" "approve(address,uint256)" "$SWAP_ROUTER02" "$AMOUNT_WEI" \
  --rpc-url "$RPC" --private-key "$USER_PK" --json)
APPROVE_TX=$(echo "$APPROVE_OUT" | jq -r .transactionHash)
echo "  tx: $APPROVE_TX"
echo

echo "--- step 3: exactInputSingle(WETH → USDC, fee=$FEE_TIER) ---"
# struct ExactInputSingleParams {
#   address tokenIn;
#   address tokenOut;
#   uint24 fee;
#   address recipient;
#   uint256 amountIn;
#   uint256 amountOutMinimum;
#   uint160 sqrtPriceLimitX96;
# }
SIG='exactInputSingle((address,address,uint24,address,uint256,uint256,uint160))(uint256)'
# amountOutMinimum=0 — testnet, accept any rate. sqrtPriceLimitX96=0 — no price limit.
PARAMS="(${WETH9},${USDC},${FEE_TIER},${USER},${AMOUNT_WEI},0,0)"
SWAP_OUT=$(cast send "$SWAP_ROUTER02" "$SIG" "$PARAMS" \
  --rpc-url "$RPC" --private-key "$USER_PK" --json)
SWAP_TX=$(echo "$SWAP_OUT" | jq -r .transactionHash)
echo "  tx: $SWAP_TX"
echo

echo "--- after ---"
ETH_AFTER=$(cast balance "$USER" --rpc-url "$RPC")
WETH_AFTER=$(cast call "$WETH9" "balanceOf(address)(uint256)" "$USER" --rpc-url "$RPC" | awk '{print $1}')
USDC_AFTER=$(cast call "$USDC" "balanceOf(address)(uint256)" "$USER" --rpc-url "$RPC" | awk '{print $1}')
USDC_DELTA=$(( USDC_AFTER - USDC_BEFORE ))
printf "  ETH   %s wei\n" "$ETH_AFTER"
printf "  WETH  %s wei\n" "$WETH_AFTER"
printf "  USDC  %s base-units (delta +%s = %.6f USDC)\n" "$USDC_AFTER" "$USDC_DELTA" \
  "$(awk -v d="$USDC_DELTA" 'BEGIN{printf "%.6f", d/1e6}')"
echo
echo "============================================================"
echo "  ✓ Flow B swap leg executed live."
echo "  Verify on https://sepolia.etherscan.io/tx/$SWAP_TX"
echo "============================================================"
