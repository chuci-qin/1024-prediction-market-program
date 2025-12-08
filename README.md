# 1024 Prediction Market Program

Solana program for prediction market trading on 1024 platform.

## Overview

This program enables:
- **Complete Set Minting/Redemption**: 1 USDC ↔ 1 YES + 1 NO
- **Order Book Trading**: Off-chain matching, on-chain settlement
- **Oracle Integration**: Result proposal, challenge, finalization
- **Market Lifecycle**: Create → Activate → Trade → Resolve → Settle

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                  1024-prediction-market-program              │
│                     (Core Business Logic)                    │
├─────────────────────────────────────────────────────────────┤
│  Market Management    │  Order Book      │  Settlement       │
│  - CreateMarket       │  - PlaceOrder    │  - ClaimWinnings  │
│  - ActivateMarket     │  - CancelOrder   │  - Refund         │
│  - CancelMarket       │  - MatchMint     │                   │
│                       │  - MatchBurn     │                   │
├─────────────────────────────────────────────────────────────┤
│  Complete Sets        │  Oracle           │                   │
│  - MintCompleteSet    │  - ProposeResult  │                   │
│  - RedeemCompleteSet  │  - Challenge      │                   │
│                       │  - Finalize       │                   │
└───────────────┬───────────────────────────┬─────────────────┘
                │                           │
           CPI  │                           │  CPI
                ▼                           ▼
    ┌─────────────────────┐     ┌─────────────────────┐
    │ 1024-vault-program  │     │  1024-fund-program   │
    │ (User Fund Custody) │     │ (Fee Pool Management)│
    └─────────────────────┘     └─────────────────────┘
```

## Account Structure

| Account | Seeds | Description |
|---------|-------|-------------|
| `PredictionMarketConfig` | `["pm_config"]` | Global configuration |
| `Market` | `["market", market_id]` | Single prediction market |
| `Order` | `["order", market_id, order_id]` | User order |
| `Position` | `["position", market_id, owner]` | User position in market |
| `OracleProposal` | `["oracle_proposal", market_id]` | Result proposal |

## Key Instructions

### Market Management
- `CreateMarket` - Create new prediction market
- `ActivateMarket` - Activate market for trading
- `CancelMarket` - Cancel market and enable refunds

### Trading
- `MintCompleteSet` - Deposit USDC, receive YES + NO tokens
- `RedeemCompleteSet` - Return YES + NO tokens, receive USDC
- `PlaceOrder` - Submit buy/sell order
- `CancelOrder` - Cancel pending order
- `MatchMint` - Match YES buy + NO buy (mint new tokens)
- `MatchBurn` - Match YES sell + NO sell (burn tokens)

### Resolution
- `ProposeResult` - Submit oracle result proposal
- `ChallengeResult` - Challenge a proposal
- `FinalizeResult` - Finalize result after challenge window

### Settlement
- `ClaimWinnings` - Claim USDC from winning tokens
- `RefundCancelledMarket` - Refund from cancelled market

## Build

```bash
cargo build-sbf
```

## Test

```bash
cargo test
```

## Deploy

```bash
solana program deploy target/deploy/prediction_market_program.so
```

## Integration with Other Programs

### Vault Program (1024-vault-program)

New instructions to support prediction markets:
- `LockForPrediction` - Lock USDC for market participation
- `ReleaseFromPrediction` - Release locked USDC
- `PredictionSettle` - Settle winnings

### Fund Program (1024-fund-program)

New account and instructions:
- `PredictionMarketFeeConfig` - Fee configuration
- `CollectPMFee` - Collect minting/trading fees
- `DistributeMakerReward` - Distribute maker incentives

## License

MIT

