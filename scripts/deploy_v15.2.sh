#!/bin/bash
# Deploy Prediction Market Program V15.2 with RelayerChallengeResultV2
#
# This script:
# 1. Deploys the new contract to a new address
# 2. Initializes the contract state
# 3. Updates all config files with the new address
# 4. Recompiles 1024-core
#
# Usage: ./deploy_v15.2.sh [--dry-run]

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Configuration
OLD_PM_ADDR="FVtPQkdYvSNdpTA6QXYRcTBhDGgnufw2Enqmo2tQKr58"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
PM_PROGRAM_DIR="$PROJECT_ROOT/1024-prediction-market-program"
CORE_DIR="$PROJECT_ROOT/1024-core"

DRY_RUN=false
if [ "$1" == "--dry-run" ]; then
    DRY_RUN=true
    echo -e "${YELLOW}🔍 DRY RUN MODE - No actual changes will be made${NC}"
fi

echo "=========================================="
echo "Prediction Market Program V15.2 Deployment"
echo "=========================================="
echo ""
echo "Features:"
echo "  - RelayerChallengeResultV2 instruction (Index 72)"
echo "  - Unified config files for scripts"
echo ""

# Step 1: Deploy to new address
echo -e "${YELLOW}Step 1: Deploy contract to new address${NC}"
if [ "$DRY_RUN" == "false" ]; then
    echo "Generating new keypair..."
    NEW_KEYPAIR_PATH="$PM_PROGRAM_DIR/target/deploy/pm_v15.2_keypair.json"
    solana-keygen new --no-bip39-passphrase -o "$NEW_KEYPAIR_PATH" --force
    
    NEW_PM_ADDR=$(solana-keygen pubkey "$NEW_KEYPAIR_PATH")
    echo -e "${GREEN}New Program ID: $NEW_PM_ADDR${NC}"
    
    echo "Deploying program..."
    solana program deploy \
        --program-id "$NEW_KEYPAIR_PATH" \
        "$PM_PROGRAM_DIR/target/deploy/prediction_market_program.so"
    
    echo -e "${GREEN}✅ Program deployed successfully!${NC}"
else
    NEW_PM_ADDR="NEW_PROGRAM_ID_PLACEHOLDER"
    echo "  Would generate new keypair and deploy"
fi

# Step 2: Initialize contract (requires admin keypair)
echo ""
echo -e "${YELLOW}Step 2: Initialize contract state${NC}"
if [ "$DRY_RUN" == "false" ]; then
    echo "Run: node $SCRIPT_DIR/init_program.js"
    echo "  (Make sure to update PROGRAM_ID in config.js first)"
else
    echo "  Would initialize contract with admin keypair"
fi

# Step 3: Update all config files
echo ""
echo -e "${YELLOW}Step 3: Update contract addresses in all files${NC}"

update_address() {
    local file="$1"
    if [ -f "$file" ]; then
        if [ "$DRY_RUN" == "false" ]; then
            sed -i '' "s/$OLD_PM_ADDR/$NEW_PM_ADDR/g" "$file"
            echo "  Updated: $file"
        else
            echo "  Would update: $file"
        fi
    fi
}

# Prediction Market Program scripts
update_address "$PM_PROGRAM_DIR/scripts/config.js"

# Fund Program scripts  
update_address "$PROJECT_ROOT/1024-fund-program/scripts/config.js"

# 1024-core sources (hardcoded defaults)
find "$CORE_DIR" -name "*.rs" -type f -exec grep -l "$OLD_PM_ADDR" {} \; 2>/dev/null | while read f; do
    update_address "$f"
done

# Frontend
update_address "$PROJECT_ROOT/1024-chain-frontend/src/lib/solana/prediction-market.ts"

# Documentation
update_address "$PROJECT_ROOT/当前配置信息.md"

echo ""
echo -e "${YELLOW}Step 4: Recompile 1024-core${NC}"
if [ "$DRY_RUN" == "false" ]; then
    cd "$CORE_DIR"
    cargo build --release -p node -p public-api -p relayer
    echo -e "${GREEN}✅ 1024-core compiled successfully!${NC}"
else
    echo "  Would run: cargo build --release -p node -p public-api -p relayer"
fi

echo ""
echo "=========================================="
echo -e "${GREEN}Deployment Complete!${NC}"
echo "=========================================="
echo ""
echo "Old Program ID: $OLD_PM_ADDR"
echo "New Program ID: $NEW_PM_ADDR"
echo ""
echo "Next steps:"
echo "1. Restart all services: ./start-all.sh"
echo "2. Run API tests to verify"
echo "3. Update 当前配置信息.md with version notes"








