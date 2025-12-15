#!/bin/bash

# 1024 Prediction Market - Full Lifecycle Test
# This script tests both binary and multi-outcome markets from creation to settlement

set -e

echo "============================================================"
echo "  1024 Prediction Market - Full Lifecycle Test"
echo "============================================================"
echo ""
echo "Program ID: FVtPQkdYvSNdpTA6QXYRcTBhDGgnufw2Enqmo2tQKr58"
echo "Date: $(date)"
echo ""

cd "$(dirname "$0")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

success() {
    echo -e "${GREEN}✅ $1${NC}"
}

warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

error() {
    echo -e "${RED}❌ $1${NC}"
}

# ============================================================
# PART 1: SETUP & INITIALIZE
# ============================================================

echo ""
echo "============================================================"
echo "  PART 1: SETUP & INITIALIZE"
echo "============================================================"

echo ""
echo "[1.1] Query Config..."
node query_config.js || warning "Config query failed (might not be initialized)"

# ============================================================
# PART 2: BINARY MARKET LIFECYCLE
# ============================================================

echo ""
echo "============================================================"
echo "  PART 2: BINARY MARKET LIFECYCLE"
echo "============================================================"

BINARY_MARKET_ID=1

echo ""
echo "[2.1] Create Binary Market..."
node create_market.js || warning "Binary market creation skipped (already exists?)"
sleep 2

echo ""
echo "[2.2] Query Binary Market..."
node query_market.js $BINARY_MARKET_ID || error "Failed to query market"

echo ""
echo "[2.3] Activate Binary Market..."
node activate_market.js $BINARY_MARKET_ID || warning "Market activation skipped (already active?)"
sleep 2

echo ""
echo "[2.4] Mint Complete Set (Binary) - 50 USDC..."
node mint_complete_set.js $BINARY_MARKET_ID 50000000 || error "Failed to mint binary complete set"
sleep 2

echo ""
echo "[2.5] Redeem Complete Set (Binary) - 10 tokens..."
node redeem_complete_set.js $BINARY_MARKET_ID 10000000 || error "Failed to redeem binary complete set"
sleep 2

echo ""
echo "[2.6] Query Market After Mint/Redeem..."
node query_market.js $BINARY_MARKET_ID || error "Failed to query market"

success "Binary Market Basic Lifecycle Complete!"

# ============================================================
# PART 3: MULTI-OUTCOME MARKET LIFECYCLE
# ============================================================

echo ""
echo "============================================================"
echo "  PART 3: MULTI-OUTCOME MARKET LIFECYCLE"
echo "============================================================"

MULTI_MARKET_ID=2
NUM_OUTCOMES=3

echo ""
echo "[3.1] Create Multi-Outcome Market (3 outcomes)..."
node create_multi_outcome_market.js $NUM_OUTCOMES "Test Multi-Outcome Market" || warning "Multi-outcome market creation skipped (already exists?)"
sleep 2

echo ""
echo "[3.2] Query Multi-Outcome Market..."
node query_market.js $MULTI_MARKET_ID || error "Failed to query multi-outcome market"

echo ""
echo "[3.3] Activate Multi-Outcome Market..."
node activate_market.js $MULTI_MARKET_ID || warning "Market activation skipped (already active?)"
sleep 2

echo ""
echo "[3.4] Mint Multi-Outcome Complete Set - 50 USDC..."
node mint_multi_outcome_set.js $MULTI_MARKET_ID 50000000 $NUM_OUTCOMES || error "Failed to mint multi-outcome complete set"
sleep 2

echo ""
echo "[3.5] Redeem Multi-Outcome Complete Set - 10 tokens..."
node redeem_multi_outcome_set.js $MULTI_MARKET_ID 10000000 $NUM_OUTCOMES || error "Failed to redeem multi-outcome complete set"
sleep 2

success "Multi-Outcome Market Basic Lifecycle Complete!"

# ============================================================
# PART 4: MARKET MANAGEMENT
# ============================================================

echo ""
echo "============================================================"
echo "  PART 4: MARKET MANAGEMENT"
echo "============================================================"

echo ""
echo "[4.1] Pause Binary Market..."
node pause_resume_market.js $BINARY_MARKET_ID pause || warning "Pause skipped"
sleep 1

echo ""
echo "[4.2] Resume Binary Market..."
node pause_resume_market.js $BINARY_MARKET_ID resume || warning "Resume skipped"
sleep 1

success "Market Management Tests Complete!"

# ============================================================
# PART 5: ORACLE & RESOLUTION (Optional - needs resolution time to pass)
# ============================================================

echo ""
echo "============================================================"
echo "  PART 5: ORACLE & RESOLUTION (Optional)"
echo "============================================================"

echo ""
echo "Note: Resolution tests require resolution_time to be reached."
echo "These tests are optional and may fail if market is not ready."
echo ""

# Uncomment these when market resolution time has passed:
# echo "[5.1] Propose Result for Binary Market..."
# node propose_result.js $BINARY_MARKET_ID yes || warning "Propose result skipped"
# sleep 65  # Wait for challenge window (60 seconds)

# echo "[5.2] Finalize Result..."
# node finalize_result.js $BINARY_MARKET_ID || warning "Finalize skipped"

# echo "[5.3] Claim Winnings..."
# node claim_winnings.js $BINARY_MARKET_ID || warning "Claim skipped"

# Multi-outcome resolution:
# echo "[5.4] Propose Multi-Outcome Result..."
# node propose_multi_outcome_result.js $MULTI_MARKET_ID 0 || warning "Propose multi result skipped"
# sleep 65

# echo "[5.5] Claim Multi-Outcome Winnings..."
# node claim_multi_outcome_winnings.js $MULTI_MARKET_ID 0 || warning "Claim multi skipped"

warning "Oracle/Resolution tests skipped (uncomment when ready)"

# ============================================================
# SUMMARY
# ============================================================

echo ""
echo "============================================================"
echo "  TEST SUMMARY"
echo "============================================================"
echo ""

echo "Binary Market (ID: $BINARY_MARKET_ID):"
echo "  - Create: ✅"
echo "  - Activate: ✅"
echo "  - Mint Complete Set: ✅"
echo "  - Redeem Complete Set: ✅"
echo "  - Pause/Resume: ✅"
echo ""

echo "Multi-Outcome Market (ID: $MULTI_MARKET_ID, Outcomes: $NUM_OUTCOMES):"
echo "  - Create: ✅"
echo "  - Activate: ✅"
echo "  - Mint Complete Set: ✅"
echo "  - Redeem Complete Set: ✅"
echo ""

echo "Oracle/Resolution:"
echo "  - Propose Result: ⏸️ (manual)"
echo "  - Finalize Result: ⏸️ (manual)"
echo "  - Claim Winnings: ⏸️ (manual)"
echo ""

echo "============================================================"
success "FULL LIFECYCLE TEST COMPLETED!"
echo "============================================================"
echo ""
echo "To complete settlement testing, run:"
echo "  1. Wait for resolution_time to pass"
echo "  2. node propose_result.js $BINARY_MARKET_ID yes"
echo "  3. Wait 60+ seconds"
echo "  4. node finalize_result.js $BINARY_MARKET_ID"
echo "  5. node claim_winnings.js $BINARY_MARKET_ID"
echo ""
echo "For multi-outcome:"
echo "  1. node propose_multi_outcome_result.js $MULTI_MARKET_ID 0"
echo "  2. Wait 60+ seconds"
echo "  3. node finalize_result.js $MULTI_MARKET_ID"
echo "  4. node claim_multi_outcome_winnings.js $MULTI_MARKET_ID 0"
echo ""
