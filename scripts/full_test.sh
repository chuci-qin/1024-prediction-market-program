#!/bin/bash
# Full Test Suite for 1024 Prediction Market
# Run on server: bash full_test.sh

echo "============================================================"
echo "1024 Prediction Market - Full Test Suite"
echo "============================================================"
echo ""

# Test 1: Query existing markets
echo ">>> [1/10] Query Market 1"
node query_market.js 1
echo ""

# Test 2: Query Market 2
echo ">>> [2/10] Query Market 2"
node query_market.js 2
echo ""

# Test 3: Query existing orders
echo ">>> [3/10] Query Order 1 (Market 1)"
node query_order.js 1 1
echo ""

# Test 4: Redeem Complete Set (burn 10 YES + 10 NO -> 10 USDC)
echo ">>> [4/10] Redeem Complete Set (10 tokens)"
node redeem_complete_set.js 1 10000000
echo ""

# Test 5: Place a new Sell Order
echo ">>> [5/10] Place Sell Order: Sell 20 YES at $0.65"
node place_order.js 1 sell yes 0.65 20000000
echo ""

# Test 6: Query the new order
echo ">>> [6/10] Query New Sell Order (Order 3)"
node query_order.js 1 3
echo ""

# Test 7: Place matching Buy Order
echo ">>> [7/10] Place Buy Order: Buy 10 YES at $0.65"
node place_order.js 1 buy yes 0.65 10000000
echo ""

# Test 8: Execute Trade between orders
echo ">>> [8/10] Execute Trade: Order 4 (Buy) vs Order 3 (Sell)"
node execute_trade.js 1 4 3 10000000 0.65
echo ""

# Test 9: Check final balances
echo ">>> [9/10] Query Market 1 Final Status"
node query_market.js 1
echo ""

# Test 10: Show all orders
echo ">>> [10/10] Query All Orders"
node query_order.js 1 1
node query_order.js 1 2
node query_order.js 1 3
node query_order.js 1 4
echo ""

echo "============================================================"
echo "Full Test Suite Complete!"
echo "============================================================"
